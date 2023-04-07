#[derive(Clone, Copy)]
pub struct Polynomial<const N: usize> {
    pub coeffs: [f32; N],
}

impl<const N: usize> Polynomial<N> {
    pub fn value(&self, t: f32) -> f32 {
        let mut accum = self.coeffs[0];
        let mut i = 1;
        while i < N {
            accum = accum.mul_add(t, self.coeffs[i]);
            i += 1;
        }
        accum
    }
}

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
                while i < ($N - 1) {
                    let idx = i as usize;
                    coeffs[idx] = self.coeffs[idx] * f32::from($N - 1 - i);
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

macro_rules! impl_mul {
    ($N:literal ^ 2) => {
        impl_mul! {$N * $N}
        impl Polynomial<$N> {
            pub fn pow2(self) -> Polynomial<{ $N + $N - 1 }> {
                self * self
            }
        }
    };
    ($N:literal * $M:literal) => {
        impl std::ops::Mul<Polynomial<$M>> for Polynomial<$N> {
            type Output = Polynomial<{ $N + $M - 1 }>;

            fn mul(self, rhs: Polynomial<$M>) -> Self::Output {
                const NM: usize = $N + $M - 1;
                let mut coeffs = [0.0; NM];
                let mut n = 0;
                while n < $N {
                    let n_exp = $N - 1 - n;
                    let mut m = 0;
                    while m < $M {
                        let m_exp = $M - 1 - m;
                        let out_exp = n_exp + m_exp;
                        let out_idx = NM - 1 - out_exp;
                        coeffs[out_idx] += self.coeffs[n] * rhs.coeffs[m];
                        m += 1;
                    }
                    n += 1;
                }
                Polynomial { coeffs }
            }
        }
    };
}

impl_mul!(3 ^ 2);
impl_mul!(4 ^ 2);
