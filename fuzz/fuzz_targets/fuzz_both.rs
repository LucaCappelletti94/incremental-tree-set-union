#![no_main]

//! Fuzz target that runs the SAME tree and operations through both the static
//! and incremental variants, plus two independent oracles. All four must agree.

mod oracle;

use arbitrary::Arbitrary;
use incremental_tree_set_union::{IncrementalTreeSetUnion, StaticTreeSetUnion};
use libfuzzer_sys::fuzz_target;
use oracle::{naive_find, UfOracle};

/// Large enough for b=4 (needs n >= 768).
const MAX_N: usize = 1200;

#[derive(Debug, Arbitrary)]
struct FuzzInput {
    tree: TreeShape,
    ops: OpSequence,
}

#[derive(Debug, Arbitrary)]
enum TreeShape {
    Random(Vec<u16>),
    Path(u16),
    Star(u16),
}

#[derive(Debug, Arbitrary)]
enum OpSequence {
    Random(Vec<u16>),
    LinkAllThenFindAll(Vec<u16>),
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
            vec![0; n]
        }
    }
}

fuzz_target!(|input: FuzzInput| {
    let parents = build_parents(&input.tree);
    let n = parents.len();

    let mut static_tsu = StaticTreeSetUnion::from_parents(&parents);

    let mut incr_tsu = IncrementalTreeSetUnion::new(n);
    for i in 1..n {
        incr_tsu.grow(parents[i], i);
    }

    let mut uf = UfOracle::new(n);
    let mut linked = vec![false; n];

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
                        static_tsu.link(node);
                        incr_tsu.link(node);
                    }
                } else {
                    let s = static_tsu.find(node);
                    let i = incr_tsu.find(node);
                    let u = uf.find(node);
                    let naive = naive_find(node, &parents, &linked);
                    assert_eq!(s, i);
                    assert_eq!(s, u);
                    assert_eq!(s, naive);
                }
            }
        }
        OpSequence::LinkAllThenFindAll(find_targets) => {
            // Link all non-root nodes bottom-up
            for i in (1..n).rev() {
                static_tsu.link(i);
                incr_tsu.link(i);
                if !linked[i] {
                    uf.link(i, parents[i]);
                    linked[i] = true;
                }
            }
            // Find — forces full macroset traversal
            for &word in find_targets {
                let node = word as usize % n;
                let s = static_tsu.find(node);
                let i = incr_tsu.find(node);
                let u = uf.find(node);
                let naive = naive_find(node, &parents, &linked);
                assert_eq!(s, i);
                assert_eq!(s, u);
                assert_eq!(s, naive);
            }
        }
    }
});
