use crate::{AnalyticDerivatives, Constraint, Mat, SlsqpError, SlsqpMode, SlsqpObserver};

/// Persistent state in the SLSQP algorithm, corresponding to the SAVE variables in the original Fortran code.
#[derive(Debug, Clone, Default)]
pub struct SlsqpbState {
    pub alpha: f64,
    pub prev_f: f64,
    pub directional_derivative: f64,
    pub constraint_violation: f64,
    pub incompatibility_factor: f64,
    pub merit_value: f64,
    pub merit_derivative: f64,
    pub merit_value_at_start: f64,
    pub inconsistent_count: i32,
    /// It is a counter shared by one optimization
    pub reset_count: i32,
}

use std::fmt;

/// Main structure for SLSQP optimization
pub struct Slsqp<'a, O: SlsqpObserver = ()> {
    /// Objective function
    pub func: Box<dyn Fn(&[f64]) -> f64 + 'a>,
    /// Constraints
    pub constraints: Vec<Constraint<'a>>,
    /// Exact derivatives, when supplied by an explicitly opting-in caller.
    pub derivatives: Option<AnalyticDerivatives<'a>>,

    /// Init point
    pub x0: Vec<f64>,

    /// Max iteration count
    max_iter: usize,

    /// Optimized variable values
    pub x: Vec<f64>,
    /// Objective function value
    pub f: f64,
    /// Gradient of the objective function (size n+1)
    pub gradient: Vec<f64>,
    /// Constraint values vector
    pub constraint_values: Vec<f64>,
    /// Jacobian of the constraints (m x (n+1) matrix, stored in column-major order)
    pub jac: Mat,
    /// Leading dimension of the Jacobian matrix
    pub max_constraints: usize,
    /// Lower bounds for variables
    pub lower_bounds: Vec<f64>,
    /// Upper bounds for variables
    pub upper_bounds: Vec<f64>,
    /// Accuracy requirement
    pub accuracy: f64,
    /// Tolerance
    pub tolerance: f64,
    /// Iteration count
    iter_count: usize,

    /// Total number of constraints
    pub num_constraints: usize,

    /// Number of equality constraints
    pub num_equals: usize,

    /// Workspace for LSQ subproblem: multipliers and other info
    pub multipliers: Vec<f64>,
    /// Cholesky/LDL' factors of the Hessian approximation (lower triangle)
    pub hessian: Mat,
    /// Previous point x for line search and BFGS update
    pub x_prev: Vec<f64>,
    /// Lagrange multipliers for constraints
    pub lagrange_multipliers: Vec<f64>,
    /// Search direction vector from the LSQ subproblem
    pub search_direction: Vec<f64>,
    /// Internal workspace vectors for slsqpb
    pub slsqpb_u: Vec<f64>,
    /// Internal workspace vectors for slsqpb
    pub slsqpb_v: Vec<f64>,
    /// Lsei real number workspace
    pub lsei_workspace: LseiWorkspace,
    /// Persistent state across calls to slsqpb
    pub state: SlsqpbState,
    /// Observer for optimization steps and internal calculations
    pub observer: O,
    /// Current mode of the solver
    pub(crate) mode: SlsqpMode,
}

impl<'a, O: SlsqpObserver> fmt::Debug for Slsqp<'a, O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Slsqp")
            .field("mode", &self.mode)
            .field("x", &self.x)
            .field("f", &self.f)
            .field("gradient", &self.gradient)
            .field("c", &self.constraint_values)
            .field("jac", &self.jac)
            .field("max_constraints", &self.max_constraints)
            .field("lower_bounds", &self.lower_bounds)
            .field("upper_bounds", &self.upper_bounds)
            .field("accuracy", &self.accuracy)
            .field("iter_count", &self.iter_count)
            .field("num_constraints", &self.num_constraints)
            .field("num_equals", &self.num_equals)
            .field("multipliers", &self.multipliers)
            .field("hessian", &self.hessian)
            .field("x_prev", &self.x_prev)
            .field("lagrange_multipliers", &self.lagrange_multipliers)
            .field("search_direction", &self.search_direction)
            .field("slsqpb_u", &self.slsqpb_u)
            .field("slsqpb_v", &self.slsqpb_v)
            .field("lsei_workspace", &self.lsei_workspace)
            .field("state", &self.state)
            .finish()
    }
}

/// Helper struct to hold sub-slices of the lsei_real_workspace
#[derive(Debug)]
pub struct LseiWorkspace {
    /// Hessian factor (matrix E)
    pub matrix_e: Vec<f64>,
    /// Objective linear term (vector f)
    pub vector_f: Vec<f64>,
    /// Equality Jacobian (matrix C)
    pub matrix_c: Vec<f64>,
    /// Equality values (vector d)
    pub vector_d: Vec<f64>,
    /// Inequality Jacobian (matrix G)
    pub matrix_g: Vec<f64>,
    /// Inequality values (vector h)
    pub vector_h: Vec<f64>,
    /// Internal workspace for LSEI
    pub internal_workspace: Vec<f64>,
    /// Integer workspace array for lsq
    pub int_workspace: Vec<i32>,
}

impl<'a, O: SlsqpObserver> Slsqp<'a, O> {
    pub fn new_with_observer(
        x0: Vec<f64>,
        bounds: &[(f64, f64)],
        func: Box<dyn Fn(&[f64]) -> f64 + 'a>,
        mut constraints: Vec<Constraint<'a>>,
        max_iter: usize,
        acc: f64,
        observer: O,
    ) -> Self {
        // Sort constraints: equalities first, then inequalities, to meet slsqp's expectation
        constraints.sort_by(|a, b| match (a, b) {
            (Constraint::Eq(_), Constraint::Ineq(_)) => std::cmp::Ordering::Less,
            (Constraint::Ineq(_), Constraint::Eq(_)) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        });

        let accuracy = acc.abs();
        let num_vars = x0.len();
        let n_plus_1 = num_vars + 1;
        let num_constraints = constraints.len();
        let max_constraints = num_constraints.max(1);
        let num_equals = constraints
            .iter()
            .filter(|c| matches!(c, Constraint::Eq(..)))
            .count();
        let num_ineq_plus_bounds_n1 = (num_constraints - num_equals) + 2 * n_plus_1;

        let total_workspace_len = (3 * n_plus_1 + num_constraints) * (n_plus_1 + 1)
            + (n_plus_1 + 1) * (max_constraints + 2 * n_plus_1 + 2) // Added margin
            + 2 * (max_constraints + 2 * n_plus_1)
            + (n_plus_1 + max_constraints + 2 * n_plus_1) * n_plus_1
            + 2 * num_constraints
            + 3 * num_vars
            + 3 * n_plus_1
            + 1000;

        let state = SlsqpbState::default();

        let mut lower_bounds = vec![-1.0e12; num_vars];
        let mut upper_bounds = vec![1.0e12; num_vars];

        if !bounds.is_empty() {
            for (i, &(l, u)) in bounds.iter().enumerate() {
                lower_bounds[i] = l;
                upper_bounds[i] = u;
            }
        }

        let mut x = x0.clone();
        for i in 0..num_vars {
            x[i] = x[i].clamp(lower_bounds[i], upper_bounds[i]);
        }

        Self {
            x0,
            func,
            constraints,
            derivatives: None,
            x,
            f: 0.0,
            max_iter,
            // extra element used to process when constraints are conflicting
            gradient: vec![0.0; n_plus_1],
            constraint_values: vec![0.0; num_constraints],
            jac: Mat::new(max_constraints, n_plus_1),
            max_constraints,
            lower_bounds,
            upper_bounds,
            accuracy,
            tolerance: accuracy * 10.0,
            iter_count: 0,
            num_constraints,
            num_equals,
            multipliers: vec![0.0; num_constraints + 2 * n_plus_1],
            hessian: Mat::new(n_plus_1, n_plus_1),
            x_prev: vec![0.0; num_vars],
            lagrange_multipliers: vec![0.0; num_constraints + 1],
            search_direction: vec![0.0; n_plus_1],
            slsqpb_u: vec![0.0; n_plus_1],
            slsqpb_v: vec![0.0; n_plus_1],
            lsei_workspace: LseiWorkspace {
                matrix_e: vec![0.0; n_plus_1 * n_plus_1],
                vector_f: vec![0.0; n_plus_1],
                matrix_c: vec![0.0; num_equals * n_plus_1],
                vector_d: vec![0.0; num_equals],
                matrix_g: vec![0.0; num_ineq_plus_bounds_n1 * n_plus_1],
                vector_h: vec![0.0; num_ineq_plus_bounds_n1],
                internal_workspace: vec![0.0; total_workspace_len],
                int_workspace: vec![0; total_workspace_len],
            },
            state,
            observer,
            mode: SlsqpMode::Init,
        }
    }

    /// number of eq constraint
    pub fn constraint_eq_size(&self) -> usize {
        self.constraints
            .iter()
            .filter(|f| matches!(f, Constraint::Eq(..)))
            .count()
    }

    /// number of ineq constraint
    pub fn constraint_ineq_size(&self) -> usize {
        self.constraints
            .iter()
            .filter(|f| matches!(f, Constraint::Ineq(..)))
            .count()
    }

    /// Evaluates both objective function and constraints at the current point x
    pub fn evaluate_func(&mut self) {
        self.f = (self.func)(&self.x);

        for (i, con) in self.constraints.iter().enumerate() {
            match con {
                Constraint::Eq(f_con) => self.constraint_values[i] = f_con(&self.x),
                Constraint::Ineq(f_con) => self.constraint_values[i] = f_con(&self.x),
            }
        }
    }

    #[inline(always)]
    pub fn num_vars(&self) -> usize {
        self.x0.len()
    }

    // sqrt(eps) for finite difference gradient estimation
    const EPSILON: f64 = 1.4901161193847656e-08;

    /// Evaluates the gradient of the objective function using finite differences
    pub fn evaluate_gradient(&mut self) {
        if let Some(derivatives) = &self.derivatives {
            let n = self.num_vars();
            (derivatives.objective)(&self.x, &mut self.gradient[..n]);
            self.gradient[n] = 0.0;
            return;
        }
        let epsilon = Self::EPSILON;
        let f0 = self.f;
        let n = self.num_vars();
        for i in 0..n {
            let tmp = self.x[i];
            self.x[i] += epsilon;
            let f1 = (self.func)(&self.x);
            self.gradient[i] = (f1 - f0) / epsilon;
            self.x[i] = tmp;
        }
        self.gradient[n] = 0.0;
    }

    /// Evaluates the Jacobian of the constraints using finite differences
    pub fn evaluate_jacobian(&mut self) {
        if let Some(derivatives) = &self.derivatives {
            self.jac.fill(0.0);
            (derivatives.constraints)(&self.x, &mut self.jac);
            return;
        }
        let epsilon = Self::EPSILON;
        let n = self.num_vars();
        for (j, con) in self.constraints.iter().enumerate() {
            let f_con0 = self.constraint_values[j];
            for i in 0..n {
                let tmp = self.x[i];
                self.x[i] += epsilon;
                let f_con1 = match con {
                    Constraint::Eq(f_con) => f_con(&self.x),
                    Constraint::Ineq(f_con) => f_con(&self.x),
                };
                self.jac[(j, i)] = (f_con1 - f_con0) / epsilon;
                self.x[i] = tmp;
            }
        }
    }

    pub fn incr_iter_count(&mut self) -> Result<(), SlsqpError> {
        self.iter_count += 1;
        if self.iter_count > self.max_iter {
            return Err(SlsqpError::IterationLimitExceeded);
        }
        Ok(())
    }

    pub fn iter_count(&self) -> usize {
        self.iter_count
    }

    pub(crate) fn reset_hessian(&mut self) {
        self.hessian.identify();
    }
}

impl Slsqp<'static, ()> {
    pub fn new(
        x0: Vec<f64>,
        bounds: &[(f64, f64)],
        func: Box<dyn Fn(&[f64]) -> f64 + 'static>,
        constraints: Vec<Constraint<'static>>,
        max_iter: usize,
        acc: f64,
    ) -> Self {
        Self::new_with_observer(x0, bounds, func, constraints, max_iter, acc, ())
    }
}
