/* SPDX-License-Identifier: (Apache-2.0 OR MIT OR Zlib) */
/* Copyright © 2023 Violet Leonard */

#[derive(Clone, Copy)]
pub struct Polynomial<const N: usize> {
    pub coeffs: [f32; N],
}

macro_rules! one {
    ($x:tt) => {
        1
    };
}

macro_rules! poly_value {
    ($head:ident $($coeff:ident)*) => {
        impl Polynomial<{ 1 $(+ one!($coeff))* }> {
            pub fn value(&self, t: f32) -> f32 {
                let [mut $head, $($coeff,)*] = self.coeffs;
                $(
                    $head = $head * t + $coeff;
                )*
                $head
            }
        }
    };
}

poly_value! { a b }
poly_value! { a b c }
poly_value! { a b c d }
poly_value! { a b c d e }
poly_value! { a b c d e f }
poly_value! { a b c d e f g }

impl<const N: usize> std::fmt::Debug for Polynomial<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const SUPERS: &[char] = &[
            '\u{00B2}', '\u{00B3}', '\u{2074}', '\u{2075}', '\u{2076}', '\u{2077}', '\u{2078}',
            '\u{2079}',
        ];
        write!(f, "Polynomial {{ ")?;
        for (i, coeff) in self.coeffs.into_iter().enumerate() {
            let sign = if coeff.is_sign_negative() { '-' } else { '+' };
            let coeff_abs = coeff.abs();
            if i == (N - 1) {
                if i == 0 {
                    write!(f, "{coeff}")?;
                } else {
                    write!(f, " {sign} {coeff_abs}")?;
                }
            } else if i == (N - 2) {
                if i == 0 {
                    write!(f, "{coeff}t")?;
                } else {
                    write!(f, " {sign} {coeff_abs}t")?;
                }
            } else {
                let exp = N - 1 - i;
                let sup = SUPERS[exp - 2];
                if i == 0 {
                    write!(f, "{coeff}t{sup}")?;
                } else {
                    write!(f, " {sign} {coeff_abs}t{sup}")?;
                }
            }
        }
        write!(f, " }}")?;
        Ok(())
    }
}

impl Polynomial<2> {
    pub fn root(&self) -> f32 {
        let [a, b] = self.coeffs;
        -b / a
    }
}

impl Polynomial<3> {
    pub fn roots(&self) -> [f32; 2] {
        let [a, b, c] = self.coeffs;
        if a == 0.0 {
            return [-c / b, f32::NAN];
        }
        let square = b.powi(2) - (4.0 * a * c);
        let sqrt = square.sqrt();
        let plus = (-b + sqrt) / (2.0 * a);
        let minus = (-b - sqrt) / (2.0 * a);
        [plus, minus]
    }
}

macro_rules! impl_derivative {
    ($N:literal newtons) => {
        impl_derivative! { $N }
        impl Polynomial<$N> {
            pub fn newtons_root(&self, mut guess: f32, mut iters: u8) -> f32 {
                let dself = self.derivative();
                while iters > 0 {
                    guess = guess - (self.value(guess) / dself.value(guess));
                    iters -= 1;
                }
                guess
            }
        }
    };
    ($N:literal) => {
        impl Polynomial<$N> {
            pub fn derivative(&self) -> Polynomial<{ $N - 1 }> {
                let mut coeffs = [0.0; $N - 1];
                let mut i = 0_u8;
                const LAST: u8 = $N - 1;
                while i < LAST {
                    let idx = i as usize;
                    coeffs[idx] = self.coeffs[idx] * ((LAST - i) as f32);
                    i += 1;
                }
                Polynomial { coeffs }
            }
        }
    };
}

impl_derivative!(3);
impl_derivative!(4 newtons);
impl_derivative!(5);
impl_derivative!(6 newtons);
impl_derivative!(7);

impl<const N: usize> std::ops::Add for Polynomial<N> {
    type Output = Polynomial<N>;

    fn add(self, rhs: Self) -> Self::Output {
        let mut coeffs = [0.0; N];
        let mut i = 0;
        while i < N {
            coeffs[i] = self.coeffs[i] + rhs.coeffs[i];
            i += 1;
        }
        Polynomial { coeffs }
    }
}

impl<const N: usize> std::ops::Sub for Polynomial<N> {
    type Output = Polynomial<N>;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut coeffs = [0.0; N];
        let mut i = 0;
        while i < N {
            coeffs[i] = self.coeffs[i] - rhs.coeffs[i];
            i += 1;
        }
        Polynomial { coeffs }
    }
}

impl<const N: usize> std::ops::Add<f32> for Polynomial<N> {
    type Output = Polynomial<N>;

    fn add(self, rhs: f32) -> Self::Output {
        let mut coeffs = self.coeffs;
        coeffs[N - 1] += rhs;
        Polynomial { coeffs }
    }
}

impl<const N: usize> std::ops::Sub<f32> for Polynomial<N> {
    type Output = Polynomial<N>;

    fn sub(self, rhs: f32) -> Self::Output {
        let mut coeffs = self.coeffs;
        coeffs[N - 1] -= rhs;
        Polynomial { coeffs }
    }
}

impl<const N: usize> std::ops::Mul<f32> for Polynomial<N> {
    type Output = Polynomial<N>;

    fn mul(self, rhs: f32) -> Self::Output {
        let mut coeffs = [0.0; N];
        let mut i = 0;
        while i < N {
            coeffs[i] = self.coeffs[i] * rhs;
            i += 1;
        }
        Polynomial { coeffs }
    }
}

impl Polynomial<3> {
    pub fn pow2(self) -> Polynomial<5> {
        let [a, b, c] = self.coeffs;
        let coeffs = [
            a * a,
            (a * b) + (b * a),
            (a * c) + (b * b) + (c * a),
            (b * c) + (c * b),
            c * c,
        ];
        Polynomial { coeffs }
    }

    pub fn split_within01(&self, t: f32) -> [Self; 2] {
        let [a, b, c] = self.coeffs;
        let first_half = [a * t * t, b * t, c];
        let second_half = [
            a * t * t - 2.0 * a * t + a,
            -2.0 * a * t * t + 2.0 * a * t - b * t + b,
            a * t * t + b * t + c,
        ];
        [
            Self { coeffs: first_half },
            Self {
                coeffs: second_half,
            },
        ]
    }
}

impl Polynomial<4> {
    pub fn pow2(self) -> Polynomial<7> {
        let [a, b, c, d] = self.coeffs;
        let coeffs = [
            a * a,
            (a * b) + (b * a),
            (a * c) + (b * b) + (c * a),
            (a * d) + (b * c) + (c * b) + (d * a),
            (b * d) + (c * c) + (d * b),
            (c * d) + (d * c),
            d * d,
        ];
        Polynomial { coeffs }
    }

    pub fn split_within01(&self, t: f32) -> [Self; 2] {
        let [a, b, c, d] = self.coeffs;
        let first_half = [a * t * t * t, b * t * t, c * t, d];
        let second_half = [
            -a * t * t * t + 3.0 * a * t * t - 3.0 * a * t + a,
            3.0 * a * t * t * t - 6.0 * a * t * t + 3.0 * a * t + b * t * t - 2.0 * b * t + b,
            -3.0 * a * t * t * t + 3.0 * a * t * t - 2.0 * b * t * t + 2.0 * b * t - c * t + c,
            a * t * t * t + b * t * t + c * t + d,
        ];
        [
            Self { coeffs: first_half },
            Self {
                coeffs: second_half,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cubic_split() {
        let original = Polynomial {
            coeffs: [1.0, 2.0, -3.0, 4.0],
        };
        let [left, right] = original.split_within01(0.5);
        assert_eq!(left.value(0.0), original.value(0.0));
        assert_eq!(left.value(0.5), original.value(0.25));
        assert_eq!(left.value(1.0), original.value(0.5));
        assert_eq!(right.value(0.0), original.value(0.5));
        assert_eq!(right.value(0.5), original.value(0.75));
        assert_eq!(right.value(1.0), original.value(1.0));
    }
}
