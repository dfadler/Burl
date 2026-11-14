//! Port of `src/core/quadtree.cpp` as an arena, storing caller ids instead of `QuadItem*`.
//!
//! What the simulation depends on, and what is preserved:
//! - leaf item lists keep insertion order (`std::list` push_back); children are TL, TR, BL, BR;
//! - bounds tests are closed intervals, and an item is added to EVERY overlapping child, so an item
//!   can sit in several leaves. [`QuadTree::visit_items_in_bounds`] reports each occurrence together
//!   with the item's leaf count (`node_count`); callers (`UserForceFunctor`, `DirForceFunctor`) dedupe
//!   on `node_count != 1` themselves, exactly as the C++ functors do;
//! - an item overlapping no child at split time is silently dropped;
//! - the root has depth 1, and a leaf at `depth >= max_depth` accepts unlimited items.
//!
//! Only the queries the simulation calls are ported (`visitItemsInBounds`, `visitItemsAt`). The
//! `std::set` variants, frustum queries and GL display lists have no sim callers (core-lib.md 9(c));
//! the renderer owns its own culling.

use crate::bounds::Bounds2D;
use glam::{vec2, Vec2};

struct Node {
    bounds: Bounds2D,
    depth: i32,
    children: Option<[usize; 4]>,
    /// Indices into `QuadTree::entries`, in insertion order.
    items: Vec<usize>,
}

impl Node {
    fn empty(&self) -> bool {
        self.items.is_empty() && self.children.is_none()
    }
}

struct Entry<T> {
    id: T,
    bounds: Bounds2D,
    node_count: u32,
}

pub struct QuadTree<T> {
    nodes: Vec<Node>,
    entries: Vec<Entry<T>>,
    max_node_depth: i32,
    max_node_items: usize,
    /// Leaf-list entries currently stored (an item in k leaves counts k times).
    pub item_count: i32,
    pub unique_item_count: i32,
    pub node_count: i32,
}

impl<T: Copy> QuadTree<T> {
    pub fn new(bounds: Bounds2D, max_node_depth: i32, max_node_items: usize) -> Self {
        let mut t = QuadTree {
            nodes: Vec::new(),
            entries: Vec::new(),
            max_node_depth,
            max_node_items,
            item_count: 0,
            unique_item_count: 0,
            node_count: 0,
        };
        t.new_node(bounds, 0);
        t
    }

    fn new_node(&mut self, bounds: Bounds2D, parent_depth: i32) -> usize {
        self.nodes.push(Node {
            bounds,
            depth: parent_depth + 1,
            children: None,
            items: Vec::new(),
        });
        self.node_count += 1;
        self.nodes.len() - 1
    }

    pub fn add_item(&mut self, id: T, bounds: Bounds2D) {
        self.entries.push(Entry {
            id,
            bounds,
            node_count: 0,
        });
        let e = self.entries.len() - 1;
        self.add(0, e);
        self.unique_item_count += 1;
    }

    fn allow_more_items(&self, n: usize) -> bool {
        let node = &self.nodes[n];
        node.children.is_none()
            && (node.depth >= self.max_node_depth || node.items.len() < self.max_node_items)
    }

    fn add(&mut self, n: usize, e: usize) {
        if self.allow_more_items(n) {
            self.item_count += 1;
            self.entries[e].node_count += 1;
            self.nodes[n].items.push(e);
            return;
        }
        if self.nodes[n].children.is_some() {
            self.add_to_child(n, e);
            return;
        }

        let b = self.nodes[n].bounds;
        let depth = self.nodes[n].depth;
        let middle = b.centre() - b.min;
        let relmax = b.max - b.min;
        let quads = [
            (b.min, b.min + middle), // top left
            (
                b.min + vec2(middle.x, 0.0),
                b.min + vec2(relmax.x, middle.y),
            ), // top right
            (
                b.min + vec2(0.0, middle.y),
                b.min + vec2(middle.x, relmax.y),
            ), // bottom left
            (b.min + middle, b.max), // bottom right
        ];
        let mut ids = [0; 4];
        for (i, (lo, hi)) in quads.into_iter().enumerate() {
            ids[i] = self.new_node(Bounds2D::new(lo, hi), depth);
        }
        self.nodes[n].children = Some(ids);

        for oi in std::mem::take(&mut self.nodes[n].items) {
            self.item_count -= 1;
            self.entries[oi].node_count -= 1;
            self.add_to_child(n, oi);
        }
        self.add_to_child(n, e);
    }

    fn add_to_child(&mut self, n: usize, e: usize) {
        let Some(children) = self.nodes[n].children else {
            return;
        };
        for c in children {
            if self.nodes[c].bounds.overlaps(&self.entries[e].bounds) {
                self.add(c, e);
            }
        }
    }

    /// `visitItemsInBounds`: a non-empty node is visited as a leaf, wholesale, with no per-item test.
    /// `visit` gets the id and that item's leaf count.
    pub fn visit_items_in_bounds(&self, bounds: &Bounds2D, mut visit: impl FnMut(T, u32)) {
        self.in_bounds(0, bounds, &mut visit);
    }

    fn in_bounds(&self, n: usize, bounds: &Bounds2D, visit: &mut impl FnMut(T, u32)) {
        let node = &self.nodes[n];
        if !node.items.is_empty() {
            for &e in &node.items {
                visit(self.entries[e].id, self.entries[e].node_count);
            }
        } else if let Some(children) = node.children {
            for c in children {
                if !self.nodes[c].empty() && bounds.overlaps(&self.nodes[c].bounds) {
                    self.in_bounds(c, bounds, visit);
                }
            }
        }
    }

    /// `visitItemsAt`: descend the single child whose closed bounds first contain `pos`.
    pub fn visit_items_at(&self, pos: Vec2, mut visit: impl FnMut(T, u32)) {
        let mut n = 0;
        loop {
            let node = &self.nodes[n];
            if !node.items.is_empty() {
                for &e in &node.items {
                    visit(self.entries[e].id, self.entries[e].node_count);
                }
                return;
            }
            let Some(children) = node.children else {
                return;
            };
            match children
                .into_iter()
                .find(|&c| self.nodes[c].bounds.contains(pos))
            {
                Some(c) => n = c,
                None => return,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bb(x0: f32, y0: f32, x1: f32, y1: f32) -> Bounds2D {
        Bounds2D::new(vec2(x0, y0), vec2(x1, y1))
    }

    fn collect(t: &QuadTree<u32>, q: &Bounds2D) -> Vec<(u32, u32)> {
        let mut v = Vec::new();
        t.visit_items_in_bounds(q, |id, n| v.push((id, n)));
        v
    }

    #[test]
    fn depth_one_is_a_single_unbounded_leaf_in_insertion_order() {
        let mut t = QuadTree::new(bb(-100.0, -100.0, 100.0, 100.0), 1, 1);
        for (i, x) in [50.0, -50.0, 0.0, 70.0].into_iter().enumerate() {
            t.add_item(i as u32, bb(x - 1.0, 0.0, x + 1.0, 1.0));
        }
        assert_eq!(t.node_count, 1);
        assert_eq!(
            collect(&t, &bb(-1000.0, -1000.0, 1000.0, 1000.0)),
            [(0, 1), (1, 1), (2, 1), (3, 1)]
        );
    }

    #[test]
    fn splits_visit_tl_tr_bl_br_regardless_of_insertion_order() {
        let mut t = QuadTree::new(bb(0.0, 0.0, 100.0, 100.0), 6, 1);
        // insert BR, BL, TR, TL
        t.add_item(3, bb(70.0, 70.0, 71.0, 71.0));
        t.add_item(2, bb(10.0, 70.0, 11.0, 71.0));
        t.add_item(1, bb(70.0, 10.0, 71.0, 11.0));
        t.add_item(0, bb(10.0, 10.0, 11.0, 11.0));
        assert!(t.node_count > 1);
        assert_eq!(
            collect(&t, &bb(0.0, 0.0, 100.0, 100.0)),
            [(0, 1), (1, 1), (2, 1), (3, 1)]
        );
    }

    #[test]
    fn split_reinserts_existing_items_in_list_order() {
        // capacity 2: the third item splits the root; items 0 and 1 both land in TL and must keep order.
        let mut t = QuadTree::new(bb(0.0, 0.0, 100.0, 100.0), 2, 2);
        t.add_item(0, bb(10.0, 10.0, 11.0, 11.0));
        t.add_item(1, bb(12.0, 10.0, 13.0, 11.0));
        t.add_item(2, bb(90.0, 90.0, 91.0, 91.0));
        t.add_item(3, bb(14.0, 10.0, 15.0, 11.0));
        assert_eq!(
            collect(&t, &bb(0.0, 0.0, 30.0, 30.0)),
            [(0, 1), (1, 1), (3, 1)]
        );
    }

    #[test]
    fn item_spanning_leaves_reported_per_leaf_with_node_count() {
        let mut t = QuadTree::new(bb(0.0, 0.0, 100.0, 100.0), 6, 1);
        t.add_item(7, bb(40.0, 10.0, 60.0, 11.0)); // straddles x=50: TL and TR
        t.add_item(8, bb(10.0, 10.0, 11.0, 11.0));
        let got = collect(&t, &bb(0.0, 0.0, 100.0, 100.0));
        assert!(
            got.iter().filter(|&&(id, n)| id == 7 && n == 2).count() == 2,
            "{got:?}"
        );
        assert_eq!(t.unique_item_count, 2);
    }

    #[test]
    fn closed_bounds_item_on_split_line_lands_in_both_children() {
        let mut t = QuadTree::new(bb(0.0, 0.0, 100.0, 100.0), 6, 1);
        t.add_item(0, bb(50.0, 10.0, 50.0, 10.0));
        t.add_item(1, bb(10.0, 10.0, 11.0, 11.0));
        assert_eq!(collect(&t, &bb(49.0, 9.0, 51.0, 11.0)), [(0, 2), (0, 2)]);
    }

    #[test]
    fn item_outside_root_is_dropped_after_split() {
        let mut t = QuadTree::new(bb(0.0, 0.0, 100.0, 100.0), 6, 1);
        t.add_item(0, bb(10.0, 10.0, 11.0, 11.0));
        t.add_item(1, bb(20.0, 20.0, 21.0, 21.0)); // forces split
        t.add_item(2, bb(500.0, 500.0, 501.0, 501.0)); // overlaps no child
        assert_eq!(t.unique_item_count, 3);
        let got = collect(&t, &bb(-1000.0, -1000.0, 1000.0, 1000.0));
        assert!(got.iter().all(|&(id, _)| id != 2));
    }

    #[test]
    fn query_skips_non_overlapping_children() {
        let mut t = QuadTree::new(bb(0.0, 0.0, 100.0, 100.0), 6, 1);
        t.add_item(0, bb(10.0, 10.0, 11.0, 11.0));
        t.add_item(3, bb(90.0, 90.0, 91.0, 91.0));
        assert_eq!(collect(&t, &bb(85.0, 85.0, 95.0, 95.0)), [(3, 1)]);
    }

    #[test]
    fn visit_at_follows_one_path() {
        let mut t = QuadTree::new(bb(0.0, 0.0, 100.0, 100.0), 6, 1);
        t.add_item(0, bb(10.0, 10.0, 11.0, 11.0));
        t.add_item(1, bb(90.0, 90.0, 91.0, 91.0));
        let mut got = Vec::new();
        t.visit_items_at(vec2(95.0, 95.0), |id, _| got.push(id));
        assert_eq!(got, [1]);
        got.clear();
        t.visit_items_at(vec2(500.0, 0.0), |id, _| got.push(id));
        assert!(got.is_empty());
    }

    #[test]
    fn max_depth_leaf_is_unbounded() {
        // depth 2 = root + one level; items piling into one quadrant must not split further.
        let mut t = QuadTree::new(bb(0.0, 0.0, 100.0, 100.0), 2, 1);
        for i in 0..5 {
            t.add_item(i, bb(10.0 + i as f32, 10.0, 11.0 + i as f32, 11.0));
        }
        assert_eq!(t.node_count, 5);
        assert_eq!(t.item_count, 5);
    }
}
