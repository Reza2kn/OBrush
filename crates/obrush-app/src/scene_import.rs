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
