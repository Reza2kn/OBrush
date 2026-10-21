/// DAXPY: CONSTANT TIMES A VECTOR PLUS A VECTOR.
/// y = alpha * x + y
pub fn vector_add_scaled(n: usize, alpha: f64, x: &[f64], y: &mut [f64]) {
    if n == 0 {
        return;
    }
    if alpha == 0.0 {
        return;
    }

    y[0..n]
        .iter_mut()
        .zip(x[0..n].iter())
        .for_each(|(y_i, x_i)| {
            *y_i += alpha * x_i;
        });
}

/// DCOPY: COPIES A VECTOR, X, TO A VECTOR, Y.
/// y = x
pub fn vector_copy(n: usize, x: &[f64], inc_x: usize, y: &mut [f64], inc_y: usize) {
    if n == 0 {
        return;
    }

    if inc_x == 1 && inc_y == 1 {
        // CODE FOR BOTH INCREMENTS EQUAL TO 1
        y[0..n]
            .iter_mut()
            .zip(x[0..n].iter())
            .for_each(|(y_i, x_i)| {
                *y_i = *x_i;
            });
    } else {
        // CODE FOR UNEQUAL INCREMENTS OR EQUAL INCREMENTS NOT EQUAL TO 1
        let mut index_x = 0;
        let mut index_y = 0;

        for _ in 0..n {
            y[index_y] = x[index_x];
            index_x += inc_x;
            index_y += inc_y;
        }
    }
}

/// DSCAL: SCALES A VECTOR BY A CONSTANT.
/// x = alpha * x
pub fn vector_scale(n: usize, alpha: f64, x: &mut [f64]) {
    if n == 0 {
        return;
    }

    x[0..n].iter_mut().for_each(|x_i| *x_i *= alpha);
}

/// DSCAL: SCALES A VECTOR BY A CONSTANT.
/// x = alpha * x
pub fn vector_mul(x: &mut [f64], alpha: f64) {
    x.iter_mut().for_each(|x_i| *x_i *= alpha);
}

/// DSROTG: CONSTRUCT GIVENS PLANE ROTATION.
pub fn givens_rotation_construct(a: &mut f64, b: &mut f64, cos: &mut f64, sin: &mut f64) {
    let scale_reference = if a.abs() > b.abs() { *a } else { *b };
    let scale = a.abs() + b.abs();
    let radius;
    let reconstruction_parameter;

    if scale == 0.0 {
        *cos = 1.0;
        *sin = 0.0;
        radius = 0.0;
    } else {
        radius = scale * (((*a / scale).powi(2) + (*b / scale).powi(2)).sqrt());
        let radius_signed = if scale_reference >= 0.0 {
            radius.abs()
        } else {
            -radius.abs()
        };
        let radius = radius_signed;
        *cos = *a / radius;
        *sin = *b / radius;
    }

    reconstruction_parameter = if cos.abs() > 0.0 && cos.abs() <= *sin {
        1.0 / *cos
    } else {
        *sin
    };
    *a = radius;
    *b = reconstruction_parameter;
}

/// DSROT: APPLIES A PLANE ROTATION.
pub fn givens_rotation_apply(n: usize, x: &mut [f64], y: &mut [f64], cos: f64, sin: f64) {
    if n == 0 {
        return;
    }

    x[..n]
        .iter_mut()
        .zip(y[..n].iter_mut())
        .for_each(|(x_i, y_i)| {
            let temp_x = cos * *x_i + sin * *y_i;
            *y_i = cos * *y_i - sin * *x_i;
            *x_i = temp_x;
        });
}

/// DDOT: FORMS THE DOT PRODUCT OF TWO VECTORS.
pub fn vector_dot_product(n: usize, x: &[f64], inc_x: usize, y: &[f64], inc_y: usize) -> f64 {
    let mut dot_product = 0.0;
    if n == 0 {
        return dot_product;
    }

    if inc_x == 1 && inc_y == 1 {
        // CODE FOR BOTH INCREMENTS EQUAL TO 1
        x[0..n].iter().zip(y[0..n].iter()).for_each(|(x_i, y_i)| {
            dot_product += *x_i * *y_i;
        });
    } else {
        // CODE FOR UNEQUAL INCREMENTS OR EQUAL INCREMENTS NOT EQUAL TO 1
        let mut index_x = 0;
        let mut index_y = 0;

        for _ in 0..n {
            dot_product += x[index_x] * y[index_y];
            index_x += inc_x;
            index_y += inc_y;
        }
    }
    dot_product
}

/// DNRM2: EUCLIDEAN NORM OF THE N-VECTOR.
pub fn vector_norm(n: usize, x: &[f64]) -> f64 {
    if n == 0 {
        return 0.0;
    }

    let small_threshold = 8.232e-11;
    let large_threshold = 1.304e19;

    let mut sum_squares_scaled = 0.0;
    let mut scale_factor = 0.0;

    let mut i = 0;
    let mut phase = 1;

    while i < n {
        let abs_xi = x[i].abs();

        match phase {
            1 => {
                // Phase 1: Small values, potentially zero or very small
                if abs_xi > small_threshold {
                    // Transition to Phase 3 (Normal values) or Phase 4 (Large values)
                    let high_threshold_scaled = large_threshold / n as f64;
                    for j in i..n {
                        let abs_xj = x[j].abs();
                        if abs_xj >= high_threshold_scaled {
                            // Move to Phase 4 (Large values)
                            let new_scale = abs_xj;
                            sum_squares_scaled = (sum_squares_scaled / new_scale) / new_scale + 1.0;
                            scale_factor = new_scale;
                            for k in j + 1..n {
                                let abs_xk = x[k].abs();
                                if abs_xk > scale_factor {
                                    sum_squares_scaled =
                                        1.0 + sum_squares_scaled * (scale_factor / x[k]).powi(2);
                                    scale_factor = abs_xk;
                                } else {
                                    sum_squares_scaled += (x[k] / scale_factor).powi(2);
                                }
                            }
                            return scale_factor * sum_squares_scaled.sqrt();
                        }
                        sum_squares_scaled += x[j].powi(2);
                    }
                    return sum_squares_scaled.sqrt();
                } else if abs_xi != 0.0 {
                    // Move to Phase 2 (Very small values, requiring scaling)
                    phase = 2;
                    scale_factor = abs_xi;
                    sum_squares_scaled = (x[i] / scale_factor).powi(2);
                }
            }
            2 => {
                // Phase 2: Very small values
                if abs_xi > small_threshold {
                    // Transition to Phase 3 (Normal values)
                    sum_squares_scaled = (sum_squares_scaled * scale_factor) * scale_factor;
                    let high_threshold_scaled = large_threshold / n as f64;
                    if abs_xi >= high_threshold_scaled {
                        // Move to Phase 4 (Large values)
                        let new_scale = abs_xi;
                        sum_squares_scaled = (sum_squares_scaled / new_scale) / new_scale + 1.0;
                        scale_factor = new_scale;
                        for k in i + 1..n {
                            let abs_xk = x[k].abs();
                            if abs_xk > scale_factor {
                                sum_squares_scaled =
                                    1.0 + sum_squares_scaled * (scale_factor / x[k]).powi(2);
                                scale_factor = abs_xk;
                            } else {
                                sum_squares_scaled += (x[k] / scale_factor).powi(2);
                            }
                        }
                        return scale_factor * sum_squares_scaled.sqrt();
                    } else {
                        sum_squares_scaled += x[i].powi(2);
                        // Continue Phase 3 loop
                        for j in i + 1..n {
                            let abs_xj = x[j].abs();
                            if abs_xj >= high_threshold_scaled {
                                let new_scale = abs_xj;
                                sum_squares_scaled =
                                    (sum_squares_scaled / new_scale) / new_scale + 1.0;
                                scale_factor = new_scale;
                                for k in j + 1..n {
                                    let abs_xk = x[k].abs();
                                    if abs_xk > scale_factor {
                                        sum_squares_scaled = 1.0
                                            + sum_squares_scaled * (scale_factor / x[k]).powi(2);
                                        scale_factor = abs_xk;
                                    } else {
                                        sum_squares_scaled += (x[k] / scale_factor).powi(2);
                                    }
                                }
                                return scale_factor * sum_squares_scaled.sqrt();
                            }
                            sum_squares_scaled += x[j].powi(2);
                        }
                        return sum_squares_scaled.sqrt();
                    }
                } else {
                    // Still in Phase 2
                    if abs_xi > scale_factor {
                        sum_squares_scaled =
                            1.0 + sum_squares_scaled * (scale_factor / x[i]).powi(2);
                        scale_factor = abs_xi;
                    } else {
                        sum_squares_scaled += (x[i] / scale_factor).powi(2);
                    }
                }
            }
            _ => unreachable!(),
        }
        i += 1;
    }

    if phase == 1 {
        0.0
    } else {
        scale_factor * sum_squares_scaled.sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_add_scaled() {
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        let mut y = [1.0, 1.0, 1.0, 1.0, 1.0];
        vector_add_scaled(5, 2.0, &x, &mut y);
        assert_eq!(y, [3.0, 5.0, 7.0, 9.0, 11.0]);
    }

    #[test]
    fn test_vector_copy() {
        let x = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let mut y = [0.0; 8];
        vector_copy(8, &x, 1, &mut y, 1);
        assert_eq!(y, x);

        let mut y2 = [0.0; 4];
        vector_copy(4, &x, 2, &mut y2, 1);
        assert_eq!(y2, [1.0, 3.0, 5.0, 7.0]);
    }

    #[test]
    fn test_vector_scale() {
        let mut x = [1.0, 2.0, 3.0, 4.0, 5.0];
        vector_scale(5, 2.0, &mut x);
        assert_eq!(x, [2.0, 4.0, 6.0, 8.0, 10.0]);
    }

    #[test]
    fn test_givens_rotation_construct() {
        let mut a = 3.0;
        let mut b = 4.0;
        let mut cos = 0.0;
        let mut sin = 0.0;
        givens_rotation_construct(&mut a, &mut b, &mut cos, &mut sin);
        // radius = sqrt(3^2 + 4^2) = 5
        // cos = 3/5 = 0.6
        // sin = 4/5 = 0.8
        // reconstruction_parameter = 1/cos = 1/0.6 = 1.6666666666666667 (since cos > 0 and cos <= sin)
        assert!((a - 5.0).abs() < 1e-10);
        assert!((cos - 0.6).abs() < 1e-10);
        assert!((sin - 0.8).abs() < 1e-10);
        assert!((b - (1.0 / 0.6)).abs() < 1e-10);
    }

    #[test]
    fn test_givens_rotation_apply() {
        let mut x = [1.0, 2.0];
        let mut y = [3.0, 4.0];
        let cos = 0.6;
        let sin = 0.8;
        givens_rotation_apply(2, &mut x, &mut y, cos, sin);
        // x[0] = 0.6*1 + 0.8*3 = 0.6 + 2.4 = 3.0
        // y[0] = 0.6*3 - 0.8*1 = 1.8 - 0.8 = 1.0
        // x[1] = 0.6*2 + 0.8*4 = 1.2 + 3.2 = 4.4
        // y[1] = 0.6*4 - 0.8*2 = 2.4 - 1.6 = 0.8
        assert!((x[0] - 3.0).abs() < 1e-10);
        assert!((y[0] - 1.0).abs() < 1e-10);
        assert!((x[1] - 4.4).abs() < 1e-10);
        assert!((y[1] - 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_vector_dot_product() {
        let x = [1.0, 2.0, 3.0];
        let y = [4.0, 5.0, 6.0];
        let dot = vector_dot_product(3, &x, 1, &y, 1);
        // 1*4 + 2*5 + 3*6 = 4 + 10 + 18 = 32
        assert_eq!(dot, 32.0);

        let dot2 = vector_dot_product(2, &x, 2, &y, 1);
        // x[0]*y[0] + x[2]*y[1] = 1*4 + 3*5 = 4 + 15 = 19
        assert_eq!(dot2, 19.0);
    }

    #[test]
    fn test_vector_norm() {
        let x = [3.0, 4.0];
        assert_eq!(vector_norm(2, &x), 5.0);

        let x2 = [0.0, 0.0, 1.0, 0.0];
        assert_eq!(vector_norm(4, &x2), 1.0);

        // Test with large values (Phase 4)
        let x3 = [1e20, 1e20];
        let expected = 2.0f64.sqrt() * 1e20;
        assert!((vector_norm(2, &x3) - expected).abs() / expected < 1e-14);

        // Test with small values (Phase 2)
        let x4 = [1e-20, 1e-20];
        let expected = 2.0f64.sqrt() * 1e-20;
        assert!((vector_norm(2, &x4) - expected).abs() / expected < 1e-14);
    }
}
