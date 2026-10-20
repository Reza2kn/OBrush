//! Portable CPU geometry and sculpting primitives for OBrush.
//!
//! The engine deliberately has no graphics, windowing, or filesystem dependency,
//! so the same document and brush behavior can run on native and WebAssembly.

mod alpha;
mod attributes;
mod brush;
mod decimate;
mod decode_limits;
mod document;
mod extract;
mod layers;
mod locked_accumulator;
mod locked_stamp;
mod mask_convention;
mod mask_filter;
mod mask_hierarchy;
mod mask_region;
mod mesh;
mod mesh_io;
mod morph;
mod multires;
mod obj_io;
mod paint_operations;
mod paint_preparation;
mod paint_response;
mod primitives;
pub mod project_assets;
mod refinement;
mod remesh;
pub mod resource_limits;
mod reverse_subdivision;
mod scene_geometry_export;
mod scene_hit;
mod scene_staging;
mod scene_v7;
mod sgl;
mod sgl_export;
mod source_appearance;
mod source_attribute_pick;
mod source_geometry;
mod source_geometry_controls;
mod source_import_options;
mod source_normals;
mod source_ordered_tree;
mod source_paint_normal_view;
mod source_pick_snapshot;
mod source_picked_normal;
#[cfg(test)]
mod source_primitive_levels_tests;
pub mod source_ring;
mod source_worker_cold_capture;
mod source_worker_delta;
mod source_worker_retained_seed;
mod source_worker_retained_warm;
mod source_worker_sparse_preview;
mod source_worker_transaction;
mod source_worker_warm_io;
mod static_subdivision;
mod subtools;
mod topology;
mod transform;
mod uv_bake;

pub use alpha::RasterAlpha;
pub use attributes::{
    AttributeExport, AttributeExportPolicy, AttributeExportReport, CornerUv, SurfaceTopology,
    DEFAULT_VERTEX_MATERIAL, MAX_UV_COORDINATES, TRIANGLE_CORNER,
};
pub use brush::{
    BrushKind, BrushResult, BrushSettings, DrawStroke, LocalTransformStroke, MoveStroke,
};
pub use decimate::{DecimatePhase, DecimateProgress, DecimateReport, DecimateSettings};
pub use document::{
    CertifiedSourceRenderSnapshot, PreparedSourceGeometryDab, SourceGeometryLockedPreview,
    SourceGeometryPreparationTime, SourceGeometryReleaseTiming,
};
pub use document::{
    Document, ImportedSubtool, MorphTargetDeletionDisposition, SglAppendReport,
    SourcePaintLockedAction, SourcePaintLockedMode, SourcePaintLockedReport,
    SourcePaintLockedSample, SourcePaintLockedStroke, SubdivisionDeletionReport,
};
pub use extract::{MaskExtractReport, MaskExtractSelection, MaskExtractSettings};
pub use glam::Vec3;
pub use layers::{
    LayerSummary, MAX_LAYER_NAME_BYTES, MAX_LAYER_STORAGE_BYTES, MAX_LAYER_VECTORS,
    MAX_SCULPT_LAYERS,
};
pub use locked_accumulator::{
    LockedAccumulationAction, LockedAccumulationReport, LockedAccumulationSample, LockedAccumulator,
};
pub use locked_stamp::{LockedStamp, LockedStampAction, LockedStampReport, LockedStampSample};
pub use mask_convention::{MaskConvention, MaskConversionPolicy, MaskConversionReport};
pub use mask_filter::{MaskFilterOperation, MaskFilterReport, MaskFilterSettings};
pub use mask_region::{
    MaskRegion, MaskRegionKind, MaskRegionReport, MaskRegionSettings, MASK_REGION_BOUNDARY_EPSILON,
    MAX_MASK_REGION_POINTS, MAX_MASK_REGION_WORK,
};
pub use mesh::{Hit, Mesh, Ray, MAX_TRIANGLES, MAX_VERTICES};
pub use mesh_io::{
    MeshFileFormat, MeshImport, MeshImportReport, PlyEncoding, StlEncoding, MAX_MESH_IO_SCALARS,
    MAX_PLY_FACE_VERTICES, MAX_PLY_HEADER_BYTES, MAX_PLY_TRIANGULATION_WORK,
};
pub use morph::{MorphTargetSummary, MAX_MORPH_STORAGE_BYTES};
pub use paint_operations::{PaintFillReport, PaintPickInterpolation, PaintSample};
pub use primitives::{
    CylinderSettings, SubtoolMergePolicy, SubtoolMergeReport, SubtoolMergeSettings, TorusSettings,
    MAX_PRIMITIVE_DIMENSION, MIN_PRIMITIVE_DIMENSION,
};
pub use refinement::PatchRefinement;
pub use remesh::{RemeshPhase, RemeshProgress, RemeshReport, RemeshSettings, SelectedRemeshReport};
pub use reverse_subdivision::{
    ReverseSubdivisionCorrespondence, ReverseSubdivisionOutcome, ReverseSubdivisionReport,
    ReverseSubdivisionSettings, MAX_REVERSE_STORAGE_BYTES, MAX_REVERSE_WORK,
};
pub use scene_geometry_export::{
    ObjColorOptions, SceneGeometryExport, SceneGeometryFormat, SceneGeometryReport,
};
pub use scene_hit::{ObjectRenderTransform, SceneHitConversionReport, SceneRay64, SceneRayHit64};
pub use scene_staging::{
    ObjectAffineF32V1, SglObjectOrigin, SglPickingPolicy, SglSceneOrigin, SglStageReport,
    SglStageSettings, StagedSglObject, StagedSglScene, EDITABLE_SGL_REQUIRED_PROJECT_VERSION,
    MAX_STAGED_SCENE_BYTES,
};
pub use scene_v7::{SceneObjectMetadata, SceneObjectOrigin};
pub use sgl::{
    SglByteOrder, SglCameraSettings, SglDisplaySettings, SglMesh, SglRenderSettings, SglScene,
    SglUvData, MAX_SGL_UV_COORDS,
};
pub use sgl_export::{
    SglExport, SglExportLossPolicy, SglExportReport, SglExportScope, SglExportSettings,
};
pub use source_appearance::{
    baseline_source_render_defaults, resolve_source_appearance, SourceAppearance,
    SourceAppearanceOrigin, SourceShaderRole,
};
pub use source_attribute_pick::{
    prepare_source_attribute_pick, SourceAttributePickError, SourceAttributePickFailure,
    SourceAttributePickInverseSum, SourceAttributePickMaskPolicy, SourceAttributePickRejection,
    SourceAttributePickReport, SourceAttributePickSample, SourceAttributePickUndefined,
    MAX_SOURCE_ATTRIBUTE_PICK_DIAGNOSTIC_BYTES, MAX_SOURCE_ATTRIBUTE_PICK_REPORT_BYTES,
};
pub use source_import_options::{
    prepare_source_mesh_group, prepare_source_mesh_import, PreparedSourceImportOptions,
    PreparedSourceMeshGroup, PreparedSourceMeshImport, SourceImportCenterReason,
    SourceImportCenterReport, SourceImportNormalization, SourceImportObjectReport,
    SourceImportOptions, SourceImportOptionsReport,
};
pub use source_normals::{
    SourceNormalFaceOrigin, SourceNormalField, SourceNormalPolicy, SourceNormalReport,
    SourceNormalStorage, MAX_SOURCE_NORMAL_BYTES,
};
pub use source_picked_normal::{
    prepare_source_picked_normal, prepare_source_picked_normal_alpha_none,
    prepare_source_picked_normal_typed, SourceOriginalFaceContact, SourcePickedNormalDisposition,
    SourcePickedNormalError, SourcePickedNormalRejection, SourcePickedNormalReport,
    SourcePickedNormalUndefined, MAX_SOURCE_PICK_COORDINATE,
};
pub use source_worker_cold_capture::SourceWorkerColdCapture;
pub use source_worker_delta::{
    decode_source_worker_mesh_delta, encode_source_worker_mesh_delta, MAX_SOURCE_WORKER_DELTA_BYTES,
};
pub use source_worker_retained_seed::{
    decode_source_worker_retained_seed, encode_source_worker_retained_seed,
};
pub use source_worker_retained_warm::{
    decode_source_worker_retained_warm, encode_source_worker_retained_warm,
    MAX_SOURCE_WORKER_WARM_PACKET_BYTES,
};
pub use source_worker_sparse_preview::{
    encode_source_worker_sparse_preview_rows, encode_source_worker_sparse_preview_rows_mode,
    SourceWorkerPreviewRow, SourceWorkerSparsePreviewOverlay, MAX_SOURCE_WORKER_PREVIEW_BYTES,
    MAX_SOURCE_WORKER_PREVIEW_OVERLAY_ROWS,
};
pub use source_worker_transaction::{SourceWorkerTransaction, SourceWorkerTransactionError};
pub use static_subdivision::{
    StaticSubdivisionMap, StaticSubdivisionOutcome, StaticSubdivisionReport,
    StaticSubdivisionSettings, MAX_STATIC_MAP_BYTES,
};
pub use subtools::{
    SubtoolHit, SubtoolRenderSnapshot, SubtoolSummary, MAX_SUBTOOLS, MAX_SUBTOOL_NAME_BYTES,
};
pub use topology::{
    execute_topology, execute_topology_project, execute_topology_project_with_cancel,
    execute_topology_with_cancel, TopologyOperation, TopologyReport,
};
pub use transform::ObjectTransform;

#[cfg(test)]
mod tests;

pub use uv_bake::{
    bake_uv_with_cancel, UvBakeChannel, UvBakeExport, UvBakeImage, UvBakeJob, UvBakePhase,
    UvBakeReport, UvBakeSize, MAX_UV_BAKE_RASTER_WORK, MAX_UV_BAKE_SCRATCH_BYTES,
};

pub use mask_hierarchy::{
    MaskConversionWord, MaskHierarchyConversionReport, MaskLevelConversionReport,
};
pub use paint_preparation::{
    prepare_source_paint_hit, prepare_source_paint_locked_pass, prepare_source_paint_pass,
    prepare_source_paint_pass_alpha_none, prepare_source_paint_pass_with_normals,
    prepare_source_paint_pass_with_picked_normal, LockTravelRadius, PaintPreparationInput,
    PaintViewCapture, PreparedSourcePaintAlphaNonePass, PreparedSourcePaintHit,
    PreparedSourcePaintNormalPass,
};
pub use paint_response::{
    paint_tablet_local_radius_squared, PaintResponseAlphaFrame, PaintResponseOperation,
    PaintResponseOptions, PaintResponsePass, PaintResponseReport, PaintResponseRequest,
    PaintTabletIntensity, MAX_PAINT_RESPONSE_SCRATCH_BYTES,
};

pub use source_geometry::{
    project_source_drag_center, ProjectedSourceDragCenter, SourceClayOrderPolicy,
    SourceFlattenPlane, SourceGeometryError, SourceGeometryNoop, SourceGeometryNormalState,
    SourceGeometryOptions, SourceGeometryPass, SourceGeometryPhaseTime, SourceGeometryReport,
    SourceGeometryStroke, SourceGeometryTool, MAX_SOURCE_GEOMETRY_OPERATION_BYTES,
    MAX_SOURCE_GEOMETRY_SCRATCH_BYTES, MAX_SOURCE_GEOMETRY_STATE_BYTES,
    SOURCE_CLAY_SINGLE_LEAF_FACES, SOURCE_GEOMETRY_CHUNK_ROWS,
};
pub use source_ordered_tree::{
    SourceLeafHandle, SourceOrderedCell, SourceOrderedSelection, SourceOrderedTreeSnapshot,
    MAX_SOURCE_ORDERED_TREE_BYTES, SOURCE_ORDERED_TREE_LEAF_FACES, SOURCE_ORDERED_TREE_MAX_DEPTH,
};

pub use source_paint_normal_view::{
    prepare_source_paint_pass_alpha_none_with_retained, PreparedSourcePaintRetainedPass,
    SourcePaintNormalProducerError, SourcePaintNormalProducerKind, SourcePaintNormalProducerReport,
    SourcePaintNormalView, SourceRetainedPaintStroke,
};

pub use source_geometry_controls::{
    prepare_source_geometry_contacts, prepare_source_geometry_press_contacts,
    SourceGeometryAlphaTrack, SourceGeometryAlphaTracks, SourceGeometryContacts,
    SourceGeometryControlledReport, SourceGeometryControls, SourceMirrorPlane,
};

// Immutable, incrementally admitted read-only Source Paint sampling.
pub use source_pick_snapshot::{
    SourceAttributePickView, SourcePickAdmissionError, SourcePickAdmissionProgress,
    SourcePickQueryAdmission, SourcePickQueryAdmissionError, SourcePickQueryBudget,
    SourcePickQueryError, SourcePickQueryProgress, SourcePickSnapshotAdmission,
    SourcePickWorkBudget, SourcePreparedPickQuery, SourceSnapshotRayError,
    ValidatedSourcePickSnapshot, MAX_SOURCE_PICK_ADMISSION_OPERATIONS,
    MAX_SOURCE_PICK_CORNER_READS, MAX_SOURCE_PICK_FACE_TESTS, MAX_SOURCE_PICK_INDEX_FACES,
    MAX_SOURCE_PICK_TRIANGLE_TESTS,
};

mod source_pick_interval;

pub mod resident_normal_program;
/// Bounded, GPU-independent input and scalar reference for resident geometry.
/// The optional GPU crate consumes this validated snapshot; studio brushes
/// continue to use the shared CPU path until product integration is verified.
pub mod resident_point_program;
pub mod resident_upload;
