//! Independently authored projection/shape/mask/history contracts. No GPU or
//! reference application is used. Selection/unchanged masks are bit-exact.
use obrush_core::{
    Document, MaskRegion, MaskRegionKind, MaskRegionReport, MaskRegionSettings, Mesh, Vec3,
};
use serde_json::{json, Value};
use std::sync::Arc;

const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];
const VIEWPORT: [f64; 4] = [0.0, 0.0, 128.0, 128.0];
const BOUNDARY: f64 = 1e-7;

fn fixture(points: &[[f32; 3]]) -> Mesh {
    assert!(points.len() >= 3);
    let mut obj = String::new();
    for p in points {
        obj.push_str(&format!("v {} {} {}\n", p[0], p[1], p[2]));
    }
    obj.push_str("f 1 2 3\n");
    let mut mesh = Mesh::from_obj(&obj).unwrap();
    for (i, m) in mesh.masks.iter_mut().enumerate() {
        *m = (i % 7) as f32 / 8.0;
    }
    for (i, c) in mesh.colors.iter_mut().enumerate() {
        *c = [
            (i % 5) as f32 / 5.0,
            (i % 7) as f32 / 7.0,
            (i % 11) as f32 / 11.0,
        ];
    }
    mesh
}

fn screen_fixture(points: &[[f64; 2]]) -> Mesh {
    fixture(
        &points
            .iter()
            .map(|p| [(p[0] / 64.0 - 1.0) as f32, (1.0 - p[1] / 64.0) as f32, 0.0])
            .collect::<Vec<_>>(),
    )
}

fn rect(start: [f64; 2], end: [f64; 2], erase: bool) -> MaskRegionSettings {
    MaskRegionSettings {
        region: MaskRegion::Rectangle { start, end },
        view_projection: IDENTITY,
        viewport: VIEWPORT,
        erase,
    }
}

fn lasso(points: Vec<[f64; 2]>, erase: bool) -> MaskRegionSettings {
    MaskRegionSettings {
        region: MaskRegion::Lasso { points },
        view_projection: IDENTITY,
        viewport: VIEWPORT,
        erase,
    }
}

fn bits(mesh: &Mesh) -> Value {
    json!({
        "positions":mesh.positions.iter().map(|p|p.to_array().map(f32::to_bits)).collect::<Vec<_>>(),
        "normals":mesh.normals.iter().map(|p|p.to_array().map(f32::to_bits)).collect::<Vec<_>>(),
        "masks":mesh.masks.iter().map(|p|p.to_bits()).collect::<Vec<_>>(),
        "colors":mesh.colors.iter().map(|p|p.map(f32::to_bits)).collect::<Vec<_>>(),
        "triangles":mesh.triangles,
    })
}

fn state(document: &Document) -> String {
    document.to_json().unwrap()
}

fn strip_masks(value: &mut Value) {
    match value {
        Value::Object(fields) => {
            fields.remove("masks");
            for v in fields.values_mut() {
                strip_masks(v);
            }
        }
        Value::Array(values) => {
            for v in values {
                strip_masks(v);
            }
        }
        _ => (),
    }
}

fn nonmask(document: &Document) -> String {
    let mut value: Value = serde_json::from_str(&state(document)).unwrap();
    strip_masks(&mut value);
    serde_json::to_string(&value).unwrap()
}

fn projection(point: Vec3, settings: &MaskRegionSettings) -> Option<[f64; 2]> {
    let p = [
        f64::from(point.x),
        f64::from(point.y),
        f64::from(point.z),
        1.0,
    ];
    let clip: [f64; 4] = std::array::from_fn(|row| {
        (0..4)
            .map(|column| f64::from(settings.view_projection[column * 4 + row]) * p[column])
            .sum()
    });
    if !clip.iter().all(|v| v.is_finite())
        || clip[3] <= 0.0
        || (0..3).any(|i| clip[i] < -clip[3] || clip[i] > clip[3])
    {
        return None;
    }
    let [x, y, w, h] = settings.viewport;
    Some([
        x + (clip[0] / clip[3] + 1.0) * w / 2.0,
        y + (1.0 - clip[1] / clip[3]) * h / 2.0,
    ])
}

fn segment_distance(point: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let v = [b[0] - a[0], b[1] - a[1]];
    let p = [point[0] - a[0], point[1] - a[1]];
    let length = v[0] * v[0] + v[1] * v[1];
    let t = if length == 0.0 {
        0.0
    } else {
        ((p[0] * v[0] + p[1] * v[1]) / length).clamp(0.0, 1.0)
    };
    (point[0] - a[0] - t * v[0]).hypot(point[1] - a[1] - t * v[1])
}

fn inside(point: [f64; 2], region: &MaskRegion) -> bool {
    match region {
        MaskRegion::Rectangle { start, end } => {
            start[0] != end[0]
                && start[1] != end[1]
                && (0..2)
                    .all(|i| point[i] >= start[i].min(end[i]) && point[i] <= start[i].max(end[i]))
        }
        MaskRegion::Lasso { points } => {
            let mut odd = false;
            for i in 0..points.len() {
                let a = points[i];
                let b = points[(i + 1) % points.len()];
                let in_bbox = (0..2).all(|k| {
                    point[k] >= a[k].min(b[k]) - BOUNDARY && point[k] <= a[k].max(b[k]) + BOUNDARY
                });
                if in_bbox && segment_distance(point, a, b) <= BOUNDARY {
                    return true;
                }
                if (a[1] > point[1]) != (b[1] > point[1]) {
                    let intersection = a[0] + (point[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1]);
                    if point[0] < intersection {
                        odd = !odd;
                    }
                }
            }
            odd
        }
    }
}

fn oracle(mesh: &Mesh, settings: &MaskRegionSettings) -> (Vec<f32>, usize, usize) {
    let mut masks = mesh.masks.clone();
    let mut projected = 0;
    let mut selected = 0;
    for (i, &point) in mesh.positions.iter().enumerate() {
        let Some(screen) = projection(point, settings) else {
            continue;
        };
        projected += 1;
        if inside(screen, &settings.region) {
            selected += 1;
            let target = if settings.erase { 0.0 } else { 1.0 };
            if masks[i] != target {
                masks[i] = target;
            }
        }
    }
    (masks, projected, selected)
}

fn report_check(
    report: &MaskRegionReport,
    source: &[f32],
    output: &[f32],
    settings: &MaskRegionSettings,
    projected: usize,
    selected: usize,
) {
    match settings.region {
        MaskRegion::Rectangle { .. } => assert!(matches!(report.kind, MaskRegionKind::Rectangle)),
        MaskRegion::Lasso { .. } => assert!(matches!(report.kind, MaskRegionKind::Lasso)),
    }
    assert_eq!(report.erase, settings.erase);
    assert_eq!(report.vertices, source.len());
    assert_eq!(report.projected_vertices, projected);
    assert_eq!(report.selected_vertices, selected);
    let points = match &settings.region {
        MaskRegion::Rectangle { .. } => 4,
        MaskRegion::Lasso { points } => points.len(),
    };
    let work = match settings.region {
        MaskRegion::Rectangle { .. } => 0,
        MaskRegion::Lasso { .. } => source.len() as u64 * points as u64,
    };
    assert_eq!(report.polygon_points, points);
    assert_eq!(report.point_edge_work, work);
    let changed = source
        .iter()
        .zip(output)
        .filter(|(a, b)| a.to_bits() != b.to_bits())
        .count();
    let maximum = source
        .iter()
        .zip(output)
        .map(|(&a, &b)| (f64::from(a) - f64::from(b)).abs())
        .fold(0.0, f64::max);
    assert_eq!(report.changed_vertices, changed);
    assert_eq!(report.max_abs_change.to_bits(), maximum.to_bits());
}

fn selected(
    mesh: &Mesh,
    settings: &MaskRegionSettings,
    case: &str,
) -> (Vec<f32>, MaskRegionReport) {
    let before = bits(mesh);
    let (expected, projected, count) = oracle(mesh, settings);
    let (actual, report) = mesh.region_masks(settings).unwrap();
    assert_eq!(bits(mesh), before, "Mesh source must be immutable");
    let actual_bits = actual.iter().map(|m| m.to_bits()).collect::<Vec<_>>();
    let expected_bits = expected.iter().map(|m| m.to_bits()).collect::<Vec<_>>();
    assert_eq!(
        actual_bits, expected_bits,
        "{case}: exact selected/outside mask bits"
    );
    report_check(&report, &mesh.masks, &actual, settings, projected, count);
    eprintln!(
        "mask_region_contract_observation: {}",
        json!({"case":case,"projected_vertices":projected,"selected_vertices":count,"changed_vertices":report.changed_vertices,"mask_bits_exact":true,"source_bits_exact":true,"point_edge_work":report.point_edge_work})
    );
    (actual, report)
}

fn known(
    mesh: &Mesh,
    settings: &MaskRegionSettings,
    indices: &[usize],
    case: &str,
) -> (Vec<f32>, MaskRegionReport) {
    let (masks, report) = selected(mesh, settings, case);
    let mut expected = mesh.masks.clone();
    for &i in indices {
        let target = if settings.erase { 0.0 } else { 1.0 };
        if expected[i] != target {
            expected[i] = target;
        }
    }
    assert_eq!(
        masks.iter().map(|m| m.to_bits()).collect::<Vec<_>>(),
        expected.iter().map(|m| m.to_bits()).collect::<Vec<_>>(),
        "{case}: analytic known indices"
    );
    assert_eq!(report.selected_vertices, indices.len());
    (masks, report)
}

fn identity_mesh() -> Mesh {
    fixture(&[
        [-1.0, -1.0, 0.0],
        [-1.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
        [1.0, -1.0, 0.0],
        [0.0, 0.0, 0.0],
        [0.5, -0.5, 0.0],
        [0.5, 0.5, 0.0],
        [0.0, 0.0, 0.5],
        [0.0, 0.0, -0.5],
        [0.0, 0.0, 1.01],
        [1.01, 0.0, 0.0],
    ])
}

#[test]
fn identity_projection_uses_top_left_screen_coordinates_and_inclusive_rectangle() {
    let mesh = identity_mesh();
    let mut settings = rect([60.0, 40.0], [85.0, 80.0], false);
    settings.viewport = [10.0, 20.0, 100.0, 80.0];
    let (_, report) = known(&mesh, &settings, &[4, 5, 6, 7, 8], "identity-rectangle");
    assert_eq!(report.projected_vertices, 9);
}

#[test]
fn reversed_rectangle_corners_select_the_same_exact_masks() {
    let mesh = identity_mesh();
    let a = rect([64.0, 32.0], [96.0, 96.0], false);
    let b = rect([96.0, 96.0], [64.0, 32.0], false);
    let (a, _) = known(&mesh, &a, &[4, 5, 6, 7, 8], "rectangle-forward");
    let (b, _) = selected(&mesh, &b, "rectangle-reversed");
    assert_eq!(a, b);
}

#[test]
fn erase_writes_zero_inside_and_preserves_outside_signed_bits() {
    let mut mesh = identity_mesh();
    mesh.masks.fill(0.625);
    mesh.masks[0] = -0.0;
    mesh.masks[4] = -0.0;
    let settings = rect([64.0, 32.0], [96.0, 96.0], true);
    let (masks, report) = known(&mesh, &settings, &[4, 5, 6, 7, 8], "erase-region");
    assert_eq!(masks[0].to_bits(), (-0.0_f32).to_bits());
    assert_eq!(masks[4].to_bits(), (-0.0_f32).to_bits());
    assert_eq!(report.changed_vertices, 4);
}

#[test]
fn zero_extent_rectangles_are_empty_but_project_all_eligible_vertices() {
    let mesh = identity_mesh();
    for settings in [
        rect([64.0, 0.0], [64.0, 128.0], false),
        rect([0.0, 64.0], [128.0, 64.0], true),
        rect([64.0, 64.0], [64.0, 64.0], false),
    ] {
        let (masks, report) = known(&mesh, &settings, &[], "zero-extent-rectangle");
        assert_eq!(masks, mesh.masks);
        assert_eq!(report.projected_vertices, 9);
        assert_eq!(report.changed_vertices, 0);
    }
}

#[test]
fn rectangle_includes_each_edge_corner_and_only_exact_bounds() {
    let mesh = screen_fixture(&[
        [16.0, 16.0],
        [112.0, 16.0],
        [112.0, 112.0],
        [16.0, 112.0],
        [64.0, 16.0],
        [112.0, 64.0],
        [64.0, 112.0],
        [16.0, 64.0],
        [64.0, 64.0],
        [8.0, 64.0],
        [120.0, 64.0],
    ]);
    known(
        &mesh,
        &rect([16.0, 16.0], [112.0, 112.0], false),
        &(0..9).collect::<Vec<_>>(),
        "rectangle-edges-corners",
    );
}

#[test]
fn convex_lasso_and_reversed_winding_match_analytic_rectangle() {
    let mesh = screen_fixture(&[
        [16.0, 16.0],
        [112.0, 16.0],
        [112.0, 112.0],
        [16.0, 112.0],
        [64.0, 64.0],
        [8.0, 64.0],
        [120.0, 64.0],
    ]);
    let points = vec![[16.0, 16.0], [112.0, 16.0], [112.0, 112.0], [16.0, 112.0]];
    let mut reversed = points.clone();
    reversed.reverse();
    let (a, _) = known(
        &mesh,
        &lasso(points, false),
        &[0, 1, 2, 3, 4],
        "convex-lasso",
    );
    let (b, _) = selected(&mesh, &lasso(reversed, false), "reversed-lasso");
    assert_eq!(a, b);
}

#[test]
fn concave_lasso_excludes_notch_and_includes_its_boundary() {
    let mesh = screen_fixture(&[
        [64.0, 32.0],
        [32.0, 80.0],
        [96.0, 80.0],
        [64.0, 80.0],
        [48.0, 80.0],
        [64.0, 48.0],
        [8.0, 64.0],
    ]);
    let points = vec![
        [16.0, 16.0],
        [112.0, 16.0],
        [112.0, 112.0],
        [80.0, 112.0],
        [80.0, 48.0],
        [48.0, 48.0],
        [48.0, 112.0],
        [16.0, 112.0],
    ];
    known(
        &mesh,
        &lasso(points, false),
        &[0, 1, 2, 4, 5],
        "concave-notch",
    );
}

#[test]
fn zero_shoelace_bowtie_is_valid_evenodd_with_inclusive_crossing() {
    let mesh = screen_fixture(&[
        [64.0, 32.0],
        [64.0, 96.0],
        [64.0, 64.0],
        [32.0, 64.0],
        [96.0, 64.0],
        [64.0, 16.0],
        [64.0, 112.0],
        [16.0, 16.0],
    ]);
    let points = vec![[16.0, 16.0], [112.0, 112.0], [16.0, 112.0], [112.0, 16.0]];
    known(
        &mesh,
        &lasso(points, false),
        &[0, 1, 2, 5, 6, 7],
        "bowtie-evenodd",
    );
}

#[test]
fn double_traced_contour_has_empty_interior_and_keeps_boundary() {
    let mesh = screen_fixture(&[[64.0, 64.0], [64.0, 16.0], [112.0, 64.0], [8.0, 64.0]]);
    let cycle = [[16.0, 16.0], [112.0, 16.0], [112.0, 112.0], [16.0, 112.0]];
    let points = cycle.into_iter().chain(cycle).collect();
    let (_, report) = known(
        &mesh,
        &lasso(points, false),
        &[1, 2],
        "double-contour-evenodd",
    );
    assert_eq!(report.polygon_points, 8);
    assert_eq!(report.point_edge_work, 32);
}

#[test]
fn consecutive_and_closing_duplicates_retain_shape_and_raw_report_count() {
    let mesh = screen_fixture(&[[64.0, 64.0], [16.0, 16.0], [8.0, 64.0]]);
    let points = vec![
        [16.0, 16.0],
        [16.0, 16.0],
        [112.0, 16.0],
        [112.0, 112.0],
        [16.0, 112.0],
        [16.0, 16.0],
    ];
    let (_, report) = known(
        &mesh,
        &lasso(points, false),
        &[0, 1],
        "repeated-closing-points",
    );
    assert_eq!(report.polygon_points, 6);
    assert_eq!(report.point_edge_work, 18);
}

#[test]
fn lasso_boundary_distance_includes_half_epsilon_and_excludes_twice_epsilon() {
    let mesh = fixture(&[
        [-5e-8, 0.0, 0.0],
        [-2e-7, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [0.1, 0.0, 0.0],
    ]);
    let mut settings = lasso(vec![[1.0, 0.5], [1.8, 0.5], [1.8, 1.5], [1.0, 1.5]], false);
    settings.viewport = [0.0, 0.0, 2.0, 2.0];
    known(&mesh, &settings, &[0, 2, 3], "lasso-distance-epsilon");
}

#[test]
fn boundary_distance_is_to_finite_segment_not_infinite_supporting_line() {
    let mesh = fixture(&[
        [9e-8, -9e-8, 0.0],
        [5e-8, -5e-8, 0.0],
        [0.0, 0.0, 0.0],
        [-0.25, 0.25, 0.0],
    ]);
    let mut settings = lasso(vec![[0.5, 0.5], [1.0, 1.0], [0.5, 1.0]], false);
    settings.viewport = [0.0, 0.0, 2.0, 2.0];
    known(
        &mesh,
        &settings,
        &[1, 2, 3],
        "finite-segment-endpoint-epsilon",
    );
}

#[test]
fn perspective_homogeneous_frustum_and_nonpositive_w_are_checked_before_division() {
    let mesh = fixture(&[
        [0.0, 0.0, -1.0],
        [0.0, 0.0, -3.0],
        [0.5, -0.5, -1.0],
        [1.0, -1.0, -2.0],
        [1.5, -1.5, -3.0],
        [0.0, 0.0, -0.5],
        [0.0, 0.0, -4.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 0.0],
        [2.0, 0.0, -2.0],
        [-2.0, 0.0, -2.0],
        [0.0, 2.0, -2.0],
        [0.0, -2.0, -2.0],
        [2.0001, 0.0, -2.0],
        [-2.0001, 0.0, -2.0],
        [0.0, 2.0001, -2.0],
        [0.0, -2.0001, -2.0],
    ]);
    let mut settings = rect([60.0, 60.0], [85.0, 80.0], false);
    settings.viewport = [10.0, 20.0, 100.0, 80.0];
    settings.view_projection = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, -2.0, -1.0, 0.0, 0.0, -3.0, 0.0,
    ];
    let (_, report) = known(
        &mesh,
        &settings,
        &[0, 1, 2, 3, 4],
        "perspective-frustum-through-depth",
    );
    assert_eq!(report.projected_vertices, 9);
    settings.region = MaskRegion::Rectangle {
        start: [10.0, 20.0],
        end: [110.0, 100.0],
    };
    known(
        &mesh,
        &settings,
        &[0, 1, 2, 3, 4, 9, 10, 11, 12],
        "inclusive-six-frustum-planes",
    );
}

#[test]
fn asymmetric_column_major_matrix_selects_known_translated_scaled_points() {
    let mesh = fixture(&[
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [-1.0, -1.0, 0.0],
    ]);
    let mut settings = rect([70.0, 69.0], [75.0, 82.0], false);
    settings.viewport = [10.0, 20.0, 100.0, 80.0];
    settings.view_projection = [
        0.5, 0.0, 0.0, 0.0, 0.0, 0.25, 0.0, 0.0, 0.0, 0.0, 0.5, 0.0, 0.25, -0.5, 0.0, 1.0,
    ];
    known(&mesh, &settings, &[0, 2], "asymmetric-column-major");
}

#[test]
fn finite_singular_and_zero_matrices_follow_projection_policy_without_inverse_requirement() {
    let mesh = identity_mesh();
    let mut settings = rect([0.0, 0.0], [128.0, 128.0], false);
    settings.view_projection = [0.0; 16];
    let (_, report) = known(&mesh, &settings, &[], "zero-matrix-nonpositive-w");
    assert_eq!(report.projected_vertices, 0);
    settings.view_projection[10] = 1.0;
    settings.view_projection[15] = 1.0;
    known(
        &mesh,
        &settings,
        &[0, 1, 2, 3, 4, 5, 6, 7, 8, 10],
        "singular-finite-projection",
    );
}

#[test]
fn arbitrarily_small_positive_w_is_eligible_and_huge_finite_coefficients_use_f64() {
    let mesh = identity_mesh();
    let mut settings = rect([0.0, 0.0], [128.0, 128.0], false);
    settings.view_projection = [0.0; 16];
    settings.view_projection[15] = f32::from_bits(1);
    known(
        &mesh,
        &settings,
        &(0..mesh.positions.len()).collect::<Vec<_>>(),
        "small-positive-w",
    );
    settings.view_projection[15] = f32::MAX;
    known(
        &mesh,
        &settings,
        &(0..mesh.positions.len()).collect::<Vec<_>>(),
        "huge-finite-w",
    );
}

#[test]
fn negative_viewport_origin_and_inclusive_coordinate_bound_are_supported() {
    let mesh = identity_mesh();
    let mut settings = rect([-100.0, -200.0], [28.0, -72.0], false);
    settings.viewport = [-100.0, -200.0, 128.0, 128.0];
    known(
        &mesh,
        &settings,
        &(0..9).collect::<Vec<_>>(),
        "negative-viewport-origin",
    );
    settings.viewport = [999_872.0, 999_872.0, 128.0, 128.0];
    settings.region = MaskRegion::Rectangle {
        start: [999_872.0, 999_872.0],
        end: [1_000_000.0, 1_000_000.0],
    };
    known(
        &mesh,
        &settings,
        &(0..9).collect::<Vec<_>>(),
        "inclusive-viewport-coordinate-bound",
    );
}

fn rejected_mesh(mesh: &Mesh, settings: &MaskRegionSettings, case: &str) {
    let before = bits(mesh);
    let result = mesh.region_masks(settings);
    assert!(result.is_err(), "{case} must reject");
    assert_eq!(bits(mesh), before);
    eprintln!(
        "mask_region_contract_rejection: {}",
        json!({"case":case,"error":result.err(),"source_bits_exact":true})
    );
}

#[test]
fn invalid_nonfinite_and_out_of_range_viewports_reject() {
    let mesh = identity_mesh();
    for viewport in [
        [0.0, 0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, -1.0, 1.0],
        [0.0, 0.0, 1.0, -1.0],
        [0.0, 0.0, 1_000_001.0, 1.0],
        [0.0, 0.0, 1.0, 1_000_001.0],
        [1_000_000.0, 0.0, 1.0, 1.0],
        [-1_000_001.0, 0.0, 1.0, 1.0],
        [0.0, 1_000_000.0, 1.0, 1.0],
        [0.0, 0.0, f64::INFINITY, 1.0],
        [f64::NAN, 0.0, 1.0, 1.0],
    ] {
        let mut settings = rect([0.0, 0.0], [128.0, 128.0], false);
        settings.viewport = viewport;
        rejected_mesh(&mesh, &settings, "invalid-viewport");
    }
}

#[test]
fn each_nonfinite_matrix_coefficient_is_rejected_even_for_empty_rectangle() {
    let mesh = identity_mesh();
    for index in 0..16 {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut settings = rect([0.0, 0.0], [0.0, 0.0], false);
            settings.view_projection[index] = value;
            rejected_mesh(&mesh, &settings, "nonfinite-matrix");
        }
    }
}

#[test]
fn invalid_shape_points_counts_and_collinearity_reject_without_shoelace_assumption() {
    let mesh = identity_mesh();
    for points in [
        vec![],
        vec![[0.0, 0.0]],
        vec![[0.0, 0.0], [1.0, 1.0]],
        vec![[1.0, 1.0]; 3],
        vec![[0.0, 0.0], [1.0, 1.0], [0.0, 0.0]],
        vec![[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]],
        vec![[0.0, 0.0], [1.0, 1.0], [f64::NAN, 2.0]],
        vec![[0.0, 0.0], [1.0, 1.0], [f64::INFINITY, 2.0]],
        vec![[0.0, 0.0], [1.0, 1.0], [1_000_001.0, 2.0]],
        (0..513)
            .map(|i| if i % 2 == 0 { [0.0, 0.0] } else { [1.0, 1.0] })
            .collect(),
    ] {
        rejected_mesh(&mesh, &lasso(points, false), "invalid-lasso");
    }
    for point in [
        [f64::NAN, 0.0],
        [0.0, f64::INFINITY],
        [1_000_001.0, 0.0],
        [-1_000_001.0, 0.0],
    ] {
        rejected_mesh(
            &mesh,
            &rect(point, [128.0, 128.0], false),
            "invalid-rectangle-point",
        );
    }
}

#[test]
fn invalid_mesh_channels_indices_and_empty_input_reject_even_when_selection_is_empty() {
    for mutation in 0..10 {
        let mut mesh = identity_mesh();
        match mutation {
            0 => mesh.positions[0].x = f32::NAN,
            1 => mesh.normals[0].x = f32::INFINITY,
            2 => mesh.masks[0] = f32::NAN,
            3 => mesh.masks[0] = 1.1,
            4 => mesh.colors[0][0] = -0.1,
            5 => mesh.triangles[0][2] = 999,
            6 => mesh.triangles[0] = [0, 0, 1],
            7 => mesh.triangles.clear(),
            8 => {
                mesh.masks.pop();
            }
            _ => {
                mesh.positions.clear();
            }
        }
        rejected_mesh(
            &mesh,
            &rect([64.0, 64.0], [64.0, 64.0], false),
            "invalid-source-empty-selection",
        );
    }
}

#[test]
fn geometric_degeneracy_duplicate_faces_and_coincident_ids_do_not_restrict_selection() {
    let mut mesh = fixture(&[
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [0.5, 0.0, 0.0],
    ]);
    mesh.triangles.extend([[0, 1, 2], [2, 1, 0]]);
    mesh.recompute_normals();
    known(
        &mesh,
        &rect([64.0, 64.0], [96.0, 96.0], false),
        &[0, 1, 2, 3],
        "degenerate-duplicate-coincident-source",
    );
}

#[test]
fn malformed_general_source_count_caps_reject_before_scratch() {
    let mut mesh = identity_mesh();
    mesh.positions
        .resize(obrush_core::MAX_VERTICES + 1, Vec3::ZERO);
    assert!(mesh
        .region_masks(&rect([0.0, 0.0], [128.0, 128.0], false))
        .is_err());
    assert_eq!(mesh.positions.len(), obrush_core::MAX_VERTICES + 1);
    let mut mesh = identity_mesh();
    mesh.triangles
        .resize(obrush_core::MAX_TRIANGLES + 1, [0, 1, 2]);
    assert!(mesh
        .region_masks(&rect([0.0, 0.0], [128.0, 128.0], false))
        .is_err());
    assert_eq!(mesh.triangles.len(), obrush_core::MAX_TRIANGLES + 1);
}

fn work_mesh(count: usize) -> Mesh {
    let mut mesh = fixture(&[[-0.75, -0.75, 0.0], [0.75, -0.75, 0.0], [0.0, 0.75, 0.0]]);
    mesh.positions.resize(count, Vec3::ZERO);
    mesh.normals.resize(count, Vec3::ZERO);
    mesh.masks.resize(count, 0.375);
    mesh.colors.resize(count, [0.2, 0.4, 0.6]);
    mesh
}

#[test]
fn lasso_work_boundary_is_accepted_and_cap_plus_one_rejects() {
    let cycle = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let points = (0..512).map(|i| cycle[i % 4]).collect();
    let settings = lasso(points, false);
    let mesh = work_mesh(62_500);
    let (masks, report) = mesh.region_masks(&settings).unwrap();
    assert_eq!(report.point_edge_work, 32_000_000);
    assert_eq!(report.projected_vertices, 62_500);
    assert_eq!(report.selected_vertices, 0);
    assert_eq!(report.changed_vertices, 0);
    assert_eq!(masks, mesh.masks);
    let mesh = work_mesh(62_501);
    assert!(mesh.region_masks(&settings).is_err());
    assert_eq!(mesh.positions.len(), 62_501);
    eprintln!(
        "mask_region_contract_observation: {}",
        json!({"case":"work-boundary-bbox-skip","vertices":62500,"polygon_points":512,"point_edge_work":32000000,"selected_vertices":0,"cap_plus_one_rejected":true,"not_a_full_edge_test_benchmark":true})
    );
}

fn with_redo(mesh: Mesh) -> Document {
    let mut document = Document::new(mesh);
    document.set_subtool_name(1, "Genuine prior redo").unwrap();
    assert!(document.undo());
    assert!(document.can_redo());
    document
}

#[test]
fn useful_rectangle_and_erase_have_one_exact_scene_history_step() {
    for erase in [false, true] {
        let mut mesh = identity_mesh();
        mesh.masks.fill(0.625);
        let mut document = with_redo(mesh);
        let before = state(&document);
        let geometry = nonmask(&document);
        let old = bits(document.active_mesh().unwrap());
        let settings = rect([64.0, 32.0], [96.0, 96.0], erase);
        let (expected, projected, count) = oracle(document.active_mesh().unwrap(), &settings);
        let report = document.mask_region(&settings).unwrap();
        assert!(report.changed_vertices > 0);
        assert!(!document.can_redo());
        assert_eq!(document.active_mesh().unwrap().masks, expected);
        assert_eq!(nonmask(&document), geometry);
        let updated = bits(document.active_mesh().unwrap());
        for key in ["positions", "normals", "colors", "triangles"] {
            assert_eq!(updated[key], old[key]);
        }
        report_check(
            &report,
            &old["masks"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| f32::from_bits(n.as_u64().unwrap() as u32))
                .collect::<Vec<_>>(),
            &document.active_mesh().unwrap().masks,
            &settings,
            projected,
            count,
        );
        let after = state(&document);
        assert_eq!(
            Document::from_json(&after).unwrap().to_json().unwrap(),
            after
        );
        assert!(document.undo());
        assert_eq!(state(&document), before);
        assert!(document.redo());
        assert_eq!(state(&document), after);
    }
}

#[test]
fn empty_and_already_matching_selection_preserve_genuine_redo_and_signed_zero() {
    for settings in [
        rect([16.0, 16.0], [16.0, 112.0], false),
        rect([-100.0, -100.0], [-80.0, -80.0], false),
        rect([0.0, 0.0], [128.0, 128.0], false),
        rect([0.0, 0.0], [128.0, 128.0], true),
    ] {
        let mut mesh = identity_mesh();
        mesh.masks.fill(if settings.erase { -0.0 } else { 1.0 });
        let mut document = with_redo(mesh);
        let before = state(&document);
        let old = bits(document.active_mesh().unwrap());
        let report = document.mask_region(&settings).unwrap();
        assert_eq!(report.changed_vertices, 0);
        assert_eq!(report.max_abs_change, 0.0);
        assert_eq!(state(&document), before);
        assert_eq!(bits(document.active_mesh().unwrap()), old);
        assert!(document.can_redo());
    }
}

#[test]
fn noops_after_coarse_sculpt_leave_unreconciled_baselines_and_redo_intact() {
    let mut document = Document::new(fixture(&[
        [-0.5, -0.5, 0.0],
        [0.5, -0.5, 0.0],
        [0.0, 0.5, 0.0],
    ]));
    document.add_subdivision_level().unwrap();
    document.add_subdivision_level().unwrap();
    document.select_level(0).unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[0] += Vec3::new(-0.007, 0.003, 0.002);
    document.active_mesh_mut().unwrap().masks.fill(1.0);
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    document.set_subtool_name(1, "Real prior redo").unwrap();
    let renamed = state(&document);
    assert!(document.undo());
    let before = state(&document);
    let old = bits(document.active_mesh().unwrap());
    for settings in [
        rect([0.0, 0.0], [128.0, 128.0], false),
        rect([64.0, 0.0], [64.0, 128.0], false),
    ] {
        let report = document.mask_region(&settings).unwrap();
        assert_eq!(report.changed_vertices, 0);
        assert_eq!(state(&document), before);
        assert_eq!(bits(document.active_mesh().unwrap()), old);
        assert!(document.can_redo());
    }
    assert!(document.redo());
    assert_eq!(state(&document), renamed);
}

fn rich() -> Document {
    let mut document = Document::new(Mesh::sphere(1));
    document.name = "Independent region source".into();
    document.add_subdivision_level().unwrap();
    document.store_morph_target().unwrap();
    document.select_level(0).unwrap();
    document.add_layer("Hidden precise offsets").unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[7] += Vec3::new(0.005, -0.003, 0.007);
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    document.set_layer_visibility(0, false).unwrap();
    document.add_layer("Faded precise offsets").unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[19] += Vec3::new(-0.002, 0.006, 0.004);
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    document.set_layer_intensity(1, 0.37).unwrap();
    document.add_layer("Recording offsets").unwrap();
    document.begin_stroke();
    for (i, m) in document
        .active_mesh_mut()
        .unwrap()
        .masks
        .iter_mut()
        .enumerate()
    {
        *m = 0.1 + (i % 13) as f32 / 16.0;
    }
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    assert!(document.take_layer_error().is_none());
    document
}

#[test]
fn rich_scene_lasso_keeps_layer_target_geometry_and_inactive_arc_exact() {
    let mut document = rich();
    let source = document.active_subtool_id().unwrap();
    let inactive = document
        .add_subtool(identity_mesh(), "Unrelated part")
        .unwrap();
    document.select_subtool(source).unwrap();
    let old_arc = document
        .subtool_render_snapshots()
        .into_iter()
        .find(|s| s.id == inactive)
        .unwrap()
        .mesh;
    let old = bits(document.active_mesh().unwrap());
    let geometry = nonmask(&document);
    let before = state(&document);
    let settings = lasso(
        vec![[64.0, 0.0], [128.0, 0.0], [128.0, 128.0], [64.0, 128.0]],
        false,
    );
    let (expected, _, _) = oracle(document.active_mesh().unwrap(), &settings);
    let report = document.mask_region(&settings).unwrap();
    assert!(report.changed_vertices > 0);
    assert_eq!(document.active_mesh().unwrap().masks, expected);
    assert_eq!(nonmask(&document), geometry);
    let new = bits(document.active_mesh().unwrap());
    for key in ["positions", "normals", "colors", "triangles"] {
        assert_eq!(new[key], old[key]);
    }
    let now = document.subtool_render_snapshots();
    assert!(Arc::ptr_eq(
        &old_arc,
        &now.iter().find(|s| s.id == inactive).unwrap().mesh
    ));
    assert_eq!(
        (
            document.level_count(),
            document.current_level(),
            document.layer_count()
        ),
        (2, 0, 3)
    );
    assert_eq!(document.morph_target_summary().unwrap().creation_level, 1);
    assert!(!document.morph_target_summary().unwrap().available);
    let after = state(&document);
    assert_eq!(
        Document::from_json(&after).unwrap().to_json().unwrap(),
        after
    );
    assert!(document.undo());
    assert_eq!(state(&document), before);
    assert!(document.redo());
    assert_eq!(state(&document), after);
}

#[test]
fn retained_masks_follow_existing_f32_loop_transport_and_preserve_fine_residual() {
    let mut mesh = fixture(&[[-0.5, -0.5, 0.0], [0.5, -0.5, 0.0], [0.0, 0.5, 0.0]]);
    mesh.masks = vec![0.2, 0.4, 0.8];
    let mut document = Document::new(mesh);
    document.add_subdivision_level().unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().masks[3] = 0.9;
    document.end_stroke();
    let old_child = document.active_mesh().unwrap().masks.clone();
    document.select_level(0).unwrap();
    let geometry = nonmask(&document);
    document
        .mask_region(&rect([0.0, 0.0], [128.0, 128.0], false))
        .unwrap();
    assert_eq!(document.active_mesh().unwrap().masks, vec![1.0; 3]);
    assert_eq!(nonmask(&document), geometry);
    document.select_level(1).unwrap();
    let predict = |m: [f32; 3]| {
        [
            (m[0] * 0.75 + m[1] * 0.125) + m[2] * 0.125,
            (m[1] * 0.75 + m[0] * 0.125) + m[2] * 0.125,
            (m[2] * 0.75 + m[1] * 0.125) + m[0] * 0.125,
            (m[0] + m[1]) * 0.5,
            (m[1] + m[2]) * 0.5,
            (m[0] + m[2]) * 0.5,
        ]
    };
    let old = predict([0.2, 0.4, 0.8]);
    let new = predict([1.0; 3]);
    let expected = old_child
        .iter()
        .enumerate()
        .map(|(i, &m)| {
            if old[i] == new[i] {
                m
            } else {
                (m + new[i] - old[i]).clamp(0.0, 1.0)
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(
        document
            .active_mesh()
            .unwrap()
            .masks
            .iter()
            .map(|m| m.to_bits())
            .collect::<Vec<_>>(),
        expected.iter().map(|m| m.to_bits()).collect::<Vec<_>>()
    );
    assert_eq!(document.active_mesh().unwrap().masks[3], 1.0);
}

fn rejected_document(mut document: Document, settings: &MaskRegionSettings, case: &str) {
    let before = state(&document);
    let old = bits(document.active_mesh().unwrap());
    let redo = document.can_redo();
    let result = document.mask_region(settings);
    assert!(result.is_err(), "{case}");
    assert_eq!(state(&document), before);
    assert_eq!(bits(document.active_mesh().unwrap()), old);
    assert_eq!(document.can_redo(), redo);
}

#[test]
fn invalid_settings_and_hidden_active_source_preserve_scene_and_redo() {
    let mut settings = rect([0.0, 0.0], [128.0, 128.0], false);
    settings.viewport[2] = 0.0;
    rejected_document(
        with_redo(identity_mesh()),
        &settings,
        "invalid-settings-document",
    );
    let mut document = Document::new(identity_mesh());
    document.set_subtool_visible(1, false).unwrap();
    document.set_subtool_name(1, "Prior redo").unwrap();
    assert!(document.undo());
    rejected_document(
        document,
        &rect([64.0, 64.0], [64.0, 64.0], false),
        "hidden-empty-selection",
    );
}

#[test]
fn pending_invalid_stroke_restores_complete_scene_before_even_empty_selection() {
    let mut document = rich();
    document.set_subtool_name(1, "Real prior redo").unwrap();
    assert!(document.undo());
    let before = state(&document);
    let old = bits(document.active_mesh().unwrap());
    document.begin_stroke();
    document.active_mesh_mut().unwrap().normals[0].x = f32::NAN;
    assert!(document
        .mask_region(&rect([64.0, 64.0], [64.0, 64.0], false))
        .is_err());
    assert_eq!(state(&document), before);
    assert_eq!(bits(document.active_mesh().unwrap()), old);
    assert!(document.can_redo());
    assert!(document.take_layer_error().is_some());
}

#[test]
fn pending_valid_stroke_finishes_before_separate_region_history_step() {
    let mut document = Document::new(identity_mesh());
    let initial = state(&document);
    document.begin_stroke();
    document.active_mesh_mut().unwrap().masks[0] = 0.73;
    let stroke = state(&document);
    let report = document
        .mask_region(&rect([0.0, 0.0], [128.0, 128.0], false))
        .unwrap();
    assert!(report.changed_vertices > 0);
    let after = state(&document);
    assert!(document.undo());
    assert_eq!(state(&document), stroke);
    assert!(document.undo());
    assert_eq!(state(&document), initial);
    assert!(document.redo());
    assert_eq!(state(&document), stroke);
    assert!(document.redo());
    assert_eq!(state(&document), after);
}

#[test]
fn invalid_public_source_without_open_stroke_rejects_without_destroying_redo() {
    let mut document = with_redo(identity_mesh());
    document.active_mesh_mut().unwrap().masks[2] = f32::NAN;
    let old = bits(document.active_mesh().unwrap());
    assert!(document
        .mask_region(&rect([64.0, 64.0], [64.0, 64.0], false))
        .is_err());
    assert_eq!(bits(document.active_mesh().unwrap()), old);
    assert!(document.can_redo());
}

#[test]
fn region_report_and_settings_roundtrip_without_a_new_project_schema() {
    let mesh = identity_mesh();
    let settings = lasso(
        vec![[16.0, 16.0], [112.0, 16.0], [112.0, 112.0], [16.0, 112.0]],
        false,
    );
    let (_, report) = selected(&mesh, &settings, "report-roundtrip");
    let encoded = serde_json::to_string(&report).unwrap();
    let decoded: MaskRegionReport = serde_json::from_str(&encoded).unwrap();
    assert_eq!(serde_json::to_string(&decoded).unwrap(), encoded);
    let encoded = serde_json::to_string(&settings).unwrap();
    let decoded: MaskRegionSettings = serde_json::from_str(&encoded).unwrap();
    assert_eq!(serde_json::to_string(&decoded).unwrap(), encoded);
}

#[test]
fn tiny_finite_noncollinear_lasso_is_valid_despite_squared_cross_underflow() {
    let mesh = fixture(&[[-1.0, -1.0, 0.0], [1.0, -1.0, 0.0], [-1.0, 1.0, 0.0]]);
    let mut settings = lasso(vec![[0.0, 0.0], [1e-200, 0.0], [0.0, 1e-200]], false);
    settings.viewport = [0.0, 0.0, 2e-200, 2e-200];
    known(&mesh, &settings, &[0, 1, 2], "tiny-noncollinear-lasso");
}

#[test]
fn tiny_and_mixed_scale_truly_collinear_lassos_remain_invalid() {
    let mesh = identity_mesh();
    for points in [
        vec![[0.0, 0.0], [1e-200, 1e-200], [2e-200, 2e-200]],
        vec![[0.0, 0.0], [1_000_000.0, 1e-200], [500_000.0, 0.5e-200]],
    ] {
        rejected_mesh(
            &mesh,
            &lasso(points, false),
            "tiny-mixed-collinear-rejection",
        );
    }
}

#[test]
fn mixed_scale_noncollinear_lasso_keeps_boundary_and_excludes_distant_point() {
    let mesh = fixture(&[[-1.0, 1.0, 0.0], [0.0, 0.0, 0.0], [1.0, 1.0, 0.0]]);
    let mut settings = lasso(vec![[0.0, 0.0], [1_000_000.0, 0.0], [0.0, 1e-200]], false);
    settings.viewport = [0.0, 0.0, 2.0, 2.0];
    known(&mesh, &settings, &[0, 2], "mixed-scale-noncollinear-lasso");
}
