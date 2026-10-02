use alloc::vec;
use alloc::vec::Vec;

/// Compute floor(log2(x)) for x >= 1. Returns 0 for x <= 1.
fn floor_log2(x: usize) -> u32 {
    if x <= 1 {
        return 0;
    }
    (usize::BITS - 1) - x.leading_zeros()
}

/// Compute ceil(log2(x)) for x >= 1. Returns 0 for x <= 1.
fn ceil_log2(x: usize) -> u32 {
    let fl = floor_log2(x);
    if x > (1usize << fl) { fl + 1 } else { fl }
}

/// Compute the iterated logarithm (log* n).
fn log_star(mut n: usize) -> u32 {
    let mut count = 0u32;
    while n > 1 {
        n = floor_log2(n) as usize;
        count += 1;
    }
    count
}

/// Compute the microset size parameter `b` given `n` (number of nodes).
///
/// Returns `(b, ceil_lg_b)` where:
/// - `b`: maximum microset size is `b - 1` nodes
/// - `ceil_lg_b`: `ceil(log2(b))`, bits per answer entry
///
/// Follows the janezb adaptive approach: start with a safe minimum,
/// then grow `b` while the answer table stays within O(n) and answers
/// pack into a single `u64`.
pub(crate) fn compute_b(n: usize) -> (u32, u32) {
    if n <= 2 {
        return (2, 1);
    }

    // Starting point: max(2, min(floor(log log n), (log* n)^2))
    let log_log_n = {
        let lg = floor_log2(n);
        if lg > 1 { floor_log2(lg as usize) } else { 0 }
    };
    let ls = log_star(n);
    let start = log_log_n.min(ls.saturating_mul(ls));
    let mut b = 2u32.max(start);

    // Grow b while constraints are satisfied
    loop {
        let bb = b + 1;
        // Constraint 1: answer table size 2^(3*bb - 4) <= n
        // Equivalently: n >> (3*bb - 4) >= 1
        let shift = 3 * bb - 4;
        if shift >= 64 || (n >> shift as usize) < (bb - 1) as usize {
            break;
        }
        // Constraint 2: (bb - 1) * ceil_log2(bb) <= 63 (fits in u64)
        let lg_bb = ceil_log2(bb as usize);
        if (bb - 1) * lg_bb > 63 {
            break;
        }
        b = bb;
    }

    let clb = ceil_log2(b as usize);
    (b, clb)
}

/// Precomputed answer table for O(1) microfind queries.
///
/// Indexed by `(forest << (b-1)) | mark`. Each entry packs `b-1`
/// answer values, each `ceil_lg_b` bits wide.
///
/// Answer value 0 means "no marked ancestor in this microset" (go to root).
/// Answer value `k` (1-indexed) means "nearest marked ancestor is
/// node `k-1` (0-indexed) in the microset".
///
/// Mark convention: bit = 0 means node IS a set name (marked);
/// bit = 1 means node has been linked (unmarked). Initially all bits are 0.
#[derive(Clone, Debug)]
pub(crate) struct AnswerTable {
    table: Vec<u64>,
    pub b: u32,
    pub ceil_lg_b: u32,
}

impl AnswerTable {
    /// Build the answer table for the given capacity.
    pub fn new(capacity: usize) -> Self {
        let (b, ceil_lg_b) = compute_b(capacity);
        let max_nodes = (b - 1) as usize; // max nodes per microset
        let table_size = 1usize << (3 * b - 4).min(63);
        let mut table = vec![u64::MAX; table_size]; // MAX = sentinel for invalid

        // For each possible forest bitstring, decode and compute answers
        let max_forest = 1u32 << (2 * max_nodes).min(31);
        let max_mark = 1u32 << max_nodes.min(31);

        // Temporary buffers
        let mut parent = [0i32; 16]; // parent[i] = parent index, -1 if root
        let mut child_count = [0u32; 16];

        for forest in 0..max_forest {
            // Decode forest bitstring to parent array
            let node_count = Self::decode_forest(forest, &mut parent, &mut child_count);
            if node_count == 0 || node_count > max_nodes {
                continue;
            }

            // For each mark bitmask
            for mark in 0..max_mark {
                // Only consider valid mark bitmasks (no bits set beyond node_count)
                if mark >> node_count != 0 {
                    continue;
                }

                // Compute answer for each node
                let mut packed: u64 = 0;
                for j in 0..node_count {
                    // Walk up to find nearest marked ancestor (bit = 0 means marked)
                    let answer = Self::compute_answer(j, mark, &parent, node_count);
                    packed |= (answer as u64) << (j as u32 * ceil_lg_b);
                }

                let index = ((forest as usize) << max_nodes) | (mark as usize);
                if index < table_size {
                    table[index] = packed;
                }
            }
        }

        Self {
            table,
            b,
            ceil_lg_b,
        }
    }

    /// Decode a forest bitstring into a parent array.
    /// Returns the number of nodes decoded.
    ///
    /// Forest encoding (janezb style): read bits LSB to MSB.
    /// For each node: count trailing zeros = number of children,
    /// then consume the '1' bit.
    fn decode_forest(
        mut forest: u32,
        parent: &mut [i32; 16],
        child_count: &mut [u32; 16],
    ) -> usize {
        if forest == 0 {
            return 0;
        }

        let mut node_count = 0usize;
        let mut branch: [usize; 16] = [0; 16];
        let mut branch_len = 0usize;
        let mut remaining: [u32; 16] = [0; 16];

        while forest > 0 {
            if node_count >= 16 {
                return 0; // overflow
            }

            // Count trailing zeros = number of children for this node
            let num_children = forest.trailing_zeros();
            // Consume the zeros and the '1' bit
            forest >>= num_children + 1;

            child_count[node_count] = num_children;

            // Pop completed nodes from the branch stack
            while branch_len > 0 && remaining[branch_len - 1] == 0 {
                branch_len -= 1;
            }

            // Set parent
            if branch_len == 0 {
                parent[node_count] = -1; // root of this subtree (parent is external)
            } else {
                let p = branch[branch_len - 1];
                parent[node_count] = p as i32;
                remaining[branch_len - 1] -= 1;
            }

            // Push this node onto the branch stack
            branch[branch_len] = node_count;
            remaining[branch_len] = num_children;
            branch_len += 1;

            node_count += 1;
        }

        // Validate: all remaining child counts should be 0
        for item in remaining.iter().take(branch_len) {
            if *item != 0 {
                return 0; // invalid forest
            }
        }

        node_count
    }

    /// Compute the answer for node `j` given a mark bitmask and parent array.
    /// Mark bit 0 = marked (set name), bit 1 = linked (not a set name).
    /// Returns 0 if no marked ancestor in microset, or k (1-indexed) if
    /// node k-1 is the nearest marked ancestor.
    fn compute_answer(j: usize, mark: u32, parent: &[i32; 16], _node_count: usize) -> u32 {
        let mut cur = j as i32;
        while cur >= 0 {
            // Check if current node is marked (bit = 0)
            if (mark >> cur as u32) & 1 == 0 {
                return (cur + 1) as u32; // 1-indexed
            }
            cur = parent[cur as usize];
        }
        0 // no marked ancestor in microset
    }

    /// Look up the packed answers for a given forest and mark.
    #[inline]
    pub fn lookup(&self, forest: u32, mark: u32) -> u64 {
        let index = ((forest as usize) << (self.b - 1)) | (mark as usize);
        debug_assert!(index < self.table.len());
        self.table[index]
    }

    /// Extract the answer for node `j` from a packed answer entry.
    #[inline]
    pub fn extract(&self, packed: u64, j: u16) -> u32 {
        let shift = j as u32 * self.ceil_lg_b;
        let mask = (1u64 << self.ceil_lg_b) - 1;
        ((packed >> shift) & mask) as u32
    }
}

/// Build a forest encoding from a child-count array (janezb formula).
///
/// `child_counts[i]` = number of children of node `i` within the microset.
/// Nodes are numbered in preorder. Returns the packed forest integer.
#[inline]
pub(crate) fn encode_forest(child_counts: &[u32], count: usize) -> u32 {
    let mut forest: u32 = 0;
    for i in (0..count).rev() {
        forest = ((forest << 1) | 1) << child_counts[i];
    }
    forest
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_b() {
        // Small n
        let (b, clb) = compute_b(2);
        assert!(b >= 2);
        assert!(clb >= 1);

        // Medium n
        let (b, _) = compute_b(1000);
        assert!((2..=10).contains(&b));

        // Large n
        let (b, _) = compute_b(1_000_000);
        assert!((4..=12).contains(&b));
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        // Simple path: 3 nodes, each with 0 or 1 child
        // node 0 has 1 child, node 1 has 1 child, node 2 has 0 children
        let child_counts = [1, 1, 0];
        let forest = encode_forest(&child_counts, 3);

        let mut parent = [0i32; 16];
        let mut cc = [0u32; 16];
        let count = AnswerTable::decode_forest(forest, &mut parent, &mut cc);
        assert_eq!(count, 3);
        assert_eq!(parent[0], -1); // root
        assert_eq!(parent[1], 0); // child of 0
        assert_eq!(parent[2], 1); // child of 1
    }

    #[test]
    fn test_encode_decode_star() {
        // Star: node 0 has 2 children (nodes 1 and 2)
        let child_counts = [2, 0, 0];
        let forest = encode_forest(&child_counts, 3);

        let mut parent = [0i32; 16];
        let mut cc = [0u32; 16];
        let count = AnswerTable::decode_forest(forest, &mut parent, &mut cc);
        assert_eq!(count, 3);
        assert_eq!(parent[0], -1);
        assert_eq!(parent[1], 0);
        assert_eq!(parent[2], 0);
    }

    #[test]
    fn test_answer_table_simple() {
        // Need b >= 4 so max_nodes >= 3. b=4 requires n >= 768.
        let at = AnswerTable::new(1000);
        assert!(at.b >= 4, "b={} too small for 3-node microsets", at.b);

        // Path microset: node 0 -> node 1 -> node 2
        let child_counts = [1, 1, 0];
        let forest = encode_forest(&child_counts, 3);

        // All nodes marked (mark = 0b000): find(j) = j+1 for all j
        let packed = at.lookup(forest, 0b000);
        assert_eq!(at.extract(packed, 0), 1); // find(0) = 0 (1-indexed = 1)
        assert_eq!(at.extract(packed, 1), 2); // find(1) = 1 (1-indexed = 2)
        assert_eq!(at.extract(packed, 2), 3); // find(2) = 2 (1-indexed = 3)

        // Node 0 linked (mark = 0b001): find(0) -> go to parent (-1) -> return 0 (no ancestor)
        let packed = at.lookup(forest, 0b001);
        assert_eq!(at.extract(packed, 0), 0); // find(0) = no ancestor in microset
        assert_eq!(at.extract(packed, 1), 2); // find(1) = 1 (still marked)
        assert_eq!(at.extract(packed, 2), 3); // find(2) = 2 (still marked)

        // Nodes 1 and 2 linked (mark = 0b110): find(2) -> 1(linked) -> 0(marked) = 1
        let packed = at.lookup(forest, 0b110);
        assert_eq!(at.extract(packed, 0), 1); // find(0) = 0 (marked)
        assert_eq!(at.extract(packed, 1), 1); // find(1) -> 0 (marked), return 1
        assert_eq!(at.extract(packed, 2), 1); // find(2) -> 1(linked) -> 0(marked), return 1
    }

    #[test]
    fn test_answer_table_siblings() {
        let at = AnswerTable::new(1000);
        assert!(at.b >= 4, "b={} too small for 3-node microsets", at.b);

        // Star: node 0 has children 1 and 2
        let child_counts = [2, 0, 0];
        let forest = encode_forest(&child_counts, 3);

        // All marked
        let packed = at.lookup(forest, 0b000);
        assert_eq!(at.extract(packed, 0), 1);
        assert_eq!(at.extract(packed, 1), 2);
        assert_eq!(at.extract(packed, 2), 3);

        // Node 1 linked (mark = 0b010): find(1) -> 0(marked) = 1
        // Node 2 still marked: find(2) = 3 (itself)
        let packed = at.lookup(forest, 0b010);
        assert_eq!(at.extract(packed, 0), 1); // 0 still marked
        assert_eq!(at.extract(packed, 1), 1); // 1 linked, parent 0 is marked
        assert_eq!(at.extract(packed, 2), 3); // 2 still marked
    }

    /// Trigger the word-packing constraint (line 71) in compute_b.
    /// For very large n, b grows until (bb-1)*ceil_log2(bb) > 63.
    #[test]
    fn test_compute_b_word_packing_limit() {
        // usize::MAX is large enough that the table-size constraint passes
        // for b up to 16, but at bb=17 the word-packing constraint
        // (16 * ceil_log2(17) = 16*5 = 80 > 63) fires.
        let (b, clb) = compute_b(usize::MAX);
        assert_eq!(b, 16);
        assert_eq!(clb, 4); // ceil_log2(16) = 4
    }

    /// Trigger the node_count >= 16 overflow guard in decode_forest.
    #[test]
    fn test_decode_forest_overflow() {
        let mut parent = [0i32; 16];
        let mut cc = [0u32; 16];

        // 0xFFFF = 16 one-bits = exactly 16 nodes. Fits in the buffer.
        let count = AnswerTable::decode_forest(0xFFFF, &mut parent, &mut cc);
        assert_eq!(count, 16);

        // 0x1FFFF = 17 one-bits. The 17th node triggers the overflow guard.
        let count = AnswerTable::decode_forest(0x1FFFF, &mut parent, &mut cc);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_floor_log2() {
        assert_eq!(floor_log2(1), 0);
        assert_eq!(floor_log2(2), 1);
        assert_eq!(floor_log2(3), 1);
        assert_eq!(floor_log2(4), 2);
        assert_eq!(floor_log2(8), 3);
        assert_eq!(floor_log2(1000), 9);
    }

    #[test]
    fn test_ceil_log2() {
        assert_eq!(ceil_log2(1), 0);
        assert_eq!(ceil_log2(2), 1);
        assert_eq!(ceil_log2(3), 2);
        assert_eq!(ceil_log2(4), 2);
        assert_eq!(ceil_log2(5), 3);
    }
}
