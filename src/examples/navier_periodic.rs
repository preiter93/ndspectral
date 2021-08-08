//! # Direct numerical simulation
//! Solver for 2-dimensional Navier-Stokes momentum equations
//! coupled with temperature equation.
//!
//! # Example
//! Solve 2-D Rayleigh Benard Convection
//! ```ignore
//! use rustpde::{Integrate, integrate};
//! use rustpde::examples::Navier2D;
//!
//! fn main() {
//!     // Parameters
//!     let (nx, ny) = (64, 64);
//!     let ra = 1e5;
//!     let pr = 1.;
//!     let adiabatic = true;
//!     let aspect = 1.0;
//!     let dt = 0.02;
//!     let mut navier = Navier2D::new(nx, ny, ra, pr, dt, adiabatic, aspect);
//!     // Set initial conditions
//!     navier.set_velocity(0.2, 1., 1.);
//!     // // Want to restart?
//!     // navier.read("data/flow100.000.h5");
//!     // Write first field
//!     navier.write();
//!     integrate(&mut navier, 100., Some(1.0));
//! }
//! ```
use super::conv_term;
use crate::bases::{cheb_dirichlet, cheb_dirichlet_bc, cheb_neumann, chebyshev, fourier_r2c};
use crate::hdf5::{read_scalar_from_hdf5, write_scalar_to_hdf5};
use crate::solver::{Hholtz, Poisson, Solve, SolverField};
use crate::Field2Complex as Field2;
use crate::Integrate;
use crate::{Field, ReadField, WriteField};
use ndarray::{s, Array2};
use num_complex::Complex;
use std::collections::HashMap;

type NumType = Complex<f64>;

/// Return viscosity from Ra, Pr, and height of the cell
pub fn get_nu(ra: f64, pr: f64, height: f64) -> f64 {
    let f = pr / (ra / height.powf(3.0));
    f.sqrt()
}

/// Return diffusivity from Ra, Pr, and height of the cell
pub fn get_ka(ra: f64, pr: f64, height: f64) -> f64 {
    let f = 1. / ((ra / height.powf(3.0)) * pr);
    f.sqrt()
}

/// Solve 2-dimensional Navier-Stokes equations
/// coupled with temperature equations
///
/// Bases: Chebyshev in x & y
///
/// Struct must be mutable, to perform the
/// update step, which advances the solution
/// by 1 timestep.
///
/// # Arguments
///
/// * `nx,ny` - The number of modes in x and y -direction
///
/// * `ra,pr` - Rayleigh and Prandtl number
///
/// * `dt` - Timestep size
///
/// * `adiabatic` - Boolean, sidewall temperature boundary condition
///
/// * `aspect` - Aspect ratio L/H
///
/// # Examples
///
/// ```
/// use rustpde::{integrate, Integrate};
/// use rustpde::examples::Navier2DPeriodic;
/// let (nx, ny) = (64, 64);
/// let ra = 1e5;
/// let pr = 1.;
/// let aspect = 1.0;
/// let dt = 0.01;
/// let mut navier = Navier2DPeriodic::new(nx, ny, ra, pr, dt, aspect);
/// // Read initial field from file
/// // navier.read("data/flow0.000.h5");
/// integrate(&mut navier, 0.2,  None);
/// ```
pub struct Navier2DPeriodic {
    /// Field for derivatives and transforms
    field: Field2,
    /// Temperature
    pub temp: Field2,
    /// Horizontal Velocity
    pub ux: Field2,
    /// Vertical Velocity
    pub uy: Field2,
    /// Pressure \[pres, pseudo pressure\]
    pub pres: [Field2; 2],
    /// Collection of solvers \[ux, uy, temp, pres\]
    solver: [SolverField<f64, 2>; 4],
    /// Buffer
    rhs: Array2<NumType>,
    /// Field for temperature boundary condition
    pub fieldbc: Option<Field2>,
    /// Viscosity
    nu: f64,
    /// Thermal diffusivity
    ka: f64,
    /// Rayleigh number
    pub ra: f64,
    /// Prandtl number
    pub pr: f64,
    /// Time
    pub time: f64,
    /// Time step size
    pub dt: f64,
    /// Scale of phsical dimension \[scale_x, scale_y\]
    pub scale: [f64; 2],
    /// diagnostics like Nu, ...
    pub diagnostics: HashMap<String, Vec<f64>>,
    /// Time intervall for write fields
    /// If none, same intervall as diagnostics
    pub write_intervall: Option<f64>,
    /// Add a solid obstacle
    pub solid: Option<Array2<f64>>,
}

impl Navier2DPeriodic {
    /// Returns Navier-Stokes Solver, an integrable type, used with `pde::integrate`
    pub fn new(nx: usize, ny: usize, ra: f64, pr: f64, dt: f64, aspect: f64) -> Self {
        let scale = [aspect, 1.];
        let nu = get_nu(ra, pr, scale[1] * 2.0);
        let ka = get_ka(ra, pr, scale[1] * 2.0);
        let ux = Field2::new(&[fourier_r2c(nx), cheb_dirichlet(ny)]);
        let uy = Field2::new(&[fourier_r2c(nx), cheb_dirichlet(ny)]);
        let temp = Field2::new(&[fourier_r2c(nx), cheb_dirichlet(ny)]);
        Self::from_fields(temp, ux, uy, nu, ka, ra, pr, dt, scale)
    }

    #[allow(clippy::too_many_arguments, clippy::similar_names)]
    fn from_fields(
        temp: Field2,
        ux: Field2,
        uy: Field2,
        nu: f64,
        ka: f64,
        ra: f64,
        pr: f64,
        dt: f64,
        scale: [f64; 2],
    ) -> Self {
        // define additional fields
        let nx = temp.v.shape()[0];
        let ny = temp.v.shape()[1];
        let field = Field2::new(&[fourier_r2c(nx), chebyshev(ny)]);
        let pres = [
            Field2::new(&[fourier_r2c(nx), chebyshev(ny)]),
            Field2::new(&[fourier_r2c(nx), cheb_neumann(ny)]),
        ];

        // define solver
        let solver_ux = SolverField::Hholtz(Hholtz::from_space(
            &ux.space,
            [dt * nu / scale[0].powf(2.), dt * nu / scale[1].powf(2.)],
        ));
        let solver_uy = SolverField::Hholtz(Hholtz::from_space(
            &uy.space,
            [dt * nu / scale[0].powf(2.), dt * nu / scale[1].powf(2.)],
        ));
        let solver_temp = SolverField::Hholtz(Hholtz::from_space(
            &temp.space,
            [dt * ka / scale[0].powf(2.), dt * ka / scale[1].powf(2.)],
        ));
        let solver_pres = SolverField::Poisson(Poisson::from_space(
            &pres[1].space,
            [1. / scale[0].powf(2.), 1. / scale[1].powf(2.)],
        ));
        let solver = [solver_ux, solver_uy, solver_temp, solver_pres];
        let rhs = Array2::<NumType>::zeros(field.vhat.raw_dim());

        // Diagnostics
        let mut diagnostics = HashMap::new();
        diagnostics.insert("time".to_string(), Vec::<f64>::new());
        diagnostics.insert("Nu".to_string(), Vec::<f64>::new());
        diagnostics.insert("Nuvol".to_string(), Vec::<f64>::new());
        diagnostics.insert("Re".to_string(), Vec::<f64>::new());

        // Initialize
        let mut navier = Self {
            field,
            temp,
            ux,
            uy,
            pres,
            solver,
            rhs,
            fieldbc: None,
            nu,
            ka,
            ra,
            pr,
            time: 0.0,
            dt,
            scale,
            diagnostics,
            write_intervall: None,
            solid: None,
        };
        navier._scale();
        // Boundary condition
        navier.set_temp_bc(Self::bc_rbc(nx, ny));
        // Initial condition
        navier.set_velocity(0.2, 1., 1.);
        // Return
        navier
    }

    /// Rescale x & y coordinates of fields.
    /// Only affects output of files
    fn _scale(&mut self) {
        for field in &mut [
            &mut self.temp,
            &mut self.ux,
            &mut self.uy,
            &mut self.pres[0],
        ] {
            field.x[0] *= self.scale[0];
            field.x[1] *= self.scale[1];
        }
    }

    /// Return field for rayleigh benard
    /// type temperature boundary conditions:
    ///
    /// T = 0.5 at the bottom and T = -0.5
    /// at the top
    pub fn bc_rbc(nx: usize, ny: usize) -> Field2 {
        use crate::bases::Transform;
        // Create base and field
        //let mut bases = [fourier_r2c(nx), cheb_dirichlet_bc(ny)];
        let mut fieldbc = Field2::new(&[fourier_r2c(nx), cheb_dirichlet_bc(ny)]);
        let mut bc = Array2::<f64>::zeros((nx, 2));

        // Set boundary condition along axis
        bc.slice_mut(s![.., 0]).fill(0.5);
        bc.slice_mut(s![.., 1]).fill(-0.5);

        // Transform
        fieldbc.space.bases[0].forward_inplace(&mut bc, &mut fieldbc.vhat, 0);
        fieldbc.backward();
        fieldbc.forward();
        fieldbc
    }

    /// Set boundary condition field for temperature
    pub fn set_temp_bc(&mut self, fieldbc: Field2) {
        self.fieldbc = Some(fieldbc);
    }

    fn zero_rhs(&mut self) {
        for r in self.rhs.iter_mut() {
            *r = Complex::new(0., 0.);
        }
    }

    /// Convection term for temperature
    fn conv_temp(&mut self, ux: &Array2<f64>, uy: &Array2<f64>) -> Array2<NumType> {
        // + ux * dTdx + uy * dTdy
        let mut conv = conv_term(&self.temp, &mut self.field, ux, [1, 0], Some(self.scale));
        conv += &conv_term(&self.temp, &mut self.field, uy, [0, 1], Some(self.scale));
        // + bc contribution
        if let Some(field) = &self.fieldbc {
            conv += &conv_term(field, &mut self.field, ux, [1, 0], Some(self.scale));
            conv += &conv_term(field, &mut self.field, uy, [0, 1], Some(self.scale));
        }
        // + solid interaction
        if let Some(solid) = &self.solid {
            let eta = 1e-3;
            self.temp.backward();
            let damp = self.fieldbc.as_ref().map_or_else(
                || -1. / eta * solid * &self.temp.v,
                |field| -1. / eta * solid * &(&self.temp.v + &field.v),
            );
            conv -= &damp;
        }
        // -> spectral space
        self.field.v.assign(&conv);
        self.field.forward();
        self.field.vhat.to_owned()
    }

    /// Convection term for ux
    fn conv_ux(&mut self, ux: &Array2<f64>, uy: &Array2<f64>) -> Array2<NumType> {
        // + ux * dudx + uy * dudy
        let mut conv = conv_term(&self.ux, &mut self.field, ux, [1, 0], Some(self.scale));
        conv += &conv_term(&self.ux, &mut self.field, uy, [0, 1], Some(self.scale));
        // + solid interaction
        if let Some(solid) = &self.solid {
            let eta = 1e-3;
            let damp = -1. / eta * solid * ux;
            conv -= &damp;
        }
        // -> spectral space
        self.field.v.assign(&conv);
        self.field.forward();
        self.field.vhat.to_owned()
    }

    /// Convection term for uy
    fn conv_uy(&mut self, ux: &Array2<f64>, uy: &Array2<f64>) -> Array2<NumType> {
        // + ux * dudx + uy * dudy
        let mut conv = conv_term(&self.uy, &mut self.field, ux, [1, 0], Some(self.scale));
        conv += &conv_term(&self.uy, &mut self.field, uy, [0, 1], Some(self.scale));
        // + solid interaction
        if let Some(solid) = &self.solid {
            let eta = 1e-3;
            let damp = -1. / eta * solid * uy;
            conv -= &damp;
        }
        // -> spectral space
        self.field.v.assign(&conv);
        self.field.forward();
        self.field.vhat.to_owned()
    }

    /// Solve horizontal momentum equation
    /// $$
    /// (1 - \delta t  \mathcal{D}) u\\_new = -dt*C(u) - \delta t grad(p) + \delta t f + u
    /// $$
    fn solve_ux(&mut self, ux: &Array2<f64>, uy: &Array2<f64>) {
        self.zero_rhs();
        // + old field
        self.rhs += &self.ux.to_ortho();
        // + pres
        self.rhs -= &(self.pres[0].grad([1, 0], Some(self.scale)) * self.dt);
        // + convection
        let conv = self.conv_ux(ux, uy);
        self.rhs -= &(conv * self.dt);
        // solve lhs
        self.solver[0].solve(&self.rhs, &mut self.ux.vhat, 0);
    }

    /// Solve vertical momentum equation
    fn solve_uy(&mut self, ux: &Array2<f64>, uy: &Array2<f64>, buoy: &Array2<NumType>) {
        self.zero_rhs();
        // + old field
        self.rhs += &self.uy.to_ortho();
        // + pres
        self.rhs -= &(self.pres[0].grad([0, 1], Some(self.scale)) * self.dt);
        // + buoyancy
        self.rhs += &(buoy * self.dt);
        // + convection
        let conv = self.conv_uy(ux, uy);
        self.rhs -= &(conv * self.dt);
        // solve lhs
        self.solver[1].solve(&self.rhs, &mut self.uy.vhat, 0);
    }

    /// Divergence: duxdx + duydy
    fn divergence(&mut self) -> Array2<NumType> {
        self.zero_rhs();
        self.rhs += &self.ux.grad([1, 0], Some(self.scale));
        self.rhs += &self.uy.grad([0, 1], Some(self.scale));
        self.rhs.to_owned()
    }

    /// Correct velocity field.
    /// $$
    /// uxnew = ux - c*dpdx
    /// $$
    /// uynew = uy - c*dpdy
    /// $$
    #[allow(clippy::similar_names)]
    fn project_velocity(&mut self, c: f64) {
        let dpdx = self.pres[1].grad([1, 0], Some(self.scale));
        let dpdy = self.pres[1].grad([0, 1], Some(self.scale));

        // self.ux.vhat -= &(c * self.ux.from_parent(&dpdx));
        // self.uy.vhat -= &(c * self.uy.from_parent(&dpdy));

        let ux_old = self.ux.vhat.clone();
        let uy_old = self.uy.vhat.clone();
        self.ux.from_ortho(&dpdx);
        self.uy.from_ortho(&dpdy);
        self.ux.vhat = &self.ux.vhat * -c;
        self.uy.vhat = &self.uy.vhat * -c;
        self.ux.vhat += &ux_old;
        self.uy.vhat += &uy_old;
    }

    /// Solve temperature equation:
    /// $$
    /// (1 - dt*D) temp\\_new = -dt*C(temp) + dt*fbc + temp
    /// $$
    fn solve_temp(&mut self, ux: &Array2<f64>, uy: &Array2<f64>) {
        self.zero_rhs();
        // + old field
        self.rhs += &self.temp.to_ortho();
        // + diffusion bc contribution
        if let Some(field) = &self.fieldbc {
            self.rhs += &(field.grad([2, 0], Some(self.scale)) * self.dt * self.ka);
            self.rhs += &(field.grad([0, 2], Some(self.scale)) * self.dt * self.ka);
        }
        // + convection
        let conv = self.conv_temp(ux, uy);
        self.rhs -= &(conv * self.dt);
        // solve lhs
        // self.temp.vhat.assign(&self.solver[2].solve(&self.rhs));
        self.solver[2].solve(&self.rhs, &mut self.temp.vhat, 0);
    }

    /// Solve pressure poisson equation
    /// $$
    /// D2 pres = f
    /// $$
    /// pseu: pseudo pressure ( in code it is pres\[1\] )
    fn solve_pres(&mut self, f: &Array2<NumType>) {
        //self.pres[1].vhat.assign(&self.solver[3].solve(&f));
        self.solver[3].solve(&f, &mut self.pres[1].vhat, 0);
        // Singularity
        self.pres[1].vhat[[0, 0]] = Complex::new(0., 0.);
    }

    fn update_pres(&mut self, div: &Array2<NumType>) {
        self.pres[0].vhat -= &(div * self.nu);
        self.pres[0].vhat += &(&self.pres[1].to_ortho() / self.dt);
    }
}

impl Navier2DPeriodic {
    /// Returns Nusselt number (heat flux at the plates)
    /// $$
    /// Nu = \langle - dTdz \rangle\\_x (0/H))
    /// $$
    pub fn eval_nu(&mut self) -> f64 {
        use super::functions::eval_nu;
        eval_nu::<NumType>(&mut self.temp, &mut self.field, &self.fieldbc, &self.scale)
    }

    /// Returns volumetric Nusselt number
    /// $$
    /// Nuvol = \langle uy*T/kappa - dTdz \rangle\\_V
    /// $$
    pub fn eval_nuvol(&mut self) -> f64 {
        use super::functions::eval_nuvol;
        eval_nuvol::<NumType>(
            &mut self.temp,
            &mut self.uy,
            &mut self.field,
            &self.fieldbc,
            self.ka,
            &self.scale,
        )
    }

    /// Returns Reynolds number based on kinetic energy
    pub fn eval_re(&mut self) -> f64 {
        use super::functions::eval_re;
        eval_re::<NumType>(
            &mut self.ux,
            &mut self.uy,
            &mut self.field,
            self.nu,
            &self.scale,
        )
    }
}

impl Integrate for Navier2DPeriodic {
    ///         Update Navier Stokes
    fn update(&mut self) {
        // Buoyancy
        let mut that = self.temp.to_ortho();
        if let Some(field) = &self.fieldbc {
            that += &field.to_ortho();
        }

        // Convection Veclocity
        self.ux.backward();
        self.uy.backward();
        let ux = self.ux.v.to_owned();
        let uy = self.uy.v.to_owned();

        // Solve Velocity
        self.solve_ux(&ux, &uy);
        self.solve_uy(&ux, &uy, &that);

        // Projection
        let div = self.divergence();
        self.solve_pres(&div);
        self.project_velocity(1.0);
        self.update_pres(&div);

        // Solve Temperature
        self.solve_temp(&ux, &uy);

        // update time
        self.time += self.dt;
    }

    fn get_time(&self) -> f64 {
        self.time
    }

    fn get_dt(&self) -> f64 {
        self.dt
    }

    fn write(&mut self) {
        use std::io::Write;

        // Write hdf5 file
        std::fs::create_dir_all("data").unwrap();

        // Write flow field
        let fname = format!("data/flow{:.*}.h5", 3, self.time);
        if let Some(dt_save) = &self.write_intervall {
            if (self.time % dt_save) < self.dt / 2.
                || (self.time % dt_save) > dt_save - self.dt / 2.
            {
                self.write_to_file(&fname);
            }
        } else {
            self.write_to_file(&fname);
        }

        // I/O
        let div = self.divergence();
        let nu = self.eval_nu();
        let nuvol = self.eval_nuvol();
        let re = self.eval_re();
        println!(
            "time = {:4.2}      |div| = {:4.2e}     Nu = {:5.3e}     Nuv = {:5.3e}    Re = {:5.3e}",
            self.time,
            norm_l2(&div),
            nu,
            nuvol,
            re,
        );

        // diagnostics
        if let Some(d) = self.diagnostics.get_mut("time") {
            d.push(self.time);
        }
        if let Some(d) = self.diagnostics.get_mut("Nu") {
            d.push(nu);
        }
        if let Some(d) = self.diagnostics.get_mut("Nuvol") {
            d.push(nuvol);
        }
        if let Some(d) = self.diagnostics.get_mut("Re") {
            d.push(re);
        }
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open("data/info.txt")
            .unwrap();
        //write!(file, "{} {}", time, nu);
        if let Err(e) = writeln!(file, "{} {} {} {}", self.time, nu, nuvol, re) {
            eprintln!("Couldn't write to file: {}", e);
        }
    }

    fn exit(&mut self) -> bool {
        // Break if divergence is nan
        let div = self.divergence();
        if norm_l2(&div).is_nan() {
            return true;
        }
        false
    }
}

fn norm_l2(array: &Array2<Complex<f64>>) -> f64 {
    array
        .iter()
        .map(|x| x.re.powi(2) + x.im.powi(2))
        .sum::<f64>()
        .sqrt()
}

impl Navier2DPeriodic {
    /// Read from existing file
    ///
    /// ## Panics
    /// Panics if file cannot be read.
    pub fn read(&mut self, fname: &str) {
        // Field
        self.temp.read(&fname, Some("temp"));
        self.ux.read(&fname, Some("ux"));
        self.uy.read(&fname, Some("uy"));
        self.pres[0].read(&fname, Some("pres"));
        // Read scalars
        self.time = read_scalar_from_hdf5::<f64>(&fname, "time", None).unwrap();
        println!(" <== {:?}", fname);
    }

    /// Initialize velocity with fourier modes
    ///
    /// ux = amp \* sin(mx)cos(nx)
    /// uy = -amp \* cos(mx)sin(nx)
    pub fn set_velocity(&mut self, amp: f64, m: f64, n: f64) {
        apply_sin_cos(&mut self.ux, amp, m, n);
        apply_cos_sin(&mut self.uy, -amp, m, n);
    }
    /// Initialize temperature with fourier modes
    ///
    /// temp = -amp \* cos(mx)sin(ny)
    pub fn set_temperature(&mut self, amp: f64, m: f64, n: f64) {
        apply_cos_sin(&mut self.temp, -amp, m, n);
    }

    /// Write fields to hdf5 file
    pub fn write_to_file(&mut self, fname: &str) {
        self.temp.backward();
        self.ux.backward();
        self.uy.backward();
        self.pres[0].backward();
        // Add boundary contribution
        if let Some(x) = &self.fieldbc {
            self.temp.v = &self.temp.v + &x.v;
        }
        // Field
        self.temp.write(&fname, Some("temp"));
        self.ux.write(&fname, Some("ux"));
        self.uy.write(&fname, Some("uy"));
        self.pres[0].write(&fname, Some("pres"));
        // Write scalars
        write_scalar_to_hdf5(&fname, "time", None, self.time).ok();
        write_scalar_to_hdf5(&fname, "ra", None, self.ra).ok();
        write_scalar_to_hdf5(&fname, "pr", None, self.pr).ok();
        write_scalar_to_hdf5(&fname, "nu", None, self.nu).ok();
        write_scalar_to_hdf5(&fname, "kappa", None, self.ka).ok();
        // Undo addition of bc
        if self.fieldbc.is_some() {
            self.temp.backward();
        }

        println!(" ==> {:?}", fname);
    }

    /// Reset time
    pub fn reset_time(&mut self) {
        self.time = 0.;
    }
}

/// Construct field f(x,y) = amp \* sin(pi\*m)cos(pi\*n)
pub fn apply_sin_cos(field: &mut Field2, amp: f64, m: f64, n: f64) {
    use std::f64::consts::PI;
    let nx = field.v.shape()[0];
    let ny = field.v.shape()[1];
    let x = &field.x[0];
    let y = &field.x[1];
    let x = &((x - x[0]) / (x[x.len() - 1] - x[0]));
    let y = &((y - y[0]) / (y[y.len() - 1] - y[0]));
    let arg_x = PI * m;
    let arg_y = PI * n;
    for i in 0..nx {
        for j in 0..ny {
            field.v[[i, j]] = amp * (arg_x * x[i]).sin() * (arg_y * y[j]).cos();
        }
    }
    field.forward()
}

/// Construct field f(x,y) = amp \* cos(pi\*m)sin(pi\*n)
pub fn apply_cos_sin(field: &mut Field2, amp: f64, m: f64, n: f64) {
    use std::f64::consts::PI;
    let nx = field.v.shape()[0];
    let ny = field.v.shape()[1];
    let x = &field.x[0];
    let y = &field.x[1];
    let x = &((x - x[0]) / (x[x.len() - 1] - x[0]));
    let y = &((y - y[0]) / (y[y.len() - 1] - y[0]));
    let arg_x = PI * m;
    let arg_y = PI * n;
    for i in 0..nx {
        for j in 0..ny {
            field.v[[i, j]] = amp * (arg_x * x[i]).cos() * (arg_y * y[j]).sin();
        }
    }
    field.forward()
}
