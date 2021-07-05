//! Composite bases are produced by a combination of basis functions from a orthonormal set.
//use ndarray::{Data, DataMut, Zip};

/// Transform from parent space to composite space and vice versa.
///
/// Parent (p) and composite space (c) are simply connected by a stencil matrix S, i.e.:
/// p = S c. The transform to_parent is done via matrix multiplication, while for the inverse
/// from_parent function, a system of linear equations is solved.
pub struct Composite {
    /// Number of coefficients in parent space
    pub n: usize,
    /// Number of coefficients in composite space
    pub m: usize,
}

/// Procedural macro which derives a composite Base
/// from its parent base (p) and a transform
/// stencil (s). Additionally, the identifier
/// which generate the stencil must be supplied (a);
/// it can deviate from the standard new() method.
#[macro_export]
macro_rules! derive_composite {
    (
        $(#[$meta:meta])* $i: ident, $p: ty, $s: ty, $a: ident
    ) => {
        $(#[$meta])*
        pub struct $i {
            /// Number of coefficients in parent space
            pub n: usize,
            /// Number of coefficients in composite space
            pub m: usize,
            parent: $p,
            stencil: $s,
        }

        impl $i {
            /// Create new Basis.
            pub fn new(n: usize) -> Self {
                let m = <$s>::get_m(n);
                let stencil = <$s>::$a(n);
                let parent = <$p>::new(n);
                $i {
                    n,
                    m,
                    stencil,
                    parent,
                }
            }

            /// Return size of physical space
            pub fn len_phys(&self) -> usize {
                self.n
            }

            /// Return size of spectral space
            pub fn len_spec(&self) -> usize {
                self.m
            }

            /// Return grid coordinates
            pub fn coords(&self) -> &Array1<f64> {
                &self.parent.x
            }
        }

        impl Transform for $i {
            type PhType = <$p as Transform>::PhType;
            type SpType = <$p as Transform>::SpType;

            /// Transform: Physical space --> Spectral space
            fn forward<R, S, D>(
                &mut self,
                input: &mut ArrayBase<R, D>,
                output: &mut ArrayBase<S, D>,
                axis: usize,
            ) where
                R: Data<Elem = Self::SpType> + DataMut + RawDataClone,
                S: Data<Elem = Self::PhType> + DataMut,
                D: Dimension + RemoveAxis,
            {
                let mut buffer = input.clone();
                self.parent.forward(input, &mut buffer, axis);
                self.stencil.from_parent(&buffer, output, axis);
            }

            /// Transform: Spectral space --> Physical space
            fn backward<R, S, D>(
                &mut self,
                input: &mut ArrayBase<R, D>,
                output: &mut ArrayBase<S, D>,
                axis: usize,
            ) where
                R: Data<Elem = Self::PhType> + DataMut + RawDataClone,
                S: Data<Elem = Self::SpType> + DataMut + RawDataClone,
                D: Dimension + RemoveAxis,
            {
                let mut buffer = output.clone();
                self.stencil.to_parent(input,&mut buffer,axis);
                self.parent.backward(&mut buffer, output, axis);
            }

            /// Differentiate array n_times in spectral space along
            /// axis.
            ///
            /// Returns derivative coefficients in parent space
            fn differentiate<T, R, S, D>(
                &self,
                input: &ArrayBase<R, D>,
                output: &mut ArrayBase<S, D>,
                n_times: usize,
                axis: usize,
            ) where
                T: LinalgScalar + Send + From<f64>,
                f64: Into<T>,
                R: Data<Elem = T>,
                S: Data<Elem = T> + RawDataClone + DataMut,
                D: Dimension,
            {
                let mut buffer = output.clone();
                self.stencil.to_parent(input,&mut buffer,axis);
                self.parent.differentiate(&buffer, output, n_times, axis);
            }

        }
    };
}
