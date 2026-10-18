//! Pinned public SculptGL preset assets; notice/provenance is bundled beside them.
//! Preset bytes and view-position sampling math are separate from runtime proof.
use crate::uv_view::{ViewTexture, decode_jpeg};
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
