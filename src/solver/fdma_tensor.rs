//! Four-diagonal matrix solver for multidimensional problems
//!
//! Only one dimension needs to be four-diagonal. All other
//! dimensions are diagonalized by an eigendecomposition. This
//! adds two matrix multiplications per dimension to the solve
//! step, one before the fdma solver, and one after.
use super::utils::{diag, eig, inv};
use super::Fdma;
use super::Solve;
use ndarray::{Array1, Array2, ArrayBase, Ix1, Ix2, Zip};
use ndarray::{Data, DataMut};

/// Tensor solver handles non-seperable multidimensional
/// systems, by diagonalizing all, but one, dimension
/// via a eigendecomposition. This makes the problem,
/// banded along the not-diagonalized direction.
///
/// In 2-D, the equations have the form:
/// .. math::
/// (A + lam_i*C)x_i = b_i
///
///  x,b: Matrix ( M x N )
///
///  A: Matrix ( N x N )
///    banded with diagonals in offsets  0, 2
///
///  C: Matrix ( N x N )
///    banded with diagonals in offsets -2, 0, 2, 4
///
///  lam: Eigenvector ( M )
///
/// Derivation:
///
/// Starting from the equation
/// .. math::
///
///  [(Ax x Cy) + (Cx x Ay)] g = f
///
/// where 'x' is the Kronecker product operator.
///
/// Multiplying it by the inverse of Cx, CxI
/// .. math::
///
/// [(CxI @ Ax x Cy) + (Ix x Ay)] g = (CxI x Iy) f
///
/// Applying a eigen-decomposition on CxI @ Ax = Qx lam QxI,
/// and multiplying the above equation with QxI from the left
/// .. math::
///
/// [(lam*QxI x Cy) + (QxI x Ay)] g = (QxI@CxI x Iy) f
///
/// This equation is solved in 3 steps:
///
/// 1. Transform f:
/// .. math::
///
///    fhat = (QxI@CxI x Iy) f = self.p.dot( f )
///
///  2. Solve the system, that is now seperatble and banded in y (y)
/// .. math::
///
///    (Ay + lam_i*Cy)ghat_i = fhat_i
///
///  3. Transfrom ghat back to g (x)
/// .. math::
///
///    g = Qx ghat = self.q.dot(ghat)
#[derive(Debug)]
pub struct FdmaTensor<const N: usize> {
    n: usize,
    fdma: [Fdma<f64>; 2],
    // Replace with [_;N-1], when const generic operations are stable
    fwd: Vec<Option<Array2<f64>>>, // Multiply before, of size (N-1)
    bwd: Vec<Option<Array2<f64>>>, // Multiply after, of size (N-1)
    lam: Vec<Array1<f64>>,         // Eigenvalues, of size (N-1)
    singular: bool,
}

impl<const N: usize> FdmaTensor<N> {
    /// Supply array of matrices a and c, as defined in the definition of FdmaTensor.
    ///
    /// Eigendecompoiton:
    ///
    /// The first N-1 dimensions are diagonalized by an eigendecomposition,
    /// If the matrices of a particular dimension are already diagonal,
    /// the respective place in variable a_is_diag should be set to true.
    /// In this case, the eigenvalues must be supplied in 'a' as a diagonal matrix,
    /// and c is not used any further.
    ///
    /// 1-Dimensional problems:
    ///
    /// In this case, only a, which must be a banded matrix, is used in solve.
    #[allow(clippy::many_single_char_names)]
    pub fn from_matrix(a: [&Array2<f64>; N], c: [&Array2<f64>; N], a_is_diag: [&bool; N]) -> Self {
        //todo!()
        let mut fwd: Vec<Option<Array2<f64>>> = Vec::new();
        let mut bwd: Vec<Option<Array2<f64>>> = Vec::new();
        let mut lam: Vec<Array1<f64>> = Vec::new();
        // Inner dimensions
        for i in 0..N - 1 {
            if *a_is_diag[i] {
                lam.push(diag(a[i], 0));
                fwd.push(None);
                bwd.push(None);
            } else {
                let xmat = inv(c[i]).dot(a[i]);
                let (l, q, p) = eig(&xmat);
                lam.push(l);
                fwd.push(Some(p.dot(&inv(c[i]))));
                bwd.push(Some(q));
            }
        }
        // Outermost
        let n = a[N - 1].shape()[0];
        let fdma = [
            Fdma::from_matrix_raw(a[N - 1]),
            Fdma::from_matrix_raw(c[N - 1]),
        ];
        // Initialize
        let mut tensor = FdmaTensor {
            n,
            fdma,
            fwd,
            bwd,
            lam,
            singular: false,
        };

        // For 1-D problems, the forward sweep
        // can already perfomered beforehand
        if N == 1 {
            tensor.fdma[0].sweep();
        }
        // Return
        tensor
    }
}

impl Solve<f64, Ix1> for FdmaTensor<1> {
    /// Solve 1-D Problem with real in and output
    fn solve<S1: Data<Elem = f64>, S2: Data<Elem = f64> + DataMut>(
        &self,
        input: &ArrayBase<S1, Ix1>,
        output: &mut ArrayBase<S2, Ix1>,
        axis: usize,
    ) {
        if input.shape()[0] != self.n {
            panic!(
                "Dimension mismatch in Tensor! Got {} vs. {}.",
                input.len(),
                self.n
            );
        }
        self.fdma[0].solve(input, output, axis);
    }
}

#[allow(unused_variables)]
impl Solve<f64, Ix2> for FdmaTensor<2> {
    /// Solve 2-D Problem with real in and output
    fn solve<S1: Data<Elem = f64>, S2: Data<Elem = f64> + DataMut>(
        &self,
        input: &ArrayBase<S1, Ix2>,
        output: &mut ArrayBase<S2, Ix2>,
        axis: usize,
    ) {
        if input.shape()[0] != self.lam[0].len() || input.shape()[1] != self.n {
            panic!(
                "Dimension mismatch in Tensor! Got {} vs. {} (0) and {} vs. {} (1).",
                input.shape()[0],
                self.lam[0].len(),
                input.shape()[1],
                self.n
            );
        }

        // Step 1: Forward Transform rhs along x
        if let Some(p) = &self.fwd[0] {
            output.assign(&p.dot(input));
        } else {
            output.assign(&input);
        }

        // Step 2: Solve along y (but iterate over all lanes in x)
        let mut helper = Array1::<f64>::zeros(output.shape()[0]);
        Zip::from(output.outer_iter_mut())
            .and(self.lam[0].outer_iter())
            .for_each(|mut out, lam| {
                let l = lam.as_slice().unwrap()[0];
                let mut fdma = &self.fdma[0] + &(&self.fdma[1] * l);
                fdma.sweep();
                helper.assign(&out);
                fdma.solve(&helper, &mut out, 0);
            });

        // Step 3: Backward Transform solution along x
        if let Some(q) = &self.bwd[0] {
            output.assign(&q.dot(output));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::{Array, Dim, Ix};

    fn approx_eq<S, D>(result: &ArrayBase<S, D>, expected: &ArrayBase<S, D>)
    where
        S: Data<Elem = f64>,
        D: ndarray::Dimension,
    {
        let dif = 1e-3;
        for (a, b) in expected.iter().zip(result.iter()) {
            if (a - b).abs() > dif {
                panic!("Large difference of values, got {} expected {}.", b, a)
            }
        }
    }

    fn test_matrix(nx: usize) -> Array<f64, Dim<[Ix; 2]>> {
        let mut matrix = Array::<f64, Dim<[Ix; 2]>>::zeros((nx, nx));
        for i in 0..nx {
            let j = (i + 1) as f64;
            matrix[[i, i]] = 0.5 * j;
            if i > 1 {
                matrix[[i, i - 2]] = 10. * j;
            }
            if i < nx - 2 {
                matrix[[i, i + 2]] = 1.5 * j;
            }
            if i < nx - 4 {
                matrix[[i, i + 4]] = 2.5 * j;
            }
        }
        matrix
    }

    #[test]
    fn test_tensor1d() {
        let nx = 6;
        let mut data = Array::<f64, Dim<[Ix; 1]>>::zeros(nx);
        let mut result = Array::<f64, Dim<[Ix; 1]>>::zeros(nx);
        for (i, v) in data.iter_mut().enumerate() {
            *v = i as f64;
        }
        let matrix = test_matrix(nx);
        let solver = FdmaTensor::from_matrix([&matrix], [&matrix], [&false]);
        solver.solve(&data, &mut result, 0);
        let recover: Array<f64, Dim<[Ix; 1]>> = matrix.dot(&result);
        approx_eq(&recover, &data);
    }

    #[test]
    fn test_tensor2d() {
        let nx = 6;

        let mut data: Array2<f64> = Array2::zeros((6, 6));
        let mut result = Array::<f64, Dim<[Ix; 2]>>::zeros((nx, nx));
        for (i, v) in data.iter_mut().enumerate() {
            *v = i as f64;
        }
        // Test arrays
        let a = ndarray::array![
            [-1.0, 0.0, 1.0, 0.0, 0.0, 0.0],
            [0.0, -1.0, 0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, -1.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, -1.0, 0.0, 1.0],
            [0.0, 0.0, 0.0, 0.0, -1.0, 0.0],
            [0.0, 0.0, 0.0, 0.0, 0.0, -1.0]
        ];
        let c = ndarray::array![
            [0.41666, 0.0, -0.2083, 0.0, 0.041666, 0.0],
            [0.0, 0.104166, 0.0, -0.0833, 0.0, 0.0208],
            [-0.0208, 0.0, 0.0542, 0.0, -0.0333, 0.0],
            [0.0, -0.0125, 0.0, 0.033333, 0.0, -0.020833],
            [0.0, 0.0, -0.00833, 0.0, 0.00833, 0.0],
            [0.0, 0.0, 0.0, -0.00595, 0.0, 0.00595]
        ];

        let solver = FdmaTensor::from_matrix([&a, &a], [&c, &c], [&false, &false]);
        solver.solve(&data, &mut result, 0);
        println!("{:?}", result);
        // let recover: Array<f64, Dim<[Ix; 1]>> = matrix.dot(&result);

        // Recover b
        let x = result.clone();
        let recover = a.dot(&x).dot(&(c.t())) + c.dot(&x).dot(&(a.t()));
        approx_eq(&recover, &data);
    }
}
