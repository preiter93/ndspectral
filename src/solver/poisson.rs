//! Poisson Solver
//! Solve equations of the form:
//! ..math:
//!  c * D2 vhat = f
//!
//! where D2 is the second derivative.
//! Alternatively, if defined, multiply rhs
//! before the solve step, i.e.
//! ..math:
//!  c * D2 vhat = A f
//!
//! For multidimensional equations, apply
//! eigendecomposition on the non - outermost
//! dimensions of the form
//! ..math:
//!   ``` (A + lam_i*C) x_i^* = b_i^* ```
//!
//! Chebyshev bases: The equation becomes
//! banded after multiplication with the pseudoinverse
//! of D2 (B2). In this case, the second equation is
//! solved, with A = B2.

use super::utils::vec_to_array;
use super::{FdmaTensor, MatVec};
use crate::bases::BaseBasics;
use crate::bases::LaplacianInverse;
use crate::bases::SpaceBase;
use crate::solver::{Solve, SolveReturn};
use crate::Base;
use crate::SolverScalar;
use ndarray::prelude::*;
use std::ops::{Add, Div, Mul};

/// Container for Poisson Solver
#[derive(Clone)]
pub struct Poisson<T, const N: usize> {
    solver: Box<FdmaTensor<T, N>>,
    matvec: Vec<Option<MatVec<T>>>,
}

impl<const N: usize> Poisson<f64, N> {
    /// Construct Poisson solver from field
    pub fn from_space(space: &SpaceBase<f64, N>, c: [f64; N]) -> Self {
        let solver = Self::solver_from_space(space, c);
        let matvec: Vec<Option<MatVec<f64>>> = space
            .bases
            .iter()
            .map(|base| Self::matvec_from_base(base))
            .collect();

        Self { solver, matvec }
    }

    // Construct solver, depending on bases
    fn solver_from_space(space: &SpaceBase<f64, N>, c: [f64; N]) -> Box<FdmaTensor<f64, N>> {
        // a
        let vec = Self::get_a_from_space(space, c);
        let a = vec_to_array::<&Array2<f64>, N>(vec.iter().collect());
        // c
        let vec = Self::get_c_from_space(space, c);
        let arr_c = vec_to_array::<&Array2<f64>, N>(vec.iter().collect());
        // is_diag
        let vec = Self::get_is_diag_from_space(space);
        let is_diag = vec_to_array::<&bool, N>(vec.iter().collect());

        let mut solver = FdmaTensor::from_matrix(a, arr_c, is_diag);

        // Handle singularity (2D)
        if N == 2 && solver.lam[0][0].abs() < 1e-10 {
            solver.lam[0] -= 1e-10;
            println!("Poisson seems singular! Eigenvalue 0 is manipulated to help out.");
        }
        Box::new(solver)
    }

    /// a refers to A of multidimensional case, see explanation of Poisson.
    fn get_a_from_space(space: &SpaceBase<f64, N>, c: [f64; N]) -> Vec<Array2<f64>> {
        space
            .bases
            .iter()
            .zip(c.iter())
            .map(|(base, c)| Self::matrix_from_base(base, *c).0)
            .collect()
    }

    /// c refers to C of multidimensional case, see explanation of Poisson.
    /// Only used for N > 1.
    fn get_c_from_space(space: &SpaceBase<f64, N>, c: [f64; N]) -> Vec<Array2<f64>> {
        space
            .bases
            .iter()
            .zip(c.iter())
            .map(|(base, c)| Self::matrix_from_base(base, *c).1)
            .collect()
    }

    fn get_is_diag_from_space(space: &SpaceBase<f64, N>) -> Vec<bool> {
        space.bases.iter().map(|base| Self::is_diag(base)).collect()
    }

    fn is_diag(base: &Base<f64>) -> bool {
        match base {
            Base::Chebyshev(_) | Base::CompositeChebyshev(_) => false,
            Base::FourierC2c(_) | Base::FourierR2c(_) => true,
        }
    }

    /// Returns the solver for the lhs, depending on the base
    fn matrix_from_base(base: &Base<f64>, c: f64) -> (Array2<f64>, Array2<f64>) {
        let mass = base.mass();
        let lap = base.laplace();
        let pinv = base.laplace_inv();
        let eye = base.laplace_inv_eye();
        let c_t: f64 = c;
        match base {
            Base::Chebyshev(_) => (
                eye.dot(&mass) * c_t,
                (eye.dot(&pinv)).dot(&mass.slice(s![.., 2..])),
            ),
            Base::CompositeChebyshev(_) => (eye.dot(&mass) * c_t, (eye.dot(&pinv)).dot(&mass)),
            Base::FourierC2c(_) | Base::FourierR2c(_) => {
                // println!("Not to me: Manipulate Laplacian in Poisson. Find better solution!!");
                // lap[[0, 0]] = 1e-10;
                (lap * c_t, mass)
            }
        }
    }

    /// Returns the solver for the rhs, depending on the base
    #[allow(clippy::unnecessary_wraps)]
    fn matvec_from_base(base: &Base<f64>) -> Option<MatVec<f64>> {
        use crate::solver::MatVecDot;
        match base {
            Base::Chebyshev(_) | Base::CompositeChebyshev(_) => {
                let pinv = base.laplace_inv();
                let mat = pinv.slice(ndarray::s![2.., ..]).to_owned();
                let matvec = MatVec::MatVecDot(MatVecDot::new(&mat));
                Some(matvec)
            }
            Base::FourierC2c(_) | Base::FourierR2c(_) => None,
        }
    }
}

#[allow(unused_variables)]
impl<A> Solve<A, ndarray::Ix1> for Poisson<f64, 1>
where
    A: SolverScalar
        + Div<f64, Output = A>
        + Mul<f64, Output = A>
        + Add<f64, Output = A>
        + From<f64>,
{
    /// # Example
    fn solve<S1, S2>(
        &self,
        input: &ArrayBase<S1, Ix1>,
        output: &mut ArrayBase<S2, Ix1>,
        axis: usize,
    ) where
        S1: ndarray::Data<Elem = A>,
        S2: ndarray::Data<Elem = A> + ndarray::DataMut,
    {
        if let Some(matvec) = &self.matvec[0] {
            let buffer = matvec.solve(input, 0);
            self.solver.solve(&buffer, output, 0);
        } else {
            self.solver.solve(input, output, 0);
        }
    }
}

#[allow(unused_variables)]
impl<A> Solve<A, ndarray::Ix2> for Poisson<f64, 2>
where
    A: SolverScalar
        + Div<f64, Output = A>
        + Mul<f64, Output = A>
        + Add<f64, Output = A>
        + From<f64>,
{
    /// # Example
    fn solve<S1, S2>(
        &self,
        input: &ArrayBase<S1, Ix2>,
        output: &mut ArrayBase<S2, Ix2>,
        axis: usize,
    ) where
        S1: ndarray::Data<Elem = A>,
        S2: ndarray::Data<Elem = A> + ndarray::DataMut,
    {
        // Matvec
        let mut rhs = self.matvec[0]
            .as_ref()
            .map_or_else(|| input.to_owned(), |x| x.solve(input, 0));
        if let Some(x) = &self.matvec[1] {
            rhs = x.solve(&rhs, 1);
        };
        // Solve fdma-tensor
        self.solver.solve(&rhs, output, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::{Field, Field1, Field2, Field2Complex};
    use crate::{cheb_dirichlet, fourier_r2c};
    use ndarray::array;
    use num_complex::Complex;
    //use std::f64::consts::PI;

    fn approx_eq<S, D>(result: &ndarray::ArrayBase<S, D>, expected: &ndarray::ArrayBase<S, D>)
    where
        S: ndarray::Data<Elem = f64>,
        D: ndarray::Dimension,
    {
        let dif = 1e-3;
        for (a, b) in expected.iter().zip(result.iter()) {
            if (a - b).abs() > dif {
                panic!("Large difference of values, got {} expected {}.", b, a)
            }
        }
    }

    fn approx_eq_complex<S, D>(result: &ArrayBase<S, D>, expected: &ArrayBase<S, D>)
    where
        S: ndarray::Data<Elem = Complex<f64>>,
        D: ndarray::Dimension,
    {
        let dif = 1e-3;
        for (a, b) in expected.iter().zip(result.iter()) {
            if (a.re - b.re).abs() > dif || (a.im - b.im).abs() > dif {
                panic!("Large difference of values, got {} expected {}.", b, a)
            }
        }
    }

    #[test]
    fn test_poisson1d() {
        let nx = 8;
        let bases = [cheb_dirichlet::<f64>(nx)];
        let field = Field1::new(&bases);
        let poisson = Poisson::from_space(&field.space, [1.0]);
        let mut b: Array1<f64> = Array1::zeros(nx);
        let mut x = Array1::<f64>::zeros(nx - 2);
        for (i, bi) in b.iter_mut().enumerate() {
            *bi = (i + 1) as f64;
        }

        // Solve Poisson
        poisson.solve(&b, &mut x, 0);
        // Python (pypde's) solution
        let y = array![0.1042, 0.0809, 0.0625, 0.0393, -0.0417, -0.0357];

        approx_eq(&x, &y);
    }

    #[test]
    fn test_poisson2d() {
        let (nx, ny) = (8, 7);
        let bases = [cheb_dirichlet::<f64>(nx), cheb_dirichlet::<f64>(ny)];
        let field = Field2::new(&bases);
        let poisson = Poisson::from_space(&field.space, [1.0, 1.0]);
        let mut x = Array2::<f64>::zeros((nx - 2, ny - 2));
        let b: Array2<f64> = array![
            [1., 2., 3., 4., 5., 6., 7.],
            [1., 2., 3., 4., 5., 6., 7.],
            [1., 2., 3., 4., 5., 6., 7.],
            [1., 2., 3., 4., 5., 6., 7.],
            [1., 2., 3., 4., 5., 6., 7.],
            [1., 2., 3., 4., 5., 6., 7.],
            [1., 2., 3., 4., 5., 6., 7.],
            [1., 2., 3., 4., 5., 6., 7.],
        ];

        // Solve Poisson
        poisson.solve(&b, &mut x, 0);
        // Python (pypde's) solution
        let y = array![
            [0.01869736, 0.0244178, 0.01403203, -0.0202917, -0.0196697],
            [-0.0027890, -0.004035, -0.0059870, -0.0023490, -0.0046850],
            [-0.0023900, -0.007947, -0.0085570, -0.0189310, -0.0223680],
            [-0.0038940, -0.006622, -0.0096270, -0.0079020, -0.0120490],
            [0.00025400, -0.006752, -0.0082940, -0.0316230, -0.0361640],
            [-0.0001120, -0.004374, -0.0066430, -0.0216410, -0.0262570],
        ];

        approx_eq(&x, &y);
    }

    #[test]
    fn test_poisson2d_complex() {
        let (nx, ny) = (8, 7);
        let bases = [cheb_dirichlet::<f64>(nx), cheb_dirichlet::<f64>(ny)];
        let field = Field2::new(&bases);
        let poisson = Poisson::from_space(&field.space, [1.0, 1.0]);
        let mut x_cmpl = Array2::<Complex<f64>>::zeros((nx - 2, ny - 2));
        let b: Array2<f64> = array![
            [1., 2., 3., 4., 5., 6., 7.],
            [1., 2., 3., 4., 5., 6., 7.],
            [1., 2., 3., 4., 5., 6., 7.],
            [1., 2., 3., 4., 5., 6., 7.],
            [1., 2., 3., 4., 5., 6., 7.],
            [1., 2., 3., 4., 5., 6., 7.],
            [1., 2., 3., 4., 5., 6., 7.],
            [1., 2., 3., 4., 5., 6., 7.],
        ];

        let b_cmpl: Array2<Complex<f64>> = b.mapv(|x| Complex::new(x, x));

        // Solve Poisson
        poisson.solve(&b_cmpl, &mut x_cmpl, 0);
        // Python (pypde's) solution
        let y = array![
            [0.01869736, 0.0244178, 0.01403203, -0.0202917, -0.0196697],
            [-0.0027890, -0.004035, -0.0059870, -0.0023490, -0.0046850],
            [-0.0023900, -0.007947, -0.0085570, -0.0189310, -0.0223680],
            [-0.0038940, -0.006622, -0.0096270, -0.0079020, -0.0120490],
            [0.00025400, -0.006752, -0.0082940, -0.0316230, -0.0361640],
            [-0.0001120, -0.004374, -0.0066430, -0.0216410, -0.0262570],
        ];
        let y_cmpl: Array2<Complex<f64>> = y.mapv(|x| Complex::new(x, x));

        approx_eq_complex(&x_cmpl, &y_cmpl);
    }

    #[test]
    fn test_poisson2d_cd_cd() {
        // Init
        let (nx, ny) = (8, 7);
        let bases = [cheb_dirichlet::<f64>(nx), cheb_dirichlet::<f64>(ny)];
        let mut field = Field2::new(&bases);
        let poisson = Poisson::from_space(&field.space, [1.0, 1.0]);
        let x = bases[0].coords();
        let y = bases[1].coords();

        // Analytical field and solution
        let n = std::f64::consts::PI / 2.;
        let mut expected = field.v.clone();
        for (i, xi) in x.iter().enumerate() {
            for (j, yi) in y.iter().enumerate() {
                field.v[[i, j]] = (n * xi).cos() * (n * yi).cos();
                expected[[i, j]] = -1. / (n * n * 2.) * field.v[[i, j]];
            }
        }

        // Solve
        field.forward();
        let input = field.to_ortho();
        let mut result = Array2::<f64>::zeros(field.vhat.raw_dim());
        poisson.solve(&input, &mut result, 0);
        field.vhat.assign(&result);
        field.backward();

        // Compare
        approx_eq(&field.v, &expected);
    }

    #[test]
    fn test_poisson2d_fo_cd() {
        // Init
        let (nx, ny) = (16, 7);
        let bases = [fourier_r2c::<f64>(nx), cheb_dirichlet::<f64>(ny)];
        let mut field = Field2Complex::new(&bases);
        let poisson = Poisson::from_space(&field.space, [1.0, 1.0]);
        let x = bases[0].coords();
        let y = bases[1].coords();

        // Analytical field and solution
        let n = std::f64::consts::PI / 2.;
        let mut expected = field.v.clone();
        for (i, xi) in x.iter().enumerate() {
            for (j, yi) in y.iter().enumerate() {
                field.v[[i, j]] = (xi).cos() * (n * yi).cos();
                expected[[i, j]] = -1. / (1. + n * n) * field.v[[i, j]];
            }
        }

        // Solve
        field.forward();
        let input = field.to_ortho();
        let mut result = Array2::<Complex<f64>>::zeros(field.vhat.raw_dim());
        poisson.solve(&input, &mut result, 0);
        field.vhat.assign(&result);
        field.backward();

        // Compare
        approx_eq(&field.v, &expected);
    }
}
