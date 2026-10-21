use crate::blas::{vector_add_scaled, vector_copy, vector_dot_product, vector_norm, vector_scale};
use crate::ldl::ldl;
use crate::{Slsqp, SlsqpEvent, SlsqpObserver};
use crate::{SlsqpError, SlsqpMode};

impl<'a, O: SlsqpObserver> Slsqp<'a, O> {
    /// The main logic is located in this method.
    pub fn step(&mut self) -> Result<SlsqpMode, SlsqpError> {
        let mode_result = match self.mode {
            SlsqpMode::Init => {
                self.evaluate_func();
                self.evaluate_gradient();
                self.evaluate_jacobian();

                self.reset_hessian();
                self.compute_direction()
            }
            SlsqpMode::LineSearch => {
                self.state.alpha = 1.0;

                let mut line_search_trials = 1;
                self.line_search_iexact();

                loop {
                    self.evaluate_func();

                    self.state.merit_value = self.f;
                    for j in 0..self.num_constraints {
                        let h1_val = if j < self.num_equals {
                            self.constraint_values[j]
                        } else {
                            0.0
                        };
                        self.state.merit_value +=
                            self.lagrange_multipliers[j] * (-self.constraint_values[j]).max(h1_val);
                    }

                    let merit_change = self.state.merit_value - self.state.merit_value_at_start;
                    if merit_change <= self.state.merit_derivative / 10.0 || line_search_trials > 10
                    {
                        // if convergence meets success conditions, it returns Success and optimization is finished
                        // otherwise, it returns EvalGrad
                        break self.check_convergence();
                    } else {
                        self.state.alpha = (self.state.merit_derivative
                            / (2.0 * (self.state.merit_derivative - merit_change)))
                            .max(0.1);
                        line_search_trials += 1;
                        self.line_search_iexact();
                    }
                }
            }
            SlsqpMode::EvalGrad => {
                self.evaluate_gradient();
                self.evaluate_jacobian();

                let num_variables = self.x.len();
                for i in 0..num_variables {
                    self.slsqpb_u[i] = self.gradient[i]
                        - vector_dot_product(
                            self.num_constraints,
                            self.jac.col(i),
                            1,
                            &self.multipliers,
                            1,
                        )
                        - self.slsqpb_v[i];
                }

                for i in 0..num_variables {
                    let mut h1_val = 0.0;
                    for j in i + 1..num_variables {
                        h1_val += self.hessian[(j, i)] * self.search_direction[j];
                    }
                    self.slsqpb_v[i] = self.search_direction[i] + h1_val;
                }

                for i in 0..num_variables {
                    self.slsqpb_v[i] = self.hessian[(i, i)] * self.slsqpb_v[i];
                }

                for i in (0..num_variables).rev() {
                    let mut h1_val = 0.0;
                    for j in 0..i {
                        h1_val += self.hessian[(i, j)] * self.slsqpb_v[j];
                    }
                    self.slsqpb_v[i] += h1_val;
                }

                let h1_val =
                    vector_dot_product(num_variables, &self.search_direction, 1, &self.slsqpb_u, 1);
                let h2_val =
                    vector_dot_product(num_variables, &self.search_direction, 1, &self.slsqpb_v, 1);
                let h3_val = 0.2 * h2_val;
                if h1_val < h3_val {
                    let h4_val = (h2_val - h3_val) / (h2_val - h1_val);
                    vector_scale(num_variables, h4_val, &mut self.slsqpb_u);
                    vector_add_scaled(
                        num_variables,
                        1.0 - h4_val,
                        &self.slsqpb_v,
                        &mut self.slsqpb_u,
                    );
                }
                let h1_upd = h1_val.max(h3_val);
                ldl(
                    num_variables,
                    &mut self.hessian,
                    &mut self.slsqpb_u,
                    1.0 / h1_upd,
                    &mut self.slsqpb_v,
                );
                ldl(
                    num_variables,
                    &mut self.hessian,
                    &mut self.slsqpb_v,
                    -1.0 / h2_val,
                    &mut self.slsqpb_u,
                );

                self.compute_direction()
            }
            SlsqpMode::Success => Ok(SlsqpMode::Success),
        };

        if let Ok(mode) = mode_result {
            self.mode = mode;
        }
        self.emit_step_event();
        mode_result
    }

    /// emit step event, captures solver key values
    fn emit_step_event(&mut self) {
        if self.observer.is_active() {
            let n = self.num_vars();
            self.observer.on_event(SlsqpEvent::Step {
                iter: self.iter_count(),
                mode: self.mode,
                x: &self.x,
                f: self.f,
                g: &self.gradient[..n],
                c: &self.constraint_values,
                alpha: self.state.alpha,
                s: &self.search_direction[..n],
                h: self.hessian.as_slice(),
            });
        }
    }

    /// Compute search direction.
    /// If reached the optimal solution, it also returns Success
    fn compute_direction(&mut self) -> Result<SlsqpMode, SlsqpError> {
        let num_variables = self.x.len();
        let one = 1.0;
        let hun = 100.0;
        let ten = 10.0;
        let two = 2.0;

        self.incr_iter_count()?;

        self.slsqpb_u[..num_variables].copy_from_slice(&self.lower_bounds[..num_variables]);
        self.slsqpb_v[..num_variables].copy_from_slice(&self.upper_bounds[..num_variables]);
        vector_add_scaled(num_variables, -one, &self.x, &mut self.slsqpb_u);
        vector_add_scaled(num_variables, -one, &self.x, &mut self.slsqpb_v);
        self.state.incompatibility_factor = one;

        match self.lsq(num_variables) {
            Ok(_) => {}
            Err(mut err) => {
                if matches!(err, SlsqpError::SingularMatrixC) && num_variables == self.num_equals {
                    err = SlsqpError::IncompatibleConstraints;
                }
                match err {
                    SlsqpError::IncompatibleConstraints => {
                        for j in 0..self.num_constraints {
                            if j < self.num_equals {
                                self.jac[(j, num_variables)] = -self.constraint_values[j];
                            } else {
                                self.jac[(j, num_variables)] =
                                    (-self.constraint_values[j]).max(0.0);
                            }
                        }
                        for i in 0..num_variables {
                            self.search_direction[i] = 0.0;
                        }
                        self.state.merit_derivative = 0.0;
                        self.gradient[num_variables] = 0.0;
                        let n_idx = self.num_vars();
                        self.hessian[(n_idx, n_idx)] = hun;
                        self.search_direction[num_variables] = one;
                        self.slsqpb_u[num_variables] = 0.0;
                        self.slsqpb_v[num_variables] = one;
                        self.state.inconsistent_count = 0;

                        loop {
                            let r = self.lsq(num_variables + 1);
                            self.state.incompatibility_factor =
                                one - self.search_direction[num_variables];
                            match r {
                                Err(SlsqpError::IncompatibleConstraints) => {
                                    self.hessian[(n_idx, n_idx)] *= ten;
                                    self.state.inconsistent_count += 1;
                                    if self.state.inconsistent_count > 5 {
                                        return Err(err);
                                    }
                                }
                                Err(err) => {
                                    return Err(err);
                                }
                                Ok(_) => {
                                    break;
                                }
                            }
                        }
                    }
                    _ => {
                        return Err(err);
                    }
                }
            }
        }

        for i in 0..num_variables {
            self.slsqpb_v[i] = self.gradient[i]
                - vector_dot_product(
                    self.num_constraints,
                    self.jac.col(i),
                    1,
                    &self.multipliers,
                    1,
                );
        }
        self.state.prev_f = self.f;
        self.x_prev.copy_from_slice(&self.x);

        self.state.directional_derivative =
            vector_dot_product(num_variables, &self.gradient, 1, &self.search_direction, 1);
        let mut merit_change = self.state.directional_derivative.abs();
        self.state.constraint_violation = 0.0;
        for j in 0..self.num_constraints {
            let h3_val = if j < self.num_equals {
                self.constraint_values[j]
            } else {
                0.0
            };
            self.state.constraint_violation += (-self.constraint_values[j]).max(h3_val);
            let h3_abs = self.multipliers[j].abs();
            self.lagrange_multipliers[j] =
                h3_abs.max((self.lagrange_multipliers[j] + h3_abs) / two);
            merit_change += h3_abs * self.constraint_values[j].abs();
        }

        if merit_change < self.accuracy && self.state.constraint_violation < self.accuracy {
            return Ok(SlsqpMode::Success);
        }

        merit_change = 0.0;
        for j in 0..self.num_constraints {
            let h3_val = if j < self.num_equals {
                self.constraint_values[j]
            } else {
                0.0
            };
            merit_change += self.lagrange_multipliers[j] * (-self.constraint_values[j]).max(h3_val);
        }
        self.state.merit_value_at_start = self.f + merit_change;
        self.state.merit_derivative =
            self.state.directional_derivative - merit_change * self.state.incompatibility_factor;

        if self.state.merit_derivative >= 0.0 {
            self.state.reset_count += 1;
            if self.state.reset_count > 5 {
                return self.check_relaxed_convergence();
            }
            self.reset_hessian();
            self.compute_direction()
        } else {
            // find a downward direction, do line min search
            Ok(SlsqpMode::LineSearch)
        }
    }

    fn line_search_iexact(&mut self) {
        let num_variables = self.x.len();
        self.state.merit_derivative = self.state.alpha * self.state.merit_derivative;
        let alpha = self.state.alpha;
        vector_scale(num_variables, alpha, &mut self.search_direction);
        vector_copy(num_variables, &self.x_prev, 1, &mut self.x, 1);
        vector_add_scaled(num_variables, 1.0, &self.search_direction, &mut self.x);
    }

    fn check_convergence(&mut self) -> Result<SlsqpMode, SlsqpError> {
        self.state.constraint_violation = 0.0;
        for j in 0..self.num_constraints {
            let h1_val = if j < self.num_equals {
                self.constraint_values[j]
            } else {
                0.0
            };
            self.state.constraint_violation += (-self.constraint_values[j]).max(h1_val);
        }
        let f0 = self.state.prev_f;
        if ((self.f - f0).abs() < self.accuracy
            || vector_norm(self.num_vars(), &self.search_direction) < self.accuracy)
            && self.state.constraint_violation < self.accuracy
        {
            Ok(SlsqpMode::Success)
        } else {
            Ok(SlsqpMode::EvalGrad)
        }
    }

    fn check_relaxed_convergence(&mut self) -> Result<SlsqpMode, SlsqpError> {
        let num_variables = self.x.len();
        if ((self.f - self.state.prev_f).abs() < self.tolerance
            || vector_norm(num_variables, &self.search_direction) < self.tolerance)
            && self.state.constraint_violation < self.tolerance
        {
            Ok(SlsqpMode::Success)
        } else {
            Err(SlsqpError::PositiveDirectionalDerivative)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Constraint;

    #[test]
    fn test_slsqpb_basic() {
        let variables = [2.0, 2.0];
        let objective_value;
        let mut constraint_values = [0.0];
        let mut objective_gradient = [0.0, 0.0, 0.0]; // n+1
        let mut constraints_jacobian = [0.0, 0.0, 0.0];

        let func = Box::new(|x: &[f64]| x[0] * x[0] + x[1] * x[1]);
        let constraints = vec![Constraint::Eq(Box::new(|x: &[f64]| x[0] + x[1] - 2.0))];

        let bounds = [(-10.0, 10.0), (-10.0, 10.0)];
        let mut slsqp = Slsqp::new(variables.to_vec(), &bounds, func, constraints, 100, 1e-6);

        // Initial evaluation
        objective_value = slsqp.x[0] * slsqp.x[0] + slsqp.x[1] * slsqp.x[1];
        constraint_values[0] = slsqp.x[0] + slsqp.x[1] - 2.0;
        objective_gradient[0] = 2.0 * slsqp.x[0];
        objective_gradient[1] = 2.0 * slsqp.x[1];
        objective_gradient[2] = 0.0;
        constraints_jacobian[0] = 1.0;
        constraints_jacobian[1] = 1.0;
        constraints_jacobian[2] = 0.0;

        slsqp.f = objective_value;
        slsqp.constraint_values.copy_from_slice(&constraint_values);
        slsqp.gradient.copy_from_slice(&objective_gradient);
        slsqp.jac.data.copy_from_slice(&constraints_jacobian);

        // Loop for reverse communication
        for _ in 0..100 {
            let mode = slsqp.step().unwrap();

            if mode == SlsqpMode::Success {
                break;
            } else if mode == SlsqpMode::LineSearch {
                // Function evaluation
                slsqp.f = slsqp.x[0] * slsqp.x[0] + slsqp.x[1] * slsqp.x[1];
                slsqp.constraint_values[0] = slsqp.x[0] + slsqp.x[1] - 2.0;
            } else if mode == SlsqpMode::EvalGrad {
                // Gradient evaluation
                slsqp.gradient[0] = 2.0 * slsqp.x[0];
                slsqp.gradient[1] = 2.0 * slsqp.x[1];
                slsqp.gradient[2] = 0.0;

                slsqp.jac[(0, 0)] = 1.0; // df/dx1
                slsqp.jac[(0, 1)] = 1.0; // df/dx2
                slsqp.jac[(0, 2)] = 0.0;
            } else {
                panic!("SLSQP failed with status {}", mode);
            }
        }

        assert!(slsqp.mode == SlsqpMode::Success);
        assert!((slsqp.x[0] - 1.0).abs() < 1e-4);
        assert!((slsqp.x[1] - 1.0).abs() < 1e-4);
    }
}
