//! Golden test vectors verified against the bzliu94 Python reference
//! implementation of Gabow-Tarjan (https://github.com/bzliu94/algorithms).
//!
//! Semantics: initially all nodes are set names (marked). `link(v)` removes
//! v as a set name. `find(v)` returns the nearest still-marked ancestor.

use incremental_tree_set_union::{IncrementalTreeSetUnion, StaticTreeSetUnion};

/// Build a parent array from (parent, child) edge pairs.
/// The root is identified as the node that is never a child.
fn parents_from_edges(n: usize, edges: &[(usize, usize)]) -> Vec<usize> {
    let mut parents = vec![0usize; n];
    let mut is_child = vec![false; n];
    for &(p, c) in edges {
        parents[c] = p;
        is_child[c] = true;
    }
    // Root points to itself
    for i in 0..n {
        if !is_child[i] {
            parents[i] = i;
        }
    }
    parents
}

/// Naive oracle: walk up the tree checking marks.
/// Returns the nearest ancestor of v that is still a set name.
fn naive_find(v: usize, parents: &[usize], linked: &[bool]) -> usize {
    let mut cur = v;
    loop {
        if !linked[cur] {
            return cur; // still a set name
        }
        let p = parents[cur];
        if p == cur {
            return cur; // root
        }
        cur = p;
    }
}

#[test]
fn test1_nine_node_tree() {
    // Tree: 0→{1,2}, 1→{3,4}, 2→5, 3→6, 6→{7,8}
    let edges = &[
        (0, 1),
        (0, 2),
        (1, 3),
        (1, 4),
        (2, 5),
        (3, 6),
        (6, 7),
        (6, 8),
    ];
    let parents = parents_from_edges(9, edges);
    let mut tsu = StaticTreeSetUnion::from_parents(&parents);

    // Verified against bzliu94 Python:
    tsu.link(7);
    tsu.link(3);
    assert_eq!(tsu.find(8), 8);
    assert_eq!(tsu.find(6), 6);
    tsu.link(6);
    assert_eq!(tsu.find(6), 1);
}

#[test]
fn test2_star() {
    // Tree: 0→{1,2,3,4,5}
    let edges = &[(0, 1), (0, 2), (0, 3), (0, 4), (0, 5)];
    let parents = parents_from_edges(6, edges);
    let mut tsu = StaticTreeSetUnion::from_parents(&parents);

    tsu.link(3);
    assert_eq!(tsu.find(1), 1);
    assert_eq!(tsu.find(3), 0);
    tsu.link(1);
    assert_eq!(tsu.find(2), 2);
}

#[test]
fn test3_path_10() {
    // Tree: 0→1→2→3→4→5→6→7→8→9
    let edges: Vec<(usize, usize)> = (0..9).map(|i| (i, i + 1)).collect();
    let parents = parents_from_edges(10, &edges);
    let mut tsu = StaticTreeSetUnion::from_parents(&parents);

    tsu.link(5);
    assert_eq!(tsu.find(9), 9);
    assert_eq!(tsu.find(5), 4);
    tsu.link(2);
    assert_eq!(tsu.find(9), 9);
    assert_eq!(tsu.find(3), 3);
    assert_eq!(tsu.find(0), 0);
}

#[test]
fn test4_binary_tree_15() {
    // Complete binary tree: 0→{1,2}, 1→{3,4}, 2→{5,6}, 3→{7,8}, ...
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
    let parents = parents_from_edges(15, edges);
    let mut tsu = StaticTreeSetUnion::from_parents(&parents);

    tsu.link(7);
    tsu.link(3);
    tsu.link(1);
    assert_eq!(tsu.find(8), 8);
    assert_eq!(tsu.find(10), 10);
    assert_eq!(tsu.find(14), 14);
    tsu.link(14);
    assert_eq!(tsu.find(14), 6);
}

#[test]
fn test5_path_12() {
    // Tree: 0→1→2→...→11
    let edges: Vec<(usize, usize)> = (0..11).map(|i| (i, i + 1)).collect();
    let parents = parents_from_edges(12, &edges);
    let mut tsu = StaticTreeSetUnion::from_parents(&parents);

    tsu.link(11);
    assert_eq!(tsu.find(11), 10);
    tsu.link(5);
    assert_eq!(tsu.find(10), 10);
    assert_eq!(tsu.find(6), 6);
    tsu.link(1);
    assert_eq!(tsu.find(11), 10);
}

#[test]
fn test6_path_20() {
    // Tree: 0→1→2→...→19
    let edges: Vec<(usize, usize)> = (0..19).map(|i| (i, i + 1)).collect();
    let parents = parents_from_edges(20, &edges);
    let mut tsu = StaticTreeSetUnion::from_parents(&parents);

    tsu.link(3);
    tsu.link(7);
    tsu.link(12);
    tsu.link(17);
    assert_eq!(tsu.find(19), 19);
    assert_eq!(tsu.find(15), 15);
    assert_eq!(tsu.find(10), 10);
    assert_eq!(tsu.find(5), 5);
    assert_eq!(tsu.find(1), 1);
    tsu.link(19);
    assert_eq!(tsu.find(19), 18);
}

// ========== Incremental variant tests ==========

/// Helper: build an IncrementalTreeSetUnion from edges (parent, child).
/// Edges must be in a valid grow order (parent must exist before child).
fn build_incremental(capacity: usize, edges: &[(usize, usize)]) -> IncrementalTreeSetUnion {
    let mut itsu = IncrementalTreeSetUnion::new(capacity);
    for &(p, c) in edges {
        itsu.grow(p, c);
    }
    itsu
}

#[test]
fn test_incremental_test1() {
    let edges = &[
        (0, 1),
        (0, 2),
        (1, 3),
        (1, 4),
        (2, 5),
        (3, 6),
        (6, 7),
        (6, 8),
    ];
    let mut itsu = build_incremental(9, edges);
    itsu.link(7);
    itsu.link(3);
    assert_eq!(itsu.find(8), 8);
    assert_eq!(itsu.find(6), 6);
    itsu.link(6);
    assert_eq!(itsu.find(6), 1);
}

#[test]
fn test_incremental_test2() {
    let mut itsu = build_incremental(6, &[(0, 1), (0, 2), (0, 3), (0, 4), (0, 5)]);
    itsu.link(3);
    assert_eq!(itsu.find(1), 1);
    assert_eq!(itsu.find(3), 0);
    itsu.link(1);
    assert_eq!(itsu.find(2), 2);
}

#[test]
fn test_incremental_test3() {
    let edges: Vec<(usize, usize)> = (0..9).map(|i| (i, i + 1)).collect();
    let mut itsu = build_incremental(10, &edges);
    itsu.link(5);
    assert_eq!(itsu.find(9), 9);
    assert_eq!(itsu.find(5), 4);
    itsu.link(2);
    assert_eq!(itsu.find(9), 9);
    assert_eq!(itsu.find(3), 3);
    assert_eq!(itsu.find(0), 0);
}

#[test]
fn test_incremental_test4() {
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
    let mut itsu = build_incremental(15, edges);
    itsu.link(7);
    itsu.link(3);
    itsu.link(1);
    assert_eq!(itsu.find(8), 8);
    assert_eq!(itsu.find(10), 10);
    assert_eq!(itsu.find(14), 14);
    itsu.link(14);
    assert_eq!(itsu.find(14), 6);
}

/// Load all cases from golden_vectors.json (generated by generate_golden_vectors.py
/// from the bzliu94 Python reference implementation) and validate both the
/// static and incremental variants against every recorded find result.
#[test]
fn test_json_golden_vectors() {
    let json_bytes = include_bytes!("golden_vectors.json");
    let json_str = std::str::from_utf8(json_bytes).expect("invalid UTF-8 in golden_vectors.json");
    let root: serde_json::Value =
        serde_json::from_str(json_str).expect("failed to parse golden_vectors.json");

    let cases = root["cases"].as_array().expect("cases is not an array");
    let mut total_finds = 0usize;

    for (ci, case) in cases.iter().enumerate() {
        let name = case["name"].as_str().unwrap_or("unnamed");
        let n = case["n"].as_u64().unwrap() as usize;
        let edges: Vec<(usize, usize)> = case["edges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| {
                let arr = e.as_array().unwrap();
                (
                    arr[0].as_u64().unwrap() as usize,
                    arr[1].as_u64().unwrap() as usize,
                )
            })
            .collect();

        let parents = parents_from_edges(n, &edges);
        let mut tsu = StaticTreeSetUnion::from_parents(&parents);
        let mut itsu = build_incremental(n, &edges);
        let mut linked = vec![false; n];

        let trace = case["trace"].as_array().unwrap();
        for entry in trace {
            let op = entry["op"].as_str().unwrap();
            let node = entry["node"].as_u64().unwrap() as usize;
            match op {
                "link" => {
                    tsu.link(node);
                    itsu.link(node);
                    linked[node] = true;
                }
                "find" => {
                    let expected = entry["result"].as_u64().unwrap() as usize;
                    let got_static = tsu.find(node);
                    let got_incr = itsu.find(node);
                    let got_naive = naive_find(node, &parents, &linked);

                    assert_eq!(
                        got_static, expected,
                        "case {ci} '{name}': static find({node}) = {got_static}, python = {expected}"
                    );
                    assert_eq!(
                        got_incr, expected,
                        "case {ci} '{name}': incremental find({node}) = {got_incr}, python = {expected}"
                    );
                    assert_eq!(
                        got_naive, expected,
                        "case {ci} '{name}': naive find({node}) = {got_naive}, python = {expected}"
                    );
                    total_finds += 1;
                }
                _ => panic!("unknown op: {op}"),
            }
        }
    }

    assert!(
        total_finds >= 100,
        "expected at least 100 find assertions from JSON, got {total_finds}"
    );
}
