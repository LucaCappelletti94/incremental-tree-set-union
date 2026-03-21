#![allow(clippy::needless_range_loop)]

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use incremental_tree_set_union::{IncrementalTreeSetUnion, StaticTreeSetUnion};
use rand::prelude::*;

use disjoint_sets::UnionFind as DsUnionFind;
use union_find::{QuickUnionUf, UnionByRank, UnionFind as UfTrait};

fn random_tree_parents(n: usize, rng: &mut impl Rng) -> Vec<usize> {
    let mut parents = vec![0usize; n];
    parents[0] = 0;
    for i in 1..n {
        parents[i] = rng.random_range(0..i);
    }
    parents
}

fn path_tree_parents(n: usize) -> Vec<usize> {
    let mut parents = vec![0usize; n];
    for i in 1..n {
        parents[i] = i - 1;
    }
    parents
}

// ── Overall scaling ────────────────────────────────────────────────────────

fn bench_static_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("static/scaling");
    for &n in &[1_000, 10_000, 100_000, 1_000_000] {
        let mut rng = StdRng::seed_from_u64(42);
        let parents = random_tree_parents(n, &mut rng);
        group.bench_with_input(BenchmarkId::new("build+link+find", n), &n, |b, &n| {
            b.iter(|| {
                let mut rng = StdRng::seed_from_u64(123);
                let mut tsu = StaticTreeSetUnion::from_parents(&parents);
                for _ in 0..n / 2 {
                    tsu.link(rng.random_range(1..n));
                }
                let mut sum = 0usize;
                for _ in 0..n {
                    sum += tsu.find(rng.random_range(0..n));
                }
                sum
            });
        });
    }
    group.finish();
}

fn bench_incremental_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental/scaling");
    for &n in &[1_000, 10_000, 100_000] {
        let mut rng = StdRng::seed_from_u64(42);
        let parents = random_tree_parents(n, &mut rng);
        group.bench_with_input(BenchmarkId::new("grow+link+find", n), &n, |b, &n| {
            b.iter(|| {
                let mut rng = StdRng::seed_from_u64(123);
                let mut itsu = IncrementalTreeSetUnion::new(n);
                for i in 1..n {
                    itsu.grow(parents[i], i);
                }
                for _ in 0..n / 2 {
                    itsu.link(rng.random_range(1..n));
                }
                let mut sum = 0usize;
                for _ in 0..n {
                    sum += itsu.find(rng.random_range(0..n));
                }
                sum
            });
        });
    }
    group.finish();
}

// ── Effect of b parameter ──────────────────────────────────────────────────
// Tests at exact threshold boundaries where b changes.
// b=2: n<64, b=3: n=64..767, b=4: n=768..16383, b=5: n=16384..524287

fn bench_b_parameter(c: &mut Criterion) {
    let mut group = c.benchmark_group("static/b_parameter");
    // Each pair: (n, expected_b)
    let cases = [
        (50, 2),      // b=2, max_nodes=1
        (63, 2),      // b=2 boundary
        (64, 3),      // b=3 onset
        (500, 3),     // b=3 mid-range
        (767, 3),     // b=3 boundary
        (768, 4),     // b=4 onset
        (5_000, 4),   // b=4 mid-range
        (16_383, 4),  // b=4 boundary
        (16_384, 5),  // b=5 onset
        (100_000, 5), // b=5 mid-range
    ];

    for &(n, expected_b) in &cases {
        let mut rng = StdRng::seed_from_u64(42);
        let parents = random_tree_parents(n, &mut rng);
        let label = format!("n={n}_b={expected_b}");
        group.bench_with_input(
            BenchmarkId::new("find_after_full_link", &label),
            &n,
            |b, &n| {
                b.iter(|| {
                    let mut tsu = StaticTreeSetUnion::from_parents(&parents);
                    // Link all non-root nodes — forces maximum macroset traversal
                    for i in 1..n {
                        tsu.link(i);
                    }
                    // Find all nodes — each must traverse macroset chain to root
                    let mut sum = 0usize;
                    for i in 0..n {
                        sum += tsu.find(i);
                    }
                    sum
                });
            },
        );
    }
    group.finish();
}

// ── Tree shape effect ──────────────────────────────────────────────────────
// Compare random vs path vs star trees at the same n.

fn bench_tree_shape(c: &mut Criterion) {
    let mut group = c.benchmark_group("static/tree_shape");
    let n = 10_000; // b=4

    let mut rng = StdRng::seed_from_u64(42);
    let random_parents = random_tree_parents(n, &mut rng);
    let path_parents = path_tree_parents(n);
    let star_parents = vec![0usize; n]; // all children of root

    for (name, parents) in [
        ("random", &random_parents),
        ("path", &path_parents),
        ("star", &star_parents),
    ] {
        group.bench_with_input(BenchmarkId::new("link_all+find_all", name), &n, |b, &n| {
            b.iter(|| {
                let mut tsu = StaticTreeSetUnion::from_parents(parents);
                for i in 1..n {
                    tsu.link(i);
                }
                let mut sum = 0usize;
                for i in 0..n {
                    sum += tsu.find(i);
                }
                sum
            });
        });
    }
    group.finish();
}

// ── Isolated operation costs ───────────────────────────────────────────────

fn bench_isolated_ops(c: &mut Criterion) {
    let n = 10_000;
    let mut rng = StdRng::seed_from_u64(42);
    let parents = random_tree_parents(n, &mut rng);

    let mut group = c.benchmark_group("static/isolated_ops");

    group.bench_function("construction", |b| {
        b.iter(|| StaticTreeSetUnion::from_parents(&parents));
    });

    group.bench_function("find_no_links", |b| {
        let mut tsu = StaticTreeSetUnion::from_parents(&parents);
        b.iter(|| {
            let mut sum = 0usize;
            for i in 0..n {
                sum += tsu.find(i);
            }
            sum
        });
    });

    group.bench_function("find_all_linked", |b| {
        b.iter(|| {
            let mut tsu = StaticTreeSetUnion::from_parents(&parents);
            for i in 1..n {
                tsu.link(i);
            }
            let mut sum = 0usize;
            for i in 0..n {
                sum += tsu.find(i);
            }
            sum
        });
    });

    group.finish();
}

// ── Query-only comparison (construction excluded) ──────────────────────────

fn bench_query_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_only");
    for &n in &[1_000, 10_000, 100_000, 1_000_000] {
        let mut rng = StdRng::seed_from_u64(42);
        let parents = random_tree_parents(n, &mut rng);

        let mut link_rng = StdRng::seed_from_u64(123);
        let link_targets: Vec<usize> = (0..n / 2).map(|_| link_rng.random_range(1..n)).collect();
        let mut find_rng = StdRng::seed_from_u64(456);
        let find_targets: Vec<usize> = (0..n).map(|_| find_rng.random_range(0..n)).collect();

        // Pre-build all structures with links applied
        let mut tsu = StaticTreeSetUnion::from_parents(&parents);
        for &v in &link_targets {
            tsu.link(v);
        }

        let mut ds = DisjointSetsBaseline::new(n);
        for &v in &link_targets {
            ds.link(v, parents[v]);
        }

        let mut uf = UnionFindBaseline::new(n);
        for &v in &link_targets {
            uf.link(v, parents[v]);
        }

        group.bench_with_input(BenchmarkId::new("gabow_tarjan", n), &n, |b, &_n| {
            b.iter(|| {
                let mut sum = 0usize;
                for &v in &find_targets {
                    sum += tsu.find(v);
                }
                sum
            });
        });

        group.bench_with_input(BenchmarkId::new("disjoint_sets", n), &n, |b, &_n| {
            b.iter(|| {
                let mut sum = 0usize;
                for &v in &find_targets {
                    sum += ds.find(v);
                }
                sum
            });
        });

        group.bench_with_input(BenchmarkId::new("union_find_crate", n), &n, |b, &_n| {
            b.iter(|| {
                let mut sum = 0usize;
                for &v in &find_targets {
                    sum += uf.find(v);
                }
                sum
            });
        });
    }
    group.finish();
}

// ── Comparison against standard union-find crates ──────────────────────────
// Both baseline crates need a name-tracking wrapper: standard UF returns an
// arbitrary representative, but tree set union returns the *nearest unlinked
// ancestor*. The wrapper maintains name[root] = nearest_unlinked_ancestor.

/// Wrapper around `disjoint-sets` crate with name tracking.
struct DisjointSetsBaseline {
    uf: DsUnionFind,
    name: Vec<usize>,
}

impl DisjointSetsBaseline {
    fn new(n: usize) -> Self {
        Self {
            uf: DsUnionFind::new(n),
            name: (0..n).collect(),
        }
    }

    fn link(&mut self, v: usize, tree_parent: usize) {
        let rv = self.uf.find(v);
        let rp = self.uf.find(tree_parent);
        if rv == rp {
            return;
        }
        let result_name = self.name[rp];
        self.uf.union(v, tree_parent);
        let new_root = self.uf.find(v);
        self.name[new_root] = result_name;
    }

    fn find(&self, v: usize) -> usize {
        let root = self.uf.find(v);
        self.name[root]
    }
}

/// Wrapper around `union-find` crate with name tracking.
struct UnionFindBaseline {
    uf: QuickUnionUf<UnionByRank>,
    name: Vec<usize>,
}

impl UnionFindBaseline {
    fn new(n: usize) -> Self {
        Self {
            uf: QuickUnionUf::<UnionByRank>::new(n),
            name: (0..n).collect(),
        }
    }

    fn link(&mut self, v: usize, tree_parent: usize) {
        let rv = self.uf.find(v);
        let rp = self.uf.find(tree_parent);
        if rv == rp {
            return;
        }
        let result_name = self.name[rp];
        self.uf.union(v, tree_parent);
        let new_root = self.uf.find(v);
        self.name[new_root] = result_name;
    }

    fn find(&mut self, v: usize) -> usize {
        let root = self.uf.find(v);
        self.name[root]
    }
}

fn bench_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("comparison");
    for &n in &[1_000, 10_000, 100_000, 1_000_000] {
        let mut rng = StdRng::seed_from_u64(42);
        let parents = random_tree_parents(n, &mut rng);

        // Pre-generate deterministic operation sequences
        let mut link_rng = StdRng::seed_from_u64(123);
        let link_targets: Vec<usize> = (0..n / 2).map(|_| link_rng.random_range(1..n)).collect();
        let mut find_rng = StdRng::seed_from_u64(456);
        let find_targets: Vec<usize> = (0..n).map(|_| find_rng.random_range(0..n)).collect();

        group.bench_with_input(BenchmarkId::new("gabow_tarjan", n), &n, |b, &_n| {
            b.iter(|| {
                let mut tsu = StaticTreeSetUnion::from_parents(&parents);
                for &v in &link_targets {
                    tsu.link(v);
                }
                let mut sum = 0usize;
                for &v in &find_targets {
                    sum += tsu.find(v);
                }
                sum
            });
        });

        group.bench_with_input(BenchmarkId::new("disjoint_sets", n), &n, |b, &_n| {
            b.iter(|| {
                let mut ds = DisjointSetsBaseline::new(n);
                for &v in &link_targets {
                    ds.link(v, parents[v]);
                }
                let mut sum = 0usize;
                for &v in &find_targets {
                    sum += ds.find(v);
                }
                sum
            });
        });

        group.bench_with_input(BenchmarkId::new("union_find_crate", n), &n, |b, &_n| {
            b.iter(|| {
                let mut uf = UnionFindBaseline::new(n);
                for &v in &link_targets {
                    uf.link(v, parents[v]);
                }
                let mut sum = 0usize;
                for &v in &find_targets {
                    sum += uf.find(v);
                }
                sum
            });
        });
    }
    group.finish();
}

// ── Incremental comparison (apples-to-apples: all dynamic) ─────────────────

fn bench_incremental_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_comparison");
    for &n in &[1_000, 10_000, 100_000] {
        let mut rng = StdRng::seed_from_u64(42);
        let parents = random_tree_parents(n, &mut rng);

        let mut link_rng = StdRng::seed_from_u64(123);
        let link_targets: Vec<usize> = (0..n / 2).map(|_| link_rng.random_range(1..n)).collect();
        let mut find_rng = StdRng::seed_from_u64(456);
        let find_targets: Vec<usize> = (0..n).map(|_| find_rng.random_range(0..n)).collect();

        group.bench_with_input(BenchmarkId::new("gabow_tarjan_incr", n), &n, |b, &n| {
            b.iter(|| {
                let mut itsu = IncrementalTreeSetUnion::new(n);
                for i in 1..n {
                    itsu.grow(parents[i], i);
                }
                for &v in &link_targets {
                    itsu.link(v);
                }
                let mut sum = 0usize;
                for &v in &find_targets {
                    sum += itsu.find(v);
                }
                sum
            });
        });

        group.bench_with_input(BenchmarkId::new("disjoint_sets", n), &n, |b, &_n| {
            b.iter(|| {
                let mut ds = DisjointSetsBaseline::new(n);
                for &v in &link_targets {
                    ds.link(v, parents[v]);
                }
                let mut sum = 0usize;
                for &v in &find_targets {
                    sum += ds.find(v);
                }
                sum
            });
        });

        group.bench_with_input(BenchmarkId::new("union_find_crate", n), &n, |b, &_n| {
            b.iter(|| {
                let mut uf = UnionFindBaseline::new(n);
                for &v in &link_targets {
                    uf.link(v, parents[v]);
                }
                let mut sum = 0usize;
                for &v in &find_targets {
                    sum += uf.find(v);
                }
                sum
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_static_scaling,
    bench_incremental_scaling,
    bench_b_parameter,
    bench_tree_shape,
    bench_isolated_ops,
    bench_query_only,
    bench_comparison,
    bench_incremental_comparison,
);
criterion_main!(benches);
