//! Complete consumer preflight precedes the current document's command barrier.
//! Geometry, raw origins, and exact encoded appearance remain owned by core.
use crate::renderer::{ViewportStyle, consumer_style_for_mesh};
use obrush_core::{Document, Mesh, MeshImportReport, SglStageSettings, StagedSglScene};

pub(crate) enum PreparedImport {
    Whole {
        document: Box<Document>,
        report: Option<MeshImportReport>,
    },
    Append {
        mesh: Box<Mesh>,
        report: MeshImportReport,
        title: String,
    },
}
pub(crate) fn prepare(
    name: &str,
    bytes: &[u8],
    append: bool,
    style: &ViewportStyle,
) -> Result<PreparedImport, String> {
    obrush_core::resource_limits::check_document_input_size(name, bytes.len())?;
    let extension = name
        .rsplit_once('.')
        .map_or("", |(_, ext)| ext)
        .to_ascii_lowercase();
    let title = name
        .rsplit_once('.')
        .map_or(name, |(stem, _)| stem)
        .to_owned();
    if matches!(extension.as_str(), "obj" | "ply" | "stl") {
        let imported = match extension.as_str() {
            "obj" => Mesh::import_obj(bytes)?,
            "ply" => Mesh::import_ply(bytes)?,
            _ => Mesh::import_stl(bytes)?,
        };
        if append {
            return Ok(PreparedImport::Append {
                mesh: Box::new(imported.mesh),
                report: imported.report,
                title,
            });
        }
        let mut document = Document::new(imported.mesh);
        document.name = title;
        document.to_json()?;
        return Ok(PreparedImport::Whole {
            document: Box::new(document),
            report: Some(imported.report),
        });
    }
    if append {
        return Err(
            "Append accepts OBJ, PLY and STL. Open a project or SGL to replace the whole scene"
                .into(),
        );
    }
    let document = if extension == "sgl" {
        let staged = StagedSglScene::from_sgl_bytes(bytes, &SglStageSettings::default())?;
        // Validate appearance for hidden as well as selected visible objects.
        for object in &staged.objects {
            let metadata = obrush_core::SceneObjectMetadata {
                source_center_initialized: object.source_center_initialized,
                symmetry_offset: 0.0,
                affine: object.affine.clone(),
                render: object.render.clone(),
                picking_policy: staged.picking_policy,
                origin: Some(obrush_core::SceneObjectOrigin {
                    header: staged.origin.clone(),
                    object: object.origin.clone(),
                }),
            };
            consumer_style_for_mesh(Some(&metadata), &object.mesh, style)?;
        }
        let mut document = Document::empty();
        document.name = title;
        document.install_staged_sgl_scene(staged)?;
        document
    } else {
        Document::from_json(
            std::str::from_utf8(bytes).map_err(|_| "OBrush project must be UTF-8 JSON")?,
        )?
    };
    document.preferred_scene_picking_policy()?;
    for object in document.subtool_summaries() {
        consumer_style_for_mesh(
            document.subtool_object_metadata(object.id)?,
            document.subtool_mesh(object.id)?,
            style,
        )?;
    }
    // Output and pooled origin limits are checked before receiving-state changes.
    document.to_json()?;
    Ok(PreparedImport::Whole {
        document: Box::new(document),
        report: None,
    })
}

/// Normal host installation boundary after its stroke/gesture barrier. Core
/// validates scene/history/storage again; a receiving-state error is atomic.
pub(crate) fn install(
    document: &mut Document,
    prepared: PreparedImport,
) -> Result<Option<MeshImportReport>, String> {
    match prepared {
        PreparedImport::Whole {
            document: imported,
            report,
        } => {
            document.replace_document(*imported)?;
            Ok(report)
        }
        PreparedImport::Append {
            mesh,
            report,
            title,
        } => {
            if report.imported_normals {
                document.add_subtool_preserving_normals(*mesh, &title)?;
            } else {
                document.add_subtool(*mesh, &title)?;
            }
            Ok(Some(report))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Independent ASCII facets: one real face and one exact repeated corner.
    const MIXED_STL: &[u8] = b"solid fixture\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 2 0 0\nvertex 0 3 0\nendloop\nendfacet\nfacet normal 0 0 1\nouter loop\nvertex 9 9 9\nvertex 9 9 9\nvertex 10 9 9\nendloop\nendfacet\nendsolid fixture\n";

    #[test]
    fn stl_empty_facet_skip_is_undoable_for_open_and_append() {
        for append in [false, true] {
            let mut document = Document::new(Mesh::sphere(1));
            let before = document.to_json().unwrap();
            let prepared = prepare(
                "independent.stl",
                MIXED_STL,
                append,
                &ViewportStyle::default(),
            )
            .unwrap();
            let report = install(&mut document, prepared).unwrap().unwrap();
            assert_eq!(report.source_faces, 2);
            assert_eq!(report.skipped_degenerate_faces, 1);
            assert_eq!(report.triangulated_faces, 1);
            let mesh = document.active_mesh().unwrap();
            assert_eq!(mesh.positions.len(), 3);
            assert_eq!(mesh.triangles, [[0, 1, 2]]);
            mesh.validate().unwrap();
            let imported = document.to_json().unwrap();
            assert!(document.undo());
            assert_eq!(document.to_json().unwrap(), before);
            assert!(document.redo());
            assert_eq!(document.to_json().unwrap(), imported);

            // A malformed later facet still rejects the entire next import;
            // neither the receiving scene nor its redo branch is consumed.
            assert!(document.undo());
            let malformed = std::str::from_utf8(MIXED_STL)
                .unwrap()
                .replace("vertex 10 9 9", "vertex NaN 9 9");
            assert!(
                prepare(
                    "invalid.stl",
                    malformed.as_bytes(),
                    append,
                    &ViewportStyle::default()
                )
                .is_err()
            );
            assert_eq!(document.to_json().unwrap(), before);
            assert!(document.can_redo());
        }
    }
}
