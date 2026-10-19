//! View-only guide and selection uniforms. No brush geometry or stored channels change.
//! The active Source-object guide uses its normalized per-object mirror plane.
//! Legacy brush offsets and non-X symmetry normals remain separate.
use glam::{DMat3, DMat4, DVec3};
use obrush_core::{
    BrushSettings, Document, Mesh, SceneObjectMetadata, SglDisplaySettings, SourceMirrorPlane,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum GuidePlaneProfile {
    #[default]
    ActiveSourceObject,
    CurrentBrush,
    ReferenceCenterPreview,
}
impl GuidePlaneProfile {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::ActiveSourceObject => "Active object plane",
            Self::CurrentBrush => "Current brush plane",
            Self::ReferenceCenterPreview => "Reference center preview",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SourceDisplayStyle {
    pub show_grid: bool,
    pub show_contour: bool,
    pub contour_color: [f32; 4],
    pub show_symmetry_line: bool,
    pub guide_profile: GuidePlaneProfile,
    /// Captured from the real brush each frame, not a second product offset.
    pub brush_symmetry_enabled: bool,
    pub brush_offset: f32,
    /// None preserves single-mesh callers; actual scene host captures Some(ids).
    pub selected_ids: Option<Vec<u32>>,
}
impl Default for SourceDisplayStyle {
    fn default() -> Self {
        Self {
            show_grid: false,
            show_contour: false,
            contour_color: [0.3, 0.0, 0.0, 1.0],
            show_symmetry_line: false,
            guide_profile: GuidePlaneProfile::default(),
            brush_symmetry_enabled: false,
            brush_offset: 0.0,
            selected_ids: None,
        }
    }
}
impl SourceDisplayStyle {
    pub(crate) fn capture(&mut self, document: &Document, brush: &BrushSettings) {
        self.selected_ids = Some(document.selected_subtool_ids());
        self.brush_symmetry_enabled = brush.symmetry_x;
        self.brush_offset = brush.symmetry_offset;
    }
    pub(crate) fn selected(&self, id: u32, active: bool) -> bool {
        self.selected_ids
            .as_ref()
            .map_or(active, |ids| ids.contains(&id))
    }
    pub(crate) fn guide_enabled(&self, active: bool) -> bool {
        self.show_symmetry_line && self.brush_symmetry_enabled && active
    }
    /// Modern SGL header changes the global line flag only after validated
    /// installation. Old/absent headers keep the existing global preference.
    pub(crate) fn import_header(&mut self, header: Option<&SglDisplaySettings>) {
        if let Some(header) = header {
            self.show_grid = header.show_grid;
            self.show_contour = header.show_contour;
            self.show_symmetry_line = header.show_symmetry_line;
        }
    }
    /// Global display switches are exported from the current real view state.
    /// The contour color is a separate UI/URL preference, not an SGL header bit.
    pub(crate) fn export_header(&self, origin: Option<&SglDisplaySettings>) -> SglDisplaySettings {
        let mut header = origin.cloned().unwrap_or(SglDisplaySettings {
            show_grid: false,
            show_symmetry_line: false,
            show_contour: false,
        });
        header.show_grid = self.show_grid;
        header.show_contour = self.show_contour;
        header.show_symmetry_line = self.show_symmetry_line;
        header
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GuidePlaneUniforms {
    pub origin: [f32; 3],
    pub normal: [f32; 3],
}

pub(crate) fn guide_plane_uniforms(
    display: &SourceDisplayStyle,
    mesh: &Mesh,
    metadata: Option<&SceneObjectMetadata>,
    source_root_loose: Option<[f64; 6]>,
    model_view: &[f32; 16],
    normal_matrix: &[f32; 9],
) -> Result<GuidePlaneUniforms, String> {
    if !display.brush_offset.is_finite()
        || model_view
            .iter()
            .chain(normal_matrix)
            .any(|x| !x.is_finite())
    {
        return Err("Symmetry guide is outside its finite display range".into());
    }
    let local_origin = match display.guide_profile {
        GuidePlaneProfile::ActiveSourceObject => match metadata {
            Some(metadata) => {
                let bounds = source_root_loose.ok_or("Source mirror guide has no root bounds")?;
                let plane = SourceMirrorPlane::from_relative_offset(
                    metadata.affine.interaction_center,
                    [1.0, 0.0, 0.0],
                    metadata.symmetry_offset,
                    bounds,
                )
                .map_err(|error| format!("Source mirror guide: {error:?}"))?;
                DVec3::from_array(plane.origin.map(f64::from))
            }
            None => DVec3::new(f64::from(display.brush_offset), 0.0, 0.0),
        },
        // The actual legacy host converts positions into object-local space,
        // but leaves this offset unchanged before core reflects local X.
        GuidePlaneProfile::CurrentBrush => DVec3::new(f64::from(display.brush_offset), 0.0, 0.0),
        GuidePlaneProfile::ReferenceCenterPreview => match metadata {
            Some(metadata) => DVec3::from_array(metadata.affine.interaction_center.map(f64::from)),
            None => {
                let (low, high) = mesh.positions.iter().fold(
                    (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY)),
                    |(low, high), point| (low.min(point.as_dvec3()), high.max(point.as_dvec3())),
                );
                (low + high) * 0.5
            }
        },
    };
    // f64 sums then f32 output follow the source JavaScript uniform preparation
    // for supplied f32 matrices. Equality of camera producers is a separate gate.
    let origin = DMat4::from_cols_array(&model_view.map(f64::from)).transform_point3(local_origin);
    let transformed_normal = DMat3::from_cols_array(&normal_matrix.map(f64::from)) * DVec3::X;
    let length = transformed_normal.length();
    if !origin.is_finite() || !length.is_finite() || length == 0.0 {
        return Err("Symmetry guide has no finite origin or normal".into());
    }
    let origin = origin.as_vec3();
    let normal = (transformed_normal / length).as_vec3();
    if !origin.is_finite() || !normal.is_finite() {
        return Err("Symmetry guide exceeds GPU display range".into());
    }
    Ok(GuidePlaneUniforms {
        origin: origin.to_array(),
        normal: normal.to_array(),
    })
}

/// Real panel multi-selection, separate from the existing merge checklist.
/// Removing the final selected item keeps it selected; active is always selected.
pub(crate) fn panel_selection(
    current: &[u32],
    active: Option<u32>,
    clicked: u32,
    toggle: bool,
) -> (Vec<u32>, Option<u32>) {
    if !toggle {
        return (vec![clicked], Some(clicked));
    }
    let mut ids = current.to_vec();
    if let Some(index) = ids.iter().position(|id| *id == clicked) {
        if ids.len() == 1 {
            return (ids, active.or(Some(clicked)));
        }
        ids.remove(index);
        let active = ids.first().copied();
        (ids, active)
    } else {
        ids.push(clicked);
        (ids, Some(clicked))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use obrush_core::ObjectAffineF32V1;
    fn triangle() -> Mesh {
        Mesh::from_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap()
    }
    #[test]
    fn capture_reads_every_selected_id_without_geometry_or_history_edits() {
        let mut document = Document::new(triangle());
        let second = document.add_subtool(triangle(), "Second").unwrap();
        document.add_subtool(triangle(), "Third").unwrap();
        assert!(document.undo());
        let first = document.subtool_summaries()[0].id;
        document
            .select_subtools(&[first, second], Some(second))
            .unwrap();
        let before = document.to_json().unwrap();
        let history = (document.can_undo(), document.can_redo());
        let mut display = SourceDisplayStyle::default();
        let brush = BrushSettings {
            symmetry_x: true,
            symmetry_offset: 0.25,
            ..Default::default()
        };
        display.capture(&document, &brush);
        assert!(display.selected(first, false));
        assert!(display.selected(second, true));
        assert!(!display.selected(second + 100, false));
        assert_eq!(display.brush_offset.to_bits(), 0.25f32.to_bits());
        assert!(display.brush_symmetry_enabled);
        assert_eq!(document.to_json().unwrap(), before);
        assert_eq!((document.can_undo(), document.can_redo()), history);
        assert!(history.1);
    }
    #[test]
    fn modern_header_updates_all_three_display_flags() {
        let original = SglDisplaySettings {
            show_grid: true,
            show_symmetry_line: true,
            show_contour: true,
        };
        let mut display = SourceDisplayStyle::default();
        display.import_header(Some(&original));
        assert!(display.show_grid);
        assert!(display.show_contour);
        assert!(display.show_symmetry_line);
        display.show_grid = false;
        display.show_contour = false;
        display.show_symmetry_line = false;
        let exported = display.export_header(Some(&original));
        assert!(!exported.show_grid && !exported.show_contour);
        assert!(!exported.show_symmetry_line);
        assert!(original.show_symmetry_line);
    }
    #[test]
    fn absent_legacy_header_keeps_global_grid_and_line_choices() {
        let mut display = SourceDisplayStyle {
            show_grid: true,
            show_symmetry_line: true,
            ..Default::default()
        };
        display.import_header(None);
        assert!(display.show_grid);
        assert!(display.show_symmetry_line);
        let header = display.export_header(None);
        assert!(header.show_grid);
        assert!(header.show_symmetry_line);
        assert!(!header.show_contour);
        assert_eq!(display.contour_color, [0.3, 0.0, 0.0, 1.0]);
    }
    #[test]
    fn panel_multi_selection_preserves_real_redo_and_keeps_active_selected() {
        let mut document = Document::new(triangle());
        let second = document.add_subtool(triangle(), "Second").unwrap();
        document.add_subtool(triangle(), "Third").unwrap();
        assert!(document.undo());
        let first = document.subtool_summaries()[0].id;
        let history = (document.can_undo(), document.can_redo());
        let (ids, active) = panel_selection(&[second], Some(second), first, true);
        document.select_subtools(&ids, active).unwrap();
        assert_eq!(document.selected_subtool_ids(), [second, first]);
        assert_eq!(document.active_subtool_id(), Some(first));
        assert_eq!((document.can_undo(), document.can_redo()), history);
        let (ids, active) = panel_selection(&ids, active, first, true);
        document.select_subtools(&ids, active).unwrap();
        assert_eq!(document.selected_subtool_ids(), [second]);
        assert_eq!(document.active_subtool_id(), Some(second));
        assert_eq!(
            panel_selection(&ids, active, second, true),
            (vec![second], Some(second))
        );
        assert_eq!((document.can_undo(), document.can_redo()), history);
    }
    #[test]
    fn removing_nonactive_panel_member_reactivates_first_remaining_source_member() {
        assert_eq!(
            panel_selection(&[3, 1, 2], Some(2), 1, true),
            (vec![3, 2], Some(3))
        );
    }
    #[test]
    fn active_only_guide_and_all_selected_membership_have_separate_meanings() {
        let display = SourceDisplayStyle {
            show_symmetry_line: true,
            brush_symmetry_enabled: true,
            selected_ids: Some(vec![11, 24]),
            ..Default::default()
        };
        assert!(display.selected(11, false));
        assert!(display.selected(24, true));
        assert!(!display.selected(17, false));
        assert!(!display.guide_enabled(false));
        assert!(display.guide_enabled(true));
        let disabled = SourceDisplayStyle {
            brush_symmetry_enabled: false,
            ..display
        };
        assert!(!disabled.guide_enabled(true));
    }
    fn affine_matrices() -> ([f32; 16], [f32; 9]) {
        (
            [
                2., 0., 0., 0., 1., 3., 0., 0., 0., 0., -4., 0., 10., 20., 30., 1.,
            ],
            [0.5, -1. / 6., 0., 0., 1. / 3., 0., 0., 0., -0.25],
        )
    }
    #[test]
    fn current_brush_plane_has_independent_affine_origin_and_normal_expectations() {
        let (mv, normal) = affine_matrices();
        let display = SourceDisplayStyle {
            brush_offset: 1.5,
            ..Default::default()
        };
        let plane = guide_plane_uniforms(&display, &triangle(), None, None, &mv, &normal).unwrap();
        assert_eq!(plane.origin, [13., 20., 30.]);
        let expected = [
            (3.0f64 / 10.0f64.sqrt()) as f32,
            (-1.0f64 / 10.0f64.sqrt()) as f32,
            0.,
        ];
        for (actual, expected) in plane.normal.into_iter().zip(expected) {
            assert!((actual - expected).abs() <= 1e-7);
        }
    }
    #[test]
    fn explicit_reference_preview_uses_interaction_center_at_zero_offset() {
        let (mv, normal) = affine_matrices();
        let metadata = SceneObjectMetadata {
            source_center_initialized: false,
            symmetry_offset: 0.0,
            affine: ObjectAffineF32V1 {
                interaction_center: [4., -2., 1.],
                local_to_world: glam::Mat4::IDENTITY.to_cols_array(),
            },
            render: None,
            picking_policy: obrush_core::SglPickingPolicy::SourceScaleRowV1,
            origin: None,
        };
        let display = SourceDisplayStyle {
            guide_profile: GuidePlaneProfile::ReferenceCenterPreview,
            brush_offset: 0.75,
            ..Default::default()
        };
        let mesh = Mesh::from_obj("v 3 -3 1\nv 5 -3 1\nv 3 -1 1\nf 1 2 3\n").unwrap();
        let plane =
            guide_plane_uniforms(&display, &mesh, Some(&metadata), None, &mv, &normal).unwrap();
        assert_eq!(plane.origin, [16., 14., 26.]);
        // The existing brush offset is not silently treated as normalized source offset.
        assert_eq!(display.brush_offset, 0.75);
    }
    #[test]
    fn active_source_guide_matches_core_per_object_plane_through_affine_view() {
        let mut mesh =
            Mesh::from_obj("v 0 0 0\nv 2 0 0\nv 0 4 0\nv 0 0 6\nf 1 2 3\nf 1 3 4\n").unwrap();
        mesh.mask_convention = obrush_core::MaskConvention::UnprotectedF32V1;
        let loose = obrush_core::SourceGeometryNormalState::seed(&mesh)
            .unwrap()
            .root_loose();
        let mut document = Document::empty();
        for (name, offset) in [("Zero", 0.0), ("Offset", 0.5)] {
            let metadata = SceneObjectMetadata {
                source_center_initialized: true,
                symmetry_offset: offset,
                affine: ObjectAffineF32V1 {
                    interaction_center: [1., 2., 3.],
                    local_to_world: glam::Mat4::IDENTITY.to_cols_array(),
                },
                render: None,
                picking_policy: obrush_core::SglPickingPolicy::SourceScaleRowV1,
                origin: None,
            };
            document
                .add_subtool_with_metadata(mesh.snapshot(), name, false, metadata)
                .unwrap();
        }
        let (mv, normal) = affine_matrices();
        let display = SourceDisplayStyle {
            show_symmetry_line: true,
            brush_symmetry_enabled: true,
            brush_offset: -10.,
            ..Default::default()
        };
        assert_eq!(display.guide_profile, GuidePlaneProfile::ActiveSourceObject);
        let ids: Vec<_> = document
            .subtool_summaries()
            .iter()
            .map(|summary| summary.id)
            .collect();
        for (id, offset) in ids.into_iter().zip([0.0, 0.5]) {
            document.select_subtools(&[id], Some(id)).unwrap();
            let before = document.to_json().unwrap();
            let core = document
                .source_geometry_mirror_plane([1., 2., 3.], offset, [1., 0., 0.])
                .unwrap();
            let guide = guide_plane_uniforms(
                &display,
                &mesh,
                document.active_object_metadata(),
                Some(loose),
                &mv,
                &normal,
            )
            .unwrap();
            let expected = DMat4::from_cols_array(&mv.map(f64::from))
                .transform_point3(DVec3::from_array(core.origin.map(f64::from)))
                .as_vec3()
                .to_array();
            assert_eq!(guide.origin, expected);
            assert_eq!(document.to_json().unwrap(), before);
        }
    }
    #[test]
    fn source_guide_rejects_missing_root_bounds_instead_of_drawing_wrong_plane() {
        let mesh = triangle();
        let metadata = SceneObjectMetadata {
            source_center_initialized: true,
            symmetry_offset: 0.5,
            affine: ObjectAffineF32V1 {
                interaction_center: [0.5, 0.5, 0.],
                local_to_world: glam::Mat4::IDENTITY.to_cols_array(),
            },
            render: None,
            picking_policy: obrush_core::SglPickingPolicy::SourceScaleRowV1,
            origin: None,
        };
        let (mv, normal) = affine_matrices();
        assert!(
            guide_plane_uniforms(
                &SourceDisplayStyle::default(),
                &mesh,
                Some(&metadata),
                None,
                &mv,
                &normal,
            )
            .is_err()
        );
    }
    #[test]
    fn nonfinite_or_zero_normal_rejects_without_mutating_geometry() {
        let mesh = triangle();
        let before = serde_json::to_vec(&mesh).unwrap();
        let (mut mv, normal) = affine_matrices();
        mv[12] = f32::NAN;
        assert!(
            guide_plane_uniforms(
                &SourceDisplayStyle::default(),
                &mesh,
                None,
                None,
                &mv,
                &normal
            )
            .is_err()
        );
        let (mv, _) = affine_matrices();
        assert!(
            guide_plane_uniforms(
                &SourceDisplayStyle::default(),
                &mesh,
                None,
                None,
                &mv,
                &[0.; 9]
            )
            .is_err()
        );
        assert_eq!(serde_json::to_vec(&mesh).unwrap(), before);
    }
}
