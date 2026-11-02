//! `Bounds2D` from `src/core/bounds.h:34-110` (the GL `draw` is render-side and omitted).
//! `first` is kept: an empty box contains nothing and the first `update` seeds min and max.

use glam::Vec2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds2D {
    pub min: Vec2,
    pub max: Vec2,
    pub first: bool,
}

impl Default for Bounds2D {
    fn default() -> Self {
        Bounds2D {
            min: Vec2::ZERO,
            max: Vec2::ZERO,
            first: true,
        }
    }
}

impl Bounds2D {
    /// Two-point constructor: normalises so `min <= max` per axis (via `update`).
    pub fn new(a: Vec2, b: Vec2) -> Self {
        let mut r = Self::default();
        r.update(a);
        r.update(b);
        r
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn centre(&self) -> Vec2 {
        self.min + (self.max - self.min) * 0.5
    }

    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    pub fn area(&self) -> f32 {
        self.width() * self.height()
    }

    pub fn update(&mut self, p: Vec2) {
        if self.first {
            self.min = p;
            self.max = p;
            self.first = false;
            return;
        }
        if self.min.x > p.x {
            self.min.x = p.x;
        }
        if self.min.y > p.y {
            self.min.y = p.y;
        }
        if self.max.x < p.x {
            self.max.x = p.x;
        }
        if self.max.y < p.y {
            self.max.y = p.y;
        }
    }

    /// Grow to include `b`'s corners. As in C++ this ignores `b.first`, so an empty `b`
    /// contributes its (0,0) corners.
    pub fn update_bounds(&mut self, b: &Bounds2D) {
        self.update(b.min);
        self.update(b.max);
    }

    pub fn set_bounds(&mut self, b: &Bounds2D) {
        self.reset();
        self.update_bounds(b);
    }

    pub fn set_point(&mut self, p: Vec2) {
        self.reset();
        self.update(p);
    }

    /// Closed interval test; an unseeded (`first`) box contains nothing.
    pub fn contains(&self, p: Vec2) -> bool {
        !self.first
            && self.min.x <= p.x
            && self.min.y <= p.y
            && self.max.x >= p.x
            && self.max.y >= p.y
    }

    /// Closed interval overlap. Does NOT look at `first` (C++ doesn't either).
    pub fn overlaps(&self, b: &Bounds2D) -> bool {
        !(self.max.y < b.min.y
            || self.min.y > b.max.y
            || self.max.x < b.min.x
            || self.min.x > b.max.x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::vec2;

    #[test]
    fn two_point_ctor_normalises() {
        let b = Bounds2D::new(vec2(3.0, -1.0), vec2(-2.0, 4.0));
        assert_eq!((b.min, b.max), (vec2(-2.0, -1.0), vec2(3.0, 4.0)));
        assert_eq!((b.width(), b.height(), b.area()), (5.0, 5.0, 25.0));
        assert_eq!(b.centre(), vec2(0.5, 1.5));
    }

    #[test]
    fn empty_contains_nothing_but_still_overlaps() {
        let e = Bounds2D::default();
        assert!(!e.contains(Vec2::ZERO));
        assert!(e.overlaps(&Bounds2D::new(vec2(-1.0, -1.0), vec2(1.0, 1.0))));
    }

    #[test]
    fn closed_intervals() {
        let a = Bounds2D::new(Vec2::ZERO, vec2(1.0, 1.0));
        assert!(
            a.contains(vec2(1.0, 0.0))
                && a.contains(vec2(0.0, 0.0))
                && a.contains(vec2(0.0, 1.0))
                && a.contains(vec2(1.0, 1.0))
        );
        assert!(!a.contains(vec2(1.0001, 0.0)));
        let touching = Bounds2D::new(vec2(1.0, 1.0), vec2(2.0, 2.0));
        assert!(a.overlaps(&touching));
        assert!(!a.overlaps(&Bounds2D::new(vec2(1.5, 0.0), vec2(2.0, 1.0))));
    }

    #[test]
    fn set_and_update() {
        let mut b = Bounds2D::default();
        b.set_point(vec2(5.0, 5.0));
        b.update(vec2(1.0, 9.0));
        assert_eq!((b.min, b.max), (vec2(1.0, 5.0), vec2(5.0, 9.0)));
        let mut c = Bounds2D::default();
        c.update(vec2(100.0, 100.0));
        c.set_bounds(&b);
        assert_eq!(c, b);
    }
}
