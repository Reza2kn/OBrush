//! Render-only UV seams and bounded viewport images. The sculpt mesh stays welded.
use std::collections::HashMap;

use obrush_core::{Mesh, TRIANGLE_CORNER};

pub(crate) const MAX_VIEW_TEXTURE_SIDE: u32 = 2048;
pub(crate) const MAX_VIEW_TEXTURE_BYTES: usize = 16 * 1024 * 1024;
const MAX_UV_SCRATCH_BYTES: usize = 256 * 1024 * 1024;
pub(crate) const MAX_CORRESPONDENCE_DIAGNOSTIC_ROWS: usize = 100_000;

#[derive(Clone, Debug)]
pub struct ViewTexture {
    pub(crate) name: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
    /// The first row is the bottom image row; no global GL unpack flip is used.
    pub(crate) pixels_bottom_up: Vec<u8>,
}

impl ViewTexture {
    pub fn from_rgba(name: &str, width: u32, height: u32, rgba: &[u8]) -> Result<Self, String> {
        let bytes = texture_storage_bytes(width, height)
            .ok_or("Texture dimensions must be 1–2048 pixels within the 16 MiB limit")?;
        if rgba.len() != bytes {
            return Err("Texture RGBA data does not match the declared dimensions".into());
        }
        let row = width as usize * 4;
        let mut pixels_bottom_up = Vec::new();
        pixels_bottom_up
            .try_reserve_exact(bytes)
            .map_err(|_| "Cannot allocate bounded viewport texture")?;
        for y in (0..height as usize).rev() {
            pixels_bottom_up.extend_from_slice(&rgba[y * row..(y + 1) * row]);
        }
        Ok(Self {
            name: name.chars().take(80).collect(),
            width,
            height,
            pixels_bottom_up,
        })
    }
}

pub(crate) fn decode_png(name: &str, bytes: &[u8]) -> Result<ViewTexture, String> {
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("Texture PNG exceeds the 16 MiB input limit".into());
    }
    if bytes.len() < 33
        || bytes[..8] != [137, 80, 78, 71, 13, 10, 26, 10]
        || &bytes[12..16] != b"IHDR"
    {
        return Err("Select a valid PNG viewport texture".into());
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    texture_storage_bytes(width, height)
        .ok_or("PNG viewport texture dimension/allocation limit exceeded")?;
    let mut reader =
        image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_VIEW_TEXTURE_SIDE);
    limits.max_image_height = Some(MAX_VIEW_TEXTURE_SIDE);
    limits.max_alloc = Some(32 * 1024 * 1024);
    reader.limits(limits);
    let rgba = reader
        .decode()
        .map_err(|e| format!("PNG texture rejected: {e}"))?
        .to_rgba8();
    if rgba.width() != width || rgba.height() != height {
        return Err("Decoded PNG dimensions differ from header".into());
    }
    ViewTexture::from_rgba(name, width, height, rgba.as_raw())
}

/// Shared bounded JPEG decoding for viewport images, including MatCaps. Header
/// dimensions are checked before pixel conversion and against the actual decode.
pub(crate) fn decode_jpeg(name: &str, bytes: &[u8]) -> Result<ViewTexture, String> {
    if bytes.len() > 16 * 1024 * 1024 || bytes.len() < 4 || bytes[..2] != [255, 216] {
        return Err("Viewport JPEG requires valid bytes within the 16 MiB input limit".into());
    }
    let dimensions =
        image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Jpeg)
            .into_dimensions()
            .map_err(|e| format!("JPEG header rejected: {e}"))?;
    texture_storage_bytes(dimensions.0, dimensions.1)
        .ok_or("JPEG texture dimension/allocation limit exceeded")?;
    let mut reader =
        image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Jpeg);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_VIEW_TEXTURE_SIDE);
    limits.max_image_height = Some(MAX_VIEW_TEXTURE_SIDE);
    limits.max_alloc = Some(32 * 1024 * 1024);
    reader.limits(limits);
    let rgba = reader
        .decode()
        .map_err(|e| format!("JPEG texture rejected: {e}"))?
        .to_rgba8();
    if (rgba.width(), rgba.height()) != dimensions {
        return Err("Decoded JPEG dimensions differ from header".into());
    }
    ViewTexture::from_rgba(name, dimensions.0, dimensions.1, rgba.as_raw())
}

pub(crate) fn decode_view_image(name: &str, bytes: &[u8]) -> Result<ViewTexture, String> {
    if bytes.starts_with(&[255, 216]) {
        decode_jpeg(name, bytes)
    } else {
        decode_png(name, bytes)
    }
}

pub(crate) fn texture_storage_bytes(width: u32, height: u32) -> Option<usize> {
    if width == 0 || height == 0 || width > MAX_VIEW_TEXTURE_SIDE || height > MAX_VIEW_TEXTURE_SIDE
    {
        return None;
    }
    let bytes = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    (bytes <= MAX_VIEW_TEXTURE_BYTES).then_some(bytes)
}

pub(crate) fn texture_gpu_storage_bytes(width: u32, height: u32) -> Option<usize> {
    let mut bytes = texture_storage_bytes(width, height)?;
    if width.is_power_of_two() && height.is_power_of_two() {
        let (mut w, mut h) = (width, height);
        while w > 1 || h > 1 {
            w = (w / 2).max(1);
            h = (h / 2).max(1);
            bytes = bytes.checked_add((w as usize).checked_mul(h as usize)?.checked_mul(4)?)?;
        }
    }
    (bytes <= 22 * 1024 * 1024).then_some(bytes)
}

/// Upper bound before seam hash-map/output allocation. Geometry, material and UV
/// rows are 40+8+8 bytes, with the separate canonical render index array.
pub(crate) fn uv_storage_upper_bound(mesh: &Mesh) -> Option<usize> {
    let surface = mesh.surface.as_ref()?;
    surface.uv.as_ref()?;
    if surface.faces.len() > obrush_core::MAX_TRIANGLES
        || mesh.triangles.len() > obrush_core::MAX_TRIANGLES
    {
        return None;
    }
    let corners = surface.faces.iter().try_fold(0_usize, |n, f| {
        n.checked_add(if f[3] == TRIANGLE_CORNER { 3 } else { 4 })
    })?;
    uv_count_storage_upper_bound(corners, mesh.triangles.len())
}

pub(crate) fn uv_count_storage_upper_bound(corners: usize, triangles: usize) -> Option<usize> {
    if triangles > obrush_core::MAX_TRIANGLES || triangles.checked_mul(3)? > i32::MAX as usize {
        return None;
    }
    // Budget output, correspondence, and an intentionally conservative map
    // scratch bound before reserving. A large seam count can be rejected even
    // when geometric vertices are few; this is an explicit renderer limit.
    if corners
        .checked_mul(112)?
        .checked_add(triangles.checked_mul(12)?)?
        > MAX_UV_SCRATCH_BYTES
    {
        return None;
    }
    corners
        .checked_mul(56)?
        .checked_add(triangles.checked_mul(12)?)
}

#[derive(Debug)]
pub(crate) struct PackedUvMesh {
    pub vertices: Vec<[f32; 10]>,
    pub materials: Option<Vec<[f32; 2]>>,
    pub uv: Vec<[f32; 2]>,
    pub triangles: Vec<[u32; 3]>,
    /// Render-row -> [welded geometric vertex ID, original corner UV ID].
    /// Equal-valued distinct UV IDs deliberately remain distinct rows.
    pub correspondence: Vec<[u32; 2]>,
}

impl PackedUvMesh {
    pub fn build(mesh: &Mesh, available_gpu_bytes: usize) -> Result<Self, String> {
        Self::build_normals(mesh, available_gpu_bytes, None)
    }
    pub fn build_with_source_normals(
        mesh: &Mesh,
        available_gpu_bytes: usize,
        normals: &obrush_core::SourceNormalField,
    ) -> Result<Self, String> {
        if !normals.matches_mesh(mesh) {
            return Err("Source UV normals do not match welded geometry".into());
        }
        Self::build_normals(mesh, available_gpu_bytes, Some(normals.vertex_normals()))
    }
    pub fn build_with_source_geometry_normals(
        mesh: &Mesh,
        available_gpu_bytes: usize,
        normals: &obrush_core::SourceGeometryNormalState,
    ) -> Result<Self, String> {
        if !normals.matches_mesh(mesh) {
            return Err("Selected-refresh UV normals do not match welded geometry".into());
        }
        Self::build_normals(mesh, available_gpu_bytes, Some(normals.vertex_normals()))
    }
    fn build_normals(
        mesh: &Mesh,
        available_gpu_bytes: usize,
        source_normals: Option<&[glam::Vec3]>,
    ) -> Result<Self, String> {
        let required = uv_storage_upper_bound(mesh)
            .ok_or("Missing UV channel or bounded UV render allocation exceeded")?;
        if required > available_gpu_bytes {
            return Err("UV render geometry exceeds the remaining scene GPU memory limit".into());
        }
        mesh.validate()?;
        let surface = mesh
            .surface
            .as_ref()
            .ok_or("UV rendering requires original face metadata")?;
        let source = surface
            .uv
            .as_ref()
            .ok_or("UV rendering requires complete corner UVs")?;
        let maximum = surface
            .faces
            .iter()
            .map(|f| if f[3] == TRIANGLE_CORNER { 3 } else { 4 })
            .sum();
        let mut pairs = HashMap::<(u32, u32), u32>::new();
        pairs
            .try_reserve(maximum)
            .map_err(|_| "Cannot allocate bounded UV correspondence")?;
        let mut correspondence = Vec::new();
        correspondence
            .try_reserve_exact(maximum)
            .map_err(|_| "Cannot allocate bounded render UV rows")?;
        let mut triangles = Vec::new();
        triangles
            .try_reserve_exact(mesh.triangles.len())
            .map_err(|_| "Cannot allocate bounded UV triangles")?;
        for (face, uv) in surface.faces.iter().zip(&source.face_indices) {
            let arity = if face[3] == TRIANGLE_CORNER { 3 } else { 4 };
            let mut mapped = [0; 4];
            for corner in 0..arity {
                let pair = (face[corner], uv[corner]);
                mapped[corner] = match pairs.get(&pair) {
                    Some(&index) => index,
                    None => {
                        let index = u32::try_from(correspondence.len())
                            .map_err(|_| "UV render index overflow")?;
                        pairs.insert(pair, index);
                        correspondence.push([pair.0, pair.1]);
                        index
                    }
                };
            }
            triangles.push([mapped[0], mapped[1], mapped[2]]);
            if arity == 4 {
                triangles.push([mapped[0], mapped[2], mapped[3]]);
            }
        }
        let mut vertices = Vec::new();
        let mut uv = Vec::new();
        vertices
            .try_reserve_exact(correspondence.len())
            .map_err(|_| "Cannot allocate bounded UV geometry")?;
        uv.try_reserve_exact(correspondence.len())
            .map_err(|_| "Cannot allocate bounded UV coordinates")?;
        let mut materials = mesh.materials.as_ref().map(|_| Vec::new());
        if let Some(rows) = materials.as_mut() {
            rows.try_reserve_exact(correspondence.len())
                .map_err(|_| "Cannot allocate bounded UV materials")?;
        }
        for &[geometric, texture] in &correspondence {
            let i = geometric as usize;
            let p = mesh.positions[i];
            let n = source_normals.map_or(mesh.normals[i], |rows| rows[i]);
            let color = mesh.colors[i];
            vertices.push([
                p.x,
                p.y,
                p.z,
                n.x,
                n.y,
                n.z,
                mesh.protection_for_display(i),
                color[0],
                color[1],
                color[2],
            ]);
            uv.push(source.coordinates[texture as usize]);
            if let Some(rows) = materials.as_mut() {
                rows.push(mesh.materials.as_ref().unwrap()[i]);
            }
        }
        Ok(Self {
            vertices,
            materials,
            uv,
            triangles,
            correspondence,
        })
    }
}

/// Stable order-sensitive diagnostic fingerprint, not a cryptographic provenance
/// hash. Independent consumers still compare exact GPU bytes and source hashes.
pub(crate) fn correspondence_fingerprint(rows: &[[u32; 2]]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for value in rows.iter().flatten() {
        for byte in value.to_le_bytes() {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
        }
    }
    format!("{hash:016x}")
}

#[cfg(test)]
pub(crate) fn srgb_linear(value: f32) -> f32 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn seam_fixture() -> Mesh {
        Mesh::import_obj(b"v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\nvt 0 0\nvt 0.25 1\nf 1/1 2/2 3/3\nf 1/5 3/6 4/4\n").unwrap().mesh
    }
    #[test]
    fn equal_valued_and_distinct_corner_seams_map_to_exact_welded_rows() {
        let mesh = seam_fixture();
        let original = serde_json::to_string(&mesh).unwrap();
        let packed = PackedUvMesh::build(&mesh, 4096).unwrap();
        assert_eq!(
            packed.correspondence,
            [[0, 0], [1, 1], [2, 2], [0, 4], [2, 5], [3, 3]]
        );
        assert_eq!(packed.triangles, [[0, 1, 2], [3, 4, 5]]);
        assert_eq!(
            packed.uv,
            [[0., 0.], [1., 0.], [1., 1.], [0., 0.], [0.25, 1.], [0., 1.]]
        );
        assert_eq!(packed.vertices[0], packed.vertices[3]);
        assert_eq!(packed.vertices[2], packed.vertices[4]);
        assert!(packed.materials.is_none());
        assert_eq!(original, serde_json::to_string(&mesh).unwrap());
    }

    #[test]
    fn uv_display_conversion_keeps_canonical_source_mask_words_unchanged() {
        let mut mesh = seam_fixture();
        mesh.mask_convention = obrush_core::MaskConvention::UnprotectedF32V1;
        mesh.masks = vec![-0.0, 1.0, 1e-9, 0.25];
        let before = mesh.masks.iter().map(|v| v.to_bits()).collect::<Vec<_>>();
        let packed = PackedUvMesh::build(&mesh, 4096).unwrap();
        for (row, &[geometry, _uv]) in packed.correspondence.iter().enumerate() {
            let expected = 1.0_f32 - mesh.masks[geometry as usize];
            assert_eq!(packed.vertices[row][6].to_bits(), expected.to_bits());
        }
        assert_eq!(
            mesh.masks.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            before
        );
        assert_eq!(
            mesh.mask_convention,
            obrush_core::MaskConvention::UnprotectedF32V1
        );
    }
    #[test]
    fn original_quad_uses_canonical_render_diagonal_and_keeps_unused_uv_prefix() {
        let mesh = Mesh::import_obj(b"v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nvt 9 9\nvt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\nf 1/2 2/3 3/4 4/5\n").unwrap().mesh;
        let packed = PackedUvMesh::build(&mesh, 4096).unwrap();
        assert_eq!(packed.correspondence, [[0, 1], [1, 2], [2, 3], [3, 4]]);
        assert_eq!(packed.triangles, [[0, 1, 2], [0, 2, 3]]);
        assert_eq!(
            mesh.surface
                .as_ref()
                .unwrap()
                .uv
                .as_ref()
                .unwrap()
                .coordinates[0],
            [9., 9.]
        );
    }
    #[test]
    fn masks_rgb_and_explicit_material_rows_duplicate_by_geometry_not_uv() {
        let mut mesh = seam_fixture();
        mesh.masks = [0., 0.25, 0.5, 1.].to_vec();
        mesh.colors = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.], [1., 1., 1.]].to_vec();
        mesh.materials = Some([[0.1, 0.2], [0.3, 0.4], [0.5, 0.6], [0.7, 0.8]].to_vec());
        let packed = PackedUvMesh::build(&mesh, 4096).unwrap();
        assert_eq!(packed.vertices[3][6..], [0., 1., 0., 0.]);
        assert_eq!(packed.vertices[4][6..], [0.5, 0., 0., 1.]);
        assert_eq!(
            packed.materials.unwrap(),
            [
                [0.1, 0.2],
                [0.3, 0.4],
                [0.5, 0.6],
                [0.1, 0.2],
                [0.5, 0.6],
                [0.7, 0.8]
            ]
        );
    }
    #[test]
    fn invalid_or_unavailable_uv_and_insufficient_gpu_budget_reject_before_pack() {
        let mut mesh = seam_fixture();
        assert!(PackedUvMesh::build(&mesh, 1).is_err());
        mesh.surface
            .as_mut()
            .unwrap()
            .uv
            .as_mut()
            .unwrap()
            .face_indices[0][0] = 999;
        assert!(PackedUvMesh::build(&mesh, 4096).is_err());
        assert!(PackedUvMesh::build(&Mesh::cube(), 4096).is_err());
    }
    #[test]
    fn image_rgba_size_limit_and_vertical_orientation_are_independent_of_browser() {
        let rgba = [
            255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 0, 255, 255, 255, 255,
        ];
        let image = ViewTexture::from_rgba("analytic2x2", 2, 2, &rgba).unwrap();
        assert_eq!(
            image.pixels_bottom_up,
            [
                0, 0, 255, 0, 255, 255, 255, 255, 255, 0, 0, 255, 0, 255, 0, 128
            ]
        );
        assert_eq!(texture_storage_bytes(2048, 2048), Some(16 * 1024 * 1024));
        assert_eq!(texture_gpu_storage_bytes(2, 2), Some(20));
        assert_eq!(texture_gpu_storage_bytes(3, 2), Some(24));
        assert_eq!(texture_gpu_storage_bytes(2048, 2048), Some(22_369_620));
        for (w, h) in [(0, 1), (1, 0), (2049, 1), (u32::MAX, u32::MAX)] {
            assert!(texture_storage_bytes(w, h).is_none());
        }
        assert!(ViewTexture::from_rgba("short", 2, 2, &rgba[..12]).is_err());
    }
    #[test]
    fn shared_png_decoder_preserves_hidden_rgb_and_rejects_corrupt_header() {
        let bytes = include_bytes!("../tests/fixtures/independent-hidden-rgb2x2.png");
        let image = decode_png("known", bytes).unwrap();
        assert_eq!(
            image.pixels_bottom_up,
            [
                0, 0, 255, 255, 255, 255, 255, 64, 255, 0, 0, 0, 0, 255, 0, 128
            ]
        );
        assert!(decode_png("empty", &[]).is_err());
        let mut invalid = bytes.to_vec();
        invalid[16..20].copy_from_slice(&2049_u32.to_be_bytes());
        assert!(decode_png("huge", &invalid).is_err());
    }
    #[test]
    fn authored_uv_jpeg_decodes_bottom_up_and_rejects_invalid_or_oversize_inputs() {
        let jpeg = include_bytes!("../tests/fixtures/c294_uv_quadrants.jpg");
        let png = include_bytes!("../tests/fixtures/c294_uv_quadrants.png");
        let image = decode_view_image("quadrants.jpg", jpeg).unwrap();
        let lossless = decode_view_image("quadrants.png", png).unwrap();
        assert_eq!((image.width, image.height), (64, 64));
        assert_eq!((lossless.width, lossless.height), (64, 64));
        let pixel = |image: &ViewTexture, x: usize, y_top_down: usize| -> [u8; 4] {
            let offset = ((63 - y_top_down) * 64 + x) * 4;
            image.pixels_bottom_up[offset..offset + 4]
                .try_into()
                .expect("RGBA pixel")
        };
        for (x, y, expected) in [
            (8, 8, [236, 41, 31, 255]),
            (40, 8, [31, 211, 66, 255]),
            (8, 40, [36, 61, 226, 255]),
            (40, 40, [241, 216, 41, 255]),
        ] {
            assert_eq!(pixel(&lossless, x, y), expected);
            for (actual, target) in pixel(&image, x, y).into_iter().zip(expected) {
                assert!(u8::abs_diff(actual, target) <= 8);
            }
        }
        assert!(decode_view_image("corrupt.jpg", &[255, 216, 0, 0]).is_err());
        let marker = jpeg
            .windows(2)
            .position(|window| window == [255, 192] || window == [255, 194])
            .expect("Authored JPEG has a SOF marker");
        let mut oversize_header = jpeg.to_vec();
        oversize_header[marker + 7..marker + 9].copy_from_slice(&2049_u16.to_be_bytes());
        assert!(decode_view_image("wide.jpg", &oversize_header).is_err());
        assert!(decode_view_image("huge.jpg", &vec![255; 16 * 1024 * 1024 + 1]).is_err());
    }
    #[test]
    fn independent_rational_curvature_fields_and_premultiplied_blend() {
        let golden: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/renderer-analytic.json")).unwrap();
        let vector = |value: &serde_json::Value| {
            glam::Vec3::from_array(std::array::from_fn(|i| value[i].as_f64().unwrap() as f32))
        };
        for row in golden["curvature"].as_array().unwrap() {
            let n = vector(&row["normal"]);
            let dx = vector(&row["dx"]);
            let dy = vector(&row["dy"]);
            let cur = ((n - dx).cross(n + dx).y - (n - dy).cross(n + dy).x)
                * (row["strength"].as_f64().unwrap() as f32)
                * 80.0
                / (row["depth"].as_f64().unwrap() as f32);
            let dark = (-cur * 15.0).clamp(0.0, 1.0);
            let bright = (cur * 25.0).clamp(0.0, 1.0);
            let factor = (1.0 - dark * 0.7) * (1.0 - bright) + 2.0 * bright;
            assert!(
                (factor - row["expected_factor"].as_f64().unwrap() as f32).abs() < 1e-6,
                "{}",
                row["name"]
            );
        }
        let row = &golden["premultiplied_blend"];
        let alpha = row["opacity"].as_f64().unwrap() as f32;
        let actual =
            vector(&row["source_rgb"]) * alpha + vector(&row["background_rgb"]) * (1.0 - alpha);
        assert!((actual - vector(&row["expected_rgb"])).abs().max_element() < 1e-6);
    }
    #[test]
    fn seam_storage_accounts_for_separate_streams_and_rejects_overflow_before_allocation() {
        assert_eq!(uv_count_storage_upper_bound(6, 2), Some(360));
        assert!(uv_count_storage_upper_bound(usize::MAX, 2).is_none());
        assert!(uv_count_storage_upper_bound(6, usize::MAX).is_none());
        assert!(uv_count_storage_upper_bound(8_000_000, 2_000_000).is_none());
    }
    #[test]
    fn exact_srgb_transfer_includes_the_piecewise_join_and_endpoints() {
        assert_eq!(srgb_linear(0.), 0.);
        assert_eq!(srgb_linear(1.), 1.);
        assert!((srgb_linear(0.04045) - 0.003130805).abs() < 1e-8);
        assert!((srgb_linear(0.5) - 0.21404114).abs() < 1e-7);
    }
}

#[cfg(test)]
mod source_normal_seam_tests {
    use super::*;
    fn double_quad_seam() -> Mesh {
        Mesh::import_obj(b"v 0 0 0\nv 2 0 0\nv 2 1 1\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\nvt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\nf 1/1 2/2 3/3 4/4\nf 1/5 2/6 3/7 4/8\n").unwrap().mesh
    }
    #[test]
    fn every_seam_row_copies_shared_raw_original_quad_normal() {
        let m = double_quad_seam();
        let before = serde_json::to_vec(&m).unwrap();
        let f = obrush_core::SourceNormalField::compute(
            &m,
            obrush_core::SourceNormalPolicy::StaticOriginalFaces,
        )
        .unwrap();
        let packed = PackedUvMesh::build_with_source_normals(&m, 16384, &f).unwrap();
        assert_eq!(
            packed.correspondence,
            [
                [0, 0],
                [1, 1],
                [2, 2],
                [3, 3],
                [0, 4],
                [1, 5],
                [2, 6],
                [3, 7]
            ]
        );
        assert_eq!(
            packed.triangles,
            [[0, 1, 2], [0, 2, 3], [4, 5, 6], [4, 6, 7]]
        );
        assert!(
            packed
                .vertices
                .iter()
                .all(|row| row[3..6] == [-1., -2., 4.])
        );
        assert_eq!(before, serde_json::to_vec(&m).unwrap());
    }
    #[test]
    fn source_recipe_changes_only_normal_columns_of_existing_seam_layout() {
        let m = double_quad_seam();
        let f = obrush_core::SourceNormalField::compute(
            &m,
            obrush_core::SourceNormalPolicy::StaticOriginalFaces,
        )
        .unwrap();
        let old = PackedUvMesh::build(&m, 16384).unwrap();
        let new = PackedUvMesh::build_with_source_normals(&m, 16384, &f).unwrap();
        assert_eq!(old.correspondence, new.correspondence);
        assert_eq!(old.triangles, new.triangles);
        assert_eq!(old.materials, new.materials);
        assert_eq!(old.uv, new.uv);
        for (a, b) in old.vertices.iter().zip(&new.vertices) {
            assert_eq!(a[..3], b[..3]);
            assert_eq!(a[6..], b[6..]);
        }
        assert_ne!(old.vertices[1][3..6], new.vertices[1][3..6]);
    }
    #[test]
    fn invalid_field_and_seam_budget_never_use_a_legacy_substitute() {
        let mut m = double_quad_seam();
        let f = obrush_core::SourceNormalField::compute(
            &m,
            obrush_core::SourceNormalPolicy::StaticOriginalFaces,
        )
        .unwrap();
        assert!(PackedUvMesh::build_with_source_normals(&m, 1, &f).is_err());
        m.positions[0].x = 0.5;
        assert!(PackedUvMesh::build_with_source_normals(&m, 16384, &f).is_err());
    }
}
