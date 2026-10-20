//! Pinned public SculptGL preset assets; notice/provenance is bundled beside them.
//! Preset bytes and view-position sampling math are separate from runtime proof.
use crate::uv_view::{ViewTexture, decode_jpeg, texture_gpu_storage_bytes};
use std::sync::{Arc, OnceLock};

pub(crate) const MATCAP_NAMES: [&str; 9] = [
    "matcap FV",
    "Red clay",
    "Skin hazardousarts",
    "Skin Hazardousarts2",
    "Pearl",
    "Clay",
    "Skin",
    "Green",
    "White",
];
const BYTES: [&[u8]; 9] = [
    include_bytes!("../assets/sculptgl/matcaps/matcapFV.jpg"),
    include_bytes!("../assets/sculptgl/matcaps/redClay.jpg"),
    include_bytes!("../assets/sculptgl/matcaps/skinHazardousarts.jpg"),
    include_bytes!("../assets/sculptgl/matcaps/skinHazardousarts2.jpg"),
    include_bytes!("../assets/sculptgl/matcaps/pearl.jpg"),
    include_bytes!("../assets/sculptgl/matcaps/clay.jpg"),
    include_bytes!("../assets/sculptgl/matcaps/skin.jpg"),
    include_bytes!("../assets/sculptgl/matcaps/green.jpg"),
    include_bytes!("../assets/sculptgl/matcaps/white.jpg"),
];
static DECODED: [OnceLock<Result<Arc<ViewTexture>, String>>; 9] = [const { OnceLock::new() }; 9];
pub(crate) const MAX_CUSTOM_MATCAPS: usize = 4;
pub(crate) const MAX_CUSTOM_GPU_BYTES: usize = 32 * 1024 * 1024;

/// A bounded viewport catalog. Project version 8 stores its canonical pixels
/// in the shared core; Source-object metadata retains stable built-in IDs.
#[derive(Clone, Debug, Default)]
pub(crate) struct CustomMatcaps {
    images: Vec<Arc<ViewTexture>>,
    gpu_bytes: usize,
}

impl CustomMatcaps {
    pub(crate) fn from_project(
        images: &[obrush_core::project_assets::ProjectImage],
        max_texture_side: u32,
    ) -> Result<Self, String> {
        let mut catalog = Self::default();
        for image in images {
            let texture = ViewTexture::from_project_image(image)?;
            catalog.add(texture, max_texture_side)?;
        }
        Ok(catalog)
    }
    pub(crate) fn len(&self) -> usize {
        self.images.len()
    }

    pub(crate) fn get(&self, index: u32) -> Option<Arc<ViewTexture>> {
        let offset = index.checked_sub(MATCAP_NAMES.len() as u32)? as usize;
        self.images.get(offset).cloned()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (u32, &Arc<ViewTexture>)> {
        self.images
            .iter()
            .enumerate()
            .map(|(i, image)| ((MATCAP_NAMES.len() + i) as u32, image))
    }

    pub(crate) fn gpu_bytes(&self) -> usize {
        self.gpu_bytes
    }

    pub(crate) fn add(&mut self, image: ViewTexture, max_texture_side: u32) -> Result<u32, String> {
        if self.images.len() >= MAX_CUSTOM_MATCAPS {
            return Err("Custom MatCap catalog is full (4 session images)".into());
        }
        if image.width > max_texture_side || image.height > max_texture_side {
            return Err("MatCap exceeds this GPU's texture-size limit".into());
        }
        let bytes = texture_gpu_storage_bytes(image.width, image.height)
            .ok_or("MatCap GPU allocation limit exceeded")?;
        let total = self
            .gpu_bytes
            .checked_add(bytes)
            .ok_or("MatCap GPU allocation limit exceeded")?;
        if total > MAX_CUSTOM_GPU_BYTES {
            return Err("Custom MatCaps exceed the 32 MiB GPU budget".into());
        }
        let id = (MATCAP_NAMES.len() + self.images.len()) as u32;
        self.images.push(Arc::new(image));
        self.gpu_bytes = total;
        Ok(id)
    }
}
pub(crate) fn preset(index: u32) -> Result<Arc<ViewTexture>, String> {
    let i = usize::try_from(index).map_err(|_| "Matcap index exceeds catalog")?;
    let cache = DECODED
        .get(i)
        .ok_or("Encoded Matcap index has no matching preset asset")?;
    cache
        .get_or_init(|| decode_jpeg(MATCAP_NAMES[i], BYTES[i]).map(Arc::new))
        .clone()
}
/// Independent scalar oracle for source's deliberately unnormalized x/y basis.
#[cfg(test)]
pub(crate) fn lookup(normal: glam::DVec3, preserved_eye: glam::DVec3) -> glam::DVec2 {
    let z = preserved_eye.normalize();
    let x = glam::DVec3::new(-z.z, 0.0, z.x);
    let y = x.cross(z);
    glam::DVec2::new(normal.dot(x), normal.dot(y)) * 0.5 + glam::DVec2::splat(0.5)
}

#[cfg(test)]
mod import_tests {
    use super::*;
    use crate::uv_view::decode_png;

    const PNG: &[u8] = include_bytes!("../tests/fixtures/c291_asymmetric_matcap.png");

    #[test]
    fn authored_image_decodes_with_explicit_bottom_row_and_stable_catalog_id() {
        let image = decode_png("c291_asymmetric_matcap.png", PNG).unwrap();
        assert_eq!((image.width, image.height), (64, 64));
        // Input is top-down; the GPU byte buffer begins with the bottom row.
        assert_eq!(&image.pixels_bottom_up[0..4], &[0, 255, 216, 255]);
        assert_eq!(
            &image.pixels_bottom_up[63 * 64 * 4..63 * 64 * 4 + 4],
            &[0, 0, 40, 255]
        );
        let mut catalog = CustomMatcaps::default();
        let first = catalog.add(image, 2048).unwrap();
        assert_eq!(first, MATCAP_NAMES.len() as u32);
        assert_eq!(
            catalog.get(first).unwrap().name,
            "c291_asymmetric_matcap.png"
        );
        assert_eq!(catalog.len(), 1);
        assert!(catalog.gpu_bytes() >= 64 * 64 * 4);
        assert!(catalog.get(first + 1).is_none());
    }

    #[test]
    fn invalid_inputs_and_capacity_reject_without_catalog_mutation() {
        assert!(decode_png("bad.png", b"not a png").is_err());
        assert!(decode_png("large.png", &vec![0; 16 * 1024 * 1024 + 1]).is_err());
        let mut header = PNG.to_vec();
        header[16..20].copy_from_slice(&2049_u32.to_be_bytes());
        assert!(decode_png("wide.png", &header).is_err());
        let image = decode_png("fixture.png", PNG).unwrap();
        let mut catalog = CustomMatcaps::default();
        assert!(catalog.add(image.clone(), 32).is_err());
        assert_eq!(catalog.len(), 0);
        for index in 0..MAX_CUSTOM_MATCAPS {
            assert_eq!(
                catalog.add(image.clone(), 2048).unwrap(),
                (9 + index) as u32
            );
        }
        let bytes = catalog.gpu_bytes();
        assert!(catalog.add(image, 2048).is_err());
        assert_eq!(catalog.len(), MAX_CUSTOM_MATCAPS);
        assert_eq!(catalog.gpu_bytes(), bytes);
    }

    #[test]
    fn public_jpeg_decoder_can_supply_custom_catalog() {
        let image = decode_jpeg(
            "author.jpg",
            include_bytes!("../assets/sculptgl/matcaps/clay.jpg"),
        )
        .unwrap();
        let mut catalog = CustomMatcaps::default();
        assert_eq!(catalog.add(image, 2048).unwrap(), 9);
    }

    #[test]
    fn cumulative_gpu_budget_rejects_before_mutating_catalog() {
        let rgba = vec![0_u8; 2048 * 2048 * 4];
        let image = ViewTexture::from_rgba("large.png", 2048, 2048, &rgba).unwrap();
        let mut catalog = CustomMatcaps::default();
        catalog.add(image.clone(), 2048).unwrap();
        let first_bytes = catalog.gpu_bytes();
        assert!(first_bytes > 16 * 1024 * 1024);
        assert!(catalog.add(image, 2048).is_err());
        assert_eq!(catalog.len(), 1);
        assert_eq!(catalog.gpu_bytes(), first_bytes);
    }
}
