/* SPDX-License-Identifier: (Apache-2.0 OR MIT) */
/* Copyright © 2023 Violet Leonard */

use crate::math::Polynomial;

const NEWTONS_ITERS: u8 = 4;

pub enum Segment {
    LoopPoint(f32, f32),
    Line(Line),
    Quad(QuadCurve),
    Cubic(CubicCurve),
}

impl Segment {
    pub fn point(&self, t: f32) -> (f32, f32) {
        match self {
            Self::LoopPoint(_, _) => unreachable!(),
            Self::Line(line) => line.point(t),
            Self::Quad(quad) => quad.point(t),
            Self::Cubic(curve) => curve.point(t),
        }
    }

    pub fn nearest_t(&self, point: (f32, f32)) -> f32 {
        match self {
            Self::LoopPoint(_, _) => unreachable!(),
            Self::Line(line) => line.nearest_t(point),
            Self::Quad(quad) => quad.nearest_t(point),
            Self::Cubic(curve) => curve.nearest_t(point),
        }
    }

    pub fn direction(&self, t: f32) -> (f32, f32) {
        match self {
            Self::LoopPoint(x, y) => (*x, *y),
            Self::Line(line) => line.direction(t),
            Self::Quad(quad) => quad.direction(t),
            Self::Cubic(curve) => curve.direction(t),
        }
    }

    pub fn bbox(&self) -> EdgeBoundingBox {
        match self {
            Self::LoopPoint(_, _) => unreachable!(),
            Self::Line(line) => line.bbox(),
            Self::Quad(quad) => quad.bbox(),
            Self::Cubic(curve) => curve.bbox(),
        }
    }
}

impl From<Line> for Segment {
    fn from(v: Line) -> Self {
        Self::Line(v)
    }
}

impl From<QuadCurve> for Segment {
    fn from(v: QuadCurve) -> Self {
        Self::Quad(v)
    }
}

impl From<CubicCurve> for Segment {
    fn from(v: CubicCurve) -> Self {
        Self::Cubic(v)
    }
}

pub struct EdgeBoundingBox {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

pub trait Edge {
    fn point(&self, t: f32) -> (f32, f32);
    fn nearest_t(&self, point: (f32, f32)) -> f32;
    fn direction(&self, t: f32) -> (f32, f32);
    fn bbox(&self) -> EdgeBoundingBox;
}

pub struct Line {
    start: (f32, f32),
    end: (f32, f32),
}

impl Line {
    pub fn new(start: (f32, f32), end: (f32, f32)) -> Self {
        Self { start, end }
    }
}

impl Edge for Line {
    fn point(&self, t: f32) -> (f32, f32) {
        let x = (self.start.0 * (1.0 - t)) + (self.end.0 * t);
        let y = (self.start.1 * (1.0 - t)) + (self.end.1 * t);
        (x, y)
    }

    fn nearest_t(&self, point: (f32, f32)) -> f32 {
        let vx = self.end.0 - self.start.0;
        let vy = self.end.1 - self.start.1;
        let ux = self.start.0 - point.0;
        let uy = self.start.1 - point.1;
        let wx = self.end.0 - point.0;
        let wy = self.end.1 - point.1;
        let vu = (vx * ux) + (vy * uy);
        let vv = (vx * vx) + (vy * vy);
        let t = -vu / vv;
        let start = (ux * ux) + (uy * uy);
        let end = (wx * wx) + (wy * wy);
        if (0.0..=1.0).contains(&t) {
            t
        } else if start < end {
            0.0
        } else {
            1.0
        }
    }

    fn direction(&self, _t: f32) -> (f32, f32) {
        (self.end.0 - self.start.0, self.end.1 - self.start.1)
    }

    fn bbox(&self) -> EdgeBoundingBox {
        EdgeBoundingBox {
            left: self.start.0.min(self.end.0),
            right: self.start.0.max(self.end.0),
            top: self.start.1.max(self.end.1),
            bottom: self.start.1.min(self.end.1),
        }
    }
}

pub struct QuadCurve {
    x_poly: Polynomial<3>,
    y_poly: Polynomial<3>,
}

impl QuadCurve {
    pub fn new(start: (f32, f32), control: (f32, f32), end: (f32, f32)) -> Self {
        let x_poly = Polynomial {
            coeffs: [
                -2.0 * control.0 + start.0 + end.0,
                2.0 * control.0 - 2.0 * start.0,
                start.0,
            ],
        };
        let y_poly = Polynomial {
            coeffs: [
                -2.0 * control.1 + start.1 + end.1,
                2.0 * control.1 - 2.0 * start.1,
                start.1,
            ],
        };
        Self { x_poly, y_poly }
    }
}

impl Edge for QuadCurve {
    fn point(&self, t: f32) -> (f32, f32) {
        let x = self.x_poly.value(t);
        let y = self.y_poly.value(t);
        (x, y)
    }

    fn nearest_t(&self, point: (f32, f32)) -> f32 {
        let x_point = Polynomial {
            coeffs: [0.0, 0.0, point.0],
        };
        let y_point = Polynomial {
            coeffs: [0.0, 0.0, point.1],
        };
        let distance_sq = (self.x_poly - x_point).pow2() + (self.y_poly - y_point).pow2();
        let dd = distance_sq.derivative();
        let start_dist_sq = distance_sq.value(0.0);
        let end_dist_sq = distance_sq.value(1.0);
        let (mut best_dist_sq, mut best_t) = if start_dist_sq < end_dist_sq {
            (start_dist_sq, 0.0)
        } else {
            (end_dist_sq, 1.0)
        };
        let mut test = 0.0;
        while test <= 1.0 {
            let root = dd.newtons_root(test, NEWTONS_ITERS);
            if (0.0..=1.0).contains(&root) {
                let dist_sq = distance_sq.value(root);
                if dist_sq < best_dist_sq {
                    best_dist_sq = dist_sq;
                    best_t = root;
                }
            }
            test += 0.25;
        }
        best_t
    }

    fn direction(&self, t: f32) -> (f32, f32) {
        let x = self.x_poly.derivative().value(t);
        let y = self.y_poly.derivative().value(t);
        (x, y)
    }

    fn bbox(&self) -> EdgeBoundingBox {
        let tx = self.x_poly.derivative().root().clamp(0.0, 1.0);
        let ty = self.y_poly.derivative().root().clamp(0.0, 1.0);
        let possible_x = [0.0, tx, 1.0].map(|t| self.x_poly.value(t));
        let possible_y = [0.0, ty, 1.0].map(|t| self.y_poly.value(t));
        EdgeBoundingBox {
            left: possible_x.into_iter().fold(f32::INFINITY, |a, b| a.min(b)),
            right: possible_x
                .into_iter()
                .fold(f32::NEG_INFINITY, |a, b| a.max(b)),
            top: possible_y
                .into_iter()
                .fold(f32::NEG_INFINITY, |a, b| a.max(b)),
            bottom: possible_y.into_iter().fold(f32::INFINITY, |a, b| a.min(b)),
        }
    }
}

pub struct CubicCurve {
    x_poly: Polynomial<4>,
    y_poly: Polynomial<4>,
}

impl CubicCurve {
    pub fn new(
        start: (f32, f32),
        control_s: (f32, f32),
        control_e: (f32, f32),
        end: (f32, f32),
    ) -> Self {
        let x_poly = Polynomial {
            coeffs: [
                -start.0 + 3.0 * control_s.0 - 3.0 * control_e.0 + end.0,
                3.0 * start.0 - 6.0 * control_s.0 + 3.0 * control_e.0,
                -3.0 * start.0 + 3.0 * control_s.0,
                start.0,
            ],
        };
        let y_poly = Polynomial {
            coeffs: [
                -start.1 + 3.0 * control_s.1 - 3.0 * control_e.1 + end.1,
                3.0 * start.1 - 6.0 * control_s.1 + 3.0 * control_e.1,
                -3.0 * start.1 + 3.0 * control_s.1,
                start.1,
            ],
        };
        Self { x_poly, y_poly }
    }
}

impl Edge for CubicCurve {
    fn point(&self, t: f32) -> (f32, f32) {
        let x = self.x_poly.value(t);
        let y = self.y_poly.value(t);
        (x, y)
    }

    fn nearest_t(&self, point: (f32, f32)) -> f32 {
        let x_point = Polynomial {
            coeffs: [0.0, 0.0, 0.0, point.0],
        };
        let y_point = Polynomial {
            coeffs: [0.0, 0.0, 0.0, point.1],
        };
        let distance_sq = (self.x_poly - x_point).pow2() + (self.y_poly - y_point).pow2();
        let dd = distance_sq.derivative();
        let start_dist_sq = distance_sq.value(0.0);
        let end_dist_sq = distance_sq.value(1.0);
        let (mut best_dist_sq, mut best_t) = if start_dist_sq < end_dist_sq {
            (start_dist_sq, 0.0)
        } else {
            (end_dist_sq, 1.0)
        };
        let mut test = 0.0;
        while test <= 1.0 {
            let root = dd.newtons_root(test, NEWTONS_ITERS);
            if (0.0..=1.0).contains(&root) {
                let dist_sq = distance_sq.value(root);
                if dist_sq < best_dist_sq {
                    best_dist_sq = dist_sq;
                    best_t = root;
                }
            }
            test += 0.25;
        }
        best_t
    }

    fn direction(&self, t: f32) -> (f32, f32) {
        let x = self.x_poly.derivative().value(t);
        let y = self.y_poly.derivative().value(t);
        (x, y)
    }

    fn bbox(&self) -> EdgeBoundingBox {
        let [tx_a, tx_b] = self.x_poly.derivative().roots();
        let [ty_a, ty_b] = self.y_poly.derivative().roots();
        let possible_x = [0.0, tx_a, tx_b, 1.0].map(|t| self.x_poly.value(t.clamp(0.0, 1.0)));
        let possible_y = [0.0, ty_a, ty_b, 1.0].map(|t| self.y_poly.value(t.clamp(0.0, 1.0)));
        EdgeBoundingBox {
            left: possible_x.into_iter().fold(f32::INFINITY, |a, b| a.min(b)),
            right: possible_x
                .into_iter()
                .fold(f32::NEG_INFINITY, |a, b| a.max(b)),
            top: possible_y
                .into_iter()
                .fold(f32::NEG_INFINITY, |a, b| a.max(b)),
            bottom: possible_y.into_iter().fold(f32::INFINITY, |a, b| a.min(b)),
        }
    }
}
