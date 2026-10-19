use obrush_core::{
    Mesh, MeshFileFormat, PlyEncoding, StlEncoding, Vec3, MAX_TRIANGLES, MAX_VERTICES,
};

const ASCII_STL: &str = "solid independent\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 2 0 0\nvertex 0 3 0\nendloop\nendfacet\nendsolid independent\n";
const BASE_HEADER: &str = "ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar uint vertex_indices\nend_header\n";
const BASE_ROWS: &str = "0 0 0\n2 0 0\n0 3 0\n3 0 1 2\n";
fn basic_ply() -> Vec<u8> {
    format!("{BASE_HEADER}{BASE_ROWS}").into_bytes()
}
fn simple_mesh() -> Mesh {
    Mesh::from_obj("v 0 0 0\nv 2 0 0\nv 0 3 0\nf 1 2 3\n").unwrap()
}
fn stl_binary(points: &[[f32; 3]], attributes: &[u16], magic: Option<[u8; 4]>) -> Vec<u8> {
    assert_eq!(points.len(), attributes.len() * 3);
    let mut bytes = vec![0_u8; 80];
    if let Some(rgb) = magic {
        bytes[..6].copy_from_slice(b"COLOR=");
        bytes[6..10].copy_from_slice(&rgb);
    }
    bytes.extend_from_slice(&(attributes.len() as u32).to_le_bytes());
    for (face, &attribute) in attributes.iter().enumerate() {
        for value in [0_f32, 0_f32, 1_f32] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for point in &points[face * 3..face * 3 + 3] {
            for value in point {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        bytes.extend_from_slice(&attribute.to_le_bytes());
    }
    bytes
}
fn triangle_points() -> [[f32; 3]; 3] {
    [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 3.0, 0.0]]
}
fn binary_ply(big: bool, unknown: bool) -> Vec<u8> {
    let endian = if big { "big" } else { "little" };
    let extra = if unknown {
        "property list uchar short ids\n"
    } else {
        ""
    };
    let mut bytes = format!("ply\nformat binary_{endian}_endian 1.0\nelement vertex 3\nproperty double z\nproperty uchar blue\nproperty short x\nproperty float nx\nproperty uchar red\nproperty float ny\nproperty uchar green\nproperty double y\nproperty float nz\n{extra}element face 1\nproperty list ushort int vertex_index\nend_header\n").into_bytes();
    for (index, p) in triangle_points().iter().enumerate() {
        macro_rules! push {
            ($value:expr) => {{
                let value = $value;
                bytes.extend_from_slice(&if big {
                    value.to_be_bytes()
                } else {
                    value.to_le_bytes()
                });
            }};
        }
        push!(f64::from(p[2]));
        bytes.push((index * 50) as u8);
        push!(p[0] as i16);
        push!(0.25_f32);
        bytes.push(255);
        push!(-0.5_f32);
        bytes.push(128);
        push!(f64::from(p[1]));
        push!(0.75_f32);
        if unknown {
            bytes.push(2);
            push!(-3_i16);
            push!(7_i16);
        }
    }
    bytes.extend_from_slice(&if big {
        3_u16.to_be_bytes()
    } else {
        3_u16.to_le_bytes()
    });
    for index in [0_i32, 1, 2] {
        bytes.extend_from_slice(&if big {
            index.to_be_bytes()
        } else {
            index.to_le_bytes()
        });
    }
    bytes
}
fn exact_fields(a: &Mesh, b: &Mesh) {
    assert_eq!(
        a.positions
            .iter()
            .flat_map(|v| v.to_array().map(f32::to_bits))
            .collect::<Vec<_>>(),
        b.positions
            .iter()
            .flat_map(|v| v.to_array().map(f32::to_bits))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        a.normals
            .iter()
            .flat_map(|v| v.to_array().map(f32::to_bits))
            .collect::<Vec<_>>(),
        b.normals
            .iter()
            .flat_map(|v| v.to_array().map(f32::to_bits))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        a.colors
            .iter()
            .flatten()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>(),
        b.colors
            .iter()
            .flatten()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>()
    );
    assert_eq!(a.triangles, b.triangles);
}
#[test]
fn ascii_stl_independent_exact_positions_and_winding() {
    let parsed = Mesh::import_stl(ASCII_STL.as_bytes()).unwrap();
    assert_eq!(
        parsed.mesh.positions,
        triangle_points().map(Vec3::from_array)
    );
    assert_eq!(parsed.mesh.triangles, [[0, 1, 2]]);
    assert_eq!(parsed.mesh.normals, [Vec3::Z; 3]);
    assert_eq!(parsed.report.format, MeshFileFormat::Stl);
    assert_eq!(parsed.report.encoding, "ascii");
    assert!(!parsed.report.imported_normals);
    assert!(!parsed.report.imported_colors);
}
#[test]
fn binary_stl_independent_exact_positions_and_winding() {
    let parsed = Mesh::import_stl(&stl_binary(&triangle_points(), &[0], None)).unwrap();
    assert_eq!(
        parsed.mesh.positions,
        triangle_points().map(Vec3::from_array)
    );
    assert_eq!(parsed.mesh.triangles, [[0, 1, 2]]);
    assert_eq!(parsed.report.encoding, "binary_little_endian");
}
#[test]
fn binary_stl_solid_header_is_not_misclassified() {
    let mut bytes = stl_binary(&triangle_points(), &[0], None);
    bytes[..5].copy_from_slice(b"solid");
    assert_eq!(
        Mesh::import_stl(&bytes).unwrap().report.encoding,
        "binary_little_endian"
    );
}
#[test]
fn stl_exact_welding_retains_shared_topology() {
    let points = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -0.0, 0.0],
        [0.0, 1.0, 0.0],
        [-1.0, 0.0, 0.0],
    ];
    let parsed = Mesh::import_stl(&stl_binary(&points, &[0, 0], None)).unwrap();
    assert_eq!(parsed.mesh.positions.len(), 4);
    assert_eq!(parsed.mesh.triangles, [[0, 1, 2], [0, 2, 3]]);
    assert_eq!(parsed.report.welded_vertices, 2);
}
#[test]
fn stl_viscam_rgb_is_explicit_and_exact() {
    let parsed = Mesh::import_stl(&stl_binary(
        &triangle_points(),
        &[0x8000 | (31 << 10) | (15 << 5) | 7],
        None,
    ))
    .unwrap();
    assert_eq!(parsed.mesh.colors, [[1.0, 15.0 / 31.0, 7.0 / 31.0]; 3]);
    assert_eq!(parsed.report.encoding, "binary_little_endian_viscam");
    assert!(parsed.report.imported_colors);
}
#[test]
fn stl_magics_facet_rgb_order_is_explicit() {
    let parsed = Mesh::import_stl(&stl_binary(
        &triangle_points(),
        &[(31) | (9 << 5) | (2 << 10)],
        Some([1, 2, 3, 255]),
    ))
    .unwrap();
    assert_eq!(parsed.mesh.colors, [[1.0, 9.0 / 31.0, 2.0 / 31.0]; 3]);
    assert_eq!(parsed.report.encoding, "binary_little_endian_magics");
}
#[test]
fn stl_magics_default_header_color_and_alpha_loss_report() {
    let parsed = Mesh::import_stl(&stl_binary(
        &triangle_points(),
        &[0x8000],
        Some([25, 127, 255, 12]),
    ))
    .unwrap();
    assert_eq!(parsed.mesh.colors, [[25.0 / 255.0, 127.0 / 255.0, 1.0]; 3]);
    assert!(parsed.report.warnings.iter().any(|w| w.contains("alpha")));
}
#[test]
fn stl_conflicting_facet_colors_create_explicit_seams() {
    let points = [triangle_points(), triangle_points()].concat();
    let parsed = Mesh::import_stl(&stl_binary(&points, &[0xFC00, 0x801F], None)).unwrap();
    assert_eq!(parsed.mesh.positions.len(), 6);
    assert_eq!(parsed.mesh.colors[..3], [[1.0, 0.0, 0.0]; 3]);
    assert_eq!(parsed.mesh.colors[3..], [[0.0, 0.0, 1.0]; 3]);
    assert!(parsed
        .report
        .warnings
        .iter()
        .any(|w| w.contains("separate vertices")));
}
#[test]
fn stl_unknown_attribute_words_are_reported() {
    let parsed = Mesh::import_stl(&stl_binary(&triangle_points(), &[23], None)).unwrap();
    assert!(parsed
        .report
        .warnings
        .iter()
        .any(|w| w.contains("1 non-color")));
    assert!(!parsed.report.imported_colors);
}
#[test]
fn stl_binary_export_has_independent_layout_and_geometric_normal() {
    let mesh = simple_mesh();
    let data = mesh.to_stl(StlEncoding::BinaryUncolored).unwrap();
    assert_eq!(data.len(), 134);
    assert_eq!(&data[80..84], &1_u32.to_le_bytes());
    assert_eq!(
        &data[84..96],
        &[
            0_f32.to_le_bytes(),
            0_f32.to_le_bytes(),
            1_f32.to_le_bytes()
        ]
        .concat()
    );
    assert_eq!(&data[132..134], &[0, 0]);
    assert_eq!(
        Mesh::import_stl(&data).unwrap().mesh.positions,
        mesh.positions
    );
}
#[test]
fn stl_color_export_averages_then_quantizes_to_five_bits() {
    let mut mesh = simple_mesh();
    mesh.colors = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]].to_vec();
    let data = mesh.to_stl(StlEncoding::Binary).unwrap();
    assert_eq!(
        u16::from_le_bytes(data[132..134].try_into().unwrap()),
        0x8000 | (10 << 10) | (10 << 5) | 10
    );
    assert_eq!(
        Mesh::import_stl(&data).unwrap().mesh.colors,
        [[10.0 / 31.0; 3]; 3]
    );
}
#[test]
fn sculptgl_stl_export_has_blank_header_and_exact_colored_facet() {
    let mut mesh = simple_mesh();
    mesh.colors = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]].to_vec();
    let before = mesh.clone();
    let bytes = mesh.to_stl_sculptgl().unwrap();
    assert_eq!(bytes.len(), 134);
    assert!(bytes[..80].iter().all(|&byte| byte == 0));
    assert_eq!(&bytes[80..84], &1_u32.to_le_bytes());
    let mut expected_facet = stl_binary(
        &triangle_points(),
        &[0x8000 | (10 << 10) | (10 << 5) | 10],
        None,
    );
    // The source float32 cross product retains negative zero on this axis.
    expected_facet[88..92].copy_from_slice(&(-0.0_f32).to_le_bytes());
    assert_eq!(&bytes[84..], &expected_facet[84..]);
    assert_eq!(mesh.positions, before.positions);
    assert_eq!(mesh.colors, before.colors);
}
#[test]
fn stl_ascii_export_round_trip_retains_geometry() {
    let mesh = simple_mesh();
    let bytes = mesh.to_stl(StlEncoding::Ascii).unwrap();
    let parsed = Mesh::import_stl(&bytes).unwrap();
    assert_eq!(mesh.positions, parsed.mesh.positions);
    assert_eq!(mesh.triangles, parsed.mesh.triangles);
}
#[test]
fn stl_malformed_ascii_fails_for_each_grammar_boundary() {
    for bad in [
        ASCII_STL.replace("outer loop", "wrong loop"),
        ASCII_STL.replace("endloop", ""),
        ASCII_STL.replace("endfacet", ""),
        ASCII_STL.replace("endsolid independent", "endsolid other"),
        format!("{ASCII_STL}garbage\n"),
        ASCII_STL.replace("vertex 2 0 0", "vertex 2 0 0 9"),
        ASCII_STL.replace("endsolid independent", ""),
    ] {
        assert!(Mesh::import_stl(bad.as_bytes()).is_err(), "{bad}");
    }
}
#[test]
fn stl_truncation_trailing_bytes_hostile_counts_and_repeated_vertices_fail() {
    let valid = stl_binary(&triangle_points(), &[0], None);
    for length in [0, 1, 79, 80, 83, 84, 95, 133] {
        assert!(Mesh::import_stl(&valid[..length]).is_err());
    }
    let mut bad = valid.clone();
    bad.push(0);
    assert!(Mesh::import_stl(&bad).is_err());
    for count in [0, MAX_TRIANGLES as u32 + 1, u32::MAX] {
        let mut bad = valid.clone();
        bad[80..84].copy_from_slice(&count.to_le_bytes());
        assert!(Mesh::import_stl(&bad).is_err());
    }
    let bad = stl_binary(&[[0.0; 3], [0.0; 3], [1.0; 3]], &[0], None);
    assert!(Mesh::import_stl(&bad).is_err());
}
#[test]
fn stl_nonfinite_normals_coordinates_and_excessive_positions_fail() {
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1_000_001.0] {
        let mut points = triangle_points();
        points[0][0] = value;
        assert!(Mesh::import_stl(&stl_binary(&points, &[0], None)).is_err());
    }
    for value in [f32::NAN, f32::INFINITY, f32::MAX] {
        let mut bytes = stl_binary(&triangle_points(), &[0], None);
        bytes[84..88].copy_from_slice(&value.to_le_bytes());
        assert!(Mesh::import_stl(&bytes).is_err());
    }
}
#[test]
fn ply_ascii_independent_exact_geometry() {
    let parsed = Mesh::import_ply(&basic_ply()).unwrap();
    assert_eq!(
        parsed.mesh.positions,
        triangle_points().map(Vec3::from_array)
    );
    assert_eq!(parsed.mesh.triangles, [[0, 1, 2]]);
    assert_eq!(parsed.report.format, MeshFileFormat::Ply);
    assert_eq!(parsed.report.encoding, "ascii");
}
#[test]
fn ply_binary_little_endian_mixed_property_order_exact_attributes() {
    check_mixed_binary(false, false);
}
#[test]
fn ply_binary_big_endian_mixed_property_order_exact_attributes() {
    check_mixed_binary(true, false);
}
#[test]
fn ply_binary_unknown_lists_are_consumed_and_reported() {
    check_mixed_binary(false, true);
    check_mixed_binary(true, true);
}
fn check_mixed_binary(big: bool, unknown: bool) {
    let parsed = Mesh::import_ply(&binary_ply(big, unknown)).unwrap();
    assert_eq!(
        parsed.mesh.positions,
        triangle_points().map(Vec3::from_array)
    );
    assert_eq!(parsed.mesh.normals, [Vec3::new(0.25, -0.5, 0.75); 3]);
    for (index, color) in parsed.mesh.colors.iter().enumerate() {
        assert_eq!(*color, [1.0, 128.0 / 255.0, (index * 50) as f32 / 255.0]);
    }
    assert!(parsed.report.imported_normals);
    assert!(parsed.report.imported_colors);
    assert_eq!(
        parsed.report.skipped_properties,
        if unknown { vec!["vertex.ids"] } else { vec![] }
    );
}
#[test]
fn ply_ascii_unknown_elements_scalar_and_list_fields_reported() {
    let text="ply\nformat ascii 1.0\nelement edges 1\nproperty int id\nproperty list uchar int peers\nelement vertex 3\nproperty uchar quality\nproperty float z\nproperty float x\nproperty float y\nelement face 1\nproperty int material\nproperty list uchar uint vertex_index\nend_header\n7 2 -4 9\n3 0 0 0\n3 0 2 0\n3 0 0 3\n8 3 0 1 2\n";
    let parsed = Mesh::import_ply(text.as_bytes()).unwrap();
    assert_eq!(
        parsed.mesh.positions,
        triangle_points().map(Vec3::from_array)
    );
    assert_eq!(
        parsed.report.skipped_properties,
        ["edges.id", "edges.peers", "vertex.quality", "face.material"]
    );
}
#[test]
fn ply_faces_can_precede_vertices() {
    let text="ply\nformat ascii 1.0\nelement face 1\nproperty list uchar uint vertex_index\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nend_header\n3 0 1 2\n0 0 0\n2 0 0\n0 3 0\n";
    assert_eq!(
        Mesh::import_ply(text.as_bytes()).unwrap().mesh.triangles,
        [[0, 1, 2]]
    );
}
#[test]
fn ply_quad_and_concave_polygon_preserve_area_winding_and_counts() {
    let text="ply\nformat ascii 1.0\nelement vertex 5\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar uint vertex_indices\nend_header\n0 0 0\n2 0 0\n2 2 0\n1 1 0\n0 2 0\n5 0 1 2 3 4\n";
    let parsed = Mesh::import_ply(text.as_bytes()).unwrap();
    assert_eq!(parsed.mesh.triangles.len(), 3);
    assert_eq!(parsed.report.triangulated_faces, 1);
    let areas = parsed
        .mesh
        .triangles
        .iter()
        .map(|t| {
            let a = parsed.mesh.positions[t[0] as usize];
            let b = parsed.mesh.positions[t[1] as usize];
            let c = parsed.mesh.positions[t[2] as usize];
            (b - a).cross(c - a).z * 0.5
        })
        .collect::<Vec<_>>();
    assert!(areas.iter().all(|&a| a > 0.0));
    assert_eq!(areas.iter().sum::<f32>(), 3.0);
    let quad = text
        .replace("element vertex 5", "element vertex 4")
        .replace("1 1 0\n", "")
        .replace("5 0 1 2 3 4", "4 0 1 2 3");
    let imported_quad = Mesh::import_ply(quad.as_bytes()).unwrap();
    assert_eq!(imported_quad.mesh.triangles.len(), 2);
    assert_eq!(
        imported_quad.mesh.surface.as_ref().unwrap().faces,
        vec![[0, 1, 2, 3]]
    );
}
#[test]
fn ply_unsuitable_quad_diagonal_is_triangulated_and_reported() {
    let text="ply\nformat ascii 1.0\nelement vertex 4\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar uint vertex_indices\nend_header\n0 0 0\n2 0 0\n2 2 0\n1 0.5 0\n4 0 1 2 3\n";
    let parsed = Mesh::import_ply(text.as_bytes()).unwrap();
    assert_eq!(parsed.mesh.triangles.len(), 2);
    assert_eq!(parsed.mesh.surface.as_ref().unwrap().faces.len(), 2);
    assert!(parsed
        .mesh
        .surface
        .as_ref()
        .unwrap()
        .faces
        .iter()
        .all(|face| face[3] == u32::MAX));
    assert!(parsed
        .report
        .warnings
        .iter()
        .any(|warning| warning.contains("1 quads whose 0–2 diagonal is unsuitable")));
}
#[test]
fn ply_nonplanar_self_intersecting_repeated_and_out_of_range_faces_fail() {
    let square="ply\nformat ascii 1.0\nelement vertex 4\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar uint vertex_indices\nend_header\n0 0 0\n1 0 0\n1 1 0\n0 1 0\n4 0 1 2 3\n";
    for bad in [
        square.replace("1 1 0", "1 1 0.5"),
        square.replace("4 0 1 2 3", "4 0 2 1 3"),
        square.replace("4 0 1 2 3", "4 0 1 2 2"),
        square.replace("4 0 1 2 3", "4 0 1 2 4"),
        square.replace("4 0 1 2 3", "2 0 1"),
    ] {
        assert!(Mesh::import_ply(bad.as_bytes()).is_err(), "{bad}");
    }
}
#[test]
fn ply_all_export_encodings_preserve_exact_float32_fields() {
    let mut mesh = simple_mesh();
    mesh.positions[0].x = -0.0;
    mesh.normals = [Vec3::new(-0.0, 0.5, 0.75); 3].to_vec();
    mesh.colors = [[0.1, 0.2, 0.3], [0.0, 1.0, 0.33333334], [0.7, 0.9, -0.0]].to_vec();
    mesh.masks.fill(1.0);
    for encoding in [
        PlyEncoding::Ascii,
        PlyEncoding::BinaryLittleEndian,
        PlyEncoding::BinaryBigEndian,
    ] {
        let bytes = mesh.to_ply(encoding).unwrap();
        let parsed = Mesh::import_ply(&bytes).unwrap();
        exact_fields(&mesh, &parsed.mesh);
        assert_eq!(parsed.mesh.masks, [0.0; 3]);
    }
}
#[test]
fn ply_ascii_integer_rgb_and_float_rgb_have_distinct_explicit_ranges() {
    for (kind, values, color) in [
        ("uchar", "255 128 0", [1.0, 128.0 / 255.0, 0.0]),
        ("float", "1 0.5 0.25", [1.0, 0.5, 0.25]),
    ] {
        let header = BASE_HEADER.replace(
            "element face 1",
            &format!(
                "property {kind} red\nproperty {kind} green\nproperty {kind} blue\nelement face 1"
            ),
        );
        let rows = format!("0 0 0 {values}\n2 0 0 {values}\n0 3 0 {values}\n3 0 1 2\n");
        let parsed = Mesh::import_ply(format!("{header}{rows}").as_bytes()).unwrap();
        assert_eq!(parsed.mesh.colors, [color; 3]);
    }
}
#[test]
fn ply_incomplete_normals_colors_and_list_position_fields_fail() {
    for property in [
        "property float nx\n",
        "property uchar red\n",
        "property list uchar float x\n",
    ] {
        let header = BASE_HEADER.replace("element face 1", &format!("{property}element face 1"));
        assert!(Mesh::import_ply(format!("{header}{BASE_ROWS}").as_bytes()).is_err());
    }
}
#[test]
fn ply_malformed_headers_and_hostile_counts_fail_before_body_decode() {
    for bad in [
        BASE_HEADER.replace("ply\n", "not_ply\n"),
        BASE_HEADER.replace("ascii 1.0", "ascii 2.0"),
        BASE_HEADER.replace("ascii 1.0", "unknown 1.0"),
        BASE_HEADER.replace(
            "element vertex 3",
            &format!("element vertex {}", MAX_VERTICES + 1),
        ),
        BASE_HEADER.replace(
            "element face 1",
            &format!("element face {}", MAX_TRIANGLES + 1),
        ),
        BASE_HEADER.replace("element vertex 3", "element vertex -1"),
        BASE_HEADER.replace("property float x", "property float x\nproperty float x"),
        BASE_HEADER.replace("property float x", "property strange x"),
        BASE_HEADER.replace("list uchar uint", "list float uint"),
        BASE_HEADER.replace("end_header", "oops"),
    ] {
        assert!(
            Mesh::import_ply(format!("{bad}{BASE_ROWS}").as_bytes()).is_err(),
            "{bad}"
        );
    }
}
#[test]
fn ply_ascii_row_boundaries_prevent_shifted_or_extra_properties() {
    for rows in [
        "0 0\n0 2 0 0\n0 3 0\n3 0 1 2\n",
        "0 0 0 9\n2 0 0\n0 3 0\n3 0 1 2\n",
        "0 0 0\n2 0 0\n0 3 0\n3 0 1 2\n9\n",
    ] {
        assert!(Mesh::import_ply(format!("{BASE_HEADER}{rows}").as_bytes()).is_err());
    }
}
#[test]
fn ply_binary_truncation_and_undeclared_bytes_fail() {
    let data = binary_ply(false, false);
    for cut in 1..=15 {
        assert!(Mesh::import_ply(&data[..data.len() - cut]).is_err());
    }
    let mut bad = data;
    bad.push(1);
    assert!(Mesh::import_ply(&bad).is_err());
}
#[test]
fn ply_source_inferred_sculptgl_zero_padding_is_narrow_and_reported() {
    let mut data = binary_ply(false, false);
    data.splice(4..4, b"comment created by SculptGL\n".iter().copied());
    data.extend_from_slice(&[0; 14]);
    let parsed = Mesh::import_ply(&data).unwrap();
    assert!(parsed
        .report
        .warnings
        .iter()
        .any(|w| w.contains("14 zero bytes")));
    let mut bad = data.clone();
    *bad.last_mut().unwrap() = 1;
    assert!(Mesh::import_ply(&bad).is_err());
    let bad = data[..data.len() - 1].to_vec();
    assert!(Mesh::import_ply(&bad).is_err());
    let mut no_comment = binary_ply(false, false);
    no_comment.extend_from_slice(&[0; 14]);
    assert!(Mesh::import_ply(&no_comment).is_err());
}
#[test]
fn ply_sculptgl_curved_quad_uses_reference_padding_and_stays_bounded() {
    let header = b"ply\nformat binary_little_endian 1.0\ncomment created by SculptGL\nelement vertex 4\nproperty float x\nproperty float y\nproperty float z\nproperty uchar red\nproperty uchar green\nproperty uchar blue\nelement face 1\nproperty list uchar uint vertex_indices\nend_header\n";
    let mut data = header.to_vec();
    for point in [
        [0.0_f32, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.2],
        [0.0, 1.0, 0.0],
    ] {
        for component in point {
            data.extend_from_slice(&component.to_le_bytes());
        }
        data.extend_from_slice(&[64, 128, 255]);
    }
    data.push(4);
    for index in 0_u32..4 {
        data.extend_from_slice(&index.to_le_bytes());
    }
    data.extend_from_slice(&[0; 65]); // 17 face bytes + 48 reserved quad bytes.
    let imported = Mesh::import_ply(&data).unwrap();
    assert_eq!(imported.report.source_faces, 1);
    assert_eq!(imported.mesh.triangles.len(), 2);
    assert_eq!(
        imported.mesh.surface.as_ref().unwrap().faces,
        vec![[0, 1, 2, 3]]
    );
    assert_eq!(imported.mesh.colors[0], [64.0 / 255.0, 128.0 / 255.0, 1.0]);
    assert!(imported
        .report
        .warnings
        .iter()
        .any(|w| w.contains("65 zero bytes")));
    let mut bad = data.clone();
    *bad.last_mut().unwrap() = 1;
    assert!(Mesh::import_ply(&bad).is_err());
    assert!(Mesh::import_ply(&data[..data.len() - 1]).is_err());
    let without_comment = [
        b"ply\nformat binary_little_endian 1.0\n".as_slice(),
        &data[b"ply\nformat binary_little_endian 1.0\ncomment created by SculptGL\n".len()..],
    ]
    .concat();
    assert!(Mesh::import_ply(&without_comment).is_err());
}
#[test]
fn ply_nonfinite_scalars_and_excessive_coordinates_fail() {
    for rows in [
        BASE_ROWS.replace("2 0 0", "NaN 0 0"),
        BASE_ROWS.replace("2 0 0", "inf 0 0"),
        BASE_ROWS.replace("2 0 0", "1000001 0 0"),
    ] {
        assert!(Mesh::import_ply(format!("{BASE_HEADER}{rows}").as_bytes()).is_err());
    }
    let mut data = binary_ply(false, false);
    let body = data.windows(11).position(|w| w == b"end_header\n").unwrap() + 11;
    data[body..body + 8].copy_from_slice(&f64::NAN.to_le_bytes());
    assert!(Mesh::import_ply(&data).is_err());
}
#[test]
fn ply_unknown_nonfinite_values_are_rejected_not_silently_skipped() {
    let header = BASE_HEADER.replace("element face 1", "property float ignored\nelement face 1");
    let rows = "0 0 0 NaN\n2 0 0 0\n0 3 0 0\n3 0 1 2\n";
    assert!(Mesh::import_ply(format!("{header}{rows}").as_bytes()).is_err());
}
#[test]
fn ply_negative_float_and_large_list_indices_fail() {
    for rows in [
        BASE_ROWS.replace("3 0 1 2", "3 -1 1 2"),
        BASE_ROWS.replace("3 0 1 2", "3 0 1 4294967295"),
        BASE_ROWS.replace("3 0 1 2", "513 0 1 2"),
    ] {
        assert!(Mesh::import_ply(format!("{BASE_HEADER}{rows}").as_bytes()).is_err());
    }
    let header = BASE_HEADER.replace("list uchar uint", "list uchar float");
    assert!(Mesh::import_ply(format!("{header}{BASE_ROWS}").as_bytes()).is_err());
}
#[test]
fn ply_header_size_and_property_count_are_bounded() {
    let long = format!("ply\ncomment {}\n{BASE_HEADER}", "x".repeat(65536));
    assert!(Mesh::import_ply(long.as_bytes()).is_err());
    let props = (0..65)
        .map(|i| format!("property float p{i}\n"))
        .collect::<String>();
    let header = BASE_HEADER.replace("element face 1", &format!("{props}element face 1"));
    assert!(Mesh::import_ply(format!("{header}{BASE_ROWS}").as_bytes()).is_err());
}
#[test]
fn ply_ambiguous_face_index_property_names_are_rejected() {
    let header = BASE_HEADER.replace(
        "end_header",
        "property list uchar uint vertex_index\nend_header",
    );
    assert!(Mesh::import_ply(format!("{header}{BASE_ROWS}").as_bytes()).is_err());
}
#[test]
fn exporters_validate_mutated_mesh_before_writing() {
    let mut mesh = simple_mesh();
    mesh.triangles[0][1] = 999;
    for encoding in [
        PlyEncoding::Ascii,
        PlyEncoding::BinaryLittleEndian,
        PlyEncoding::BinaryBigEndian,
    ] {
        assert!(mesh.to_ply(encoding).is_err());
    }
    for encoding in [
        StlEncoding::Ascii,
        StlEncoding::Binary,
        StlEncoding::BinaryUncolored,
    ] {
        assert!(mesh.to_stl(encoding).is_err());
    }
}
#[test]
fn imports_leave_an_existing_mesh_exactly_unchanged_on_failure() {
    let mesh = simple_mesh();
    let before = serde_json::to_vec(&mesh).unwrap();
    assert!(Mesh::import_ply(b"ply\n").is_err());
    assert!(Mesh::import_stl(b"solid bad\n").is_err());
    assert_eq!(before, serde_json::to_vec(&mesh).unwrap());
}
#[test]
fn ply_ascii_crlf_is_supported() {
    assert_eq!(
        Mesh::import_ply(
            &format!("{BASE_HEADER}{BASE_ROWS}")
                .replace('\n', "\r\n")
                .into_bytes()
        )
        .unwrap()
        .mesh
        .triangles,
        [[0, 1, 2]]
    );
}
#[test]
fn stl_ascii_blank_lines_and_crlf_are_supported() {
    assert_eq!(
        Mesh::import_stl(&ASCII_STL.replace('\n', "\r\n\r\n").into_bytes())
            .unwrap()
            .mesh
            .triangles,
        [[0, 1, 2]]
    );
}
#[test]
fn ply_triangulation_work_cap_is_checked_before_large_polygon_processing() {
    let count = 400;
    let vertices = (0..count)
        .map(|i| {
            let theta = i as f64 * std::f64::consts::TAU / count as f64;
            format!("{} {} 0\n", theta.cos(), theta.sin())
        })
        .collect::<String>();
    let indices = (0..count)
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let text=format!("ply\nformat ascii 1.0\nelement vertex {count}\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list ushort uint vertex_indices\nend_header\n{vertices}{count} {indices}\n");
    assert!(Mesh::import_ply(text.as_bytes())
        .unwrap_err()
        .contains("triangulation work limit"));
}
