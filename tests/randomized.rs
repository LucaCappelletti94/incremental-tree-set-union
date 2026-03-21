#![allow(clippy::needless_range_loop)]
//! Randomized property-based tests against two independent oracles:
//! 1. A naive walk-up-the-tree oracle (O(depth) per find)
//! 2. A standard union-find oracle with path compression and set naming
//!
//! Tests both StaticTreeSetUnion and IncrementalTreeSetUnion.

use incremental_tree_set_union::{IncrementalTreeSetUnion, StaticTreeSetUnion};
use rand::prelude::*;

/// Naive oracle: walk up the tree checking marks. O(depth) per find.
fn naive_find(v: usize, parents: &[usize], linked: &[bool]) -> usize {
    let mut cur = v;
    loop {
        if !linked[cur] {
            return cur;
        }
        let p = parents[cur];
        if p == cur {
            return cur; // root
        }
        cur = p;
    }
}

/// Standard union-find oracle with path compression and explicit set naming.
/// This is an independent implementation of the same abstract problem:
/// each set has a "name" (the nearest unlinked ancestor), and link(v)
/// merges v's set into parent(v)'s set.
struct UnionFindOracle {
    uf_parent: Vec<usize>,
    uf_size: Vec<usize>,
    /// The "name" of each set: the nearest unlinked ancestor that serves
    /// as the representative. Stored at union-find roots.
    set_name: Vec<usize>,
}

impl UnionFindOracle {
    fn new(n: usize) -> Self {
        Self {
            uf_parent: (0..n).collect(),
            uf_size: vec![1; n],
            set_name: (0..n).collect(), // initially each node is its own set name
        }
    }

    fn uf_find(&mut self, mut u: usize) -> usize {
        let mut root = u;
        while self.uf_parent[root] != root {
            root = self.uf_parent[root];
        }
        while self.uf_parent[u] != root {
            let next = self.uf_parent[u];
            self.uf_parent[u] = root;
            u = next;
        }
        root
    }

    /// link(v): merge v's set into parent(v)'s set.
    /// The resulting set's name is parent(v)'s set's name.
    fn link(&mut self, v: usize, tree_parent: usize) {
        let rv = self.uf_find(v);
        let rp = self.uf_find(tree_parent);
        if rv == rp {
            return;
        }
        // The resulting set's name is the parent's set's name
        let name = self.set_name[rp];
        // Union by size
        let (big, small) = if self.uf_size[rv] >= self.uf_size[rp] {
            (rv, rp)
        } else {
            (rp, rv)
        };
        self.uf_parent[small] = big;
        self.uf_size[big] += self.uf_size[small];
        self.set_name[big] = name;
    }

    /// find(v): return the name of v's set.
    fn find(&mut self, v: usize) -> usize {
        let root = self.uf_find(v);
        self.set_name[root]
    }
}

/// Generate a random tree of `n` nodes as a parent array.
/// Returns (parents, grow_order) where grow_order lists edges
/// in a valid incremental grow order.
fn random_tree(n: usize, rng: &mut impl Rng) -> (Vec<usize>, Vec<(usize, usize)>) {
    let mut parents = vec![0usize; n];
    parents[0] = 0; // root
    let mut grow_order = Vec::new();
    for i in 1..n {
        let p = rng.random_range(0..i);
        parents[i] = p;
        grow_order.push((p, i));
    }
    (parents, grow_order)
}

/// Run a random test with n nodes and m operations on the static variant.
/// Checks against both naive walk-up and standard union-find oracles.
fn test_static_random(n: usize, m: usize, seed: u64) {
    let mut rng = StdRng::seed_from_u64(seed);
    let (parents, _) = random_tree(n, &mut rng);
    let mut tsu = StaticTreeSetUnion::from_parents(&parents);
    let mut linked = vec![false; n];
    let mut uf = UnionFindOracle::new(n);

    for _ in 0..m {
        let op: u32 = rng.random_range(0..3);
        match op {
            0 | 1 => {
                let v = rng.random_range(0..n);
                let got = tsu.find(v);
                let naive = naive_find(v, &parents, &linked);
                let uf_ans = uf.find(v);
                assert_eq!(got, naive, "static find({v}): got {got}, naive {naive}");
                assert_eq!(got, uf_ans, "static find({v}): got {got}, uf {uf_ans}");
            }
            2 => {
                let v = rng.random_range(0..n);
                if parents[v] != v && !linked[v] {
                    tsu.link(v);
                    uf.link(v, parents[v]);
                    linked[v] = true;
                }
            }
            _ => unreachable!(),
        }
    }
}

/// Run a random test with n nodes and m operations on the incremental variant.
/// Checks against both naive walk-up and standard union-find oracles.
fn test_incremental_random(n: usize, m: usize, seed: u64) {
    let mut rng = StdRng::seed_from_u64(seed);
    let (parents, grow_order) = random_tree(n, &mut rng);
    let mut itsu = IncrementalTreeSetUnion::new(n);

    for &(p, c) in &grow_order {
        itsu.grow(p, c);
    }

    let mut linked = vec![false; n];
    let mut uf = UnionFindOracle::new(n);

    for _ in 0..m {
        let op: u32 = rng.random_range(0..3);
        match op {
            0 | 1 => {
                let v = rng.random_range(0..n);
                let got = itsu.find(v);
                let naive = naive_find(v, &parents, &linked);
                let uf_ans = uf.find(v);
                assert_eq!(got, naive, "incr find({v}): got {got}, naive {naive}");
                assert_eq!(got, uf_ans, "incr find({v}): got {got}, uf {uf_ans}");
            }
            2 => {
                let v = rng.random_range(0..n);
                if parents[v] != v && !linked[v] {
                    itsu.link(v);
                    uf.link(v, parents[v]);
                    linked[v] = true;
                }
            }
            _ => unreachable!(),
        }
    }
}

/// Test incremental with interleaved grow/link/find.
/// Checks against both naive walk-up and standard union-find oracles.
fn test_incremental_interleaved(n: usize, seed: u64) {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut itsu = IncrementalTreeSetUnion::new(n);
    let mut parents = vec![0usize; n];
    parents[0] = 0;
    let mut linked = vec![false; n];
    let mut uf = UnionFindOracle::new(n);
    let mut current_n = 1usize;

    for _ in 0..(n * 3) {
        let op: u32 = if current_n < n {
            rng.random_range(0..4)
        } else {
            rng.random_range(0..3)
        };
        match op {
            0 | 1 => {
                if current_n > 0 {
                    let v = rng.random_range(0..current_n);
                    let got = itsu.find(v);
                    let naive = naive_find(v, &parents, &linked);
                    let uf_ans = uf.find(v);
                    assert_eq!(got, naive, "find({v}) at n={current_n}");
                    assert_eq!(
                        got, uf_ans,
                        "find({v}) at n={current_n}: got {got}, uf {uf_ans}"
                    );
                }
            }
            2 => {
                if current_n > 1 {
                    let v = rng.random_range(1..current_n);
                    if !linked[v] {
                        itsu.link(v);
                        uf.link(v, parents[v]);
                        linked[v] = true;
                    }
                }
            }
            3 => {
                if current_n < n {
                    let p = rng.random_range(0..current_n);
                    let w = current_n;
                    parents[w] = p;
                    itsu.grow(p, w);
                    current_n += 1;
                }
            }
            _ => unreachable!(),
        }
    }
}

// --- Static variant tests ---

#[test]
fn test_static_random_small() {
    for seed in 0..50 {
        test_static_random(20, 100, seed);
    }
}

#[test]
fn test_static_random_medium() {
    for seed in 0..10 {
        test_static_random(200, 1000, seed);
    }
}

#[test]
fn test_static_random_large() {
    for seed in 0..5 {
        test_static_random(2000, 10000, seed);
    }
}

// --- Incremental variant tests ---

#[test]
fn test_incremental_random_small() {
    for seed in 0..50 {
        test_incremental_random(20, 100, seed);
    }
}

#[test]
fn test_incremental_random_medium() {
    for seed in 0..10 {
        test_incremental_random(200, 1000, seed);
    }
}

#[test]
fn test_incremental_random_large() {
    for seed in 0..5 {
        test_incremental_random(2000, 10000, seed);
    }
}

// --- Interleaved grow/link/find ---

#[test]
fn test_incremental_interleaved_small() {
    for seed in 0..50 {
        test_incremental_interleaved(30, seed);
    }
}

#[test]
fn test_incremental_interleaved_medium() {
    for seed in 0..10 {
        test_incremental_interleaved(300, seed);
    }
}

// --- All three must agree: static, incremental, and union-find oracle ---

#[test]
fn test_static_vs_incremental_vs_uf() {
    for seed in 0..20 {
        let mut rng = StdRng::seed_from_u64(seed);
        let n = 100;
        let (parents, grow_order) = random_tree(n, &mut rng);

        let mut static_tsu = StaticTreeSetUnion::from_parents(&parents);
        let mut incr_tsu = IncrementalTreeSetUnion::new(n);
        for &(p, c) in &grow_order {
            incr_tsu.grow(p, c);
        }

        let mut linked = vec![false; n];
        let mut uf = UnionFindOracle::new(n);

        for _ in 0..500 {
            let op: u32 = rng.random_range(0..3);
            match op {
                0 | 1 => {
                    let v = rng.random_range(0..n);
                    let s = static_tsu.find(v);
                    let i = incr_tsu.find(v);
                    let u = uf.find(v);
                    let naive = naive_find(v, &parents, &linked);
                    assert_eq!(s, i, "seed={seed} find({v}): static={s}, incr={i}");
                    assert_eq!(s, u, "seed={seed} find({v}): static={s}, uf={u}");
                    assert_eq!(s, naive, "seed={seed} find({v}): static={s}, naive={naive}");
                }
                2 => {
                    let v = rng.random_range(0..n);
                    if parents[v] != v && !linked[v] {
                        static_tsu.link(v);
                        incr_tsu.link(v);
                        uf.link(v, parents[v]);
                        linked[v] = true;
                    }
                }
                _ => unreachable!(),
            }
        }
    }
}
