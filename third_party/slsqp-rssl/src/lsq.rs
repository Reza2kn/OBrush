use crate::blas::{vector_dot_product, vector_mul};
use crate::lsei::lsei;
use crate::solver::LseiWorkspace;
use crate::{Slsqp, SlsqpError, SlsqpObserver};

impl<O: SlsqpObserver> Slsqp<'_, O> {
    /// LSQ: SOLUTION OF THE QUADRATIC PROGRAM
    /// Do lsq for num of vars, num_vars can be 1 more than problem's n in case of IncompatibleConstraints
    pub(crate) fn lsq(&mut self, num_vars: usize) -> Result<(), SlsqpError> {
        let workspace = self;
        let num_constraints = workspace.num_constraints;
        let num_equals = workspace.num_equals;

        let num_vars_plus_1 = num_vars + 1;
        let num_inequalities = num_constraints - num_equals;
        let total_inequalities = num_inequalities + num_vars + num_vars;

        // determine whether to solve problem with inconsistent linearization (inconsistency_flag=1) or not (inconsistency_flag=0)
        let inconsistency_flag = if num_vars == workspace.num_vars() + 1 {
            1
        } else {
            0
        };
        let num_vars_filtered = num_vars - inconsistency_flag;

        let mut e_idx = 0;
        let mut f_aux_idx = 0;

        {
            let Slsqp {
                lsei_workspace:
                    LseiWorkspace {
                        matrix_e,
                        vector_f,
                        matrix_c,
                        vector_d,
                        matrix_g,
                        vector_h,
                        ..
                    },
                hessian,
                slsqpb_u,
                slsqpb_v,
                gradient,
                jac,
                ..
            } = workspace;

            for index in 0..num_vars_filtered {
                let num_remaining_vars = num_vars - index;
                let diagonal_element = hessian[(index, index)].sqrt();

                // matrix_e(e_idx) = ZERO
                for k in 0..num_remaining_vars {
                    matrix_e[e_idx + k] = 0.0;
                }

                // CALL dcopy_ (num_remaining_vars-inconsistency_flag, l(hessian_idx), 1, matrix_e(e_idx), num_vars)
                if num_remaining_vars > inconsistency_flag {
                    let count = num_remaining_vars - inconsistency_flag;
                    for k in 0..count {
                        matrix_e[e_idx + k * num_vars] = hessian[(index + k, index)];
                    }
                    // CALL dscal_sl (num_remaining_vars-inconsistency_flag, diagonal_element, matrix_e(e_idx), num_vars)
                    for k in 0..count {
                        matrix_e[e_idx + k * num_vars] *= diagonal_element;
                    }
                }

                matrix_e[e_idx] = diagonal_element;

                // vector_f(i) = (objective_grad(i) - vector_dot_product (i-1, matrix_e(f_aux_idx), 1, vector_f, 1))/diagonal_element
                let dot_product = vector_dot_product(index, &matrix_e[f_aux_idx..], 1, vector_f, 1);
                vector_f[index] = (gradient[index] - dot_product) / diagonal_element;

                e_idx += num_vars_plus_1;
                f_aux_idx += num_vars;
            }

            if inconsistency_flag == 1 {
                // When inconsistency_flag is 1, num_vars_filtered = num_vars - 1
                // So we use the last diagonal element
                matrix_e[e_idx] = hessian[(num_vars - 1, num_vars - 1)];
                for k in 0..num_vars_filtered {
                    matrix_e[f_aux_idx + k] = 0.0;
                }
                vector_f[num_vars - 1] = 0.0;
            }

            vector_mul(&mut vector_f[..num_vars], -1.0);

            if num_equals > 0 {
                // RECOVER MATRIX C FROM UPPER PART OF A
                for i in 0..num_equals {
                    for j in 0..num_vars {
                        matrix_c[i + j * num_equals] = jac[(i, j)];
                    }
                }

                // RECOVER VECTOR D FROM UPPER PART OF B
                for i in 0..num_equals {
                    vector_d[i] = -workspace.constraint_values[i];
                }
            }

            if num_inequalities > 0 {
                // RECOVER MATRIX G FROM LOWER PART OF A
                for i in 0..num_inequalities {
                    for j in 0..num_vars {
                        matrix_g[i + j * total_inequalities] = jac[(num_equals + i, j)];
                    }
                }
            }

            // AUGMENT MATRIX G BY +I AND -I
            let matrix_g_pos_i_idx = num_inequalities;
            for i in 0..num_vars {
                for k in 0..num_vars {
                    matrix_g[matrix_g_pos_i_idx + i + k * total_inequalities] = 0.0;
                }
            }
            for i in 0..num_vars {
                matrix_g[matrix_g_pos_i_idx + i + i * total_inequalities] = 1.0;
            }

            let matrix_g_neg_i_idx = matrix_g_pos_i_idx + num_vars;
            for i in 0..num_vars {
                for k in 0..num_vars {
                    matrix_g[matrix_g_neg_i_idx + i + k * total_inequalities] = 0.0;
                }
            }
            for i in 0..num_vars {
                matrix_g[matrix_g_neg_i_idx + i + i * total_inequalities] = -1.0;
            }

            if num_inequalities > 0 {
                for i in 0..num_inequalities {
                    vector_h[i] = -workspace.constraint_values[num_equals + i];
                }
            }

            // AUGMENT VECTOR H BY XL AND XU
            let vector_h_xl_idx = num_inequalities;
            for i in 0..num_vars {
                vector_h[vector_h_xl_idx + i] = slsqpb_u[i];
            }
            let vector_h_xu_idx = vector_h_xl_idx + num_vars;
            for i in 0..num_vars {
                vector_h[vector_h_xu_idx + i] = -slsqpb_v[i];
            }
        }

        lsei(num_vars, workspace)?;

        let Slsqp {
            lsei_workspace:
                LseiWorkspace {
                    internal_workspace: lsei_internal_work,
                    ..
                },
            multipliers,
            ..
        } = workspace;

        // restore Lagrange multipliers
        for i in 0..num_constraints {
            multipliers[i] = lsei_internal_work[i];
        }
        for i in 0..num_vars_filtered {
            multipliers[num_constraints + i] = lsei_internal_work[num_constraints + i];
        }
        for i in 0..num_vars_filtered {
            multipliers[num_constraints + num_vars_filtered + i] =
                lsei_internal_work[num_constraints + num_vars + i];
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::Constraint;

    use super::*;

    #[test]
    fn test_lsq_basic() {
        let num_vars = 2;
        let objective_grad = [0.0, 0.0];
        // x1 + x2 = 2 => a = [1, 1], b = [-2] (since lsq solves Ax = -b)
        let constraint_jacobian_a = [1.0, 1.0];
        let constraint_values_b = [-2.0];
        let xl = [-10.0, -10.0];
        let xu = [10.0, 10.0];
        let x_opt = [0.0, 0.0];
        let func = Box::new(|_: &[f64]| 0.0);
        let constraints = vec![Constraint::Eq(Box::new(|_: &[f64]| 0.0))];
        let mut workspace = Slsqp::new(vec![0.0; 2], &[], func, constraints, 100, 1.0e-6);

        // Copy test hessian and bounds into workspace
        workspace.hessian[(0, 0)] = 1.0;
        workspace.hessian[(1, 0)] = 0.0;
        workspace.hessian[(1, 1)] = 1.0;
        workspace.slsqpb_u[..xl.len()].copy_from_slice(&xl);
        workspace.slsqpb_v[..xu.len()].copy_from_slice(&xu);
        workspace.search_direction[..x_opt.len()].copy_from_slice(&x_opt);
        workspace.gradient[..objective_grad.len()].copy_from_slice(&objective_grad);
        workspace.jac.data[..constraint_jacobian_a.len()].copy_from_slice(&constraint_jacobian_a);
        workspace.constraint_values[..constraint_values_b.len()]
            .copy_from_slice(&constraint_values_b);

        assert!(workspace.lsq(num_vars).is_ok());
        assert!((workspace.search_direction[0] - 1.0).abs() < 1e-10);
        assert!((workspace.search_direction[1] - 1.0).abs() < 1e-10);
    }
}
