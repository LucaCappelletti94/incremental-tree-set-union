use alloc::vec;
use alloc::vec::Vec;

use crate::answer_table::{AnswerTable, encode_forest};
use crate::macroset::Macroset;
use crate::microset::{NO_PARENT, NodeInfo};

/// Incremental tree set union data structure (Gabow-Tarjan Section 3).
///
/// Supports `grow`, `link`, and `find` operations on a dynamically
/// growing rooted tree. The paper claims O(m + n) total time, but this
/// implementation achieves O(m + n log log n) because the forest encoding
/// requires an O(b) preorder rebuild per grow (see README for details).
///
/// # Semantics
///
/// - The tree starts with a single root node (node 0).
/// - `grow(v, w)` adds node `w` as a child of existing node `v`.
/// - `link(v)` removes `v` as a set name, merging it into `parent(v)`'s set.
/// - `find(v)` returns the nearest ancestor of `v` that is still a set name.
///   A node is its own ancestor. Initially all nodes are set names.
#[derive(Clone, Debug)]
pub struct IncrementalTreeSetUnion {
    node_info: Vec<NodeInfo>,
    microsets: Vec<IncrMicroset>,
    answer_table: AnswerTable,
    macroset: Macroset,
    n: usize,
    capacity: usize,
    tree_children: Vec<Vec<u32>>,
    micro_child_counts: Vec<Vec<u32>>,
    in_macroset: Vec<bool>,
}

/// Incremental microset — uses a Vec for nodes since grow/split modify them.
#[derive(Clone, Debug)]
struct IncrMicroset {
    nodes: Vec<u32>,
    mark: u32,
    forest: u32,
    root: u32,
}

/// O(1) microfind for incremental variant (uses IncrMicroset's Vec).
#[inline]
fn incr_microfind(
    v: u32,
    node_info: &[NodeInfo],
    microsets: &[IncrMicroset],
    answer_table: &AnswerTable,
) -> u32 {
    let info = &node_info[v as usize];
    let ms = &microsets[info.micro as usize];
    let packed = answer_table.lookup(ms.forest, ms.mark);
    let k = answer_table.extract(packed, info.number);
    if k == 0 {
        ms.root
    } else {
        ms.nodes[(k - 1) as usize]
    }
}

impl IncrementalTreeSetUnion {
    /// Create a new structure with a single root node (node 0).
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(2);
        let answer_table = AnswerTable::new(capacity);

        let mut node_info = vec![NodeInfo::default(); capacity];
        node_info[0] = NodeInfo {
            micro: 0,
            number: 0,
            parent: NO_PARENT,
            level: 0,
        };

        let root_ms = IncrMicroset {
            nodes: vec![0],
            mark: 0,
            forest: encode_forest(&[0], 1),
            root: NO_PARENT,
        };

        let macroset = Macroset::new(capacity);
        let tree_children = vec![Vec::new(); capacity];
        let micro_child_counts = vec![vec![0u32]];
        let in_macroset = vec![false; capacity];

        Self {
            node_info,
            microsets: vec![root_ms],
            answer_table,
            macroset,
            n: 1,
            capacity,
            tree_children,
            micro_child_counts,
            in_macroset,
        }
    }

    /// Add node `w` as a child of existing node `v`.
    pub fn grow(&mut self, v: usize, w: usize) {
        assert!(v < self.n, "node v={v} not in tree");
        assert_eq!(w, self.n, "node w={w} must equal len() ({})", self.n);
        assert!(w < self.capacity, "node w={w} exceeds capacity");

        self.node_info[w] = NodeInfo {
            micro: 0,
            number: 0,
            parent: v as u32,
            level: self.node_info[v].level + 1,
        };
        self.tree_children[v].push(w as u32);
        self.n += 1;

        let ms_idx = self.node_info[v].micro as usize;
        self.node_info[w].micro = ms_idx as u32;
        let new_size = self.microsets[ms_idx].nodes.len() + 1;

        if new_size as u32 == self.answer_table.b {
            self.rebuild_microset_with_new_node(ms_idx, w as u32);
            self.split_microset(ms_idx);
        } else {
            self.rebuild_microset_with_new_node(ms_idx, w as u32);
        }
    }

    fn rebuild_microset_with_new_node(&mut self, ms_idx: usize, new_node: u32) {
        let ms = &self.microsets[ms_idx];
        let ms_root = ms.root;
        let old_mark = ms.mark;
        let old_nodes: Vec<u32> = ms.nodes.clone();

        let mut members: Vec<u32> = old_nodes.clone();
        if !members.contains(&new_node) {
            members.push(new_node);
        }

        // Use a small vec for membership check — members.len() is typically ≤ b
        let is_member = |nid: u32| -> bool { members.contains(&nid) };

        let mut roots = Vec::new();
        for &nid in &members {
            let p = self.node_info[nid as usize].parent;
            if p == NO_PARENT || !is_member(p) {
                roots.push(nid);
            }
        }

        let mut new_nodes = Vec::new();
        let mut new_child_counts = Vec::new();
        let mut new_mark: u32 = 0;
        let mut stack: Vec<u32> = Vec::new();
        for &r in roots.iter().rev() {
            stack.push(r);
        }

        while let Some(v) = stack.pop() {
            let idx = new_nodes.len();
            new_nodes.push(v);

            if let Some(old_idx) = old_nodes.iter().position(|&n| n == v) {
                if (old_mark >> old_idx) & 1 == 1 {
                    new_mark |= 1 << idx;
                }
            }

            let mut cc = 0u32;
            for &child in self.tree_children[v as usize].iter().rev() {
                if is_member(child) {
                    stack.push(child);
                    cc += 1;
                }
            }
            new_child_counts.push(cc);
        }

        for (idx, &nid) in new_nodes.iter().enumerate() {
            self.node_info[nid as usize].micro = ms_idx as u32;
            self.node_info[nid as usize].number = idx as u16;
        }

        let forest = encode_forest(&new_child_counts, new_nodes.len());

        self.microsets[ms_idx] = IncrMicroset {
            nodes: new_nodes,
            mark: new_mark,
            forest,
            root: ms_root,
        };

        debug_assert!(ms_idx < self.micro_child_counts.len());
        self.micro_child_counts[ms_idx] = new_child_counts;
    }

    fn split_microset(&mut self, ms_idx: usize) {
        let old_nodes: Vec<u32> = self.microsets[ms_idx].nodes.clone();
        let old_mark = self.microsets[ms_idx].mark;
        let old_root = self.microsets[ms_idx].root;
        let bb = old_nodes.len();

        // Build local tree structure. bb is at most b (typically 3-10).
        let mut local_children: Vec<Vec<usize>> = vec![Vec::new(); bb];
        let mut local_parent: Vec<Option<usize>> = vec![None; bb];

        for (i, &nid) in old_nodes.iter().enumerate() {
            let parent_nid = self.node_info[nid as usize].parent;
            if parent_nid != NO_PARENT {
                if let Some(p) = old_nodes.iter().position(|&n| n == parent_nid) {
                    local_children[p].push(i);
                    local_parent[i] = Some(p);
                }
            }
        }

        let b = self.answer_table.b;
        let threshold_4x = b + 2;

        // Postorder
        let mut postorder = Vec::with_capacity(bb);
        {
            let mut stack: Vec<(usize, bool)> = Vec::new();
            for (i, lp) in local_parent.iter().enumerate().take(bb) {
                if lp.is_none() {
                    stack.push((i, false));
                }
            }
            while let Some((v, expanded)) = stack.pop() {
                if expanded {
                    postorder.push(v);
                } else {
                    stack.push((v, true));
                    for &c in local_children[v].iter().rev() {
                        stack.push((c, false));
                    }
                }
            }
        }

        let mut d = vec![1u32; bb];
        let mut carved = vec![false; bb];
        let mut new_microsets: Vec<(Vec<usize>, u32)> = Vec::new();

        for &v in &postorder {
            d[v] = 1;
            let mut child_idx = 0;
            let mut prev_child_idx = 0; // tracks where the last carve stopped
            let child_list = &local_children[v];

            loop {
                while 4 * d[v] <= threshold_4x && child_idx < child_list.len() {
                    let w = child_list[child_idx];
                    d[v] += d[w];
                    child_idx += 1;
                }

                if 4 * d[v] <= threshold_4x {
                    break;
                }

                if child_idx == prev_child_idx {
                    break;
                }

                // Collect subtree nodes of children[prev_child_idx..child_idx]
                // (only the NEW children since the last carve)
                let mut ms_nodes = Vec::new();
                for &c in &child_list[prev_child_idx..child_idx] {
                    let mut stack = vec![c];
                    while let Some(s) = stack.pop() {
                        ms_nodes.push(s);
                        carved[s] = true;
                        for &gc in local_children[s].iter().rev() {
                            stack.push(gc);
                        }
                    }
                }

                if !ms_nodes.is_empty() {
                    new_microsets.push((ms_nodes, old_nodes[v]));
                }

                d[v] = 1;
                prev_child_idx = child_idx;
                if child_idx >= child_list.len() {
                    break;
                }
            }
        }

        let remaining: Vec<usize> = (0..bb).filter(|&i| !carved[i]).collect();

        // Helper: build a microset from a set of local indices
        let build_ms = |indices: &[usize],
                        ext_root: u32,
                        ms_id: u32,
                        node_info: &mut [NodeInfo]|
         -> (IncrMicroset, Vec<u32>) {
            let in_set: Vec<bool> = {
                let mut s = vec![false; bb];
                for &li in indices {
                    s[li] = true;
                }
                s
            };
            // Find roots and do preorder
            let mut nodes = Vec::new();
            let mut child_counts_vec = Vec::new();
            let mut mark: u32 = 0;
            let mut stack = Vec::new();
            for &li in indices {
                if local_parent[li].is_none() || !in_set[local_parent[li].unwrap()] {
                    stack.push(li);
                }
            }
            while let Some(v) = stack.pop() {
                let idx = nodes.len();
                let nid = old_nodes[v];
                nodes.push(nid);
                node_info[nid as usize].micro = ms_id;
                node_info[nid as usize].number = idx as u16;
                if (old_mark >> v) & 1 == 1 {
                    mark |= 1 << idx;
                }
                let mut cc = 0u32;
                for &c in local_children[v].iter().rev() {
                    if in_set[c] {
                        stack.push(c);
                        cc += 1;
                    }
                }
                child_counts_vec.push(cc);
            }
            let forest = encode_forest(&child_counts_vec, nodes.len());
            (
                IncrMicroset {
                    nodes,
                    mark,
                    forest,
                    root: ext_root,
                },
                child_counts_vec,
            )
        };

        // Create carved microsets
        for (ms_nodes, ext_root) in &new_microsets {
            let new_ms_id = self.microsets.len() as u32;
            let (ms, cc) = build_ms(ms_nodes, *ext_root, new_ms_id, &mut self.node_info);
            self.microsets.push(ms);
            self.micro_child_counts.push(cc);
            if !self.in_macroset[*ext_root as usize] {
                self.macroset
                    .make_set(*ext_root, self.node_info[*ext_root as usize].level);
                self.in_macroset[*ext_root as usize] = true;
            }
        }

        // Remaining
        let (ms, cc) = build_ms(&remaining, old_root, ms_idx as u32, &mut self.node_info);
        self.microsets[ms_idx] = ms;
        self.micro_child_counts[ms_idx] = cc;
    }

    /// Link node `v`.
    #[inline]
    pub fn link(&mut self, v: usize) {
        assert!(
            self.node_info[v].parent != NO_PARENT,
            "cannot link the root"
        );
        let info = &self.node_info[v];
        self.microsets[info.micro as usize].mark |= 1 << info.number;
    }

    /// Find the nearest ancestor of `v` that is still a set name.
    #[inline]
    #[must_use]
    pub fn find(&mut self, v: usize) -> usize {
        let mut x = v as u32;
        let mut y = incr_microfind(x, &self.node_info, &self.microsets, &self.answer_table);

        debug_assert_ne!(y, NO_PARENT, "root must always be a set name");

        let x_micro = self.node_info[x as usize].micro;
        let y_micro = self.node_info[y as usize].micro;

        if x_micro != y_micro {
            x = self.macroset.top(y);

            loop {
                y = incr_microfind(x, &self.node_info, &self.microsets, &self.answer_table);
                debug_assert_ne!(y, NO_PARENT, "root must always be a set name");

                let x_micro = self.node_info[x as usize].micro;
                let y_micro = self.node_info[y as usize].micro;

                if x_micro == y_micro {
                    break;
                }

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
