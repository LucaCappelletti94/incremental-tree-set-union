#!/usr/bin/env python3
"""
Generate benchmark plots from criterion JSON output.

Usage:
  cargo bench                    # run benchmarks first
  python3 benches/plot_benchmarks.py   # generate plots in docs/

Reads from target/criterion/*/new/estimates.json
Writes SVG plots to docs/
"""

import json
import sys
from pathlib import Path

try:
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
except ImportError:
    print("matplotlib is required: pip install matplotlib", file=sys.stderr)
    sys.exit(1)

CRITERION_DIR = Path("target/criterion")
DOCS_DIR = Path("docs")


def read_estimate(group: str, bench: str, param: str) -> float | None:
    """Read the mean point estimate (in nanoseconds) for a benchmark."""
    path = CRITERION_DIR / group / bench / param / "new" / "estimates.json"
    if not path.exists():
        return None
    with open(path) as f:
        data = json.load(f)
    return data["mean"]["point_estimate"]


def read_group(group: str, bench: str, params: list[str]) -> dict[str, float]:
    results = {}
    for p in params:
        val = read_estimate(group, bench, p)
        if val is not None:
            results[p] = val
    return results


STYLE = {
    "figure.facecolor": "white",
    "axes.facecolor": "#f8f8f8",
    "axes.grid": True,
    "grid.alpha": 0.3,
    "font.size": 11,
    "axes.titlesize": 13,
    "axes.labelsize": 11,
}

COLORS = {
    "gt_static": "#d62728",
    "gt_incr": "#e377c2",
    "disjoint_sets": "#2ca02c",
    "union_find": "#1f77b4",
}


# ── Plot 1: Static comparison ─────────────────────────────────────────────

def plot_comparison():
    """Static variant vs standard union-find crates."""
    sizes = ["1000", "10000", "100000", "1000000"]

    gt = read_group("comparison", "gabow_tarjan", sizes)
    ds = read_group("comparison", "disjoint_sets", sizes)
    uf = read_group("comparison", "union_find_crate", sizes)

    if not gt or not ds:
        print("  SKIP: comparison data missing", file=sys.stderr)
        return

    with plt.rc_context(STYLE):
        fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(13, 5))

        ns = sorted(gt.keys(), key=int)
        x = [int(n) for n in ns]
        ax1.plot(x, [gt[n] / 1e6 for n in ns], "o-",
                 label="Gabow-Tarjan (static)", color=COLORS["gt_static"], linewidth=2)
        ax1.plot(x, [ds[n] / 1e6 for n in ns if n in ds], "s-",
                 label="disjoint-sets crate", color=COLORS["disjoint_sets"], linewidth=2)
        ax1.plot(x, [uf[n] / 1e6 for n in ns if n in uf], "^-",
                 label="union-find crate", color=COLORS["union_find"], linewidth=2)
        ax1.set_xscale("log")
        ax1.set_yscale("log")
        ax1.set_xlabel("n (nodes)")
        ax1.set_ylabel("Time (ms)")
        ax1.set_title("StaticTreeSetUnion vs standard union-find\n(construct + n/2 links + n finds)")
        ax1.legend(fontsize=9)

        # Right: ratio
        ratios = []
        x_ratio = []
        for n in ns:
            if n in ds and n in uf:
                best = min(ds[n], uf[n])
                ratios.append(gt[n] / best)
                x_ratio.append(int(n))
        ax2.bar([str(x) for x in x_ratio], ratios, color=COLORS["gt_static"], alpha=0.8)
        ax2.set_xlabel("n (nodes)")
        ax2.set_ylabel("Slowdown vs fastest standard UF")
        ax2.set_title("Static variant overhead")
        ax2.axhline(y=1, color="green", linestyle="--", alpha=0.5)
        for i, r in enumerate(ratios):
            ax2.text(i, r + 0.3, f"{r:.1f}x", ha="center", fontsize=10)

        fig.tight_layout()
        fig.savefig(DOCS_DIR / "comparison.svg", format="svg", bbox_inches="tight")
        print("  wrote docs/comparison.svg")
        plt.close(fig)


# ── Plot 2: b parameter effect ─────────────────────────────────────────────

def plot_b_parameter():
    """Time per node at different b thresholds (static variant)."""
    cases = [
        ("n=50_b=2", 50, 2),
        ("n=63_b=2", 63, 2),
        ("n=64_b=3", 64, 3),
        ("n=500_b=3", 500, 3),
        ("n=767_b=3", 767, 3),
        ("n=768_b=4", 768, 4),
        ("n=5000_b=4", 5_000, 4),
        ("n=16383_b=4", 16_383, 4),
        ("n=16384_b=5", 16_384, 5),
        ("n=100000_b=5", 100_000, 5),
    ]

    data = read_group(
        "static_b_parameter", "find_after_full_link",
        [label for label, _, _ in cases]
    )
    if not data:
        data = read_group(
            "static/b_parameter", "find_after_full_link",
            [label for label, _, _ in cases]
        )
    if not data:
        print("  SKIP: b_parameter data missing", file=sys.stderr)
        return

    b_colors = {2: "#1f77b4", 3: "#ff7f0e", 4: "#2ca02c", 5: "#d62728"}

    with plt.rc_context(STYLE):
        fig, ax = plt.subplots(figsize=(10, 5))

        ns = []
        time_per_n = []
        colors = []
        for label, n, b in cases:
            if label in data:
                ns.append(n)
                time_per_n.append(data[label] / n)
                colors.append(b_colors.get(b, "gray"))

        ax.scatter(ns, time_per_n, c=colors, s=80, zorder=5)
        ax.plot(ns, time_per_n, "k-", alpha=0.3, linewidth=1)
        ax.set_xscale("log")
        ax.set_xlabel("n (nodes)")
        ax.set_ylabel("Time per node (ns)")
        ax.set_title("StaticTreeSetUnion: effect of b parameter\n(link all + find all)")

        for label, n, b in cases:
            if label in data:
                ax.annotate(f"b={b}", (n, data[label] / n),
                           textcoords="offset points", xytext=(5, 8),
                           fontsize=8, color=b_colors.get(b, "gray"))

        for threshold_n, b_label in [(64, "b: 2->3"), (768, "b: 3->4"), (16384, "b: 4->5")]:
            ax.axvline(x=threshold_n, color="gray", linestyle=":", alpha=0.5)
            ax.text(threshold_n * 1.1, ax.get_ylim()[1] * 0.9, b_label,
                   fontsize=8, color="gray")

        fig.tight_layout()
        fig.savefig(DOCS_DIR / "b_parameter.svg", format="svg", bbox_inches="tight")
        print("  wrote docs/b_parameter.svg")
        plt.close(fig)


# ── Plot 3: Query-only comparison ──────────────────────────────────────────

def plot_query_only():
    """Find-only time, construction excluded (static variant)."""
    sizes = ["1000", "10000", "100000"]

    gt = read_group("query_only", "gabow_tarjan", sizes)
    ds = read_group("query_only", "disjoint_sets", sizes)
    uf = read_group("query_only", "union_find_crate", sizes)

    if not gt or not ds:
        print("  SKIP: query_only data missing", file=sys.stderr)
        return

    with plt.rc_context(STYLE):
        fig, ax = plt.subplots(figsize=(8, 5))

        ns = sorted(gt.keys(), key=int)
        x = [int(n) for n in ns]

        ax.plot(x, [gt[n] / int(n) for n in ns], "o-",
                label="Gabow-Tarjan (static)", color=COLORS["gt_static"], linewidth=2)
        ax.plot(x, [ds[n] / int(n) for n in ns if n in ds], "s-",
                label="disjoint-sets crate", color=COLORS["disjoint_sets"], linewidth=2)
        ax.plot(x, [uf[n] / int(n) for n in ns if n in uf], "^-",
                label="union-find crate", color=COLORS["union_find"], linewidth=2)

        ax.set_xscale("log")
        ax.set_xlabel("n (nodes)")
        ax.set_ylabel("Time per find (ns)")
        ax.set_title("StaticTreeSetUnion: query-only cost\n(n finds after n/2 links, construction excluded)")
        ax.legend(fontsize=9)

        fig.tight_layout()
        fig.savefig(DOCS_DIR / "query_only.svg", format="svg", bbox_inches="tight")
        print("  wrote docs/query_only.svg")
        plt.close(fig)


# ── Plot 4: Incremental comparison ─────────────────────────────────────────

def plot_incremental():
    """Incremental variant vs standard union-find crates."""
    sizes = ["1000", "10000", "100000"]

    gt = read_group("incremental_comparison", "gabow_tarjan_incr", sizes)
    ds = read_group("incremental_comparison", "disjoint_sets", sizes)
    uf = read_group("incremental_comparison", "union_find_crate", sizes)

    if not gt or not ds:
        print("  SKIP: incremental_comparison data missing", file=sys.stderr)
        return

    with plt.rc_context(STYLE):
        fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(13, 5))

        ns = sorted(gt.keys(), key=int)
        x = [int(n) for n in ns]
        ax1.plot(x, [gt[n] / 1e6 for n in ns], "o-",
                 label="Gabow-Tarjan (incremental)", color=COLORS["gt_incr"], linewidth=2)
        ax1.plot(x, [ds[n] / 1e6 for n in ns if n in ds], "s-",
                 label="disjoint-sets crate", color=COLORS["disjoint_sets"], linewidth=2)
        ax1.plot(x, [uf[n] / 1e6 for n in ns if n in uf], "^-",
                 label="union-find crate", color=COLORS["union_find"], linewidth=2)
        ax1.set_xscale("log")
        ax1.set_yscale("log")
        ax1.set_xlabel("n (nodes)")
        ax1.set_ylabel("Time (ms)")
        ax1.set_title("IncrementalTreeSetUnion vs standard union-find\n(grow all + n/2 links + n finds)")
        ax1.legend(fontsize=9)

        # Right: ratio
        ratios = []
        x_ratio = []
        for n in ns:
            if n in ds and n in uf:
                best = min(ds[n], uf[n])
                ratios.append(gt[n] / best)
                x_ratio.append(int(n))
        ax2.bar([str(x) for x in x_ratio], ratios, color=COLORS["gt_incr"], alpha=0.8)
        ax2.set_xlabel("n (nodes)")
        ax2.set_ylabel("Slowdown vs fastest standard UF")
        ax2.set_title("Incremental variant overhead")
        ax2.axhline(y=1, color="green", linestyle="--", alpha=0.5)
        for i, r in enumerate(ratios):
            ax2.text(i, r + 2, f"{r:.0f}x", ha="center", fontsize=10)

        fig.tight_layout()
        fig.savefig(DOCS_DIR / "incremental_comparison.svg", format="svg", bbox_inches="tight")
        print("  wrote docs/incremental_comparison.svg")
        plt.close(fig)


# ── Main ────────────────────────────────────────────────────────────────────

if __name__ == "__main__":
    if not CRITERION_DIR.exists():
        print(f"No benchmark data at {CRITERION_DIR}. Run `cargo bench` first.",
              file=sys.stderr)
        sys.exit(1)

    DOCS_DIR.mkdir(exist_ok=True)
    print("Generating plots...")
    plot_comparison()
    plot_b_parameter()
    plot_query_only()
    plot_incremental()
    print("Done.")
