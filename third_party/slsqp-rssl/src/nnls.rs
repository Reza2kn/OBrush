use crate::blas::{
    givens_rotation_apply, givens_rotation_construct, vector_add_scaled, vector_copy,
    vector_dot_product, vector_norm,
};
use crate::householder::{h12_apply, h12_construct};
use crate::{MatView, SlsqpError};

/// NNLS: NONNEGATIVE LEAST SQUARES
///
/// GIVEN AN M BY N MATRIX, A, AND AN M-VECTOR, B, COMPUTE AN
/// N-VECTOR, X, WHICH SOLVES THE LEAST SQUARES PROBLEM
///
/// ```text
///              A*X = B  SUBJECT TO  X >= 0
/// ```
///
/// # Arguments
/// * `a` - M by N matrix
/// * `b` - M-vector.
/// * `x` - N-vector (output).
/// * `rnorm` - Euclidean norm of the residual vector (output).
/// * `dual_variables` - N-vector for working space/dual solution (output).
/// * `subproblem_sol` - M-vector for working space.
/// * `column_indices` - Integer working array of length N.
pub fn nnls(
    a: &mut MatView,
    b: &mut [f64],
    x: &mut [f64],
    rnorm: &mut f64,
    dual_variables: &mut [f64],
    subproblem_sol: &mut [f64],
    column_indices: &mut [i32],
) -> Result<(), SlsqpError> {
    let m = a.m;
    let n = a.n;
    let mda = a.m;

    if m == 0 || n == 0 {
        return Err(SlsqpError::MoreEqualityConstraints);
    }
    let mut iter_count = 0;
    let max_iter = 3 * n;

    // STEP ONE (INITIALIZE)
    for i in 0..n {
        column_indices[i] = (i + 1) as i32;
    }
    let mut z_begin = 1;
    let z_end = n;
    let mut num_p = 0;
    let mut n_p_plus_1 = 1;

    for i in 0..n {
        x[i] = 0.0;
    }

    // .....ENTRY LOOP A
    'loop_a: loop {
        if z_begin > z_end || num_p >= m {
            break 'loop_a;
        }
        for z_idx in z_begin..=z_end {
            let j = column_indices[z_idx - 1] as usize;
            let a_idx = (j - 1) * mda + (n_p_plus_1 - 1);
            let b_idx = n_p_plus_1 - 1;
            dual_variables[j - 1] =
                vector_dot_product(m - num_p, &a.data[a_idx..], 1, &b[b_idx..], 1);
        }

        'step_130: loop {
            // STEP THREE (TEST DUAL VARIABLES)
            let mut wmax = 0.0;
            let mut max_z_idx = 0;
            for z_idx in z_begin..=z_end {
                let j = column_indices[z_idx - 1] as usize;
                if dual_variables[j - 1] > wmax {
                    wmax = dual_variables[j - 1];
                    max_z_idx = z_idx;
                }
            }

            // .....EXIT LOOP A
            if wmax <= 0.0 {
                break 'loop_a;
            }
            let z_idx = max_z_idx;
            let j = column_indices[z_idx - 1] as usize;

            // STEP FOUR (TEST INDEX J FOR LINEAR DEPENDENCY)
            let asave = a[(n_p_plus_1 - 1, j - 1)];
            let mut up = 0.0;
            // Note: ncv=0 so c is not used
            h12_construct(
                n_p_plus_1,
                n_p_plus_1 + 1,
                m,
                &mut a.col_mut(j - 1),
                1,
                &mut up,
                &mut [],
                1,
                1,
                0,
            );

            let unorm = vector_norm(num_p, a.col(j - 1));
            let factor = 1.0e-2;
            let t = factor * a[(n_p_plus_1 - 1, j - 1)].abs();

            let mut success = false;
            if (unorm + t) - unorm > 0.0 {
                vector_copy(m, b, 1, subproblem_sol, 1);
                h12_apply(
                    n_p_plus_1,
                    n_p_plus_1 + 1,
                    m,
                    a.col(j - 1),
                    1,
                    up,
                    subproblem_sol,
                    1,
                    1,
                    1,
                );
                if subproblem_sol[n_p_plus_1 - 1] / a[(n_p_plus_1 - 1, j - 1)] > 0.0 {
                    success = true;
                }
            }

            if success {
                // STEP FIVE (ADD COLUMN)
                vector_copy(m, subproblem_sol, 1, b, 1);
                column_indices[z_idx - 1] = column_indices[z_begin - 1];
                column_indices[z_begin - 1] = j as i32;
                z_begin += 1;
                num_p = n_p_plus_1;
                n_p_plus_1 += 1;
                if z_begin <= z_end {
                    for z_idx_inner in z_begin..=z_end {
                        let jj = column_indices[z_idx_inner - 1] as usize;
                        // Safe split to avoid multiple mutable borrows of 'a'
                        if j < jj {
                            let (left, right) = a.data.split_at_mut((jj - 1) * mda);
                            h12_apply(
                                num_p,
                                n_p_plus_1,
                                m,
                                &left[(j - 1) * mda..],
                                1,
                                up,
                                right,
                                1,
                                mda,
                                1,
                            );
                        } else {
                            let (left, right) = a.data.split_at_mut((j - 1) * mda);
                            h12_apply(
                                num_p,
                                n_p_plus_1,
                                m,
                                right,
                                1,
                                up,
                                &mut left[(jj - 1) * mda..],
                                1,
                                mda,
                                1,
                            );
                        }
                    }
                }
                let k = if n_p_plus_1 < mda { n_p_plus_1 } else { mda };
                dual_variables[j - 1] = 0.0;
                let zero_val = 0.0;
                vector_copy(m - num_p, &[zero_val], 0, &mut a.col_mut(j - 1)[k - 1..], 1);

                // STEP SIX (SOLVE LEAST SQUARES SUB-PROBLEM)
                'loop_b: loop {
                    for p_idx in (0..num_p).rev() {
                        if p_idx + 1 < num_p {
                            let col_idx_prev = column_indices[p_idx + 1] as usize;
                            vector_add_scaled(
                                p_idx + 1,
                                -subproblem_sol[p_idx + 1],
                                a.col(col_idx_prev - 1),
                                subproblem_sol,
                            );
                        }
                        let col_idx = column_indices[p_idx] as usize;
                        subproblem_sol[p_idx] = subproblem_sol[p_idx] / a[(p_idx, col_idx - 1)];
                    }
                    iter_count += 1;
                    if iter_count > max_iter {
                        return Err(SlsqpError::IterationLimitExceededLSQ);
                    }

                    // STEP SEVEN TO TEN (STEP LENGTH ALGORITHM)
                    let mut alpha = 1.0;
                    let mut p_idx_alpha = 0;
                    for p_idx in 0..num_p {
                        if subproblem_sol[p_idx] <= 0.0 {
                            let l = column_indices[p_idx] as usize;
                            let t = -x[l - 1] / (subproblem_sol[p_idx] - x[l - 1]);
                            if alpha > t {
                                alpha = t;
                                p_idx_alpha = p_idx + 1;
                            }
                        }
                    }

                    for p_idx in 0..num_p {
                        let l = column_indices[p_idx] as usize;
                        x[l - 1] = (1.0 - alpha) * x[l - 1] + alpha * subproblem_sol[p_idx];
                    }

                    if p_idx_alpha == 0 {
                        continue 'loop_a;
                    }

                    // STEP ELEVEN (DELETE COLUMN)
                    let mut p_idx_to_delete = p_idx_alpha;
                    'step_250: loop {
                        let col_idx_to_delete = column_indices[p_idx_to_delete - 1] as usize;
                        x[col_idx_to_delete - 1] = 0.0;
                        p_idx_to_delete += 1;
                        for j_rot in p_idx_to_delete..=num_p {
                            let col_idx = column_indices[j_rot - 1] as usize;
                            column_indices[j_rot - 2] = col_idx as i32;

                            let mut c_rot = 0.0;
                            let mut s_rot = 0.0;
                            let mut a_prev = a[(j_rot - 2, col_idx - 1)];
                            let mut a_curr = a[(j_rot - 1, col_idx - 1)];
                            givens_rotation_construct(
                                &mut a_prev,
                                &mut a_curr,
                                &mut c_rot,
                                &mut s_rot,
                            );

                            // Apply rotation to all columns of A
                            for i_rot in 0..n {
                                let idx1 = i_rot * mda + (j_rot - 2);
                                let idx2 = i_rot * mda + (j_rot - 1);
                                let dtemp = c_rot * a.data[idx1] + s_rot * a.data[idx2];
                                a.data[idx2] = c_rot * a.data[idx2] - s_rot * a.data[idx1];
                                a.data[idx1] = dtemp;
                            }
                            // Force zero for the sub-diagonal element for numerical stability
                            a[(j_rot - 1, col_idx - 1)] = 0.0;

                            // Apply rotation to the vector b
                            let (b_left, b_right) = b.split_at_mut(j_rot - 1);
                            givens_rotation_apply(
                                1,
                                &mut b_left[j_rot - 2..],
                                b_right,
                                c_rot,
                                s_rot,
                            );
                        }
                        n_p_plus_1 = num_p;
                        num_p -= 1;
                        z_begin -= 1;
                        column_indices[z_begin - 1] = col_idx_to_delete as i32;

                        if num_p <= 0 {
                            return Err(SlsqpError::IterationLimitExceededLSQ);
                        }

                        let mut found_next = false;
                        for p_idx_check in 0..num_p {
                            let col_idx_check = column_indices[p_idx_check] as usize;
                            if x[col_idx_check - 1] <= 0.0 {
                                p_idx_to_delete = p_idx_check + 1;
                                found_next = true;
                                break;
                            }
                        }
                        if found_next {
                            continue 'step_250;
                        }

                        vector_copy(m, b, 1, subproblem_sol, 1);
                        continue 'loop_b;
                    }
                }
            } else {
                a[(n_p_plus_1 - 1, j - 1)] = asave;
                dual_variables[j - 1] = 0.0;
                continue 'step_130;
            }
        }
    }

    // STEP TWELVE (SOLUTION)
    let k = if n_p_plus_1 < m { n_p_plus_1 } else { m };
    *rnorm = vector_norm(m - num_p, &b[k - 1..]);
    if n_p_plus_1 > m {
        for i in 0..n {
            dual_variables[i] = 0.0;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nnls_basic() {
        // A = [1 0; 0 1], b = [1, 1]
        // Solution should be [1, 1]
        let mut a_data = vec![1.0, 0.0, 0.0, 1.0];
        let mut a = MatView::new(2, 2, &mut a_data);
        let mut b = [1.0, 1.0];
        let mut x = [0.0; 2];
        let mut rnorm = 0.0;
        let mut w = [0.0; 2];
        let mut z = [0.0; 2];
        let mut index = [0; 2];

        nnls(
            &mut a, &mut b, &mut x, &mut rnorm, &mut w, &mut z, &mut index,
        )
        .unwrap();

        assert!((x[0] - 1.0).abs() < 1e-10);
        assert!((x[1] - 1.0).abs() < 1e-10);
        assert!(rnorm < 1e-10);
    }

    #[test]
    fn test_nnls_constraint() {
        // A = [1 1; 1 2], b = [1, -1]
        // Unconstrained solution: [3, -2]
        // Constrained solution (x >= 0): [0, 0] or [1, 0] depending on residual
        // If x=[0,0], residual = [1, -1], norm = sqrt(2)
        // If x=[1,0], residual = [0, -2], norm = 2
        // Wait, A*x = [1, 1] if x=[1, 0]. Residual = [1-1, -1-1] = [0, -2].
        // If x=[0, 0], residual = [1, -1], norm = sqrt(2) = 1.414
        // So [0, 0] is better.
        let mut a_data = vec![1.0, 1.0, 1.0, 2.0];
        let mut a = MatView::new(2, 2, &mut a_data);
        let mut b = [1.0, -1.0];
        let mut x = [0.0; 2];
        let mut rnorm = 0.0;
        let mut w = [0.0; 2];
        let mut z = [0.0; 2];
        let mut index = [0; 2];
        nnls(
            &mut a, &mut b, &mut x, &mut rnorm, &mut w, &mut z, &mut index,
        )
        .unwrap();

        assert!(x[0] >= 0.0);
        assert!(x[1] >= 0.0);
        // Residual norm should be sqrt(2)
        assert!((rnorm - 2.0f64.sqrt()).abs() < 1e-10);
    }

    #[test]
    fn test_nnls_deletion() {
        // A = [1 1; 1 2; 1 3], b = [1, 0, -1]
        // Unconstrained solution x is approximately [2, -1]
        // NNLS should result in x[1] = 0
        let mut a_data = vec![1.0, 1.0, 1.0, 1.0, 2.0, 3.0];
        let mut a = MatView::new(3, 2, &mut a_data);
        let mut b = [1.0, 0.0, -1.0];
        let mut x = [0.0; 2];
        let mut rnorm = 0.0;
        let mut w = [0.0; 2];
        let mut z = [0.0; 3];
        let mut index = [0; 2];

        nnls(
            &mut a, &mut b, &mut x, &mut rnorm, &mut w, &mut z, &mut index,
        )
        .unwrap();

        assert!(x[0] >= 0.0);
        assert!(x[1] == 0.0);

        // If x=[x0, 0], then objective is (x0-1)^2 + (x0-0)^2 + (x0+1)^2 = 3x0^2 - 2x0 + 1 + 1 + 2x0 + 1 = 3x0^2 + 2
        // Wait, (x0-1)^2 + x0^2 + (x0+1)^2 = x0^2 - 2x0 + 1 + x0^2 + x0^2 + 2x0 + 1 = 3x0^2 + 2.
        // Minimum at x0 = 0.
        // If x0 = 0, residual is [1, 0, -1], norm = sqrt(2).
        assert!((x[0] - 0.0).abs() < 1e-10);
        assert!((rnorm - 2.0f64.sqrt()).abs() < 1e-10);
    }
}
