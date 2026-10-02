#![no_main]

mod oracle;

use arbitrary::Arbitrary;
use incremental_tree_set_union::IncrementalTreeSetUnion;
use libfuzzer_sys::fuzz_target;
use oracle::{naive_find, UfOracle};

/// Large enough for b=4 (needs capacity >= 768).
const MAX_CAPACITY: usize = 1200;

#[derive(Debug, Arbitrary)]
struct FuzzInput {
    /// Controls capacity and thus `b`. Uses u16 to reach higher values.
    capacity_hint: u16,
    ops: Vec<Op>,
}

#[derive(Debug, Arbitrary)]
enum Op {
    /// Grow a single child from a chosen parent.
    Grow(u16),
    /// Grow a burst of children from the same parent, forcing microset splits.
    GrowBurst { parent: u16, count: u8 },
    /// Grow a chain from a chosen parent (path shape within one microset).
    GrowChain { start: u16, len: u8 },
    /// Link a node.
    Link(u16),
    /// Find a node.
    Find(u16),
    /// Link a node then immediately find it (hot path test).
    LinkThenFind(u16),
}

fuzz_target!(|input: FuzzInput| {
    if input.ops.is_empty() {
        return;
    }

    // Capacity varies to exercise different b values.
    // b=2 for cap<64, b=3 for 64..767, b=4 for 768..MAX_CAPACITY.
    let capacity = (input.capacity_hint as usize % MAX_CAPACITY).max(2) + 2;
    let mut itsu = IncrementalTreeSetUnion::new(capacity);
    let mut uf = UfOracle::new(capacity);
    let mut tree_parents = vec![0usize; capacity];
    let mut linked = vec![false; capacity];
    let mut n = 1usize;

    assert_eq!(itsu.len(), 1);

    let do_grow = |n: &mut usize, parent: usize, itsu: &mut IncrementalTreeSetUnion, tree_parents: &mut [usize]| -> bool {
        if *n >= capacity { return false; }
        let p = parent % *n;
        let w = *n;
        tree_parents[w] = p;
        itsu.grow(p, w);
        *n += 1;
        assert_eq!(itsu.len(), *n);
        true
    };

    for op in &input.ops {
        match op {
            Op::Grow(choice) => {
                do_grow(&mut n, *choice as usize, &mut itsu, &mut tree_parents);
            }
            Op::GrowBurst { parent, count } => {
                let p = *parent as usize % n.max(1);
                let burst = (*count as usize).min(20).min(capacity - n);
                for _ in 0..burst {
                    if !do_grow(&mut n, p, &mut itsu, &mut tree_parents) {
                        break;
                    }
                }
            }
            Op::GrowChain { start, len } => {
                let mut p = *start as usize % n.max(1);
                let chain_len = (*len as usize).min(20).min(capacity - n);
                for _ in 0..chain_len {
                    let w = n;
                    if !do_grow(&mut n, p, &mut itsu, &mut tree_parents) {
                        break;
                    }
                    p = w;
                }
            }
            Op::Link(choice) => {
                if n < 2 { continue; }
                let v = (*choice as usize % (n - 1)) + 1;
                if !linked[v] {
                    uf.link(v, tree_parents[v]);
                    linked[v] = true;
                }
                itsu.link(v);
            }
            Op::Find(choice) => {
                let v = *choice as usize % n;
                let got = itsu.find(v);
                let uf_ans = uf.find(v);
                let naive = naive_find(v, &tree_parents, &linked);
                assert_eq!(got, uf_ans);
                assert_eq!(got, naive);
            }
            Op::LinkThenFind(choice) => {
                if n < 2 { continue; }
                let v = (*choice as usize % (n - 1)) + 1;
                if !linked[v] {
                    uf.link(v, tree_parents[v]);
                    linked[v] = true;
                }
                itsu.link(v);
                let got = itsu.find(v);
                let uf_ans = uf.find(v);
                let naive = naive_find(v, &tree_parents, &linked);
                assert_eq!(got, uf_ans);
                assert_eq!(got, naive);
            }
        }
    }
});
