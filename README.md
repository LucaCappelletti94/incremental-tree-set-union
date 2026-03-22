# incremental-tree-set-union

[![CI](https://github.com/LucaCappelletti94/incremental-tree-set-union/actions/workflows/ci.yml/badge.svg)](https://github.com/LucaCappelletti94/incremental-tree-set-union/actions/workflows/ci.yml)
[![Coverage](https://codecov.io/gh/LucaCappelletti94/incremental-tree-set-union/branch/main/graph/badge.svg)](https://codecov.io/gh/LucaCappelletti94/incremental-tree-set-union)
[![Crates.io](https://img.shields.io/crates/v/incremental-tree-set-union.svg)](https://crates.io/crates/incremental-tree-set-union)
[![docs.rs](https://docs.rs/incremental-tree-set-union/badge.svg)](https://docs.rs/incremental-tree-set-union)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/LucaCappelletti94/incremental-tree-set-union/blob/main/LICENSE)

A `no_std` Rust implementation of the Gabow-Tarjan linear-time algorithm for incremental tree set union (JCSS 1985).

> **Do not use this crate for performance.** This is a faithful implementation of a theoretically important algorithm that is **6-10x slower than standard union-find** (such as [`disjoint-sets`](https://crates.io/crates/disjoint-sets) or [`union-find`](https://crates.io/crates/union-find)) for all practical input sizes. The algorithm eliminates an inverse-Ackermann factor that is at most 4 for any input that could fit in memory. The overhead of the microset/answer-table machinery far exceeds this saving. Use standard union-find unless you need the strict O(m + n) bound for a complexity proof.

## Overview

This crate implements the algorithm from Gabow and Tarjan's 1985 paper *"A Linear-Time Algorithm for a Special Case of Disjoint Set Union"*. It solves a specialized version of the disjoint set union problem where the union structure follows a rooted tree. The static variant achieves **O(m + n) total time** for m intermixed operations on n nodes, strictly linear, with no inverse-Ackermann factor. The incremental variant achieves O(m + n log log n) due to an encoding tradeoff (see [Complexity deviation](#complexity-deviation-in-the-incremental-variant)).

The tree is partitioned into small subtrees called **microsets** (each fewer than b nodes, where b = Theta(log log n)). A precomputed **answer table** resolves queries within each microset in O(1). Standard union-find **macrosets** handle queries that cross microset boundaries.

Two variants are provided:

- **`StaticTreeSetUnion`**: the tree is known in advance (Section 2 of the paper). Achieves O(m + n) total time.
- **`IncrementalTreeSetUnion`**: the tree grows dynamically via `grow` operations (Section 3 of the paper). See [Complexity deviation](#complexity-deviation-in-the-incremental-variant) for a discussion of the time bound.

## Semantics

- **Initially**, every node is its own **set name** (marked).
- **`link(v)`** removes v as a set name, merging v into its parent's set. After this, `find` will **skip past** v. Equivalent to `unite(parent(v), v)` in standard union-find.
- **`find(v)`** returns the nearest ancestor of v that is still a set name (has not been linked). A node is its own ancestor, so `find(v) = v` if v has not been linked.
- **`grow(v, w)`** (incremental variant only) adds node w as a child of existing node v.
- **`link(root)` panics**: the root has no parent and always remains a set name.

## API usage

### Static variant (tree known in advance)

```rust
use incremental_tree_set_union::StaticTreeSetUnion;

// Tree: 0 -> {1, 2}, 1 -> {3, 4}, 2 -> 5
// parents[i] = parent of node i; parents[root] = root
let parents = vec![0, 0, 0, 1, 1, 2];
let mut tsu = StaticTreeSetUnion::from_parents(&parents);

// Initially, find(v) = v for all v (all nodes are set names)
assert_eq!(tsu.find(4), 4);
assert_eq!(tsu.find(3), 3);

// Link node 3: it is no longer a set name
tsu.link(3);
assert_eq!(tsu.find(3), 1);  // skips past 3, reaches parent 1

// Link node 1
tsu.link(1);
assert_eq!(tsu.find(3), 0);  // skips past 3 and 1, reaches root 0
assert_eq!(tsu.find(4), 4);  // 4 was never linked, still a set name
```

### Incremental variant (tree grows dynamically)

```rust
use incremental_tree_set_union::IncrementalTreeSetUnion;

let mut itsu = IncrementalTreeSetUnion::new(100); // capacity hint

// Build tree dynamically
itsu.grow(0, 1);  // add node 1 as child of root 0
itsu.grow(0, 2);  // add node 2 as child of root 0
itsu.grow(1, 3);  // add node 3 as child of node 1

// Operations work the same as the static variant
itsu.link(3);
assert_eq!(itsu.find(3), 1);

itsu.link(1);
assert_eq!(itsu.find(3), 0);
```

## How it works

The algorithm has three layers:

### Layer 1: Microsets

The tree T is partitioned into small subtrees called microsets, each containing fewer than b - 1 nodes. Each microset has an external **root** (a node outside the microset that is the parent of the microset's topmost nodes in T). The tree structure within each microset is encoded as a **forest bitmask** using unary child counts: for each node in preorder, emit one `1` bit followed by `num_children` `0` bits.

### Layer 2: Answer table

A precomputed lookup table answers "nearest marked ancestor" queries within a single microset in O(1). The table is indexed by `(forest_encoding << (b-1)) | mark_bitmask` and contains packed answers for all node positions, each `ceil(log2(b))` bits wide. The table has 2^(3b-4) entries and is constructed in O(n) time.

### Layer 3: Macrosets

Standard union-find (path compression + union by size) on the microset roots handles queries that cross microset boundaries. When `microfind` returns the external root of a microset (no marked ancestor found within), the algorithm transitions to the macroset layer to find the next microset up the tree.

### The find algorithm

```text
microfind(v):
    look up answer_table[(forest, mark)] for node v's position
    if answer = 0: return microset root (cross to macroset)
    else: return the ancestor node within the microset

find(v):
    y = microfind(v)
    if y is in a different microset than v:
        x = macro_top(macrofind(y))     // find highest node in macroset
        loop:
            y = microfind(x)
            if y is in the same microset as x: break
            macrounite(y, x)            // merge macrosets
            x = macro_top(y)            // continue from highest
    return y
```

### The parameter b

The microset size parameter b is chosen adaptively based on n to balance the answer table size against the number of macroset operations:

| n               | b | max nodes/microset | answer table entries |
| --------------- | - | ------------------ | -------------------- |
| < 64            | 2 | 1                  | 4                    |
| 64 - 767        | 3 | 2                  | 32                   |
| 768 - 16,383    | 4 | 3                  | 256                  |
| 16,384 - 524,287 | 5 | 4                  | 2,048                |
| 524,288 - ~25M  | 6 | 5                  | 16,384               |

![b parameter effect](https://raw.githubusercontent.com/LucaCappelletti94/incremental-tree-set-union/main/docs/b_parameter.svg)

## Performance

### Summary

**The static variant is 6-10x slower than standard union-find. The incremental variant is 50-120x slower.** The algorithm's value is theoretical: it proves that tree set union can be solved in strictly O(m + n) time. The constant factor makes it impractical as a faster alternative to standard union-find.

The root cause is fundamental to the algorithm: each `microfind` requires 3-4 dependent memory loads (NodeInfo, Microset, answer table, node array), versus 1 load for a path-compressed standard union-find `find`.

The factor this algorithm eliminates, the inverse Ackermann function alpha(m, n), grows so slowly that it is effectively constant. The Ackermann function grows as A(1, n) = 2n, A(2, n) ~ 2^n, A(3, n) ~ 2^2^...^2 (n times), A(4, n) ~ a tower of 2s of height 2^2^...^2. Its inverse alpha(m, n) is at most k when n <= A(k, floor(m/n)). Since A(4, 1) = 65533, we need n > 65533 for alpha to even potentially reach 5, and that only happens when m/n is very small. For all practical purposes, alpha(m, n) <= 4. For n <= 65536, alpha <= 3. In practice, the "improvement" from O(m * alpha) to O(m) saves a constant factor of at most 3-4 on the query portion of the work, while the microset/answer-table machinery adds a 5-8x overhead per query.

### Static variant vs standard union-find

Benchmarked against [`disjoint-sets`](https://crates.io/crates/disjoint-sets) and [`union-find`](https://crates.io/crates/union-find) crates on random trees (construct + n/2 links + n finds). Numbers are approximate and vary across runs and hardware:

| n         | StaticTreeSetUnion | disjoint-sets | union-find | slowdown |
| --------- | ------------------ | ------------- | ---------- | -------- |
| 1,000     | 32 us              | 3.5 us        | 3.6 us     | 9x       |
| 10,000    | 568 us             | 59 us         | 66 us      | 10x      |
| 100,000   | 7.8 ms             | 1.25 ms       | 1.44 ms    | 6x       |
| 1,000,000 | 225 ms             | 25 ms         | 32 ms      | 9x       |

![Static comparison](https://raw.githubusercontent.com/LucaCappelletti94/incremental-tree-set-union/main/docs/comparison.svg)

### Static variant, query-only (construction excluded)

| n         | StaticTreeSetUnion | disjoint-sets | union-find | slowdown |
| --------- | ------------------ | ------------- | ---------- | -------- |
| 1,000     | 5.0 us             | 0.79 us       | 0.72 us    | 7x       |
| 10,000    | 79 us              | 9.2 us        | 9.2 us     | 8.6x     |
| 100,000   | 2.0 ms             | 414 us        | 417 us     | 4.8x     |
| 1,000,000 | 64 ms              | 6.6 ms        | 6.3 ms     | 10x      |

Even with construction excluded, the per-find overhead from dependent cache misses keeps the algorithm 5-10x slower. The gap narrows at 100K (where data structures partially fit in L3 cache) but widens again at 1M when they spill to DRAM and the 3:1 load ratio per find fully manifests.

![Query-only comparison](https://raw.githubusercontent.com/LucaCappelletti94/incremental-tree-set-union/main/docs/query_only.svg)

### Incremental variant vs standard union-find

The incremental variant is substantially slower because every `grow` call does an O(b) preorder rebuild of the microset (see [Complexity deviation](#complexity-deviation-in-the-incremental-variant)):

| n       | IncrementalTreeSetUnion | disjoint-sets | union-find | slowdown |
| ------- | ----------------------- | ------------- | ---------- | -------- |
| 1,000   | 376 us                  | 3.1 us        | 3.9 us     | 120x     |
| 10,000  | 4.7 ms                 | 48 us         | 74 us      | 98x      |
| 100,000 | 58 ms                  | 1.19 ms       | 1.45 ms    | 49x      |

![Incremental comparison](https://raw.githubusercontent.com/LucaCappelletti94/incremental-tree-set-union/main/docs/incremental_comparison.svg)

## Complexity deviation in the incremental variant

The paper's Section 3 states that `grow` is O(1) amortized (without a split) and O(b) when a split occurs, giving O(m + n) total time. Our implementation does **O(b) for every grow** because the forest encoding requires a full preorder rebuild when a child is added to an interior node.

The paper uses a parent-table encoding (`parent(i, j) = k`) that can be extended in O(1) by setting one entry. We use the forest encoding (unary child-count bitmask, following the janezb C++ implementation) instead. Adding a child changes a node's child count, which shifts all subsequent bits in the bitmask and requires a complete rebuild.

We chose the forest encoding because the parent table makes the answer table dramatically larger. The parent table requires `(b-1) * ceil(log2(b))` bits per microset structure, versus `2b - 3` bits for the forest encoding. This means the answer table indexed by `(structure, mark)` grows from `2^(3b-4)` entries to `2^((b-1)(ceil(log2(b))+1))` entries:

| Encoding                   | Grow cost      | Table size (b=5)  | Table size (b=7)    |
| -------------------------- | -------------- | ----------------- | ------------------- |
| Parent table (paper)       | O(1) amortized | 65,536 entries    | 16,777,216 entries  |
| Forest bitmask (this impl) | O(b) per grow  | 2,048 entries     | 131,072 entries     |
| Ratio                      |                | 32x larger        | 128x larger         |

At b=7 (n around 25M nodes), the parent-table answer table would consume 128 MB of memory (16M entries * 8 bytes). The forest encoding needs only 1 MB. Both are still O(n) asymptotically, but the constant factor makes the parent table impractical for large inputs. The paper itself acknowledges this tradeoff on page 216: it presents the forest encoding as a way to "choose a larger value of b" with a more compact representation.

Our total time for the incremental variant is **O(m + n log log n)** instead of the paper's O(m + n), where the extra log log n factor comes from b = Theta(log log n). The static variant is unaffected and achieves the paper's O(m + n) bound.

## Paper bugs discovered

Two bugs in the published pseudocode of `GT85` were independently confirmed by both existing reference implementations:

### Bug 1: MacroFind vs MicroFind in `find` (p.217, line 7)

The paper's line 7 says `macrounite(macrofind(x), x)`. The correct version (from the earlier `GT83` technical report, p.248) is `macrounite(microfind(x), x)`. Using MacroFind can cause an infinite loop.

- **janezb (C++)**: *"Note: in line 7 of 'find', GT85 (p. 217) has MacroFind, while GT83 (p. 248) has MicroFind; the latter is correct, while the former can cause an endless loop."*
- **bzliu94 (Python)**: *"there is a bug from pseudocode here for first argument"*

### Bug 2: macro_top continuation (unstated requirement)

After `MacroUnite`, the algorithm must continue from the node closest to the tree root in the macroset component, not from an arbitrary union-find representative. Neither `GT85` nor `GT83` states this requirement.

- **janezb (C++)**: *"An important detail mentioned in neither GT85 nor GT83: we need to continue from the highest (i.e. closest to root) member of the macroset."*
- **bzliu94 (Python)**: achieves the same effect via `NamedUnionFind` where the canonical representative is always the highest element by design.

## Implementation notes

- `no_std` compatible with `alloc` (edition 2024, rust-version 1.85)
- Forest encoding (janezb style) instead of the paper's parent table: produces a 32-128x smaller answer table but prevents O(1) incremental grow
- Mark bit convention: bit = 0 means node is a set name (marked); bit = 1 means linked (unmarked)
- Answer table: 2^(3b-4) entries of `u64`, constructed in O(n) time
- Macroset: standard union-find with path compression, union by size, and explicit `macro_top` tracking

## Testing

- 51 tests across 6 files (4 integration test files + 2 unit test modules + 2 doc-tests from this README): correctness properties, edge cases (single node, path, star, binary tree, adversarial grow, n=100K), golden vectors verified against the bzliu94 Python reference implementation, and randomized tests with dual oracle (standard union-find + naive walk-up)
- 3 cargo-fuzz targets (`fuzz_static`, `fuzz_incremental`, `fuzz_both`) checking against a union-find oracle and naive walk-up oracle simultaneously, with structured tree generation (random, path, star, caterpillar) and phased operation modes (link-all-then-find-all)
- 100% line coverage (tarpaulin LLVM)

```bash
cargo test           # run all tests
./fuzz.sh            # start all 3 fuzzers in tmux panels
cargo bench          # run benchmarks
```

## Applications

The paper (Section 4) lists ten applications where tree set union removes an inverse-Ackermann or iterated-logarithm factor from the previously best-known algorithm. In all cases the improvement is **purely theoretical**: the removed factor is at most 4 (alpha) or 5 (log*) for any practical input, and the constant-factor overhead of this implementation makes it slower than standard union-find in practice.

Applications (1)-(5) use the static variant on path-shaped trees. Applications (6)-(8) use the static variant on general trees. Application (9) uses the incremental variant. Application (10) uses an extended static variant.

1. Two-processor scheduling
2. Multiprocessor scheduling (priority list and interval dag)
3. The off-line min problem
4. Matching on convex graphs and scheduling with release times and deadlines
5. VLSI channel routing
6. Nearest common ancestors (off-line)
7. Flow graph reducibility
8. Two directed spanning trees
9. Maximum cardinality matching on nonbipartite graphs
10. The set-splitting problem

Application (9) is the motivation for this crate: Blum's O(sqrt(n)(m+n)) maximum matching algorithm uses incremental tree set union to state its time bound without an alpha factor. Gabow and Tarjan also observe (p.219) that substituting tree set union into the Micali-Vazirani matching algorithm would give the O(sqrt(n)m) bound directly, avoiding a separate 50-page argument about blossom structure. Micali-Vazirani themselves did not use tree set union; they used standard union-find and proved the blossom property independently.

## References

- H. N. Gabow and R. E. Tarjan, "A linear-time algorithm for a special case of disjoint set union," *J. Comput. Syst. Sci.*, vol. 30, pp. 209-221, 1985.
- H. N. Gabow and R. E. Tarjan, "A linear-time algorithm for a special case of disjoint set union," *Proc. 15th Annual ACM Symposium on Theory of Computing*, 1983, pp. 246-251. (Earlier version with corrected pseudocode.)
- N. Blum, "Maximum Matching in General Graphs Without Explicit Consideration of Blossoms Revisited," full version, Uni Bonn, July 2016.

## License

MIT
