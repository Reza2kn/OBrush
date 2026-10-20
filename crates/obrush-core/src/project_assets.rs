//! Bounded, display-only images stored in versioned OBrush projects.
//! Pixel rows are canonical bottom-up RGBA8, independent of the input codec.
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub const MAX_IMAGE_SIDE: u32 = 2048;
pub const MAX_IMAGE_RGBA_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_CUSTOM_MATCAPS: usize = 4;
pub const MAX_CUSTOM_MATCAP_GPU_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_ASSET_RGBA_BYTES: usize = 48 * 1024 * 1024;
const MAX_NAME_BYTES: usize = 320;
const PRESET_MATCAPS: u32 = 9;

mod base64_rgba {
    use super::*;

    pub fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        let encoded = String::deserialize(deserializer)?;
        if encoded.len() > MAX_IMAGE_RGBA_BYTES.div_ceil(3) * 4 {
            return Err(serde::de::Error::custom(
                "Asset base64 exceeds the image limit",
            ));
        }
        STANDARD.decode(encoded).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectImage {
    pub name: String,
    pub width: u32,
    pub height: u32,
    #[serde(with = "base64_rgba")]
    pub rgba_bottom_up: Vec<u8>,
}

impl ProjectImage {
    pub fn new(name: &str, width: u32, height: u32, rgba_bottom_up: &[u8]) -> Result<Self, String> {
        if name.len() > MAX_NAME_BYTES || name.contains('\0') {
            return Err("Project image name exceeds 320 bytes or contains NUL".into());
        }
        let bytes = rgba_bytes(width, height)
            .ok_or("Project image dimensions exceed the 2048 pixel / 16 MiB limit")?;
        if rgba_bottom_up.len() != bytes {
            return Err("Project image RGBA byte count differs from dimensions".into());
        }
        let image = Self {
            name: name.to_owned(),
            width,
            height,
            rgba_bottom_up: rgba_bottom_up.to_vec(),
        };
        image.validate()?;
        Ok(image)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.len() > MAX_NAME_BYTES || self.name.contains('\0') {
            return Err("Project image name exceeds 320 bytes or contains NUL".into());
        }
        let bytes = rgba_bytes(self.width, self.height)
            .ok_or("Project image dimensions exceed the 2048 pixel / 16 MiB limit")?;
        if self.rgba_bottom_up.len() != bytes {
            return Err("Project image RGBA byte count differs from dimensions".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectAssets {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uv: Option<ProjectImage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub matcaps: Vec<ProjectImage>,
    /// 0=studio, 1=PBR, 2=normal, 3=UV, 4=MatCap, 5=flat.
    pub shading: u8,
    pub matcap_index: u32,
}

impl ProjectAssets {
    pub fn empty() -> Self {
        Self {
            uv: None,
            matcaps: Vec::new(),
            shading: 0,
            matcap_index: 3,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.shading > 5 {
            return Err("Unknown project image shading mode".into());
        }
        if self.matcaps.len() > MAX_CUSTOM_MATCAPS {
            return Err("Project has more than four custom MatCaps".into());
        }
        if self.matcap_index >= PRESET_MATCAPS + self.matcaps.len() as u32 {
            return Err("Project selects a missing MatCap".into());
        }
        let mut total = 0_usize;
        if let Some(image) = &self.uv {
            image.validate()?;
            total = image.rgba_bottom_up.len();
        }
        let mut matcap_gpu = 0_usize;
        for image in &self.matcaps {
            image.validate()?;
            total = total
                .checked_add(image.rgba_bottom_up.len())
                .ok_or("Asset byte overflow")?;
            matcap_gpu = matcap_gpu
                .checked_add(
                    gpu_bytes(image.width, image.height).ok_or("MatCap GPU byte overflow")?,
                )
                .ok_or("MatCap GPU byte overflow")?;
        }
        if total > MAX_ASSET_RGBA_BYTES || matcap_gpu > MAX_CUSTOM_MATCAP_GPU_BYTES {
            return Err("Project images exceed the aggregate asset budget".into());
        }
        Ok(())
    }

    pub fn rgba_bytes(&self) -> usize {
        self.uv
            .as_ref()
            .map_or(0, |image| image.rgba_bottom_up.len())
            + self
                .matcaps
                .iter()
                .map(|image| image.rgba_bottom_up.len())
                .sum::<usize>()
    }
}

fn rgba_bytes(width: u32, height: u32) -> Option<usize> {
    if width == 0 || height == 0 || width > MAX_IMAGE_SIDE || height > MAX_IMAGE_SIDE {
        return None;
    }
    let bytes = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    (bytes <= MAX_IMAGE_RGBA_BYTES).then_some(bytes)
}

fn gpu_bytes(width: u32, height: u32) -> Option<usize> {
    let mut total = rgba_bytes(width, height)?;
    if width.is_power_of_two() && height.is_power_of_two() {
        let (mut w, mut h) = (width, height);
        while w > 1 || h > 1 {
            w = (w / 2).max(1);
            h = (h / 2).max(1);
            total = total.checked_add(rgba_bytes(w, h)?)?;
        }
    }
    Some(total)
}
