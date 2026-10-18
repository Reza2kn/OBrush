use obrush_core::{Document, Mesh};

const GEOMETRY: &str = "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";

#[test]
fn zbrush_style_mrgb_comments_preserve_nonuniform_colors_across_multiple_rows() {
    let mesh = Mesh::from_obj(include_str!("fixtures/zbrush_mrgb.obj")).unwrap();
    assert_eq!(
        mesh.colors,
        vec![
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [18.0 / 255.0, 52.0 / 255.0, 86.0 / 255.0]
        ]
    );
    assert_eq!(mesh.masks, vec![0.0; 4]); // Leading M byte is deliberately ignored.
    let project = Document::new(mesh.clone()).to_json().unwrap();
    assert_eq!(
        Document::from_json(&project)
            .unwrap()
            .active_mesh()
            .unwrap()
            .colors,
        mesh.colors
    );
    assert_eq!(Mesh::from_obj(&mesh.to_obj()).unwrap().colors, mesh.colors);
}

#[test]
fn complete_mrgb_stream_overrides_inline_rgb_deterministically() {
    let source =
        "#MRGB ff102030ff405060ff708090\nv 0 0 0 1 1 1\nv 1 0 0 1 1 1\nv 0 1 0 1 1 1\nf 1 2 3\n";
    let mesh = Mesh::from_obj(source).unwrap();
    assert_eq!(mesh.colors[0], [16.0 / 255.0, 32.0 / 255.0, 48.0 / 255.0]);
    assert_eq!(mesh.colors[1], [64.0 / 255.0, 80.0 / 255.0, 96.0 / 255.0]);
    assert_eq!(
        mesh.colors[2],
        [112.0 / 255.0, 128.0 / 255.0, 144.0 / 255.0]
    );
}

#[test]
fn malformed_partial_empty_and_wrong_count_mrgb_streams_are_rejected() {
    for comment in [
        "#MRGB\n",
        "#MRGB ffffff\n",
        "#MRGB ffffffff ffffffff ffffffff ff\n",
        "#MRGB fgffffff ffffffff ffffffff\n",
        "#MRGB ffggffff ffffffff ffffffff\n",
        "#MRGB fféffff ffffffff ffffffff\n",
        "#MRGB ffffffff ffffffff\n",
        "#MRGB ffffffff ffffffff ffffffff ffffffff\n",
        "#MRGB ffffffff\n#MRGB ffffff\n",
    ] {
        let error = Mesh::from_obj(&format!("{GEOMETRY}{comment}")).unwrap_err();
        assert!(
            error.contains("MRGB"),
            "wrong diagnostic for {comment:?}: {error}"
        );
    }
}

#[test]
fn unrelated_obj_comments_do_not_create_an_mrgb_stream() {
    let mesh = Mesh::from_obj(&format!(
        "# MRGB is described here, but no data follows\n{GEOMETRY}"
    ))
    .unwrap();
    assert!(mesh.colors.iter().all(|&color| color == [0.65, 0.69, 0.74]));
}

#[test]
fn over_limit_mrgb_stream_is_refused_even_before_vertex_records() {
    let mut source = String::from("#MRGB ");
    source.push_str(&"ffffffff".repeat(obrush_core::MAX_VERTICES + 1));
    source.push('\n');
    source.push_str(GEOMETRY);
    let error = Mesh::from_obj(&source).unwrap_err();
    assert!(error.contains("MRGB vertex color limit"));
}

#[test]
fn sculptgl_mat_comments_preserve_materials_and_unmasked_weight() {
    let source = format!("#MAT 8040ff000000\n{GEOMETRY}#MAT ff007f\n");
    let mesh = Mesh::from_obj(&source).unwrap();
    assert_eq!(
        mesh.materials.as_ref().unwrap(),
        &vec![[128.0 / 255.0, 64.0 / 255.0], [0.0, 0.0], [1.0, 0.0],]
    );
    assert_eq!(mesh.masks, vec![0.0, 1.0, 1.0 - 127.0 / 255.0]);
    let project = Document::new(mesh.clone()).to_json().unwrap();
    let loaded = Document::from_json(&project).unwrap();
    assert_eq!(loaded.active_mesh().unwrap().materials, mesh.materials);
    assert_eq!(loaded.active_mesh().unwrap().masks, mesh.masks);
}

#[test]
fn malformed_partial_empty_and_wrong_count_mat_streams_are_rejected() {
    for comment in [
        "#MAT\n",
        "#MAT fffff\n",
        "#MAT ffffff ffffff ff\n",
        "#MAT fgffff ffffff ffffff\n",
        "#MAT ffffff ffffff\n",
        "#MAT ffffff ffffff ffffff ffffff\n",
        "#MAT ffffff\n#MAT fffff\n",
    ] {
        let error = Mesh::from_obj(&format!("{GEOMETRY}{comment}")).unwrap_err();
        assert!(
            error.contains("MAT"),
            "wrong diagnostic for {comment:?}: {error}"
        );
    }
}

#[test]
fn mat_vertex_material_limit_is_enforced_before_mesh_replacement() {
    let mut source = String::from("#MAT ");
    source.push_str(&"ffffff".repeat(obrush_core::MAX_VERTICES + 1));
    source.push('\n');
    source.push_str(GEOMETRY);
    let error = Mesh::from_obj(&source).unwrap_err();
    assert!(error.contains("MAT vertex material limit"));
}
