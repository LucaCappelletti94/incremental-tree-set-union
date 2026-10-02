use crate::answer_table::AnswerTable;

/// Per-node metadata: which microset contains this node, its index within
/// the microset, its parent in the tree, and its depth.
#[derive(Clone, Debug, Default)]
pub(crate) struct NodeInfo {
    /// Index of the microset containing this node.
    pub micro: u32,
    /// Position of this node within its microset (0-indexed).
    pub number: u16,
    /// Parent node ID in the tree (u32::MAX if root).
    pub parent: u32,
    /// Depth from tree root (0 for root).
    pub level: u32,
}

/// A single microset containing up to `b - 1` nodes.
/// Node IDs are stored in a flat array (`all_nodes`) shared across all
/// microsets; this struct stores the offset and length into that array.
#[derive(Clone, Debug)]
pub(crate) struct Microset {
    /// Start index into the flat `all_nodes` array.
    pub nodes_offset: u32,
    /// Mark bitmask. Bit `j` is 0 if node at position `j` is marked (a set name),
    /// 1 if linked (no longer a set name). Initially 0 (all marked).
    pub mark: u32,
    /// Forest encoding (janezb unary child-count bitmask).
    pub forest: u32,
    /// Root of this microset: the parent node (external to the microset).
    /// `u32::MAX` if this microset contains the tree root.
    pub root: u32,
}

/// Sentinel value for "no parent" / "tree root".
pub(crate) const NO_PARENT: u32 = u32::MAX;

/// O(1) microfind: find the nearest marked (set-name) ancestor of node `v`
/// within its microset.
///
/// Returns the node ID of the answer, or the microset's external root
/// if no marked ancestor exists within the microset.
#[inline]
pub(crate) fn microfind(
    v: u32,
    node_info: &[NodeInfo],
    microsets: &[Microset],
    all_nodes: &[u32],
    answer_table: &AnswerTable,
) -> u32 {
    let info = &node_info[v as usize];
    let ms = &microsets[info.micro as usize];
    let packed = answer_table.lookup(ms.forest, ms.mark);
    let k = answer_table.extract(packed, info.number);
    if k == 0 {
        ms.root // no marked ancestor in microset; return external root
    } else {
        all_nodes[(ms.nodes_offset + k - 1) as usize] // return node(i, k-1)
    }
}
