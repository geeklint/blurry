use math::Polynomial;

pub mod math;

pub trait Edge {
    fn point(&self, t: f32) -> (f32, f32);
    fn nearest_t(&self, point: (f32, f32)) -> f32;
}

struct Line {
    start: (f32, f32),
    end: (f32, f32),
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
}

struct QuadCurve {
    start: (f32, f32),
    control: (f32, f32),
    end: (f32, f32),
}

fn quad_eq(s: f32, c: f32, e: f32, t: f32) -> f32 {
    let s_factor = (1.0 - t).powi(2) * s;
    let c_factor = 2.0 * t * (1.0 - t) * c;
    let e_factor = t.powi(2) * e;
    s_factor + c_factor + e_factor
}

impl Edge for QuadCurve {
    fn point(&self, t: f32) -> (f32, f32) {
        let x = quad_eq(self.start.0, self.control.0, self.end.0, t);
        let y = quad_eq(self.start.1, self.control.1, self.end.1, t);
        (x, y)
    }

    fn nearest_t(&self, point: (f32, f32)) -> f32 {
        todo!()
    }
}

pub struct CubicCurve {
    pub start: (f32, f32),
    pub control_s: (f32, f32),
    pub control_e: (f32, f32),
    pub end: (f32, f32),
}

impl Edge for CubicCurve {
    fn point(&self, t: f32) -> (f32, f32) {
        let sx = (1.0 - t) * quad_eq(self.start.0, self.control_s.0, self.control_e.0, t);
        let sy = (1.0 - t) * quad_eq(self.start.1, self.control_s.1, self.control_e.1, t);
        let ex = t * quad_eq(self.control_s.0, self.control_e.0, self.end.0, t);
        let ey = t * quad_eq(self.control_s.1, self.control_e.1, self.end.1, t);
        (sx + ex, sy + ey)
    }

    fn nearest_t(&self, point: (f32, f32)) -> f32 {
        let x_poly = Polynomial {
            coeffs: [
                -self.start.0 + 3.0 * self.control_s.0 - 3.0 * self.control_e.0 + self.end.0,
                3.0 * self.start.0 - 6.0 * self.control_s.0 + 3.0 * self.control_e.0,
                -3.0 * self.start.0 + 3.0 * self.control_s.0,
                self.start.0,
            ],
        };
        let y_poly = Polynomial {
            coeffs: [
                -self.start.1 + 3.0 * self.control_s.1 - 3.0 * self.control_e.1 + self.end.1,
                3.0 * self.start.1 - 6.0 * self.control_s.1 + 3.0 * self.control_e.1,
                -3.0 * self.start.1 + 3.0 * self.control_s.1,
                self.start.1,
            ],
        };
        let x_point = Polynomial {
            coeffs: [0.0, 0.0, 0.0, point.0],
        };
        let y_point = Polynomial {
            coeffs: [0.0, 0.0, 0.0, point.1],
        };
        let distance_sq = (x_poly - x_point).pow2() + (y_poly - y_point).pow2();
        dbg!(distance_sq);
        let dd = distance_sq.derivative();
        let start_dist_sq = distance_sq.value(0.0);
        let end_dist_sq = distance_sq.value(1.0);
        let (mut best_dist_sq, mut best_t) = if start_dist_sq < end_dist_sq {
            (start_dist_sq, 0.0)
        } else {
            (end_dist_sq, 1.0)
        };
        for test in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let root = dd.newtons_root(test, 8);
            if (0.0..=1.0).contains(&root) {
                let dist_sq = distance_sq.value(root);
                if dist_sq < best_dist_sq {
                    best_dist_sq = dist_sq;
                    best_t = root;
                }
            }
        }
        best_t
    }
}
