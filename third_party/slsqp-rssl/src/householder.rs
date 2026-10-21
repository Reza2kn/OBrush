/// # SLSQP Rust Householder Transformation Replica
///
/// 1:1 replica of the `h12` subroutine from `slsqp_optmz.f`.
/// Strictly follows the indexing and stride logic of the original source.

/// H1: CONSTRUCTION AND APPLICATION OF A SINGLE HOUSEHOLDER TRANSFORMATION
/// Q = I + U*(U**T)/B
///
/// # Arguments
/// * `pivot_index` - Index of the pivot element (1-based)
/// * `transform_start_index` - Starting index of elements to be eliminated (1-based)
/// * `end_index` - Dimension/ending index of the vector (1-based)
/// * `h_vector` - Array storing the Householder vector
/// * `h_stride` - Storage increment (stride) between elements in `h_vector`
/// * `h_scalar` - Intermediate variable produced by H1, used for H2
/// * `v_matrix` - Matrix/vector array to be transformed
/// * `v_stride_within` - Storage increment between elements within a vector in `v_matrix`
/// * `v_stride_between` - Storage increment between different vectors in `v_matrix`
/// * `v_num_cols` - Number of vectors in `v_matrix` to be transformed
pub fn h12_construct(
    pivot_index: usize,
    transform_start_index: usize,
    end_index: usize,
    h_vector: &mut [f64],
    h_stride: usize,
    h_scalar: &mut f64,
    v_matrix: &mut [f64],
    v_stride_within: usize,
    v_stride_between: usize,
    v_num_cols: i32,
) {
    // IF (0.GE.pivot_index.OR.pivot_index.GE.transform_start_index.OR.transform_start_index.GT.end_index) GOTO 80
    if pivot_index == 0 || pivot_index >= transform_start_index || transform_start_index > end_index
    {
        return;
    }

    // cl=ABS(u(1,pivot_index))
    let mut scale = h_vector[(pivot_index - 1) * h_stride].abs();

    // ****** CONSTRUCT THE TRANSFORMATION ******
    // DO 10 j=transform_start_index,end_index
    for j in transform_start_index..=end_index {
        let abs_val = h_vector[(j - 1) * h_stride].abs();
        if abs_val > scale {
            scale = abs_val;
        }
    }

    if scale <= 0.0 {
        return;
    }

    let inv_scale = 1.0 / scale;
    // sm=(u(1,pivot_index)*inv_scale)**2
    let mut sum_sq = (h_vector[(pivot_index - 1) * h_stride] * inv_scale).powi(2);
    // DO 20 j=transform_start_index,end_index
    for j in transform_start_index..=end_index {
        sum_sq += (h_vector[(j - 1) * h_stride] * inv_scale).powi(2);
    }
    let mut norm = scale * sum_sq.sqrt();
    if h_vector[(pivot_index - 1) * h_stride] > 0.0 {
        norm = -norm;
    }
    *h_scalar = h_vector[(pivot_index - 1) * h_stride] - norm;
    h_vector[(pivot_index - 1) * h_stride] = norm;

    apply_internal(
        pivot_index,
        transform_start_index,
        end_index,
        h_vector,
        h_stride,
        *h_scalar,
        v_matrix,
        v_stride_within,
        v_stride_between,
        v_num_cols,
    );
}

/// H2: APPLICATION OF A SINGLE HOUSEHOLDER TRANSFORMATION
///
/// # Arguments
/// * `pivot_index` - Index of the pivot element (1-based)
/// * `transform_start_index` - Starting index of elements to be eliminated (1-based)
/// * `end_index` - Dimension/ending index of the vector (1-based)
/// * `h_vector` - Array storing the Householder vector
/// * `h_stride` - Storage increment (stride) between elements in `h_vector`
/// * `h_scalar` - Intermediate variable produced by H1, used for H2
/// * `v_matrix` - Matrix/vector array to be transformed
/// * `v_stride_within` - Storage increment between elements within a vector in `v_matrix`
/// * `v_stride_between` - Storage increment between different vectors in `v_matrix`
/// * `v_num_cols` - Number of vectors in `v_matrix` to be transformed
pub fn h12_apply(
    pivot_index: usize,
    transform_start_index: usize,
    end_index: usize,
    h_vector: &[f64],
    h_stride: usize,
    h_scalar: f64,
    v_matrix: &mut [f64],
    v_stride_within: usize,
    v_stride_between: usize,
    v_num_cols: i32,
) {
    if pivot_index == 0 || pivot_index >= transform_start_index || transform_start_index > end_index
    {
        return;
    }

    let scale = h_vector[(pivot_index - 1) * h_stride].abs();
    if scale <= 0.0 {
        return;
    }

    apply_internal(
        pivot_index,
        transform_start_index,
        end_index,
        h_vector,
        h_stride,
        h_scalar,
        v_matrix,
        v_stride_within,
        v_stride_between,
        v_num_cols,
    );
}

/// Internal implementation for applying Householder transformation
fn apply_internal(
    pivot_index: usize,
    transform_start_index: usize,
    end_index: usize,
    h_vector: &[f64],
    h_stride: usize,
    h_scalar: f64,
    v_matrix: &mut [f64],
    v_stride_within: usize,
    v_stride_between: usize,
    v_num_cols: i32,
) {
    if v_num_cols <= 0 {
        return;
    }

    // normalization_factor = h_scalar * h_vector[pivot]
    let normalization_factor = h_scalar * h_vector[(pivot_index - 1) * h_stride];
    if normalization_factor >= 0.0 {
        return;
    }

    let inv_normalization_factor = 1.0 / normalization_factor;

    // v_pivot_index = 1-v_stride_between+v_stride_within*(pivot_index-1)
    // 0-based: v_pivot_index = -v_stride_between + v_stride_within*(pivot_index-1)
    let mut v_pivot_index = (pivot_index - 1) * v_stride_within;
    let transform_offset = (transform_start_index - pivot_index) * v_stride_within;

    for _col in 0..v_num_cols {
        let v_transform_start_index = v_pivot_index + transform_offset;
        let mut v_current_index = v_transform_start_index;

        // projection_sum = v_matrix[v_pivot_index] * h_scalar
        let mut projection_sum = v_matrix[v_pivot_index] * h_scalar;
        // DO 50 i=transform_start_index,end_index
        for i in transform_start_index..=end_index {
            projection_sum += v_matrix[v_current_index] * h_vector[(i - 1) * h_stride];
            v_current_index += v_stride_within;
        }

        if projection_sum != 0.0 {
            projection_sum *= inv_normalization_factor;
            v_matrix[v_pivot_index] += projection_sum * h_scalar;
            let mut v_transform_index = v_transform_start_index;
            for i in transform_start_index..=end_index {
                v_matrix[v_transform_index] += projection_sum * h_vector[(i - 1) * h_stride];
                v_transform_index += v_stride_within;
            }
        }

        v_pivot_index += v_stride_between;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_h12_construction_and_application() {
        // Test construction and application of Householder transformation
        // Goal: Transform vector [3, 4] to [-5, 0]
        let mut h = [3.0, 4.0];
        let mut hs = 0.0;
        let mut c = [3.0, 4.0];

        h12_construct(1, 2, 2, &mut h, 1, &mut hs, &mut c, 1, 1, 1);

        // After construction, h[0] should be -5.0 (norm)
        assert!((h[0] + 5.0).abs() < 1e-10);
        // hs should be 3.0 - (-5.0) = 8.0
        assert!((hs - 8.0).abs() < 1e-10);
        // c[0] should become -5.0, c[1] should become 0.0
        assert!((c[0] + 5.0).abs() < 1e-10);
        assert!(c[1].abs() < 1e-10);
    }

    #[test]
    fn test_h2_application() {
        // Test independent application
        let h = [-5.0, 4.0]; // h[0] is norm
        let hs = 8.0;
        let mut c = [1.0, 2.0]; // Apply to another vector

        h12_apply(1, 2, 2, &h, 1, hs, &mut c, 1, 1, 1);

        // Verify application logic:
        // normalization_factor = hs * h[0] = 8.0 * -5.0 = -40.0
        // projection_sum = c[0]*hs + c[1]*h[1] = 1.0*8.0 + 2.0*4.0 = 8.0 + 8.0 = 16.0
        // projection_sum = projection_sum / normalization_factor = 16.0 / -40.0 = -0.4
        // c[0] = c[0] + projection_sum*hs = 1.0 + (-0.4)*8.0 = 1.0 - 3.2 = -2.2
        // c[1] = c[1] + projection_sum*h[1] = 2.0 + (-0.4)*4.0 = 2.0 - 1.6 = 0.4

        assert!((c[0] + 2.2).abs() < 1e-10);
        assert!((c[1] - 0.4).abs() < 1e-10);
    }
}
