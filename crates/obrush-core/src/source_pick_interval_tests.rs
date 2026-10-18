use super::*;
const UNIT: [[f64; 3]; 3] = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
fn check_hit(vertices: [[f64; 3]; 3], origin: [f64; 3], direction: [f64; 3]) {
    let t = scalar_hit(vertices, origin, direction)
        .expect("independently authored accepted scalar contact");
    let cert = certificate(&Descriptor::triangle(vertices), origin, direction);
    assert_ne!(cert.verdict, Verdict::ProvedMiss);
    if let Some(bounds) = cert.envelope {
        for i in 0..3 {
            let point = origin[i] + direction[i] * t;
            assert!(
                bounds[i][0] <= point && point <= bounds[i][1],
                "point {point:?} outside {:?}, cert={cert:?}",
                bounds[i]
            );
        }
    }
}
#[test]
fn c01_next_up_preserves_adjacent_positive_words() {
    assert_eq!(next_up(1.0).to_bits(), 1.0_f64.to_bits() + 1);
    assert_eq!(next_up(f64::from_bits(1)).to_bits(), 2);
}
#[test]
fn c02_next_down_preserves_adjacent_negative_words() {
    assert_eq!(next_down(-1.0).to_bits(), (-1.0_f64).to_bits() + 1);
    assert_eq!(next_up(-1.0).to_bits(), (-1.0_f64).to_bits() - 1);
}
#[test]
fn c03_both_signedzeros_are_enclosed_by_adjacent_subnormals() {
    for zero in [0.0, -0.0] {
        assert_eq!(next_up(zero).to_bits(), 1);
        assert_eq!(next_down(zero).to_bits(), (1_u64 << 63) | 1);
    }
    assert!(Interval::point(0.0)
        .plus(Interval::point(-0.0))
        .contains(-0.0));
}
#[test]
fn c04_extreme_adjacent_values_fail_closed_to_whole() {
    assert_eq!(next_up(f64::MAX), f64::INFINITY);
    assert_eq!(next_down(-f64::MAX), f64::NEG_INFINITY);
    assert_eq!(
        Interval::point(f64::MAX).times(Interval::point(2.0)),
        Interval::Whole
    );
}
#[test]
fn c05_nan_and_infinities_never_become_finite_bounds() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(Interval::point(value), Interval::Whole);
    }
}
#[test]
fn c06_outward_sum_contains_scalar_cancellation_and_rounding() {
    for (a, b) in [
        (1.0, -1.0),
        (1.0, f64::EPSILON / 2.0),
        (f64::MIN_POSITIVE, -f64::MIN_POSITIVE),
    ] {
        assert!(Interval::point(a).plus(Interval::point(b)).contains(a + b));
    }
}
#[test]
fn c07_outward_product_contains_all_literal_endpoint_products() {
    let a = Interval::range(-3.0, 2.0);
    let b = Interval::range(-7.0, 5.0);
    for x in [-3.0, 2.0] {
        for y in [-7.0, 5.0] {
            assert!(a.times(b).contains(x * y));
        }
    }
}
#[test]
fn c08_division_through_zero_is_unknown_not_a_miss() {
    assert_eq!(
        Interval::point(1.0).quotient(Interval::range(-0.0, 0.0)),
        Interval::Whole
    );
    assert_eq!(
        Interval::point(1.0).quotient(Interval::range(-1.0, 1.0)),
        Interval::Whole
    );
}
#[test]
fn c09_outward_quotient_contains_negative_denominator_endpoints() {
    let out = Interval::range(-3.0, 2.0).quotient(Interval::range(-7.0, -2.0));
    for x in [-3.0, 2.0] {
        for y in [-7.0, -2.0] {
            assert!(out.contains(x / y));
        }
    }
}
#[test]
fn c10_invalid_or_disjoint_intersection_is_unknown() {
    assert_eq!(Interval::range(2.0, 1.0), Interval::Whole);
    assert_eq!(
        Interval::range(0.0, 1.0).intersect(Interval::range(2.0, 3.0)),
        Interval::Whole
    );
}
#[test]
fn c11_tracked_product_error_encloses_literal_half_ulp_input_rounding() {
    let a = Tracked::input(Interval::point(1.0))
        .plus(Tracked::input(Interval::point(f64::EPSILON / 2.0)));
    assert!(a.floating.contains(1.0 + f64::EPSILON / 2.0));
    assert!(a.error.bounds().unwrap().1 >= f64::EPSILON / 2.0);
}
#[test]
fn c12_replay_ordinary_hit_contains_scalar_parameters() {
    let r = Replay::new(
        &Descriptor::triangle(UNIT),
        [0.25, 0.25, 1.0],
        [0.0, 0.0, -1.0],
    );
    assert!(r.determinant.floating.contains(1.0));
    assert!(r.u.floating.contains(0.25));
    assert!(r.v.floating.contains(0.25));
    assert!(r.t.floating.contains(1.0));
    let returned = certificate(
        &Descriptor::triangle(UNIT),
        [0.25, 0.25, 1.0],
        [0.0, 0.0, -1.0],
    )
    .replay;
    assert!(returned.determinant.floating.contains(1.0));
    assert!(returned.u.floating.contains(0.25));
    assert!(returned.v.floating.contains(0.25));
    assert!(returned.t.floating.contains(1.0));
    check_hit(UNIT, [0.25, 0.25, 1.0], [0.0, 0.0, -1.0]);
}
#[test]
fn c13_fixed_eps_outside_vertex_aabb_is_preserved() {
    assert_eq!(
        scalar_hit(UNIT, [-5e-16, 0.5, 1.0], [0.0, 0.0, -1.0]),
        Some(1.0)
    );
    let desc = Descriptor::triangle(UNIT);
    assert!(!desc.vertex_box[0].contains(-5e-16));
    let cert = certificate(&desc, [-5e-16, 0.5, 1.0], [0.0, 0.0, -1.0]);
    assert_eq!(cert.verdict, Verdict::Possible);
    assert!(cert.envelope.unwrap()[0][0] <= -5e-16);
    check_hit(UNIT, [-5e-16, 0.5, 1.0], [0.0, 0.0, -1.0]);
}
#[test]
fn c14_unit_triangle_vertex_contact_is_preserved() {
    check_hit(UNIT, [0.0, 0.0, 1.0], [0.0, 0.0, -1.0]);
    check_hit(UNIT, [1.0, 0.0, 1.0], [0.0, 0.0, -1.0]);
}
#[test]
fn c15_barycentric_sum_border_and_rounded_sum_are_preserved() {
    check_hit(UNIT, [0.5, 0.5, 1.0], [0.0, 0.0, -1.0]);
    check_hit(UNIT, [0.5, next_up(0.5), 1.0], [0.0, 0.0, -1.0]);
}
#[test]
fn c16_clearly_outside_barycentric_domain_is_proved_miss() {
    let o = [2.0, 2.0, 1.0];
    let d = [0.0, 0.0, -1.0];
    assert_eq!(scalar_hit(UNIT, o, d), None);
    assert_eq!(
        certificate(&Descriptor::triangle(UNIT), o, d).verdict,
        Verdict::ProvedMiss
    );
}
#[test]
fn c17_backwards_ray_is_proved_miss() {
    let o = [0.25, 0.25, -1.0];
    let d = [0.0, 0.0, -1.0];
    assert_eq!(scalar_hit(UNIT, o, d), None);
    assert_eq!(
        certificate(&Descriptor::triangle(UNIT), o, d).verdict,
        Verdict::ProvedMiss
    );
}
#[test]
fn c18_parallel_signedzero_axes_do_not_drop_real_contacts() {
    check_hit(UNIT, [0.25, 0.25, 1.0], [-0.0, 0.0, -1.0]);
    check_hit(UNIT, [0.25, 0.25, 1.0], [0.0, -0.0, -1.0]);
}
#[test]
fn c19_zero_distance_including_negative_zero_remains_accepted() {
    for zero in [0.0, -0.0] {
        assert!(scalar_hit(UNIT, [0.25, 0.25, zero], [0.0, 0.0, -1.0]).is_some());
        check_hit(UNIT, [0.25, 0.25, zero], [0.0, 0.0, -1.0]);
    }
}
#[test]
fn c20_degenerate_triangle_rejection_is_sound() {
    let degenerate = [[0.0, 0.0, 0.0]; 3];
    assert_eq!(
        scalar_hit(degenerate, [0.0, 0.0, 1.0], [0.0, 0.0, -1.0]),
        None
    );
    assert_eq!(
        certificate(
            &Descriptor::triangle(degenerate),
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0]
        )
        .verdict,
        Verdict::ProvedMiss
    );
}
#[test]
fn c21_determinant_eps_threshold_literal_scalar_cases() {
    let below = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, EPS / 2.0, 0.0]];
    assert_eq!(
        scalar_hit(below, [0.25, EPS / 8.0, 1.0], [0.0, 0.0, -1.0]),
        None
    );
    let equal = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, EPS, 0.0]];
    check_hit(equal, [0.25, EPS / 4.0, 1.0], [0.0, 0.0, -1.0]);
}
#[test]
fn c22_nonfinite_ray_is_unknown_even_with_rejecting_geometry() {
    let desc = Descriptor::triangle([[0.0; 3]; 3]);
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            certificate(&desc, [value, 0.0, 1.0], [0.0, 0.0, -1.0]).verdict,
            Verdict::Unknown
        );
    }
}
#[test]
fn c23_extreme_origin_produces_unknown_not_invented_geometric_miss() {
    let cert = certificate(
        &Descriptor::triangle(UNIT),
        [0.25, 0.25, f64::MAX],
        [0.0, 0.0, -1.0],
    );
    assert_ne!(cert.verdict, Verdict::Possible);
    assert_eq!(cert.verdict, Verdict::Unknown);
}
#[test]
fn c24_residual_envelope_handles_large_and_cancellation_offsets() {
    let shifted = UNIT.map(|p| [p[0] + 1e6, p[1] - 1e6, p[2] + 1e6]);
    check_hit(
        shifted,
        [1e6 + 0.25, -1e6 + 0.25, 1e6 + 1.0],
        [0.0, 0.0, -1.0],
    );
    let tiny = UNIT.map(|p| [p[0] * 1e-6, p[1] * 1e-6, p[2]]);
    check_hit(tiny, [2.5e-7, 2.5e-7, 1.0], [0.0, 0.0, -1.0]);
}
#[test]
fn c25_quotient_rounding_error_has_independent_third_word_witness() {
    let third = Tracked::input(Interval::point(1.0)).quotient(Tracked::input(Interval::point(3.0)));
    assert_eq!((1.0_f64 / 3.0).to_bits(), 0x3fd5_5555_5555_5555);
    assert!(third
        .floating
        .contains(f64::from_bits(0x3fd5_5555_5555_5555)));
    // Exact1/3 minus its nearest binary64 is1/(3*2^54)>2^-56.
    assert!(third.error.bounds().unwrap().1 >= f64::from_bits(0x3c70_0000_0000_0000));
}
#[test]
fn c26_tolerated_nonconvex_barycentric_sum_is_enclosed() {
    check_hit(UNIT, [1.0, 5e-16, 1.0], [0.0, 0.0, -1.0]);
}
#[test]
fn c27_tracked_overflow_is_unknown_at_every_layer() {
    let value =
        Tracked::input(Interval::point(f64::MAX)).plus(Tracked::input(Interval::point(f64::MAX)));
    assert_eq!(value.real, Interval::Whole);
    assert_eq!(value.floating, Interval::Whole);
    assert_eq!(value.error, Interval::Whole);
}
#[test]
fn c28_prototype_descriptors_and_certificates_have_fixed_holder_bounds() {
    assert!(std::mem::size_of::<Descriptor>() <= 512);
    assert!(std::mem::size_of::<Certificate>() <= 512);
    assert!(std::mem::size_of::<Replay>() <= 512);
}
#[test]
fn c29_edge_subtraction_reconstruction_residual_is_not_ignored() {
    let triangle = [[1e6, 0.0, 0.0], [1e-20, 0.0, 0.0], [1e6, 1.0, 0.0]];
    assert!((triangle[1][0] - triangle[0][0]) + triangle[0][0] != triangle[1][0]);
    check_hit(triangle, [5e5, 0.25, 1.0], [0.0, 0.0, -1.0]);
}
#[test]
fn c30_subnormal_distance_and_final_point_rounding_are_enclosed() {
    check_hit(UNIT, [0.25, 0.25, f64::from_bits(1)], [0.0, 0.0, -1.0]);
}
#[test]
fn c31_zero_direction_never_relabels_invalid_input_as_miss() {
    assert_eq!(
        certificate(
            &Descriptor::triangle(UNIT),
            [0.25, 0.25, 1.0],
            [0.0, -0.0, 0.0]
        )
        .verdict,
        Verdict::Unknown
    );
}
#[test]
fn c32_fixed_scalar_grid_has_no_rejected_authoritative_hits() {
    let desc = Descriptor::triangle(UNIT);
    for x in [-0.25, 0.0, 0.25, 0.75, 1.25] {
        for y in [-0.25, 0.0, 0.25, 0.75, 1.25] {
            for z in [-1.0, 0.0, 1.0] {
                let o = [x, y, z];
                let d = [0.0, 0.0, -1.0];
                let cert = certificate(&desc, o, d);
                if let Some(t) = scalar_hit(UNIT, o, d) {
                    assert_ne!(cert.verdict, Verdict::ProvedMiss);
                    if let Some(bounds) = cert.envelope {
                        for i in 0..3 {
                            let p = o[i] + d[i] * t;
                            assert!(bounds[i][0] <= p && p <= bounds[i][1]);
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn c33_aggregated_descriptor_contains_all_three_original_triangle_hits() {
    let triangles = [
        UNIT,
        UNIT.map(|p| [p[0] + 3.0, p[1], p[2]]),
        UNIT.map(|p| [p[0], p[1], p[2] + 2.0]),
    ];
    let aggregate = Descriptor::triangle(triangles[0])
        .hull(Descriptor::triangle(triangles[1]))
        .hull(Descriptor::triangle(triangles[2]));
    for o in [[0.25, 0.25, 3.0], [3.25, 0.25, 3.0]] {
        let d = [0.0, 0.0, -1.0];
        let cert = certificate(&aggregate, o, d);
        for triangle in triangles {
            if let Some(t) = scalar_hit(triangle, o, d) {
                assert_ne!(cert.verdict, Verdict::ProvedMiss);
                if let Some(bounds) = cert.envelope {
                    for i in 0..3 {
                        let p = o[i] + d[i] * t;
                        assert!(bounds[i][0] <= p && p <= bounds[i][1]);
                    }
                }
            }
        }
    }
}
#[test]
fn c34_unknown_child_does_not_create_a_finite_parent_certificate() {
    let ordinary = Descriptor::triangle(UNIT);
    let mut uncertain = ordinary;
    uncertain.a = [Interval::Whole; 3];
    uncertain.vertex_box = [Interval::Whole; 3];
    let parent = ordinary.hull(uncertain);
    assert_eq!(parent.vertex_box, [Interval::Whole; 3]);
    assert_eq!(
        certificate(&parent, [0.25, 0.25, 1.0], [0.0, 0.0, -1.0]).verdict,
        Verdict::Unknown
    );
}
