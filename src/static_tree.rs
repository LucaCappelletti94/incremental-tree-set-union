use alloc::vec;
use alloc::vec::Vec;

use crate::answer_table::{AnswerTable, encode_forest};
use crate::macroset::Macroset;
use crate::microset::{Microset, NO_PARENT, NodeInfo, microfind};

/// Static tree set union data structure (Gabow-Tarjan Section 2).
///
/// The tree must be known in advance. Supports `link` and `find` operations
/// in O(1) amortized time, with O(m + n) total for m operations on n nodes.
///
/// # Semantics
///
/// - Initially every node is its own **set name** (marked).
/// - `link(v)` removes `v` as a set name, merging it into `parent(v)`'s set.
///   After this, `find` will skip past `v`.
/// - `find(v)` returns the nearest ancestor of `v` that is still a set name
///   (has not been linked). A node is its own ancestor.
/// - `link(root)` panics: the root has no parent.
#[derive(Clone, Debug)]
pub struct StaticTreeSetUnion {
    node_info: Vec<NodeInfo>,
    microsets: Vec<Microset>,
    /// Flat array of all microset node IDs. Each microset's nodes are a
    /// contiguous slice at `[offset..offset+len]`.
    all_nodes: Vec<u32>,
    answer_table: AnswerTable,
    macroset: Macroset,
    n: usize,
}

/// Return the children of node `v` in a CSR representation.
#[inline]
fn csr_children<'a>(v: usize, offsets: &[u32], children: &'a [u32]) -> &'a [u32] {
    &children[offsets[v] as usize..offsets[v + 1] as usize]
}

impl StaticTreeSetUnion {
    /// Construct from a parent array. `parents[i]` is the parent of node `i`.
    /// The root node must satisfy `parents[root] == root`.
    pub fn from_parents(parents: &[usize]) -> Self {
        let n = parents.len();
        assert!(n >= 1, "tree must have at least one node");

        // Find root
        let root = (0..n)
            .find(|&i| parents[i] == i)
            .expect("tree must have a root (parents[root] == root)");

        // Build CSR children representation: 2 flat arrays, 0 extra heap allocs.
        // Pass 1: count children per node
        let mut degree = vec![0u32; n];
        for (i, &p) in parents.iter().enumerate() {
            if i != root {
                degree[p] += 1;
            }
        }
        // Pass 2: prefix sum for offsets
        let mut offsets = vec![0u32; n + 1];
        for i in 0..n {
            offsets[i + 1] = offsets[i] + degree[i];
        }
        // Pass 3: fill children array
        let num_edges = if n > 0 { n - 1 } else { 0 };
        let mut children_flat = vec![0u32; num_edges];
        let mut pos = offsets[..n].to_vec();
        for (i, &p) in parents.iter().enumerate() {
            if i != root {
                children_flat[pos[p] as usize] = i as u32;
                pos[p] += 1;
            }
        }

        // Compute levels via BFS
        let mut levels = vec![0u32; n];
        let mut queue = vec![root];
        let mut qi = 0;
        while qi < queue.len() {
            let v = queue[qi];
            qi += 1;
            for &c in csr_children(v, &offsets, &children_flat) {
                levels[c as usize] = levels[v] + 1;
                queue.push(c as usize);
            }
        }

        // Build answer table
        let answer_table = AnswerTable::new(n);
        let b = answer_table.b;

        // Initialize node_info
        let mut node_info = vec![NodeInfo::default(); n];
        for (i, &p) in parents.iter().enumerate() {
            node_info[i].parent = if i == root { NO_PARENT } else { p as u32 };
            node_info[i].level = levels[i];
        }

        // Track which nodes have been assigned to a microset
        let mut assigned = vec![false; n];
        let mut microsets_list: Vec<Microset> = Vec::new();
        let mut all_nodes: Vec<u32> = Vec::with_capacity(n);
        let mut d = vec![1u32; n];

        // Non-recursive postorder traversal
        let postorder = Self::postorder(root, &offsets, &children_flat);

        // Threshold: d(v) < (b+1)/2. Use 2*d(v) < (b+1).
        let threshold_2x = b + 1;

        for &v in &postorder {
            d[v] = 1;
            let mut child_idx = 0;
            let child_list = csr_children(v, &offsets, &children_flat);

            loop {
                while 2 * d[v] < threshold_2x && child_idx < child_list.len() {
                    let w = child_list[child_idx] as usize;
                    d[v] += d[w];
                    child_idx += 1;
                }

                if 2 * d[v] < threshold_2x {
                    break;
                }

                debug_assert!(child_idx > 0);

                let split_children = &child_list[..child_idx];
                Self::add_microset(
                    v as u32,
                    split_children,
                    &offsets,
                    &children_flat,
                    &mut assigned,
                    &mut node_info,
                    &mut microsets_list,
                    &mut all_nodes,
                );

                d[v] = 1;
                if child_idx >= child_list.len() {
                    break;
                }
            }
        }

        // Final microset: all remaining unassigned nodes (including root)
        Self::add_final_microset(
            root,
            &offsets,
            &children_flat,
            &mut assigned,
            &mut node_info,
            &mut microsets_list,
            &mut all_nodes,
        );

        // Initialize macrosets for all microset external roots
        let mut macroset = Macroset::new(n);
        for ms in &microsets_list {
            if ms.root != NO_PARENT {
                macroset.make_set(ms.root, node_info[ms.root as usize].level);
            }
        }

        Self {
            node_info,
            microsets: microsets_list,
            all_nodes,
            answer_table,
            macroset,
            n,
        }
    }

    fn postorder(root: usize, offsets: &[u32], children: &[u32]) -> Vec<usize> {
        let mut result = Vec::new();
        let mut stack: Vec<(usize, bool)> = vec![(root, false)];
        while let Some((v, expanded)) = stack.pop() {
            if expanded {
                result.push(v);
            } else {
                stack.push((v, true));
                for &c in csr_children(v, offsets, children).iter().rev() {
                    stack.push((c as usize, false));
                }
            }
        }
        result
    }

    #[allow(clippy::too_many_arguments)]
    fn add_microset(
        ms_root: u32,
        split_children: &[u32],
        offsets: &[u32],
        children: &[u32],
        assigned: &mut [bool],
        node_info: &mut [NodeInfo],
        microsets: &mut Vec<Microset>,
        all_nodes: &mut Vec<u32>,
    ) {
        let ms_id = microsets.len() as u32;
        let nodes_offset = all_nodes.len() as u32;
        let mut child_counts = [0u32; 16];
        let mut count = 0usize;

        let mut stack: Vec<u32> = Vec::new();
        for &c in split_children.iter().rev() {
            if !assigned[c as usize] {
                stack.push(c);
            }
        }

        while let Some(v) = stack.pop() {
            debug_assert!(!assigned[v as usize]);
            let idx = count;
            all_nodes.push(v);
            assigned[v as usize] = true;
            node_info[v as usize].micro = ms_id;
            node_info[v as usize].number = idx as u16;

            let mut cc = 0u32;
            for &child in csr_children(v as usize, offsets, children).iter().rev() {
                if !assigned[child as usize] {
                    stack.push(child);
                    cc += 1;
                }
            }
            child_counts[idx] = cc;
            count += 1;
        }

        let forest = encode_forest(&child_counts, count);
        microsets.push(Microset {
            nodes_offset,
            mark: 0,
            forest,
            root: ms_root,
        });
    }

    fn add_final_microset(
        root: usize,
        offsets: &[u32],
        children: &[u32],
        assigned: &mut [bool],
        node_info: &mut [NodeInfo],
        microsets: &mut Vec<Microset>,
        all_nodes: &mut Vec<u32>,
    ) {
        let ms_id = microsets.len() as u32;
        let nodes_offset = all_nodes.len() as u32;
        let mut child_counts = [0u32; 16];
        let mut count = 0usize;

        let mut stack = vec![root as u32];
        while let Some(v) = stack.pop() {
            debug_assert!(!assigned[v as usize]);
            let idx = count;
            all_nodes.push(v);
            assigned[v as usize] = true;
            node_info[v as usize].micro = ms_id;
            node_info[v as usize].number = idx as u16;

            let mut cc = 0u32;
            for &child in csr_children(v as usize, offsets, children).iter().rev() {
                if !assigned[child as usize] {
                    stack.push(child);
                    cc += 1;
                }
            }
            if idx < 16 {
                child_counts[idx] = cc;
            }
            count += 1;
        }

        let forest = encode_forest(&child_counts, count);
        microsets.push(Microset {
            nodes_offset,
            mark: 0,
            forest,
            root: NO_PARENT,
        });
    }

    /// Link node `v`: remove `v` as a set name, merging it into its
    /// parent's set.
    #[inline]
    pub fn link(&mut self, v: usize) {
        assert!(
            self.node_info[v].parent != NO_PARENT,
            "cannot link the root (it has no parent)"
        );
        let info = &self.node_info[v];
        self.microsets[info.micro as usize].mark |= 1 << info.number;
    }

    /// Find the nearest ancestor of `v` that is still a set name
    /// (has not been linked). A node is its own ancestor.
    #[inline]
    #[must_use]
    pub fn find(&mut self, v: usize) -> usize {
        let mut x = v as u32;
        let mut y = microfind(
            x,
            &self.node_info,
            &self.microsets,
            &self.all_nodes,
            &self.answer_table,
        );

        debug_assert_ne!(y, NO_PARENT, "root must always be a set name");

        let x_micro = self.node_info[x as usize].micro;
        let y_micro = self.node_info[y as usize].micro;

        if x_micro != y_micro {
            x = self.macroset.top(y);

            loop {
                y = microfind(
                    x,
                    &self.node_info,
                    &self.microsets,
                    &self.all_nodes,
                    &self.answer_table,
                );
                debug_assert_ne!(y, NO_PARENT, "root must always be a set name");

                let x_micro = self.node_info[x as usize].micro;
                let y_micro = self.node_info[y as usize].micro;

                if x_micro == y_micro {
                    break;
                }

                // [GT83] corrected: MacroUnite(MicroFind(x), x)
                self.macroset.unite(y, x);
                x = self.macroset.top(y);
            }
        }

        y as usize
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.n
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }
}
