#![no_main]

mod oracle;

use arbitrary::Arbitrary;
use incremental_tree_set_union::StaticTreeSetUnion;
use libfuzzer_sys::fuzz_target;
use oracle::{naive_find, UfOracle};

/// Large enough for b=4 (needs n >= 768). b=4 means max_nodes=3,
/// which exercises multi-node forests and non-trivial answer table indexing.
const MAX_N: usize = 1200;

#[derive(Debug, Arbitrary)]
struct FuzzInput {
    tree: TreeShape,
    ops: OpSequence,
}

#[derive(Debug, Arbitrary)]
enum TreeShape {
    /// Random tree from parent choices. Uses u16 for unbiased selection.
    Random(Vec<u16>),
    /// Path: parents[i] = i-1. Maximizes depth and microset chain length.
    Path(u16),
    /// Star: parents[i] = 0. Maximizes width, few microsets.
    Star(u16),
    /// Caterpillar: spine of depth d, each spine node has w leaves.
    Caterpillar { depth: u8, width: u8 },
}

#[derive(Debug, Arbitrary)]
enum OpSequence {
    /// Random interleaved link/find (high bit = op type).
    Random(Vec<u16>),
    /// Link all non-root nodes bottom-up (leaves first), then find all.
    /// This maximizes macroset boundary crossings.
    LinkAllThenFindAll(Vec<u16>),
    /// Link a specific fraction of nodes, then find all.
    LinkFractionThenFindAll { link_targets: Vec<u16>, find_targets: Vec<u16> },
}

fn build_parents(tree: &TreeShape) -> Vec<usize> {
    match tree {
        TreeShape::Random(choices) => {
            let n = (choices.len() + 1).min(MAX_N).max(1);
            let mut parents = vec![0usize; n];
            for i in 1..n {
                parents[i] = choices[i - 1] as usize % i;
            }
            parents
        }
        TreeShape::Path(len) => {
            let n = (*len as usize + 2).min(MAX_N).max(2);
            let mut parents = vec![0usize; n];
            for i in 1..n {
                parents[i] = i - 1;
            }
            parents
        }
        TreeShape::Star(size) => {
            let n = (*size as usize + 2).min(MAX_N).max(2);
            vec![0; n] // all parent to root; parents[0]=0 is root
        }
        TreeShape::Caterpillar { depth, width } => {
            let d = (*depth as usize).max(1).min(200);
            let w = (*width as usize).max(0).min(20);
            let n = (d + d * w).min(MAX_N).max(2);
            let mut parents = vec![0usize; n];
            // Spine: 0 -> 1 -> 2 -> ... -> d-1
            for i in 1..d.min(n) {
                parents[i] = i - 1;
            }
            // Leaves: each spine node i gets w children
            let mut next = d;
            for i in 0..d.min(n) {
                for _ in 0..w {
                    if next >= n {
                        break;
                    }
                    parents[next] = i;
                    next += 1;
                }
            }
            parents
        }
    }
}

fuzz_target!(|input: FuzzInput| {
    let parents = build_parents(&input.tree);
    let n = parents.len();

    let mut tsu = StaticTreeSetUnion::from_parents(&parents);
    let mut uf = UfOracle::new(n);
    let mut linked = vec![false; n];

    assert_eq!(tsu.len(), n);
    assert!(!tsu.is_empty());

    match &input.ops {
        OpSequence::Random(ops) => {
            for &word in ops {
                let node = word as usize % n;
                if word & 0x8000 != 0 {
                    if parents[node] != node {
                        if !linked[node] {
                            uf.link(node, parents[node]);
                            linked[node] = true;
                        }
                        tsu.link(node);
                    }
                } else {
                    let got = tsu.find(node);
                    let uf_ans = uf.find(node);
                    let naive = naive_find(node, &parents, &linked);
                    assert_eq!(got, uf_ans);
                    assert_eq!(got, naive);
                }
            }
        }
        OpSequence::LinkAllThenFindAll(find_targets) => {
            // Link all non-root nodes in reverse order (leaves first)
            for i in (1..n).rev() {
                if !linked[i] {
                    tsu.link(i);
                    uf.link(i, parents[i]);
                    linked[i] = true;
                }
            }
            // Now find — every find must traverse the full macroset chain
            for &word in find_targets {
                let node = word as usize % n;
                let got = tsu.find(node);
                let uf_ans = uf.find(node);
                let naive = naive_find(node, &parents, &linked);
                assert_eq!(got, uf_ans);
                assert_eq!(got, naive);
            }
        }
        OpSequence::LinkFractionThenFindAll { link_targets, find_targets } => {
            for &word in link_targets {
                let node = word as usize % n;
                if parents[node] != node && !linked[node] {
                    tsu.link(node);
                    uf.link(node, parents[node]);
                    linked[node] = true;
                }
            }
            for &word in find_targets {
                let node = word as usize % n;
                let got = tsu.find(node);
                let uf_ans = uf.find(node);
                let naive = naive_find(node, &parents, &linked);
                assert_eq!(got, uf_ans);
                assert_eq!(got, naive);
            }
        }
    }
});
