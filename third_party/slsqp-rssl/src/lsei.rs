use crate::blas::{vector_add_scaled, vector_copy, vector_dot_product, vector_norm};
use crate::hfti::hfti;
use crate::householder::{h12_apply, h12_construct};
use crate::nnls::nnls;
use crate::solver::LseiWorkspace;
use crate::{Slsqp, SlsqpError, SlsqpObserver};

/// LDP: LEAST DISTANCE PROGRAMMING
///
/// 1:1 replica of the `ldp` subroutine from `slsqp_optmz.f`.
pub fn least_distance_program(
    matrix_g: &[f64],
    leading_dim_g: usize,
    num_constraints: usize,
    num_vars: usize,
    vector_h: &[f64],
    solution_x: &mut [f64],
    residual_norm: &mut f64,
    internal_workspace: &mut [f64],
    int_workspace: &mut [i32],
) -> Result<(), SlsqpError> {
    if num_vars == 0 {
        return Err(SlsqpError::MoreEqualityConstraints);
    }

    for i in 0..num_vars {
        solution_x[i] = 0.0;
    }
    *residual_norm = 0.0;
    if num_constraints == 0 {
        return Ok(());
    }

    let num_vars_plus_one = num_vars + 1;
    let dual_problem = LdpDualProblem::new(internal_workspace, num_vars, num_constraints);

    // STATE DUAL PROBLEM
    // Matrix A of the dual problem is (num_vars+1) x num_constraints
    // It is stored in column-major order in dual_problem.dual_matrix
    let mut matrix_idx = 0;
    for j in 0..num_constraints {
        for i in 0..num_vars {
            dual_problem.dual_matrix[matrix_idx] = matrix_g[j + i * leading_dim_g];
            matrix_idx += 1;
        }
        dual_problem.dual_matrix[matrix_idx] = vector_h[j];
        matrix_idx += 1;
    }

    // Vector b of the dual problem is [0, ..., 0, 1] of length num_vars+1
    for i in 0..num_vars {
        dual_problem.dual_vector[i] = 0.0;
    }
    dual_problem.dual_vector[num_vars] = 1.0;

    // SOLVE DUAL PROBLEM
    let mut dual_residual_norm = 0.0;

    let mut mat_a =
        crate::MatView::new(num_vars_plus_one, num_constraints, dual_problem.dual_matrix);
    nnls(
        &mut mat_a,
        dual_problem.dual_vector,
        dual_problem.dual_solution,
        &mut dual_residual_norm,
        dual_problem.nnls_workspace_w,
        dual_problem.nnls_workspace_z,
        int_workspace,
    )?;
    if dual_residual_norm <= 0.0 {
        return Err(SlsqpError::IncompatibleConstraints);
    }

    // COMPUTE SOLUTION OF PRIMAL PROBLEM
    // The primal solution x is given by: x = (1 / (1 - h^T y)) * G^T y
    // where y is the dual solution.
    let mut primal_scale =
        1.0 - vector_dot_product(num_constraints, vector_h, 1, dual_problem.dual_solution, 1);
    if (1.0 + primal_scale - 1.0) <= 0.0 {
        return Err(SlsqpError::IncompatibleConstraints);
    }

    primal_scale = 1.0 / primal_scale;
    for j in 0..num_vars {
        solution_x[j] = primal_scale
            * vector_dot_product(
                num_constraints,
                &matrix_g[(j) * leading_dim_g..],
                1,
                dual_problem.dual_solution,
                1,
            );
    }
    *residual_norm = vector_norm(num_vars, solution_x);

    // Store scaled dual solution (Lagrange multipliers) in the beginning of the workspace
    for i in 0..num_constraints {
        dual_problem.dual_matrix[i] = primal_scale * dual_problem.dual_solution[i];
    }
    Ok(())
}

struct LdpDualProblem<'a> {
    /// Matrix A of the dual problem, size (num_vars+1) x num_constraints
    dual_matrix: &'a mut [f64],
    /// Vector b of the dual problem, size num_vars+1
    dual_vector: &'a mut [f64],
    /// Workspace for NNLS (zz), size num_vars+1
    nnls_workspace_z: &'a mut [f64],
    /// Solution of the dual problem (x), size num_constraints
    dual_solution: &'a mut [f64],
    /// Additional workspace for NNLS (w)
    nnls_workspace_w: &'a mut [f64],
}

impl<'a> LdpDualProblem<'a> {
    fn new(workspace: &'a mut [f64], num_vars: usize, num_constraints: usize) -> Self {
        let num_vars_plus_one = num_vars + 1;
        let size_dual_matrix = num_vars_plus_one * num_constraints;
        let size_dual_vector = num_vars_plus_one;
        let size_nnls_workspace_z = num_vars_plus_one;
        let size_dual_solution = num_constraints;

        let (dual_matrix, rest) = workspace.split_at_mut(size_dual_matrix);
        let (dual_vector, rest) = rest.split_at_mut(size_dual_vector);
        let (nnls_workspace_z, rest) = rest.split_at_mut(size_nnls_workspace_z);
        let (dual_solution, nnls_workspace_w) = rest.split_at_mut(size_dual_solution);

        Self {
            dual_matrix,
            dual_vector,
            nnls_workspace_z,
            dual_solution,
            nnls_workspace_w,
        }
    }
}

struct LseiInternalWorkArea<'a> {
    /// Area for multipliers in LSEI (0..num_equals)
    multipliers: &'a mut [f64],
    /// Workspace for LSI (num_equals .. offset_h_scalars)
    lsi_workspace: &'a mut [f64],
    /// Householder scalars (offset_h_scalars .. offset_transformed_e)
    householder_scalars: &'a mut [f64],
    /// Transformed matrix E (offset_transformed_e .. offset_transformed_f)
    e_transformed: &'a mut [f64],
    /// Transformed vector F (offset_transformed_f .. offset_transformed_g)
    f_transformed: &'a mut [f64],
    /// Transformed matrix G (offset_transformed_g .. end)
    g_transformed: &'a mut [f64],
}

impl<'a> LseiInternalWorkArea<'a> {
    fn new(
        workspace: &'a mut [f64],
        num_vars: usize,
        num_constraints: usize,
        num_equals: usize,
    ) -> Self {
        let num_free_vars = num_vars - num_equals;
        let num_inequals_total = (num_constraints - num_equals) + 2 * num_vars;
        let num_rows_e = num_vars;

        let num_vars_plus_one_ldp = num_free_vars + 1;
        let num_constraints_ldp = num_inequals_total;
        let size_lsi_workspace = num_vars_plus_one_ldp * num_constraints_ldp
            + 2 * num_vars_plus_one_ldp
            + 2 * num_constraints_ldp;

        let (multipliers, rest) = workspace.split_at_mut(num_equals);
        let (lsi_workspace, rest) = rest.split_at_mut(size_lsi_workspace);
        let (householder_scalars, rest) = rest.split_at_mut(num_equals);
        let (e_transformed, rest) = rest.split_at_mut(num_rows_e * num_free_vars);
        let (f_transformed, g_transformed) = rest.split_at_mut(num_rows_e);

        Self {
            multipliers,
            lsi_workspace,
            householder_scalars,
            e_transformed,
            f_transformed,
            g_transformed,
        }
    }
}

/// LSI: LEAST SQUARES WITH INEQUALITY CONSTRAINTS
///
/// 1:1 replica of the `lsi` subroutine from `slsqp_optmz.f`.
pub fn lsi(
    matrix_e: &mut [f64],
    vector_f: &mut [f64],
    matrix_g: &mut [f64],
    vector_h: &mut [f64],
    leading_dim_e: usize,
    num_rows_e: usize,
    leading_dim_g: usize,
    num_rows_g: usize,
    num_vars: usize,
    solution_x: &mut [f64],
    residual_norm: &mut f64,
    internal_workspace: &mut [f64],
    int_workspace: &mut [i32],
) -> Result<(), SlsqpError> {
    const EPS_MACHINE: f64 = 2.22e-16;

    // QR-FACTORS OF E AND APPLICATION TO F
    for i in 0..num_vars {
        let mut householder_scalar = 0.0;
        let next_i = i + 1;

        let (householder_vector, matrix_e_remainder) = if num_vars >= next_i + 1 {
            let (before, after) = (&mut *matrix_e).split_at_mut(next_i * leading_dim_e);
            (&mut before[(i) * leading_dim_e..], after)
        } else {
            (&mut matrix_e[(i) * leading_dim_e..], &mut [][..])
        };

        h12_construct(
            i + 1,
            i + 1 + 1,
            num_rows_e,
            householder_vector,
            1,
            &mut householder_scalar,
            matrix_e_remainder,
            1,
            leading_dim_e,
            (num_vars - i - 1) as i32,
        );
        h12_apply(
            i + 1,
            i + 1 + 1,
            num_rows_e,
            householder_vector,
            1,
            householder_scalar,
            vector_f,
            1,
            1,
            1,
        );
    }

    // TRANSFORM G AND H TO GET LEAST DISTANCE PROBLEM
    for i in 0..num_rows_g {
        for j in 0..num_vars {
            if matrix_e[(j) * leading_dim_e + (j)].abs() < EPS_MACHINE {
                return Err(SlsqpError::SingularMatrixE);
            }
            matrix_g[(j) * leading_dim_g + (i)] = (matrix_g[(j) * leading_dim_g + (i)]
                - vector_dot_product(
                    j,
                    &matrix_g[i..],
                    leading_dim_g,
                    &matrix_e[(j) * leading_dim_e..],
                    1,
                ))
                / matrix_e[(j) * leading_dim_e + (j)];
        }
        vector_h[i] -= vector_dot_product(num_vars, &matrix_g[i..], leading_dim_g, vector_f, 1);
    }

    least_distance_program(
        matrix_g,
        leading_dim_g,
        num_rows_g,
        num_vars,
        vector_h,
        solution_x,
        residual_norm,
        internal_workspace,
        int_workspace,
    )?;

    vector_add_scaled(num_vars, 1.0, vector_f, solution_x);
    for i in (0..num_vars).rev() {
        let temp_dot_product = if num_vars > i + 1 {
            vector_dot_product(
                num_vars - i - 1,
                &matrix_e[(i + 1) * leading_dim_e + i..],
                leading_dim_e,
                &solution_x[i + 1..],
                1,
            )
        } else {
            0.0
        };
        solution_x[i] = (solution_x[i] - temp_dot_product) / matrix_e[(i) * leading_dim_e + (i)];
    }

    let residual_norm_f = if num_rows_e > num_vars {
        vector_norm(num_rows_e - num_vars, &vector_f[num_vars..])
    } else {
        0.0
    };
    *residual_norm = (*residual_norm * *residual_norm + residual_norm_f * residual_norm_f).sqrt();
    Ok(())
}

/// LSEI: LEAST SQUARES WITH EQUALITY AND INEQUALITY CONSTRAINTS
///
/// 1:1 replica of the `lsei` subroutine from `slsqp_optmz.f`.
pub fn lsei<O: SlsqpObserver>(
    num_vars: usize,
    workspace: &mut Slsqp<'_, O>,
) -> Result<(), SlsqpError> {
    let mut residual_norm = 0.0;
    let residual_norm = &mut residual_norm;
    let num_constraints = workspace.num_constraints;
    let num_equals = workspace.num_equals;
    let num_rows_e = num_vars;
    let num_inequals_total = (num_constraints - num_equals) + 2 * num_vars;

    let ldc = num_equals.max(1);
    let lde = num_vars;
    let ldg = num_inequals_total.max(1);

    let Slsqp {
        lsei_workspace:
            LseiWorkspace {
                matrix_c,
                vector_d,
                matrix_e,
                vector_f,
                matrix_g,
                vector_h,
                internal_workspace: iw_real,
                int_workspace,
            },
        search_direction,
        ..
    } = workspace;

    let eps_machine = 2.22e-16;
    if num_equals > num_vars {
        return Err(SlsqpError::MoreEqualityConstraints);
    }
    let num_free_vars = num_vars - num_equals;

    let lsei_workspace = LseiInternalWorkArea::new(iw_real, num_vars, num_constraints, num_equals);

    for i in 0..num_equals {
        let mut householder_scalar = 0.0;
        let next_i = (i + 1 + 1).min(ldc);

        unsafe {
            let c_ptr = matrix_c.as_mut_ptr();
            let u_ptr =
                std::slice::from_raw_parts_mut(c_ptr.add(i), matrix_c.len().saturating_sub(i));
            let c_rem = if ldc >= next_i {
                std::slice::from_raw_parts_mut(
                    c_ptr.add(next_i - 1),
                    matrix_c.len().saturating_sub(next_i - 1),
                )
            } else {
                &mut []
            };

            h12_construct(
                i + 1,
                i + 1 + 1,
                num_vars,
                u_ptr,
                ldc,
                &mut householder_scalar,
                c_rem,
                ldc,
                1,
                (num_equals - i - 1) as i32,
            );
            lsei_workspace.householder_scalars[i] = householder_scalar;

            h12_apply(
                i + 1,
                i + 1 + 1,
                num_vars,
                u_ptr,
                ldc,
                householder_scalar,
                matrix_e,
                lde,
                1,
                num_rows_e as i32,
            );
            h12_apply(
                i + 1,
                i + 1 + 1,
                num_vars,
                u_ptr,
                ldc,
                householder_scalar,
                matrix_g,
                ldg,
                1,
                num_inequals_total as i32,
            );
        }
    }

    for i in 0..num_equals {
        if matrix_c[(i) * ldc + (i)].abs() < eps_machine {
            return Err(SlsqpError::SingularMatrixC);
        }
        search_direction[i] = (vector_d[i]
            - vector_dot_product(i, &matrix_c[i..], ldc, search_direction, 1))
            / matrix_c[(i) * ldc + (i)];
    }

    if num_inequals_total > num_equals {
        for i in num_equals..num_inequals_total {
            lsei_workspace.lsi_workspace[i - num_equals] = 0.0;
        }
    }

    if num_equals < num_vars {
        for i in 0..num_rows_e {
            lsei_workspace.f_transformed[i] = vector_f[i]
                - vector_dot_product(num_equals, &matrix_e[i..], lde, search_direction, 1);
        }

        for i in 0..num_rows_e {
            vector_copy(
                num_free_vars,
                &matrix_e[num_equals * lde + i..],
                lde,
                &mut lsei_workspace.e_transformed[i..],
                num_rows_e,
            );
        }

        for i in 0..num_inequals_total {
            vector_copy(
                num_free_vars,
                &matrix_g[num_equals * ldg + i..],
                ldg,
                &mut lsei_workspace.g_transformed[i..],
                num_inequals_total,
            );
        }

        if num_inequals_total == 0 {
            let max_rank = lde.max(num_vars);
            let tolerance = eps_machine.sqrt();
            let mut rank = 0;

            // Use the start of lsei_workspace.lsi_workspace for h_hfti and g_hfti
            // Since num_inequals_total == 0, lsi_workspace has size 2 * (num_free_vars + 1)
            let (h_hfti, rest) = lsei_workspace.lsi_workspace.split_at_mut(num_free_vars);
            let (g_hfti, _) = rest.split_at_mut(num_free_vars);

            hfti(
                lsei_workspace.e_transformed,
                num_rows_e,
                num_rows_e,
                num_free_vars,
                lsei_workspace.f_transformed,
                max_rank,
                1,
                tolerance,
                &mut rank,
                std::slice::from_mut(residual_norm),
                h_hfti,
                g_hfti,
                int_workspace,
            );

            vector_copy(
                num_free_vars,
                lsei_workspace.f_transformed,
                1,
                &mut search_direction[num_equals..],
                1,
            );
            if rank != num_free_vars {
                return Err(SlsqpError::RankDeficientHFTI);
            }
        } else {
            for i in 0..num_inequals_total {
                vector_h[i] -=
                    vector_dot_product(num_equals, &matrix_g[i..], ldg, search_direction, 1);
            }

            let r = lsi(
                lsei_workspace.e_transformed,
                lsei_workspace.f_transformed,
                lsei_workspace.g_transformed,
                vector_h,
                num_vars,
                num_rows_e,
                num_inequals_total,
                num_inequals_total,
                num_free_vars,
                &mut search_direction[num_equals..],
                residual_norm,
                lsei_workspace.lsi_workspace,
                int_workspace,
            );

            if num_equals > 0 {
                let norm_x_equality = vector_norm(num_equals, search_direction);
                *residual_norm =
                    (*residual_norm * *residual_norm + norm_x_equality * norm_x_equality).sqrt();
            }
            r?;
        }
    } else {
        if num_equals > 0 {
            *residual_norm = vector_norm(num_equals, search_direction);
        } else {
            *residual_norm = 0.0;
        }
    }

    for i in 0..num_rows_e {
        vector_f[i] =
            vector_dot_product(num_vars, &matrix_e[i..], lde, search_direction, 1) - vector_f[i];
    }
    for i in 0..num_equals {
        vector_d[i] = vector_dot_product(num_rows_e, &matrix_e[(i) * lde..], 1, vector_f, 1)
            - vector_dot_product(
                num_inequals_total,
                &matrix_g[(i) * ldg..],
                1,
                &lsei_workspace.lsi_workspace[..num_inequals_total],
                1,
            );
    }

    for i in (0..num_equals).rev() {
        let householder_scalar = lsei_workspace.householder_scalars[i];
        h12_apply(
            i + 1,
            i + 1 + 1,
            num_vars,
            &matrix_c[i..],
            ldc,
            householder_scalar,
            search_direction,
            1,
            1,
            1,
        );
    }

    for i in (0..num_equals).rev() {
        let temp_dot_product = if num_equals > i + 1 {
            vector_dot_product(
                num_equals - (i + 1),
                &matrix_c[i * ldc + i + 1..],
                1,
                &lsei_workspace.multipliers[i + 1..],
                1,
            )
        } else {
            0.0
        };
        lsei_workspace.multipliers[i] = (vector_d[i] - temp_dot_product) / matrix_c[i * ldc + i];
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::Constraint;

    use super::*;

    #[test]
    fn test_ldp_basic() {
        let g = [1.0, 1.0];
        let h = [2.0];
        let mut x = [0.0; 2];
        let mut xnorm = 0.0;
        let mut w = [0.0; 100];
        let mut index = [0; 2];

        least_distance_program(&g, 1, 1, 2, &h, &mut x, &mut xnorm, &mut w, &mut index).unwrap();

        assert!((x[0] - 1.0).abs() < 1e-10);
        assert!((x[1] - 1.0).abs() < 1e-10);
        assert!((xnorm - 2.0f64.sqrt()).abs() < 1e-10);
    }

    // #[test]
    // fn test_lsi_basic() {
    //     let mut e = [1.0, 0.0, 0.0, 1.0];
    //     let mut f = [2.0, 2.0];
    //     let mut g = [-1.0, -1.0];
    //     let mut h = [-2.0];
    //     let mut x = [0.0; 2];
    //     let mut xnorm = 0.0;
    //     let mut w = [0.0; 100];
    //     let mut jw = [0; 2];

    //     lsi(
    //         &mut e, &mut f, &mut g, &mut h, 2, 2, 1, 1, 2, &mut x, &mut xnorm, &mut w, &mut jw,
    //     )
    //     .unwrap();

    //     assert!((x[0] - 1.0).abs() < 1e-10);
    //     assert!((x[1] - 1.0).abs() < 1e-10);
    // }

    #[test]
    fn test_lsei_basic() {
        let n = 2;
        let m = 2; // 1 equality + 1 inequality
        let func = Box::new(|_: &[f64]| 0.0);
        let constraints = vec![
            Constraint::Ineq(Box::new(|_: &[f64]| 0.0)),
            Constraint::Eq(Box::new(|_: &[f64]| 0.0)),
        ];
        let mc = 1;
        let mut workspace = Slsqp::new(vec![0.0; n], &[], func, constraints, 100, 1.0e-6);

        let c = [1.0, -1.0];
        let d = [0.0];
        let e = [1.0, 0.0, 0.0, 1.0];
        let f = [2.0, 2.0];
        let g = [-1.0, -1.0];
        let h = [-2.0];

        {
            let Slsqp {
                lsei_workspace:
                    LseiWorkspace {
                        matrix_c,
                        vector_d,
                        matrix_e,
                        vector_f,
                        matrix_g,
                        vector_h,
                        ..
                    },
                ..
            } = &mut workspace;

            matrix_c[..c.len()].copy_from_slice(&c);
            vector_d[..d.len()].copy_from_slice(&d);
            matrix_e[..e.len()].copy_from_slice(&e);
            vector_f[..f.len()].copy_from_slice(&f);

            // The first row of g is our inequality
            let mg_total = (m - mc) + 2 * n; // 1 + 4 = 5
            for i in 0..n {
                matrix_g[i * mg_total] = g[i];
            }
            vector_h[0] = h[0];

            // The rest of g/h are for bounds, but lsei will handle them if they are set.
            // In this test, we don't have explicit bounds in the workspace setup yet
            // that lsei expects in the same way lsq sets them up.
            // Wait, lsei just takes g and h.
            // In lsq.rs, it augments G and H with bounds.
            // If we want lsei to work like the old test, we should only have mg = 1.
        }

        lsei(n, &mut workspace).unwrap();

        let x = &workspace.search_direction;
        assert!((x[0] - 1.0).abs() < 1e-10);
        assert!((x[1] - 1.0).abs() < 1e-10);
    }
}
