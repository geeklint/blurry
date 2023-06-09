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
}
