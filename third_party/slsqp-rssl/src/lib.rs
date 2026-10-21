pub mod blas;
pub mod error;
pub use error::SlsqpError;
pub mod hfti;
pub mod householder;
pub mod ldl;
pub mod lsei;
pub mod lsq;
pub mod matrix;
pub mod mode;
pub use mode::SlsqpMode;
pub mod nnls;
mod solver;
mod solver_body;
pub mod wasm;
pub use matrix::{Mat, MatView};
pub use solver::Slsqp;

/// Optional exact derivatives for callers with a known smooth objective and
/// constraint system. The Jacobian callback fills `jac[(row, column)]` in the
/// same constraint order used by the solver: equality rows first, followed by
/// inequality rows, retaining input order within each group. The final extra
/// column is workspace and should be left at zero.
pub struct AnalyticDerivatives<'a> {
    pub objective: ObjectiveGradient<'a>,
    pub constraints: ConstraintJacobian<'a>,
}

pub type ObjectiveGradient<'a> = Box<dyn Fn(&[f64], &mut [f64]) + 'a>;
pub type ConstraintJacobian<'a> = Box<dyn Fn(&[f64], &mut Mat) + 'a>;

/// A scalar inequality `g(x) >= 0` used by the exact-derivative entry point.
pub type Inequality<'a> = Box<dyn Fn(&[f64]) -> f64 + 'a>;

/// Constraint definition
pub enum Constraint<'a> {
    /// Equality constraint: f(x) == 0
    Eq(Box<dyn Fn(&[f64]) -> f64 + 'a>),
    /// Inequality constraint: f(x) >= 0
    Ineq(Box<dyn Fn(&[f64]) -> f64 + 'a>),
}

/// Events that can be observed during the optimization process.
#[derive(Debug, Clone)]
pub enum SlsqpEvent<'a> {
    /// Step event triggered at the end of each solver step
    Step {
        iter: usize,
        mode: SlsqpMode,
        x: &'a [f64],
        f: f64,
        g: &'a [f64],
        c: &'a [f64],
        alpha: f64,
        s: &'a [f64],
        h: &'a [f64],
    },
}

/// Trait for observing the optimization process.
pub trait SlsqpObserver {
    /// Returns true if the observer is active and should receive events.
    /// This allows for zero-cost abstraction when no observation is needed.
    fn is_active(&self) -> bool;

    /// Called when an event occurs during the optimization process.
    fn on_event(&mut self, event: SlsqpEvent);
}

/// Implementation of SlsqpObserver for `()`, which does nothing.
/// This is used as the default observer to ensure zero cost when no observation is needed.
impl SlsqpObserver for () {
    #[inline(always)]
    fn is_active(&self) -> bool {
        false
    }

    #[inline(always)]
    fn on_event(&mut self, _event: SlsqpEvent) {}
}

#[cfg_attr(
    feature = "wasm",
    wasm_bindgen::prelude::wasm_bindgen(getter_with_clone)
)]
#[derive(Debug, Clone)]
pub struct SlsqpResult {
    /// Optimized variable values
    pub x: Vec<f64>,
    /// Exit status code
    pub status: i32,
    /// Description of the exit status
    pub message: String,
    /// Number of iterations
    pub nit: usize,
    /// Final objective function value
    pub fun: f64,
}

#[cfg_attr(
    feature = "wasm",
    wasm_bindgen::prelude::wasm_bindgen(getter_with_clone)
)]
#[derive(Debug, Clone)]
pub struct OptimizationStep {
    /// Iteration number
    pub iter: usize,
    /// Current mode
    pub mode: SlsqpMode,
    /// Current variable values
    pub x: Vec<f64>,
    /// Current objective function value
    pub fun: f64,
    /// Current gradient of the objective function (first n elements)
    pub grad: Vec<f64>,
    /// Current constraints values
    pub constraints: Vec<f64>,
    /// Current LDL' factors of the Hessian approximation (lower triangle)
    pub l: Vec<f64>,
    /// Step size used in the line search
    pub alpha: f64,
    /// Search direction (first n elements)
    pub s: Vec<f64>,
}

/// Compatibility observer that wraps a callback.
pub struct CallbackObserver<'a> {
    callback: Option<&'a mut dyn FnMut(&OptimizationStep)>,
}

impl<'a> SlsqpObserver for CallbackObserver<'a> {
    fn is_active(&self) -> bool {
        self.callback.is_some()
    }

    fn on_event(&mut self, event: SlsqpEvent) {
        if let Some(ref mut cb) = self.callback {
            let SlsqpEvent::Step {
                iter,
                mode,
                x,
                f,
                g,
                c,
                alpha,
                s,
                h,
                ..
            } = event;

            // Only trigger callback for major phases: Init, LineSearch, or Success
            // This concentrates phases into one step per iteration in the UI
            if matches!(
                mode,
                SlsqpMode::Init | SlsqpMode::LineSearch | SlsqpMode::Success
            ) {
                cb(&OptimizationStep {
                    iter,
                    mode,
                    x: x.to_vec(),
                    fun: f,
                    grad: g.to_vec(),
                    constraints: c.to_vec(),
                    l: h.to_vec(),
                    alpha,
                    s: s.to_vec(),
                });
            }
        }
    }
}

/// Wraps the SLSQP interface, providing a high-level interface similar to scipy.optimize.fmin_slsqp.
///
/// # Parameters
/// - `func`: Objective function f(x)
/// - `x0`: Initial point
/// - `bounds`: Variable bounds [(lower, upper), ...], no bounds if empty
/// - `constraints`: List of constraints (equality or inequality)
/// - `max_iter`: Maximum number of iterations
/// - `acc`: Accuracy requirement
/// - `callback`: Optional callback function called at each major iteration
pub fn fmin_slsqp<'a>(
    func: impl Fn(&[f64]) -> f64 + 'a,
    x0: &[f64],
    bounds: &[(f64, f64)],
    constraints: Vec<Constraint<'a>>,
    max_iter: usize,
    acc: f64,
    callback: Option<&mut dyn FnMut(&OptimizationStep)>,
) -> SlsqpResult {
    let observer = CallbackObserver { callback };
    fmin_slsqp_observed(func, x0, bounds, constraints, max_iter, acc, observer)
}

/// `fmin_slsqp` with exact derivatives for an all-inequality system.
/// The Jacobian callback's row `i` corresponds to `inequalities[i]`; no
/// equality-row sorting can silently invalidate that correspondence. Existing
/// callers retain the original finite-difference behavior.
pub fn fmin_slsqp_inequalities_with_derivatives<'a>(
    func: impl Fn(&[f64]) -> f64 + 'a,
    x0: &[f64],
    bounds: &[(f64, f64)],
    inequalities: Vec<Inequality<'a>>,
    derivatives: AnalyticDerivatives<'a>,
    max_iter: usize,
    acc: f64,
) -> SlsqpResult {
    let constraints = inequalities.into_iter().map(Constraint::Ineq).collect();
    let mut solver = Slsqp::new_with_observer(
        x0.to_vec(),
        bounds,
        Box::new(func),
        constraints,
        max_iter,
        acc,
        (),
    );
    solver.derivatives = Some(derivatives);
    finish_slsqp(solver)
}

/// Version of fmin_slsqp that accepts a generic observer.
pub fn fmin_slsqp_observed<'a, O: SlsqpObserver>(
    func: impl Fn(&[f64]) -> f64 + 'a,
    x0: &[f64],
    bounds: &[(f64, f64)],
    constraints: Vec<Constraint<'a>>,
    max_iter: usize,
    acc: f64,
    observer: O,
) -> SlsqpResult {
    let solver = Slsqp::new_with_observer(
        x0.to_vec(),
        bounds,
        Box::new(func),
        constraints,
        max_iter,
        acc,
        observer,
    );

    finish_slsqp(solver)
}

fn finish_slsqp<O: SlsqpObserver>(mut solver: Slsqp<'_, O>) -> SlsqpResult {
    loop {
        match solver.step() {
            Ok(SlsqpMode::Success) => {
                let nit = solver.iter_count();
                return SlsqpResult {
                    x: solver.x,
                    status: SlsqpMode::Success as i32,
                    message: SlsqpMode::Success.message().to_string(),
                    nit,
                    fun: solver.f,
                };
            }
            Ok(_) => {}
            Err(err) => {
                let nit = solver.iter_count();
                return SlsqpResult {
                    x: solver.x,
                    status: err as i32,
                    message: err.message().to_string(),
                    nit,
                    fun: solver.f,
                };
            }
        }
    }
}
