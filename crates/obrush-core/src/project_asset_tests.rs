use super::*;

fn image(name: &str, rgba: &[u8]) -> ProjectImage {
    ProjectImage::new(name, 2, 2, rgba).unwrap()
}

const QUADRANTS: [u8; 16] = [
    255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
];
const MATCAP: [u8; 16] = [
    0, 255, 216, 255, 255, 0, 40, 255, 40, 0, 255, 255, 255, 255, 255, 255,
];

#[test]
fn version8_roundtrip_keeps_exact_pixels_selection_and_history() {
    let mut document = Document::new(Mesh::sphere(1));
    let legacy = document.to_json().unwrap();
    assert!(Document::from_json(&legacy)
        .unwrap()
        .project_assets()
        .is_none());
    let mut v1: serde_json::Value = serde_json::from_str(&legacy).unwrap();
    v1["version"] = 1.into();
    assert!(Document::from_json(&v1.to_string())
        .unwrap()
        .project_assets()
        .is_none());
    for project in [
        include_str!("../tests/fixtures/independent-legacy-loop-v2.obrush"),
        include_str!("../tests/fixtures/independent-material-uv-seam-v6.obrush"),
        include_str!("../tests/fixtures/independent-unprotected-v7.obrush"),
    ] {
        assert!(Document::from_json(project)
            .unwrap()
            .project_assets()
            .is_none());
    }
    document
        .install_project_uv_image(image("quadrants.jpg", &QUADRANTS))
        .unwrap();
    let with_uv = document.to_json().unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&with_uv).unwrap()["version"],
        8
    );
    assert_eq!(
        document
            .append_project_matcap(image("asymmetric.png", &MATCAP))
            .unwrap(),
        9
    );
    let both = document.to_json().unwrap();
    let loaded = Document::from_json(&both).unwrap();
    assert_eq!(loaded.to_json().unwrap(), both);
    let assets = loaded.project_assets().unwrap();
    assert_eq!(assets.uv.as_ref().unwrap().rgba_bottom_up, QUADRANTS);
    assert_eq!(assets.matcaps[0].rgba_bottom_up, MATCAP);
    assert_eq!((assets.shading, assets.matcap_index), (4, 9));
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), with_uv);
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), legacy);
    assert!(document.redo());
    assert_eq!(document.to_json().unwrap(), with_uv);
    assert!(document.redo());
    assert_eq!(document.to_json().unwrap(), both);
}

#[test]
fn malformed_assets_reject_before_replace_and_preserve_undo_future() {
    let mut receiving = Document::new(Mesh::sphere(1));
    receiving
        .install_project_uv_image(image("before.png", &QUADRANTS))
        .unwrap();
    let before = receiving.to_json().unwrap();
    assert!(receiving.undo());
    let past = receiving.to_json().unwrap();
    assert!(receiving.can_redo());
    let valid = serde_json::from_str::<serde_json::Value>(&before).unwrap();
    for mutate in [
        Box::new(|v: &mut serde_json::Value| v["version"] = 7.into())
            as Box<dyn Fn(&mut serde_json::Value)>,
        Box::new(|v: &mut serde_json::Value| v["assets"]["uv"]["width"] = 0.into()),
        Box::new(|v: &mut serde_json::Value| v["assets"]["uv"]["rgba_bottom_up"] = "*".into()),
        Box::new(|v: &mut serde_json::Value| v["assets"]["matcap_index"] = 40.into()),
    ] {
        let mut bad = valid.clone();
        mutate(&mut bad);
        let text = bad.to_string();
        assert!(Document::from_json(&text).is_err(), "accepted {text}");
        assert_eq!(receiving.to_json().unwrap(), past);
        assert!(receiving.can_redo());
    }
    assert!(Document::from_json(&before[..before.len() - 1]).is_err());
    assert_eq!(receiving.to_json().unwrap(), past);
    assert!(receiving.redo());
    assert_eq!(receiving.to_json().unwrap(), before);
}

#[test]
fn replacing_asset_project_is_one_reversible_event() {
    let mut receiving = Document::new(Mesh::sphere(1));
    receiving
        .install_project_uv_image(image("old.jpg", &QUADRANTS))
        .unwrap();
    let old = receiving.to_json().unwrap();
    let mut imported = Document::new(Mesh::sphere(1));
    imported
        .append_project_matcap(image("new.png", &MATCAP))
        .unwrap();
    let new = imported.to_json().unwrap();
    receiving
        .replace_document(Document::from_json(&new).unwrap())
        .unwrap();
    assert_eq!(receiving.to_json().unwrap(), new);
    assert!(receiving.undo());
    assert_eq!(receiving.to_json().unwrap(), old);
    assert!(receiving.redo());
    assert_eq!(receiving.to_json().unwrap(), new);
}
