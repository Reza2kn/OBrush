use crate::blas::{vector_dot_product, vector_norm};
use crate::householder::{h12_apply, h12_construct};

/// HFTI: RANK-DEFICIENT LEAST SQUARES ALGORITHM
///
/// 1:1 replica of the `hfti` subroutine from `slsqp_optmz.f`.
pub fn hfti(
    matrix_a: &mut [f64],
    leading_dim_a: usize,
    m_rows: usize,
    n_cols: usize,
    matrix_b: &mut [f64],
    leading_dim_b: usize,
    num_rhs: usize,
    tolerance: f64,
    rank: &mut usize,
    residual_norms: &mut [f64],
    column_norms: &mut [f64],
    householder_scalars: &mut [f64],
    pivot_indices: &mut [i32],
) {
    let min_dim = m_rows.min(n_cols);
    if min_dim == 0 {
        *rank = 0;
        return;
    }

    let norm_update_threshold = 1.0e-3;
    let mut max_column_norm = 0.0;

    // COMPUTE COLUMN NORMS AND PERFORM HOUSEHOLDER TRANSFORMATIONS WITH PIVOTING
    for j in 0..min_dim {
        if j + 1 > 1 {
            let mut pivot_index = j + 1;
            for l in j..n_cols {
                column_norms[l] -= matrix_a[l * leading_dim_a + (j - 1)].powi(2);
                if column_norms[l] > column_norms[pivot_index - 1] {
                    pivot_index = l + 1;
                }
            }

            if (max_column_norm + norm_update_threshold * column_norms[pivot_index - 1])
                > max_column_norm
            {
                pivot_indices[j] = pivot_index as i32;
                if pivot_index != j + 1 {
                    for i in 0..m_rows {
                        let tmp = matrix_a[j * leading_dim_a + i];
                        matrix_a[j * leading_dim_a + i] =
                            matrix_a[(pivot_index - 1) * leading_dim_a + i];
                        matrix_a[(pivot_index - 1) * leading_dim_a + i] = tmp;
                    }
                    column_norms[pivot_index - 1] = column_norms[j];
                }
                apply_h12_to_a_and_b(
                    matrix_a,
                    leading_dim_a,
                    m_rows,
                    n_cols,
                    matrix_b,
                    leading_dim_b,
                    num_rhs,
                    j + 1,
                    column_norms,
                );
                continue;
            }
        }

        let mut pivot_index = j + 1;
        for l in j..n_cols {
            column_norms[l] = 0.0;
            for i in j..m_rows {
                column_norms[l] += matrix_a[(l) * leading_dim_a + i].powi(2);
            }
            if column_norms[l] > column_norms[pivot_index - 1] {
                pivot_index = l + 1;
            }
        }
        max_column_norm = column_norms[pivot_index - 1];

        pivot_indices[j] = pivot_index as i32;
        if pivot_index != j + 1 {
            for i in 0..m_rows {
                let tmp = matrix_a[j * leading_dim_a + i];
                matrix_a[j * leading_dim_a + i] = matrix_a[(pivot_index - 1) * leading_dim_a + i];
                matrix_a[(pivot_index - 1) * leading_dim_a + i] = tmp;
            }
            column_norms[pivot_index - 1] = column_norms[j];
        }

        apply_h12_to_a_and_b(
            matrix_a,
            leading_dim_a,
            m_rows,
            n_cols,
            matrix_b,
            leading_dim_b,
            num_rhs,
            j + 1,
            column_norms,
        );
    }

    // DETERMINE PSEUDORANK
    let mut pseudorank = 0;
    for j in 0..min_dim {
        if matrix_a[j * leading_dim_a + j].abs() <= tolerance {
            break;
        }
        pseudorank = j + 1;
    }
    *rank = pseudorank;
    let rank_plus_one = pseudorank + 1;

    // NORM OF RESIDUALS
    for rhs_idx in 0..num_rhs {
        if m_rows > pseudorank {
            residual_norms[rhs_idx] = vector_norm(
                m_rows - pseudorank,
                &matrix_b[(rhs_idx) * leading_dim_b + rank_plus_one - 1..],
            );
        } else {
            residual_norms[rhs_idx] = 0.0;
        }
    }

    if pseudorank == 0 {
        if num_rhs > 0 {
            for rhs_idx in 0..num_rhs {
                for i in 0..n_cols {
                    matrix_b[rhs_idx * leading_dim_b + i] = 0.0;
                }
            }
        }
        return;
    }

    if pseudorank < n_cols {
        // HOUSEHOLDER DECOMPOSITION OF FIRST K ROWS
        for i in (0..pseudorank).rev() {
            let mut householder_scalar = 0.0;
            // Row-wise H12: pivot row i, transform rows 1..i-1
            let (a_rows, a_pivot) = matrix_a.split_at_mut(i);
            // a_pivot[0..] starts at row i. u is row i, ice=lda.
            h12_construct(
                i + 1,
                rank_plus_one,
                n_cols,
                &mut a_pivot[0..],
                leading_dim_a,
                &mut householder_scalar,
                a_rows,
                leading_dim_a,
                1,
                i as i32,
            );
            householder_scalars[i] = householder_scalar;
        }
    }

    for rhs_idx in 0..num_rhs {
        for i in (0..pseudorank).rev() {
            let dot = if i + 1 < pseudorank {
                vector_dot_product(
                    pseudorank - i - 1,
                    &matrix_a[(i) + (i + 1) * leading_dim_a..],
                    leading_dim_a,
                    &matrix_b[(rhs_idx) * leading_dim_b + i + 1..],
                    1,
                )
            } else {
                0.0
            };
            matrix_b[(rhs_idx) * leading_dim_b + (i)] = (matrix_b[(rhs_idx) * leading_dim_b + (i)]
                - dot)
                / matrix_a[(i) * leading_dim_a + (i)];
        }

        if pseudorank < n_cols {
            for j in rank_plus_one..=n_cols {
                matrix_b[(rhs_idx) * leading_dim_b + (j - 1)] = 0.0;
            }
            for i in 0..pseudorank {
                let householder_scalar = householder_scalars[i];
                h12_apply(
                    i + 1,
                    rank_plus_one,
                    n_cols,
                    &matrix_a[i..],
                    leading_dim_a,
                    householder_scalar,
                    &mut matrix_b[(rhs_idx) * leading_dim_b..],
                    1,
                    leading_dim_b,
                    1,
                );
            }
        }

        for j in (0..min_dim).rev() {
            let l = pivot_indices[j] as usize;
            if l != j + 1 {
                let tmp = matrix_b[(rhs_idx) * leading_dim_b + l - 1];
                matrix_b[(rhs_idx) * leading_dim_b + l - 1] =
                    matrix_b[(rhs_idx) * leading_dim_b + j];
                matrix_b[(rhs_idx) * leading_dim_b + j] = tmp;
            }
        }
    }
}

fn apply_h12_to_a_and_b(
    matrix_a: &mut [f64],
    leading_dim_a: usize,
    m_rows: usize,
    n_cols: usize,
    matrix_b: &mut [f64],
    leading_dim_b: usize,
    num_rhs: usize,
    j: usize,
    column_norms: &mut [f64],
) {
    let mut householder_scalar = 0.0;
    let (a_u, a_c) = matrix_a.split_at_mut(j * leading_dim_a);
    let pivot_column = &mut a_u[(j - 1) * leading_dim_a..];

    if n_cols > j {
        h12_construct(
            j,
            j + 1,
            m_rows,
            pivot_column,
            1,
            &mut householder_scalar,
            a_c,
            1,
            leading_dim_a,
            (n_cols - j) as i32,
        );
    } else {
        h12_construct(
            j,
            j + 1,
            m_rows,
            pivot_column,
            1,
            &mut householder_scalar,
            &mut [],
            1,
            leading_dim_a,
            0,
        );
    }
    column_norms[j - 1] = householder_scalar;
    if num_rhs > 0 {
        h12_apply(
            j,
            j + 1,
            m_rows,
            pivot_column,
            1,
            householder_scalar,
            matrix_b,
            1,
            leading_dim_b,
            num_rhs as i32,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hfti_basic() {
        let mut matrix_a = [1.0, 0.0, 0.0, 1.0];
        let mut matrix_b = [1.0, 1.0];
        let mut residual_norms = [0.0];
        let mut column_norms = [0.0; 2];
        let mut householder_scalars = [0.0; 2];
        let mut pivot_indices = [0; 2];
        let mut rank = 0;

        hfti(
            &mut matrix_a,
            2,
            2,
            2,
            &mut matrix_b,
            2,
            1,
            1e-10,
            &mut rank,
            &mut residual_norms,
            &mut column_norms,
            &mut householder_scalars,
            &mut pivot_indices,
        );

        assert_eq!(rank, 2);
        assert!((matrix_b[0] - 1.0).abs() < 1e-10);
        assert!((matrix_b[1] - 1.0).abs() < 1e-10);
        assert!(residual_norms[0] < 1e-10);
    }
}
