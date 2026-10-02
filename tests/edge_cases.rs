#![allow(clippy::needless_range_loop)]
//! Edge case tests (properties 12-16 from plan.md).

use incremental_tree_set_union::{IncrementalTreeSetUnion, StaticTreeSetUnion};

fn parents_from_edges(n: usize, edges: &[(usize, usize)]) -> Vec<usize> {
    let mut parents = vec![0usize; n];
    let mut is_child = vec![false; n];
    for &(p, c) in edges {
        parents[c] = p;
        is_child[c] = true;
    }
    for i in 0..n {
        if !is_child[i] {
            parents[i] = i;
        }
    }
    parents
}

/// Property 12: single root node.
#[test]
fn single_root() {
    // Static
    let mut tsu = StaticTreeSetUnion::from_parents(&[0]);
    assert_eq!(tsu.find(0), 0);
    assert_eq!(tsu.len(), 1);
    assert!(!tsu.is_empty());

    // Incremental
    let mut itsu = IncrementalTreeSetUnion::new(2);
    assert_eq!(itsu.find(0), 0);
    assert_eq!(itsu.len(), 1);
    assert!(!itsu.is_empty());
}

/// Property 13: path graph — all microsets are paths.
#[test]
fn path_graph() {
    let n = 50;
    let edges: Vec<(usize, usize)> = (0..n - 1).map(|i| (i, i + 1)).collect();
    let parents = parents_from_edges(n, &edges);
    let mut tsu = StaticTreeSetUnion::from_parents(&parents);

    // All nodes are set names initially
    for i in 0..n {
        assert_eq!(tsu.find(i), i);
    }

    // Link every other node
    for i in (1..n).step_by(2) {
        tsu.link(i);
    }

    // Check
    for i in 0..n {
        let expected = if i % 2 == 1 { i - 1 } else { i };
        assert_eq!(tsu.find(i), expected, "find({i})");
    }
}

/// Property 13b: path graph with incremental variant.
#[test]
fn path_graph_incremental() {
    let n = 50;
    let mut itsu = IncrementalTreeSetUnion::new(n);
    for i in 1..n {
        itsu.grow(i - 1, i);
    }

    for i in 0..n {
        assert_eq!(itsu.find(i), i);
    }

    // Link all non-root nodes
    for i in 1..n {
        itsu.link(i);
    }
    for i in 0..n {
        assert_eq!(itsu.find(i), 0);
    }
}

/// Property 14: star graph (root has n-1 children).
#[test]
fn star_graph() {
    let n = 30;
    let edges: Vec<(usize, usize)> = (1..n).map(|i| (0, i)).collect();
    let parents = parents_from_edges(n, &edges);
    let mut tsu = StaticTreeSetUnion::from_parents(&parents);

    for i in 0..n {
        assert_eq!(tsu.find(i), i);
    }

    // Link all children
    for i in 1..n {
        tsu.link(i);
    }
    for i in 0..n {
        assert_eq!(tsu.find(i), 0);
    }
}

/// Property 14b: star graph incremental.
#[test]
fn star_graph_incremental() {
    let n = 30;
    let mut itsu = IncrementalTreeSetUnion::new(n);
    for i in 1..n {
        itsu.grow(0, i);
    }

    for i in 1..n {
        itsu.link(i);
    }
    for i in 0..n {
        assert_eq!(itsu.find(i), 0);
    }
}

/// Property 15: complete binary tree.
#[test]
fn complete_binary_tree() {
    let n = 31; // depth 4
    let mut edges = Vec::new();
    for i in 0..n {
        let left = 2 * i + 1;
        let right = 2 * i + 2;
        if left < n {
            edges.push((i, left));
        }
        if right < n {
            edges.push((i, right));
        }
    }
    let parents = parents_from_edges(n, &edges);
    let mut tsu = StaticTreeSetUnion::from_parents(&parents);

    // Link all leaves
    for i in (n / 2)..n {
        tsu.link(i);
    }

    // Each leaf should find its parent
    for i in (n / 2)..n {
        let parent = (i - 1) / 2;
        assert_eq!(tsu.find(i), parent, "find(leaf {i})");
    }

    // Internal nodes still find themselves
    for i in 0..(n / 2) {
        assert_eq!(tsu.find(i), i);
    }
}

/// Property 16: adversarial grow ordering (deep before wide).
#[test]
fn adversarial_grow_order() {
    let n = 40;
    let mut itsu = IncrementalTreeSetUnion::new(n);

    // Build a deep path first: 0 -> 1 -> 2 -> ... -> 19
    for i in 1..20 {
        itsu.grow(i - 1, i);
    }

    // Then add siblings: 0 -> 20, 0 -> 21, ..., 0 -> 29
    for i in 20..30 {
        itsu.grow(0, i);
    }

    // Then add children of deep nodes: 10 -> 30, 10 -> 31, ...
    for i in 30..n {
        itsu.grow(10, i);
    }

    // Verify all finds work
    for i in 0..n {
        assert_eq!(itsu.find(i), i);
    }

    // Link some nodes and verify
    itsu.link(5);
    itsu.link(15);
    assert_eq!(itsu.find(5), 4);
    assert_eq!(itsu.find(15), 14);
    assert_eq!(itsu.find(19), 19); // unlinked
    assert_eq!(itsu.find(35), 35); // unlinked child of 10

    itsu.link(10);
    // Children of 10 (30-39) should still find themselves (they're set names)
    for i in 30..n {
        assert_eq!(itsu.find(i), i);
    }
    // But find(10) should skip to 9
    assert_eq!(itsu.find(10), 9);
}

/// Large tree test — exercises b=5 (n >= 16384).
#[test]
fn large_incremental_n100k() {
    use rand::prelude::*;
    let n = 100_000;
    let mut rng = StdRng::seed_from_u64(42);
    let mut itsu = IncrementalTreeSetUnion::new(n);
    let mut parents = vec![0usize; n];
    for i in 1..n {
        let p = rng.random_range(0..i);
        parents[i] = p;
        itsu.grow(p, i);
    }
    // Link half
    let mut linked = vec![false; n];
    for _ in 0..n / 2 {
        let v = rng.random_range(1..n);
        if !linked[v] {
            itsu.link(v);
            linked[v] = true;
        }
    }
    // Find all — compare against naive
    for i in 0..100 {
        let v = rng.random_range(0..n);
        let got = itsu.find(v);
        let mut cur = v;
        loop {
            if !linked[cur] {
                break;
            }
            if parents[cur] == cur {
                break;
            }
            cur = parents[cur];
        }
        assert_eq!(got, cur, "find({v}) at iteration {i}");
    }
}
