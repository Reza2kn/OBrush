use crate::Mat;

/// Updates the LDL' factors of matrix A by rank-one matrix sigma*z*z'
///
/// PURSPOSE:
/// Updates the LDL' factors of matrix A by rank-one matrix sigma*z*z'
///
/// INPUT ARGUMENTS:
/// order   : order of the coefficient matrix ldl_matrix
/// ldl_matrix : positive definite matrix of dimension order;
///          only the lower triangle is used and is stored column by
///          column as one dimensional array of dimension order*(order+1)/2.
/// update_vector : vector of dimension order of updating elements
/// sigma_factor : scalar factor by which the modifying dyade update_vector*update_vector' is multiplied
///
/// OUTPUT ARGUMENTS:
/// ldl_matrix : updated ldl' factors
///
/// WORKING ARRAY:
/// workspace : vector of dimension order (used only if sigma_factor < 0)
///
/// METHOD:
/// That of Fletcher and Powell as described in:
/// Fletcher, R., (1974) On the modification of LDL' factorization.
/// Powell, M.J.D. Math. Computation 28, 1067-1078.
pub fn ldl(
    order: usize,
    ldl_matrix: &mut Mat,
    update_vector: &mut [f64],
    sigma_factor: f64,
    workspace: &mut [f64],
) {
    let machine_epsilon = 2.22e-16;

    if sigma_factor == 0.0 {
        return;
    }

    let mut t_scalar = 1.0 / sigma_factor;

    if sigma_factor < 0.0 {
        // PREPARE NEGATIVE UPDATE
        for i in 0..order {
            workspace[i] = update_vector[i];
        }
        for i in 0..order {
            let v_i = workspace[i];
            t_scalar += v_i * v_i / ldl_matrix[(i, i)];
            for j in i + 1..order {
                workspace[j] -= v_i * ldl_matrix[(j, i)];
            }
        }

        if t_scalar >= 0.0 {
            t_scalar = machine_epsilon / sigma_factor;
        }

        for i in 0..order {
            let j = order - i - 1;
            let u_j = workspace[j];
            workspace[j] = t_scalar;
            t_scalar -= u_j * u_j / ldl_matrix[(j, j)];
        }
    }

    // HERE UPDATING BEGINS
    for i in 0..order {
        let v_i = update_vector[i];
        let delta = v_i / ldl_matrix[(i, i)];
        let t_next = if sigma_factor < 0.0 {
            workspace[i]
        } else {
            t_scalar + delta * v_i
        };
        let alpha = t_next / t_scalar;
        ldl_matrix[(i, i)] = alpha * ldl_matrix[(i, i)];

        if i == order - 1 {
            return;
        }

        let beta = delta / t_next;
        if alpha <= 4.0 {
            for j in i + 1..order {
                update_vector[j] -= v_i * ldl_matrix[(j, i)];
                ldl_matrix[(j, i)] += beta * update_vector[j];
            }
        } else {
            let gamma = t_scalar / t_next;
            for j in i + 1..order {
                let l_val = ldl_matrix[(j, i)];
                ldl_matrix[(j, i)] = gamma * l_val + beta * update_vector[j];
                update_vector[j] -= v_i * l_val;
            }
        }
        t_scalar = t_next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ldl_positive_update() {
        // A = L D L'
        // L = [1 0]
        //     [2 1]
        // D = [3 0]
        //     [0 4]
        // A = [1 0] [3 0] [1 2] = [3 0] [1 2] = [3 6]
        //     [2 1] [0 4] [0 1]   [6 4] [0 1]   [6 16]

        // A(0,0) = D(0,0) = 3
        // A(1,0) = L(1,0) = 2
        // A(1,1) = D(1,1) = 4
        let mut ldl_matrix = Mat::new(2, 2);
        ldl_matrix[(0, 0)] = 3.0;
        ldl_matrix[(1, 0)] = 2.0;
        ldl_matrix[(1, 1)] = 4.0;
        let order = 2;
        let mut update_vector = [1.0, 2.0];
        let sigma_factor = 1.0;
        let mut workspace = [0.0; 2];

        ldl(
            order,
            &mut ldl_matrix,
            &mut update_vector,
            sigma_factor,
            &mut workspace,
        );

        // Expected result: A + sigma * z * z'
        // A = [3 6] + 1 * [1] [1 2] = [3 6] + [1 2] = [4  8]
        //     [6 16]      [2]         [6 16]   [2 4]   [8 20]
        // New A = [4  8]
        //         [8 20]
        // LDL' factorization of [4 8; 8 20]:
        // L = [1 0]
        //     [l21 1]
        // D = [d1 0]
        //     [0 d2]
        // [d1, d1*l21; d1*l21, d1*l21^2 + d2] = [4 8; 8 20]
        // d1 = 4
        // 4 * l21 = 8 => l21 = 2
        // 4 * 2^2 + d2 = 20 => 16 + d2 = 20 => d2 = 4
        // Expected storage: (0,0)=4.0, (1,0)=2.0, (1,1)=4.0

        assert_eq!(ldl_matrix[(0, 0)], 4.0);
        assert_eq!(ldl_matrix[(1, 0)], 2.0);
        assert_eq!(ldl_matrix[(1, 1)], 4.0);
    }

    #[test]
    fn test_ldl_negative_update() {
        // Start with the result of previous test
        let mut ldl_matrix = Mat::new(2, 2);
        ldl_matrix[(0, 0)] = 4.0;
        ldl_matrix[(1, 0)] = 2.0;
        ldl_matrix[(1, 1)] = 4.0;
        let order = 2;
        let mut update_vector = [1.0, 2.0];
        let sigma_factor = -1.0;
        let mut workspace = [0.0; 2];

        ldl(
            order,
            &mut ldl_matrix,
            &mut update_vector,
            sigma_factor,
            &mut workspace,
        );

        // Expected: back to (0,0)=3.0, (1,0)=2.0, (1,1)=4.0
        assert!((ldl_matrix[(0, 0)] - 3.0).abs() < 1e-12);
        assert!((ldl_matrix[(1, 0)] - 2.0).abs() < 1e-12);
        assert!((ldl_matrix[(1, 1)] - 4.0).abs() < 1e-12);
    }
}

/// Reconstructs the full Hessian matrix from its LDL' factors.
///
/// The factors are stored in `ldl_factors` as a Mat where:
/// - Diagonal elements are D(i,i)
/// - Off-diagonal elements are L(j,i) for j > i
pub fn reconstruct_hessian(order: usize, ldl_factors: &Mat) -> Mat {
    let mut hessian = Mat::new(order, order);

    for i in 0..order {
        for j in 0..order {
            let mut sum = 0.0;
            for k in 0..=i.min(j) {
                let l_ik = if i == k { 1.0 } else { ldl_factors[(i, k)] };
                let d_kk = ldl_factors[(k, k)];
                let l_jk = if j == k { 1.0 } else { ldl_factors[(j, k)] };
                sum += l_ik * d_kk * l_jk;
            }
            hessian[(i, j)] = sum;
        }
    }
    hessian
}
