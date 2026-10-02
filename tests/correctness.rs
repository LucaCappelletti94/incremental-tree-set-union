//! Correctness property tests (properties 1-8 from plan.md, adapted).

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

/// Property 1: find before any link returns self (all nodes are set names).
#[test]
fn find_before_link_returns_self() {
    let parents = parents_from_edges(5, &[(0, 1), (0, 2), (1, 3), (1, 4)]);
    let mut tsu = StaticTreeSetUnion::from_parents(&parents);
    for i in 0..5 {
        assert_eq!(tsu.find(i), i);
    }
}

/// Property 2: find after link(v) skips v.
#[test]
fn find_after_link_skips_v() {
    let parents = parents_from_edges(3, &[(0, 1), (1, 2)]);
    let mut tsu = StaticTreeSetUnion::from_parents(&parents);
    tsu.link(2);
    assert_eq!(tsu.find(2), 1); // 2 linked, nearest set name is 1
    tsu.link(1);
    assert_eq!(tsu.find(2), 0); // 1 and 2 linked, nearest is root 0
    assert_eq!(tsu.find(1), 0);
}

/// Property 3: chain of links — find skips all linked nodes.
#[test]
fn chain_of_links() {
    // Path: 0 -> 1 -> 2 -> 3 -> 4
    let edges: Vec<(usize, usize)> = (0..4).map(|i| (i, i + 1)).collect();
    let parents = parents_from_edges(5, &edges);
    let mut tsu = StaticTreeSetUnion::from_parents(&parents);

    tsu.link(4);
    tsu.link(3);
    tsu.link(2);
    assert_eq!(tsu.find(4), 1); // 2,3,4 linked, nearest is 1
    tsu.link(1);
    assert_eq!(tsu.find(4), 0); // 1,2,3,4 linked, nearest is root
}

/// Property 4: find skips linked nodes but stops at unmarked ones.
#[test]
fn find_skips_linked_stops_at_set_name() {
    // Path: 0 -> 1 -> 2 -> 3 -> 4
    let edges: Vec<(usize, usize)> = (0..4).map(|i| (i, i + 1)).collect();
    let parents = parents_from_edges(5, &edges);
    let mut tsu = StaticTreeSetUnion::from_parents(&parents);

    tsu.link(2); // only node 2 is linked
    assert_eq!(tsu.find(4), 4); // 4 is still a set name
    assert_eq!(tsu.find(3), 3); // 3 is still a set name
    assert_eq!(tsu.find(2), 1); // 2 linked, parent 1 is set name
    assert_eq!(tsu.find(1), 1); // 1 is still a set name
    assert_eq!(tsu.find(0), 0); // 0 is still a set name
}

/// Property 5: grow preserves existing finds (incremental).
#[test]
fn grow_preserves_finds() {
    let mut itsu = IncrementalTreeSetUnion::new(10);
    itsu.grow(0, 1);
    itsu.grow(1, 2);

    itsu.link(2);
    assert_eq!(itsu.find(2), 1);

    // Growing a new node shouldn't affect existing finds
    itsu.grow(0, 3);
    assert_eq!(itsu.find(2), 1); // still correct
    assert_eq!(itsu.find(3), 3); // new node is a set name

    itsu.grow(1, 4);
    assert_eq!(itsu.find(2), 1);
    assert_eq!(itsu.find(4), 4);
}

/// Property 6: link is idempotent.
#[test]
fn link_idempotent() {
    let parents = parents_from_edges(3, &[(0, 1), (1, 2)]);
    let mut tsu = StaticTreeSetUnion::from_parents(&parents);
    tsu.link(2);
    assert_eq!(tsu.find(2), 1);
    tsu.link(2); // link again — should be a no-op
    assert_eq!(tsu.find(2), 1);
}

/// Property 7: interleaved grow/link/find.
#[test]
fn interleaved_operations() {
    let mut itsu = IncrementalTreeSetUnion::new(8);

    // Build tree: 0 -> {1, 2}
    itsu.grow(0, 1);
    assert_eq!(itsu.find(1), 1);
    itsu.grow(0, 2);
    assert_eq!(itsu.find(2), 2);

    // Link 1
    itsu.link(1);
    assert_eq!(itsu.find(1), 0);

    // Grow child of 1
    itsu.grow(1, 3);
    assert_eq!(itsu.find(3), 3); // 3 is still a set name

    // Link 3
    itsu.link(3);
    assert_eq!(itsu.find(3), 0); // 3 and 1 linked, goes to 0

    // Grow more
    itsu.grow(2, 4);
    itsu.grow(2, 5);
    assert_eq!(itsu.find(4), 4);
    assert_eq!(itsu.find(5), 5);

    itsu.link(4);
    assert_eq!(itsu.find(4), 2);
}

/// Property 8: equivalence with naive for static variant.
#[test]
fn equivalence_with_naive_static() {
    // Complete binary tree of depth 3
    let edges = &[
        (0, 1),
        (0, 2),
        (1, 3),
        (1, 4),
        (2, 5),
        (2, 6),
        (3, 7),
        (3, 8),
        (4, 9),
        (4, 10),
        (5, 11),
        (5, 12),
        (6, 13),
        (6, 14),
    ];
    let n = 15;
    let parents = parents_from_edges(n, edges);
    let mut tsu = StaticTreeSetUnion::from_parents(&parents);
    let mut linked = vec![false; n];

    fn naive_find(v: usize, parents: &[usize], linked: &[bool]) -> usize {
        let mut cur = v;
        loop {
            if !linked[cur] {
                return cur;
            }
            let p = parents[cur];
            if p == cur {
                return cur;
            }
            cur = p;
        }
    }

    // Link every other leaf
    for &v in &[7, 9, 11, 13] {
        tsu.link(v);
        linked[v] = true;
    }

    // Check all nodes
    for v in 0..n {
        assert_eq!(tsu.find(v), naive_find(v, &parents, &linked));
    }

    // Link internal nodes
    for &v in &[3, 5, 1] {
        tsu.link(v);
        linked[v] = true;
    }

    // Check all nodes again
    for v in 0..n {
        let got = tsu.find(v);
        let expected = naive_find(v, &parents, &linked);
        assert_eq!(got, expected, "find({v})={got}, expected={expected}");
    }
}
