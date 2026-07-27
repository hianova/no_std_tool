use core::ops::{Add, Mul, Sub};
use core::fmt;

/// A fixed-size matrix allocated completely on the stack (Zero-Allocation).
#[derive(Clone, Copy, PartialEq)]
pub struct Matrix<const R: usize, const C: usize> {
    pub data: [[f32; C]; R],
}

impl<const R: usize, const C: usize> Matrix<R, C> {
    /// Create a new matrix filled with zeros
    pub const fn zeros() -> Self {
        Self {
            data: [[0.0; C]; R],
        }
    }

    /// Create a new matrix from a 2D array
    pub const fn new(data: [[f32; C]; R]) -> Self {
        Self { data }
    }

    /// Map a function over all elements (Element-wise operation)
    pub fn map<F: Fn(f32) -> f32>(&self, f: F) -> Self {
        let mut res = Self::zeros();
        for i in 0..R {
            for j in 0..C {
                res.data[i][j] = f(self.data[i][j]);
            }
        }
        res
    }

    /// Transpose the matrix (R x C -> C x R)
    pub fn transpose(&self) -> Matrix<C, R> {
        let mut res = Matrix::<C, R>::zeros();
        for i in 0..R {
            for j in 0..C {
                res.data[j][i] = self.data[i][j];
            }
        }
        res
    }
}

// Matrix Addition
impl<const R: usize, const C: usize> Add for Matrix<R, C> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        let mut res = Self::zeros();
        for i in 0..R {
            for j in 0..C {
                res.data[i][j] = self.data[i][j] + rhs.data[i][j];
            }
        }
        res
    }
}

// Matrix Subtraction
impl<const R: usize, const C: usize> Sub for Matrix<R, C> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        let mut res = Self::zeros();
        for i in 0..R {
            for j in 0..C {
                res.data[i][j] = self.data[i][j] - rhs.data[i][j];
            }
        }
        res
    }
}

// Scalar Multiplication
impl<const R: usize, const C: usize> Mul<f32> for Matrix<R, C> {
    type Output = Self;
    fn mul(self, scalar: f32) -> Self::Output {
        self.map(|x| x * scalar)
    }
}

// Matrix Multiplication (Dot Product): [R x K] * [K x C] = [R x C]
impl<const R: usize, const K: usize, const C: usize> Mul<Matrix<K, C>> for Matrix<R, K> {
    type Output = Matrix<R, C>;

    #[inline(always)]
    fn mul(self, rhs: Matrix<K, C>) -> Self::Output {
        let mut res = Matrix::<R, C>::zeros();
        // Optimized i-k-j loop order for contiguous row-major memory access.
        // This allows the compiler to easily vectorize (SIMD) the innermost loop.
        for i in 0..R {
            for k in 0..K {
                let a = self.data[i][k];
                for j in 0..C {
                    res.data[i][j] += a * rhs.data[k][j];
                }
            }
        }
        res
    }
}

impl<const R: usize, const C: usize> fmt::Display for Matrix<R, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for i in 0..R {
            write!(f, "[")?;
            for j in 0..C {
                write!(f, "{:>7.4}", self.data[i][j])?;
                if j < C - 1 {
                    write!(f, ", ")?;
                }
            }
            writeln!(f, "]")?;
        }
        Ok(())
    }
}
