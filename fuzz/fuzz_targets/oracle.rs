/// Standard union-find oracle with set naming.
///
/// This is an independent implementation of the same abstract problem
/// that Gabow-Tarjan solves: disjoint set union on a tree, where each
/// set's name is the nearest unlinked ancestor.
///
/// Used as the ground-truth oracle in fuzz targets.
pub struct UfOracle {
    parent: Vec<usize>,
    size: Vec<usize>,
    name: Vec<usize>,
}

impl UfOracle {
    pub fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            size: vec![1; n],
            name: (0..n).collect(),
        }
    }

    fn root(&mut self, mut u: usize) -> usize {
        let mut r = u;
        while self.parent[r] != r {
            r = self.parent[r];
        }
        while self.parent[u] != r {
            let next = self.parent[u];
            self.parent[u] = r;
            u = next;
        }
        r
    }

    /// link(v): merge v's set into tree_parent's set.
    /// The resulting set's name is tree_parent's current set name.
    pub fn link(&mut self, v: usize, tree_parent: usize) {
        let rv = self.root(v);
        let rp = self.root(tree_parent);
        if rv == rp {
            return;
        }
        let result_name = self.name[rp];
        let (big, small) = if self.size[rv] >= self.size[rp] {
            (rv, rp)
        } else {
            (rp, rv)
        };
        self.parent[small] = big;
        self.size[big] += self.size[small];
        self.name[big] = result_name;
    }

    /// find(v): return the name of v's set (nearest unlinked ancestor).
    pub fn find(&mut self, v: usize) -> usize {
        let r = self.root(v);
        self.name[r]
    }
}

/// Naive walk-up oracle. O(depth) per find.
/// Serves as a second independent oracle to cross-check UfOracle.
pub fn naive_find(v: usize, parents: &[usize], linked: &[bool]) -> usize {
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
