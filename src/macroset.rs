use alloc::vec;
use alloc::vec::Vec;

/// Standard union-find (disjoint set forest) with path compression,
/// union by size, and `macro_top` tracking.
///
/// Used for cross-microset queries. Each microset root participates
/// as a node in the macroset. `macro_top` tracks the node closest to
/// the tree root (lowest level) in each component.
#[derive(Clone, Debug)]
pub(crate) struct Macroset {
    parent: Vec<u32>,
    size: Vec<u32>,
    /// The node closest to the tree root in this macroset component.
    /// Valid only at union-find roots.
    macro_top: Vec<u32>,
    level: Vec<u32>,
}

impl Macroset {
    pub fn new(capacity: usize) -> Self {
        Self {
            parent: vec![0; capacity],
            size: vec![0; capacity],
            macro_top: vec![0; capacity],
            level: vec![0; capacity],
        }
    }

    /// Initialize node `u` as a singleton macroset with the given tree level.
    #[inline]
    pub fn make_set(&mut self, u: u32, level: u32) {
        let i = u as usize;
        self.parent[i] = u;
        self.size[i] = 1;
        self.macro_top[i] = u;
        self.level[i] = level;
    }

    /// Find with path compression. Returns the root of `u`'s component.
    #[inline]
    pub fn find(&mut self, mut u: u32) -> u32 {
        let mut root = u;
        while self.parent[root as usize] != root {
            root = self.parent[root as usize];
        }
        // Path compression
        while self.parent[u as usize] != root {
            let next = self.parent[u as usize];
            self.parent[u as usize] = root;
            u = next;
        }
        root
    }

    /// Unite the macrosets containing `u` and `v` (union by size).
    /// The first argument `u` determines the name: after the unite,
    /// `macro_top` is set to whichever root has the smaller level
    /// (closer to the tree root).
    ///
    /// Returns the new union-find root.
    #[inline]
    pub fn unite(&mut self, u: u32, v: u32) -> u32 {
        let ru = self.find(u);
        let rv = self.find(v);
        if ru == rv {
            return ru;
        }
        // Union by size: attach smaller tree under larger
        let (big, small) = if self.size[ru as usize] >= self.size[rv as usize] {
            (ru, rv)
        } else {
            (rv, ru)
        };
        self.parent[small as usize] = big;
        self.size[big as usize] += self.size[small as usize];
        // macro_top: keep the one with smaller level (closer to root)
        let top_big = self.macro_top[big as usize];
        let top_small = self.macro_top[small as usize];
        if self.level[top_small as usize] < self.level[top_big as usize] {
            self.macro_top[big as usize] = top_small;
        }
        big
    }

    /// Get the `macro_top` (node closest to tree root) for the
    /// macroset component containing `u`.
    #[inline]
    pub fn top(&mut self, u: u32) -> u32 {
        let root = self.find(u);
        self.macro_top[root as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_singleton() {
        let mut ms = Macroset::new(4);
        ms.make_set(0, 0);
        ms.make_set(1, 1);
        assert_eq!(ms.find(0), 0);
        assert_eq!(ms.find(1), 1);
        assert_eq!(ms.top(0), 0);
        assert_eq!(ms.top(1), 1);
    }

    #[test]
    fn test_unite_and_top() {
        let mut ms = Macroset::new(4);
        ms.make_set(0, 0); // level 0 (root)
        ms.make_set(1, 1); // level 1
        ms.make_set(2, 2); // level 2
        ms.make_set(3, 3); // level 3

        // Unite 2 and 3 — top should be 2 (level 2)
        ms.unite(2, 3);
        assert_eq!(ms.top(3), 2);

        // Unite 1 and 2 — top should be 1 (level 1)
        ms.unite(1, 2);
        assert_eq!(ms.top(3), 1);
        assert_eq!(ms.top(2), 1);
        assert_eq!(ms.top(1), 1);

        // Unite 0 and 1 — top should be 0 (level 0)
        ms.unite(0, 1);
        assert_eq!(ms.top(3), 0);
    }

    #[test]
    fn test_self_unite() {
        let mut ms = Macroset::new(4);
        ms.make_set(0, 0);
        ms.make_set(1, 1);
        ms.unite(0, 1);
        // Unite nodes already in the same set — should be a no-op.
        let root = ms.unite(0, 1);
        assert_eq!(ms.top(0), 0);
        assert_eq!(ms.top(1), 0);
        // The root should still be valid.
        assert_eq!(ms.find(0), root);
        assert_eq!(ms.find(1), root);
    }

    #[test]
    fn test_path_compression() {
        let mut ms = Macroset::new(8);
        for i in 0..8 {
            ms.make_set(i, i);
        }
        // Build a chain
        ms.unite(0, 1);
        ms.unite(0, 2);
        ms.unite(0, 3);
        ms.unite(0, 4);
        // All should find the same root and top should be 0
        for i in 0..5 {
            assert_eq!(ms.top(i), 0);
        }
    }
}
