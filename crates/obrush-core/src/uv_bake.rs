//! Display-free, bounded UV channel rasterization. Raster edge/overlap rules
//! are explicit CPU policies; reference GPU pixel equivalence is a separate gate.
use crate::{Mesh, DEFAULT_VERTEX_MATERIAL, TRIANGLE_CORNER};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};

pub const MAX_UV_BAKE_SCRATCH_BYTES: usize = 144 * 1024 * 1024;
pub const MAX_UV_BAKE_RASTER_WORK: u64 = 256 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UvBakeSize {
    S256,
    S512,
    #[default]
    S1024,
    S2048,
    S4096,
}
impl UvBakeSize {
    pub const ALL: [Self; 5] = [
        Self::S256,
        Self::S512,
        Self::S1024,
        Self::S2048,
        Self::S4096,
    ];
    pub fn pixels(self) -> usize {
        match self {
            Self::S256 => 256,
            Self::S512 => 512,
            Self::S1024 => 1024,
            Self::S2048 => 2048,
            Self::S4096 => 4096,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UvBakeChannel {
    Color,
    Roughness,
    Metallic,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UvBakePhase {
    Raster,
    Gutter,
    Complete,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UvBakeReport {
    pub size: usize,
    pub channel: UvBakeChannel,
    pub triangles: usize,
    pub degenerate_uv_triangles: usize,
    pub clipped_uv_triangles: usize,
    pub estimated_raster_work: u64,
    pub raster_work: u64,
    pub covered_pixels: usize,
    pub filled_gutter_pixels: usize,
    pub empty_black_pixels: usize,
    pub scratch_bytes_bound: usize,
}
pub struct UvBakeImage {
    /// PNG row zero corresponds to high V, as for the source PaintUV/readPixels path.
    pub rgba: Vec<u8>,
    pub report: UvBakeReport,
}

pub struct UvBakeExport {
    pub png: Vec<u8>,
    pub report: UvBakeReport,
}

pub fn bake_uv_with_cancel(
    mesh: Mesh,
    size: UvBakeSize,
    channel: UvBakeChannel,
    mut cancelled: impl FnMut() -> bool,
) -> Result<UvBakeExport, String> {
    if cancelled() {
        return Err("UV baking cancelled".into());
    }
    let mut job = UvBakeJob::new(mesh, size, channel)?;
    while job.phase() != UvBakePhase::Complete {
        if cancelled() {
            return Err("UV baking cancelled".into());
        }
        job.step(32_768)?;
    }
    let image = job.into_image()?;
    if cancelled() {
        return Err("UV baking cancelled".into());
    }
    let png = image.encode_png()?;
    if cancelled() {
        return Err("UV baking cancelled".into());
    }
    Ok(UvBakeExport {
        png,
        report: image.report,
    })
}

#[derive(Clone)]
struct TrianglePlan {
    p: [[f64; 2]; 3],
    c: [[f64; 3]; 3],
    area: f64,
    bounds: [usize; 4],
    clipped: bool,
}
fn edge(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
}
fn top_left(a: [f64; 2], b: [f64; 2]) -> bool {
    b[1] < a[1] || (b[1] == a[1] && b[0] > a[0])
}
fn byte(x: f64) -> u8 {
    (x.clamp(0.0, 1.0) * 255.0).round() as u8
}
fn linear(x: f64) -> f64 {
    if x < 0.04045 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}
fn srgb(x: f64) -> f64 {
    if x < 0.0031308 {
        x * 12.92
    } else {
        1.055 * x.max(0.0).powf(1.0 / 2.4) - 0.055
    }
}
fn allocated<T: Clone>(count: usize, value: T) -> Result<Vec<T>, String> {
    let mut v = Vec::new();
    v.try_reserve_exact(count)
        .map_err(|_| "Cannot allocate bounded UV bake storage")?;
    v.resize(count, value);
    Ok(v)
}
fn plan(
    mesh: &Mesh,
    face: usize,
    half: usize,
    n: usize,
    channel: UvBakeChannel,
) -> Option<TrianglePlan> {
    let s = mesh.surface.as_ref()?;
    let uv = s.uv.as_ref()?;
    let f = s.faces[face];
    let ids = uv.face_indices[face];
    let corners = if half == 0 { [0, 1, 2] } else { [0, 2, 3] };
    let mut p = [[0.0; 2]; 3];
    let mut c = [[0.0; 3]; 3];
    for i in 0..3 {
        let coord = uv.coordinates[ids[corners[i]] as usize];
        p[i] = [
            f64::from(coord[0]) * n as f64,
            (1.0 - f64::from(coord[1])) * n as f64,
        ];
        let vertex = f[corners[i]] as usize;
        c[i] = match channel {
            UvBakeChannel::Color => mesh.colors[vertex].map(f64::from),
            UvBakeChannel::Roughness | UvBakeChannel::Metallic => {
                let m = mesh
                    .materials
                    .as_ref()
                    .map_or(DEFAULT_VERTEX_MATERIAL, |m| m[vertex]);
                [f64::from(m[usize::from(channel == UvBakeChannel::Metallic)]); 3]
            }
        };
    }
    let mut area = edge(p[0], p[1], p[2]);
    if area == 0.0 {
        return None;
    }
    if area < 0.0 {
        p.swap(1, 2);
        c.swap(1, 2);
        area = -area;
    }
    let clamp = |x: f64| x.clamp(0.0, n as f64) as usize;
    let x0 = clamp(p.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min).floor());
    let x1 = clamp(
        p.iter()
            .map(|p| p[0])
            .fold(f64::NEG_INFINITY, f64::max)
            .ceil(),
    );
    let y0 = clamp(p.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min).floor());
    let y1 = clamp(
        p.iter()
            .map(|p| p[1])
            .fold(f64::NEG_INFINITY, f64::max)
            .ceil(),
    );
    Some(TrianglePlan {
        p,
        c,
        area,
        bounds: [x0, x1, y0, y1],
        clipped: p.iter().flatten().any(|&v| v < 0.0 || v > n as f64),
    })
}

/// Owns an immutable captured mesh. Hosts advance this on a worker; dropping the
/// job cancels it without a document/history mutation or a partially exported file.
pub struct UvBakeJob {
    mesh: Mesh,
    n: usize,
    channel: UvBakeChannel,
    phase: UvBakePhase,
    raster: Vec<u8>,
    output: Vec<u8>,
    columns: Vec<[f64; 4]>,
    lut: [f64; 256],
    face: usize,
    half: usize,
    current: Option<TrianglePlan>,
    cursor: [usize; 2],
    gutter_row: usize,
    report: UvBakeReport,
}
impl UvBakeJob {
    pub fn new(mesh: Mesh, size: UvBakeSize, channel: UvBakeChannel) -> Result<Self, String> {
        Self::with_dimension(mesh, size.pixels(), channel)
    }
    fn with_dimension(mesh: Mesh, n: usize, channel: UvBakeChannel) -> Result<Self, String> {
        mesh.validate()?;
        let surface = mesh
            .surface
            .as_ref()
            .filter(|s| s.uv.is_some())
            .ok_or("Selected mesh has no corner UV channel")?;
        let pixels = n.checked_mul(n).ok_or("UV bake pixel count overflow")?;
        let bytes = pixels.checked_mul(4).ok_or("UV bake image size overflow")?;
        let scratch = bytes
            .checked_mul(2)
            .and_then(|b| b.checked_add(n * std::mem::size_of::<[f64; 4]>()))
            .ok_or("UV bake scratch size overflow")?;
        if n == 0 || n > 4096 || scratch > MAX_UV_BAKE_SCRATCH_BYTES {
            return Err("UV bake scratch allocation limit exceeded".into());
        }
        let mut estimated = 0u64;
        let mut degenerates = 0;
        let mut clipped = 0;
        for (i, f) in surface.faces.iter().enumerate() {
            for half in 0..if f[3] == TRIANGLE_CORNER { 1 } else { 2 } {
                if let Some(p) = plan(&mesh, i, half, n, channel) {
                    let [x0, x1, y0, y1] = p.bounds;
                    estimated = estimated
                        .checked_add(((x1 - x0) as u64) * ((y1 - y0) as u64))
                        .ok_or("UV bake raster work overflow")?;
                    clipped += usize::from(p.clipped);
                    if estimated > MAX_UV_BAKE_RASTER_WORK {
                        return Err(
                            "UV bake raster work limit exceeded; reduce size or UV overlap".into(),
                        );
                    }
                } else {
                    degenerates += 1;
                }
            }
        }
        let report = UvBakeReport {
            size: n,
            channel,
            triangles: mesh.triangles.len(),
            degenerate_uv_triangles: degenerates,
            clipped_uv_triangles: clipped,
            estimated_raster_work: estimated,
            raster_work: 0,
            covered_pixels: 0,
            filled_gutter_pixels: 0,
            empty_black_pixels: 0,
            scratch_bytes_bound: scratch,
        };
        Ok(Self {
            mesh,
            n,
            channel,
            phase: UvBakePhase::Raster,
            raster: allocated(bytes, 0u8)?,
            output: Vec::new(),
            columns: Vec::new(),
            lut: std::array::from_fn(|i| linear(i as f64 / 255.0)),
            face: 0,
            half: 0,
            current: None,
            cursor: [0, 0],
            gutter_row: 0,
            report,
        })
    }
    pub fn phase(&self) -> UvBakePhase {
        self.phase
    }
    pub fn report(&self) -> &UvBakeReport {
        &self.report
    }
    fn advance_triangle(&mut self) {
        let quad = self
            .mesh
            .surface
            .as_ref()
            .is_some_and(|s| s.faces[self.face][3] != TRIANGLE_CORNER);
        if quad && self.half == 0 {
            self.half = 1;
        } else {
            self.half = 0;
            self.face += 1;
        }
        self.current = None;
    }
    fn pixel_sum(&self, x: usize, y: usize) -> [f64; 4] {
        let i = (y * self.n + x) * 4;
        if self.raster[i + 3] == 0 {
            [0.0; 4]
        } else {
            [
                self.lut[self.raster[i] as usize],
                self.lut[self.raster[i + 1] as usize],
                self.lut[self.raster[i + 2] as usize],
                1.0,
            ]
        }
    }
    fn wrap(&self, x: isize) -> usize {
        x.rem_euclid(self.n as isize) as usize
    }
    fn begin_gutter(&mut self) -> Result<(), String> {
        let output = allocated(self.raster.len(), 0u8)?;
        let mut columns = allocated(self.n, [0.0; 4])?;
        for offset in -8..=8 {
            let y = self.wrap(offset);
            for (x, column) in columns.iter_mut().enumerate() {
                let p = self.pixel_sum(x, y);
                for (dst, src) in column.iter_mut().zip(p) {
                    *dst += src;
                }
            }
        }
        self.output = output;
        self.columns = columns;
        self.phase = UvBakePhase::Gutter;
        Ok(())
    }
    /// Raster work is sliced at single pixels/triangle headers. Gutter work is
    /// sliced at rows (at most4096 pixels); the budget is a work hint, not a timer.
    pub fn step(&mut self, budget: usize) -> Result<UvBakePhase, String> {
        if budget == 0 || budget > 1_048_576 {
            return Err("UV bake step budget must be1–1048576".into());
        }
        let mut used = 0;
        while self.phase == UvBakePhase::Raster && used < budget {
            let face_count = self.mesh.surface.as_ref().map_or(0, |s| s.faces.len());
            if self.face == face_count {
                self.begin_gutter()?;
                break;
            }
            if self.current.is_none() {
                self.current = plan(&self.mesh, self.face, self.half, self.n, self.channel);
                used += 1;
                if let Some(p) = &self.current {
                    self.cursor = [p.bounds[0], p.bounds[2]];
                } else {
                    self.advance_triangle();
                    continue;
                }
                if used == budget {
                    break;
                }
            }
            let Some(p) = &self.current else {
                continue;
            };
            if self.cursor[0] >= p.bounds[1] || self.cursor[1] >= p.bounds[3] {
                self.advance_triangle();
                continue;
            }
            let at = [self.cursor[0] as f64 + 0.5, self.cursor[1] as f64 + 0.5];
            let e = [
                edge(p.p[1], p.p[2], at),
                edge(p.p[2], p.p[0], at),
                edge(p.p[0], p.p[1], at),
            ];
            let include = [
                top_left(p.p[1], p.p[2]),
                top_left(p.p[2], p.p[0]),
                top_left(p.p[0], p.p[1]),
            ];
            if e.iter()
                .zip(include)
                .all(|(&v, incl)| v > 0.0 || (v == 0.0 && incl))
            {
                let i = (self.cursor[1] * self.n + self.cursor[0]) * 4;
                if self.raster[i + 3] == 0 {
                    self.report.covered_pixels += 1;
                }
                for channel in 0..3 {
                    self.raster[i + channel] = byte(
                        (p.c[0][channel] * e[0] + p.c[1][channel] * e[1] + p.c[2][channel] * e[2])
                            / p.area,
                    );
                }
                self.raster[i + 3] = 255;
            }
            self.cursor[0] += 1;
            if self.cursor[0] == p.bounds[1] {
                self.cursor[0] = p.bounds[0];
                self.cursor[1] += 1;
            }
            used += 1;
            self.report.raster_work += 1;
        }
        while self.phase == UvBakePhase::Gutter && used < budget {
            let y = self.gutter_row;
            let mut window = [0.0; 4];
            for offset in -8..=8 {
                for (dst, src) in window.iter_mut().zip(self.columns[self.wrap(offset)]) {
                    *dst += src;
                }
            }
            for x in 0..self.n {
                let i = (y * self.n + x) * 4;
                if self.raster[i + 3] != 0 {
                    self.output[i..i + 4].copy_from_slice(&self.raster[i..i + 4]);
                } else {
                    if window[3] > 0.5 {
                        for (k, value) in window[..3].iter().enumerate() {
                            self.output[i + k] = byte(srgb(value / window[3]));
                        }
                        self.report.filled_gutter_pixels += 1;
                    } else {
                        self.report.empty_black_pixels += 1;
                    }
                    self.output[i + 3] = 255;
                }
                let old = self.columns[self.wrap(x as isize - 8)];
                let new = self.columns[self.wrap(x as isize + 9)];
                for k in 0..4 {
                    window[k] += new[k] - old[k];
                }
            }
            let old_y = self.wrap(y as isize - 8);
            let new_y = self.wrap(y as isize + 9);
            for x in 0..self.n {
                let old = self.pixel_sum(x, old_y);
                let new = self.pixel_sum(x, new_y);
                for k in 0..4 {
                    self.columns[x][k] += new[k] - old[k];
                }
            }
            self.gutter_row += 1;
            used += self.n;
            if self.gutter_row == self.n {
                self.raster = Vec::new();
                self.columns = Vec::new();
                self.phase = UvBakePhase::Complete;
            }
        }
        Ok(self.phase)
    }
    pub fn into_image(self) -> Result<UvBakeImage, String> {
        if self.phase != UvBakePhase::Complete {
            return Err("UV bake is not complete".into());
        }
        Ok(UvBakeImage {
            rgba: self.output,
            report: self.report,
        })
    }
}

struct BoundedPng {
    bytes: Vec<u8>,
    limit: usize,
}
impl Write for BoundedPng {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(io::Error::other("UV PNG output allocation limit exceeded"));
        }
        self.bytes
            .try_reserve_exact(bytes.len())
            .map_err(|_| io::Error::other("Cannot allocate UV PNG output"))?;
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl UvBakeImage {
    /// Stream encoding avoids a second whole compressed-image scratch buffer.
    pub fn encode_png(&self) -> Result<Vec<u8>, String> {
        let n = self.report.size;
        let count = n
            .checked_mul(n)
            .and_then(|n| n.checked_mul(4))
            .ok_or("UV PNG size overflow")?;
        if n == 0 || n > 4096 || self.rgba.len() != count {
            return Err("Invalid UV image dimensions or byte count".into());
        }
        let limit = count + n + 1024 * 1024;
        let mut sink = BoundedPng {
            bytes: Vec::new(),
            limit,
        };
        sink.bytes
            .try_reserve_exact(limit)
            .map_err(|_| "Cannot allocate bounded UV PNG output")?;
        let mut encoder = png::Encoder::new(&mut sink, n as u32, n as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::NoCompression);
        encoder.set_filter(png::Filter::NoFilter);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        {
            let mut stream = writer.stream_writer().map_err(|e| e.to_string())?;
            stream.write_all(&self.rgba).map_err(|e| e.to_string())?;
            stream.finish().map_err(|e| e.to_string())?;
        }
        writer.finish().map_err(|e| e.to_string())?;
        Ok(sink.bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CornerUv, Document, MaskConvention, SurfaceTopology, Vec3};
    fn quad() -> Mesh {
        let mut m = Mesh::cube();
        m.positions = vec![Vec3::ZERO, Vec3::X, Vec3::new(1.0, 1.0, 0.0), Vec3::Y];
        m.triangles = vec![[0, 1, 2], [0, 2, 3]];
        m.colors = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.5],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 0.5],
        ];
        m.masks = vec![1.0; 4];
        m.materials = Some(vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]);
        m.surface = Some(SurfaceTopology {
            faces: vec![[0, 1, 2, 3]],
            uv: Some(CornerUv {
                coordinates: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                face_indices: vec![[0, 1, 2, 3]],
            }),
        });
        m.recompute_normals();
        m.validate().unwrap();
        m
    }
    fn finished(m: Mesh, n: usize, channel: UvBakeChannel, budget: usize) -> UvBakeImage {
        let mut j = UvBakeJob::with_dimension(m, n, channel).unwrap();
        while j.phase() != UvBakePhase::Complete {
            j.step(budget).unwrap();
        }
        j.into_image().unwrap()
    }
    fn uv_mut(m: &mut Mesh) -> &mut CornerUv {
        m.surface.as_mut().unwrap().uv.as_mut().unwrap()
    }
    #[test]
    fn raster_affine_fields_and_high_v_first_png_rows() {
        let b = finished(quad(), 4, UvBakeChannel::Color, 1);
        assert_eq!(b.report.covered_pixels, 16);
        assert_eq!(b.report.filled_gutter_pixels, 0);
        for y in 0..4 {
            for x in 0..4 {
                let u = (x as f64 + 0.5) / 4.0;
                let v = 1.0 - (y as f64 + 0.5) / 4.0;
                let expected = [
                    (u * 255.0).round() as u8,
                    (v * 255.0).round() as u8,
                    ((u + v) * 127.5).round() as u8,
                    255,
                ];
                assert_eq!(&b.rgba[(y * 4 + x) * 4..][..4], &expected);
            }
        }
    }
    #[test]
    fn scalar_material_interpolation_is_grayscale_without_lighting() {
        for channel in [UvBakeChannel::Roughness, UvBakeChannel::Metallic] {
            let b = finished(quad(), 4, channel, 23);
            for y in 0..4 {
                for x in 0..4 {
                    let value = if channel == UvBakeChannel::Roughness {
                        (x as f64 + 0.5) / 4.0
                    } else {
                        (y as f64 + 0.5) / 4.0
                    };
                    let expected = [(value * 255.0).round() as u8; 3];
                    assert_eq!(&b.rgba[(y * 4 + x) * 4..][..3], &expected);
                }
            }
        }
    }
    #[test]
    fn implicit_materials_use_exact_declared_defaults() {
        let mut m = quad();
        m.materials = None;
        for (channel, index) in [(UvBakeChannel::Roughness, 0), (UvBakeChannel::Metallic, 1)] {
            let b = finished(m.clone(), 4, channel, 5);
            let e = (f64::from(DEFAULT_VERTEX_MATERIAL[index]) * 255.0).round() as u8;
            assert!(b.rgba.chunks_exact(4).all(|p| p == [e, e, e, 255]));
        }
    }
    #[test]
    fn masked_stored_colors_bake_unchanged_in_both_conventions() {
        let a = finished(quad(), 17, UvBakeChannel::Color, 6);
        let mut m = quad();
        m.mask_convention = MaskConvention::UnprotectedF32V1;
        m.masks.fill(0.0);
        let b = finished(m, 17, UvBakeChannel::Color, 800);
        assert_eq!(a.rgba, b.rgba);
    }
    #[test]
    fn step_size_does_not_change_final_image_or_report() {
        let a = finished(quad(), 23, UvBakeChannel::Color, 1);
        let b = finished(quad(), 23, UvBakeChannel::Color, 1_048_576);
        assert_eq!(a.rgba, b.rgba);
        assert_eq!(a.report, b.report);
    }
    #[test]
    fn opposite_winding_retains_interpolated_channels() {
        let original = quad();
        let mut m = original.clone();
        m.triangles = vec![[0, 3, 2], [0, 2, 1]];
        let s = m.surface.as_mut().unwrap();
        s.faces[0] = [0, 3, 2, 1];
        s.uv.as_mut().unwrap().face_indices[0] = [0, 3, 2, 1];
        m.recompute_normals();
        assert_eq!(
            finished(original, 16, UvBakeChannel::Color, 30).rgba,
            finished(m, 16, UvBakeChannel::Color, 5).rgba
        );
    }
    #[test]
    fn all_degenerate_uvs_are_black_opaque_and_reported() {
        let mut m = quad();
        uv_mut(&mut m).coordinates.fill([0.5, 0.5]);
        let b = finished(m, 32, UvBakeChannel::Color, 1);
        assert_eq!(b.report.degenerate_uv_triangles, 2);
        assert_eq!(b.report.covered_pixels, 0);
        assert_eq!(b.report.empty_black_pixels, 1024);
        assert!(b.rgba.chunks_exact(4).all(|p| p == [0, 0, 0, 255]));
    }
    #[test]
    fn uv_outside_unit_square_is_clipped_and_reported() {
        let mut m = quad();
        uv_mut(&mut m).coordinates = vec![[-1.0, -1.0], [2.0, -1.0], [2.0, 2.0], [-1.0, 2.0]];
        m.colors.fill([0.25, 0.5, 0.75]);
        let b = finished(m, 17, UvBakeChannel::Color, 12);
        assert_eq!(b.report.clipped_uv_triangles, 2);
        assert_eq!(b.report.covered_pixels, 289);
        assert!(b.rgba.chunks_exact(4).all(|p| p == [64, 128, 191, 255]));
    }
    #[test]
    fn fully_outside_uvs_produce_no_covered_pixels() {
        let mut m = quad();
        for c in &mut uv_mut(&mut m).coordinates {
            c[0] += 2.0;
        }
        let b = finished(m, 8, UvBakeChannel::Color, 1);
        assert_eq!(b.report.covered_pixels, 0);
        assert_eq!(b.report.estimated_raster_work, 0);
    }
    #[test]
    fn corner_seams_use_corner_ids_and_quad_diagonal() {
        let mut m = quad();
        let uv = uv_mut(&mut m);
        uv.coordinates.splice(0..0, [[10.0, 10.0], [20.0, 20.0]]);
        uv.face_indices[0] = [2, 3, 4, 5];
        let b = finished(m, 11, UvBakeChannel::Color, 7);
        assert_eq!(b.rgba, finished(quad(), 11, UvBakeChannel::Color, 4).rgba);
    }
    fn brute_gutter(raw: &[u8], n: usize) -> Vec<u8> {
        let mut out = raw.to_vec();
        for y in 0..n {
            for x in 0..n {
                let i = (y * n + x) * 4;
                if raw[i + 3] != 0 {
                    continue;
                }
                let mut sum = [0.0; 3];
                let mut count = 0;
                for dy in -8isize..=8 {
                    for dx in -8isize..=8 {
                        let at = (((y as isize + dy).rem_euclid(n as isize) as usize) * n
                            + (x as isize + dx).rem_euclid(n as isize) as usize)
                            * 4;
                        if raw[at + 3] != 0 {
                            count += 1;
                            for k in 0..3 {
                                let c = raw[at + k] as f64 / 255.0;
                                sum[k] += if c < 0.04045 {
                                    c / 12.92
                                } else {
                                    ((c + 0.055) / 1.055).powf(2.4)
                                };
                            }
                        }
                    }
                }
                for k in 0..3 {
                    let c = if count == 0 {
                        0.0
                    } else {
                        sum[k] / count as f64
                    };
                    let encoded = if c < 0.0031308 {
                        12.92 * c
                    } else {
                        1.055 * c.powf(1.0 / 2.4) - 0.055
                    };
                    out[i + k] = (encoded.clamp(0.0, 1.0) * 255.0).round() as u8;
                }
                out[i + 3] = 255;
            }
        }
        out
    }
    #[test]
    fn rolling_gutter_matches_independent_direct_wrapped_kernel() {
        for n in [1, 4, 17, 23] {
            let mut m = quad();
            for c in &mut uv_mut(&mut m).coordinates {
                c[0] *= 0.25;
                c[1] *= 0.25;
            }
            let mut j = UvBakeJob::with_dimension(m, n, UvBakeChannel::Color).unwrap();
            // Feed independently authored quantized coverage to the gutter.
            // A normal step may legitimately finish both phases for n=1;
            // the oracle must not assume a transient raster is observable.
            let mut raw = vec![0u8; n * n * 4];
            for y in 0..n {
                for x in 0..n {
                    if (x + 3 * y) % 7 < 2 {
                        let i = (y * n + x) * 4;
                        raw[i..i + 4].copy_from_slice(&[
                            ((x * 31 + y * 17) % 256) as u8,
                            ((x * 19 + y * 43) % 256) as u8,
                            ((x * 53 + y * 11) % 256) as u8,
                            255,
                        ]);
                    }
                }
            }
            j.raster.clone_from(&raw);
            j.begin_gutter().unwrap();
            let expected = brute_gutter(&raw, n);
            while j.phase != UvBakePhase::Complete {
                j.step(3).unwrap();
            }
            let actual = j.into_image().unwrap().rgba;
            assert!(
                actual
                    .iter()
                    .zip(expected)
                    .all(|(&a, e)| a.abs_diff(e) <= 1),
                "n={n}"
            );
        }
    }
    #[test]
    fn gutters_wrap_edges_do_not_overwrite_covered_or_fill_far_holes() {
        let mut m = quad();
        for c in &mut uv_mut(&mut m).coordinates {
            c[0] *= 0.02;
            c[1] *= 0.02;
        }
        m.colors.fill([1.0, 0.0, 0.0]);
        let b = finished(m, 64, UvBakeChannel::Color, 17);
        let get = |x: usize, y: usize| &b.rgba[(y * 64 + x) * 4..][..4];
        assert_eq!(get(0, 63), [255, 0, 0, 255]);
        assert_eq!(get(63, 63), [255, 0, 0, 255]);
        assert_eq!(get(32, 32), [0, 0, 0, 255]);
        assert!(b.report.filled_gutter_pixels > 0 && b.report.empty_black_pixels > 0);
        assert_eq!(
            b.report.covered_pixels + b.report.filled_gutter_pixels + b.report.empty_black_pixels,
            4096
        );
    }
    #[test]
    fn no_uv_and_malformed_corner_inputs_reject() {
        let mut m = quad();
        m.surface.as_mut().unwrap().uv = None;
        assert!(UvBakeJob::new(m, UvBakeSize::S256, UvBakeChannel::Color).is_err());
        let mut m = quad();
        uv_mut(&mut m).face_indices[0][2] = u32::MAX;
        assert!(UvBakeJob::new(m, UvBakeSize::S256, UvBakeChannel::Color).is_err());
        let mut m = quad();
        uv_mut(&mut m).coordinates[0][0] = f32::NAN;
        assert!(UvBakeJob::new(m, UvBakeSize::S256, UvBakeChannel::Color).is_err());
    }
    #[test]
    fn overlap_work_limit_rejects_before_output_allocation() {
        let mut m = quad();
        m.triangles = m.triangles.repeat(9);
        let s = m.surface.as_mut().unwrap();
        s.faces = s.faces.repeat(9);
        let u = s.uv.as_mut().unwrap();
        u.face_indices = u.face_indices.repeat(9);
        let error = UvBakeJob::new(m, UvBakeSize::S4096, UvBakeChannel::Color)
            .err()
            .unwrap();
        assert!(error.contains("work limit"));
    }
    #[test]
    fn size_roster_and_maximum_storage_bound_are_explicit() {
        assert_eq!(
            UvBakeSize::ALL.map(UvBakeSize::pixels),
            [256, 512, 1024, 2048, 4096]
        );
        assert_eq!(UvBakeSize::default().pixels(), 1024);
        for size in UvBakeSize::ALL {
            let j = UvBakeJob::new(quad(), size, UvBakeChannel::Color).unwrap();
            assert!(j.report.scratch_bytes_bound <= MAX_UV_BAKE_SCRATCH_BYTES);
            assert_eq!(j.phase(), UvBakePhase::Raster);
        }
    }
    #[test]
    fn incomplete_cancel_and_invalid_budget_do_not_change_source_or_redo() {
        let mut d = Document::new(quad());
        d.begin_stroke();
        d.require_active_mesh_mut().unwrap().colors[0] = [1.0, 0.0, 1.0];
        d.end_stroke();
        let edited = d.to_json().unwrap();
        assert!(d.undo());
        let before = d.to_json().unwrap();
        let mut j = UvBakeJob::new(
            d.require_active_mesh().unwrap().clone(),
            UvBakeSize::S256,
            UvBakeChannel::Color,
        )
        .unwrap();
        assert!(j.step(0).is_err());
        assert!(j.step(1_048_577).is_err());
        j.step(1).unwrap();
        assert!(j.into_image().is_err());
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), edited);
    }
    #[test]
    fn normal_256_png_decodes_to_exact_rgba_and_orientation() {
        let b = finished(quad(), 256, UvBakeChannel::Color, 6000);
        let bytes = b.encode_png().unwrap();
        assert!(bytes.len() < b.rgba.len() + 1024 * 1024);
        let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        let mut reader = decoder.read_info().unwrap();
        let mut decoded = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut decoded).unwrap();
        assert_eq!((info.width, info.height), (256, 256));
        assert_eq!(info.color_type, png::ColorType::Rgba);
        assert_eq!(&decoded[..info.buffer_size()], b.rgba);
        assert!(decoded[1] > 250 && decoded[decoded.len() - 3] < 5);
    }
    #[test]
    fn forged_image_byte_count_rejects_without_png_output() {
        let mut b = finished(quad(), 4, UvBakeChannel::Color, 100);
        b.rgba.pop();
        assert!(b.encode_png().is_err());
    }
    #[test]
    fn worker_execution_cancels_before_start_or_between_work_slices() {
        assert!(
            bake_uv_with_cancel(quad(), UvBakeSize::S256, UvBakeChannel::Color, || true)
                .err()
                .unwrap()
                .contains("cancelled")
        );
        let mut calls = 0;
        let result = bake_uv_with_cancel(quad(), UvBakeSize::S256, UvBakeChannel::Color, || {
            calls += 1;
            calls == 3
        });
        assert!(result.err().unwrap().contains("cancelled"));
        assert_eq!(calls, 3);
    }
    #[test]
    fn worker_export_report_matches_completed_png_and_does_not_mutate_source() {
        let source = quad();
        let before = serde_json::to_string(&source).unwrap();
        let export = bake_uv_with_cancel(
            source.clone(),
            UvBakeSize::S256,
            UvBakeChannel::Metallic,
            || false,
        )
        .unwrap();
        assert_eq!(serde_json::to_string(&source).unwrap(), before);
        assert_eq!(export.report.channel, UvBakeChannel::Metallic);
        assert_eq!(export.report.covered_pixels, 256 * 256);
        let report = serde_json::to_string(&export.report).unwrap();
        assert_eq!(
            serde_json::from_str::<UvBakeReport>(&report).unwrap(),
            export.report
        );
        let mut reader = png::Decoder::new(std::io::Cursor::new(export.png))
            .read_info()
            .unwrap();
        let mut decoded = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut decoded).unwrap();
        assert_eq!((info.width, info.height), (256, 256));
        assert!(decoded
            .chunks_exact(4)
            .all(|p| p[0] == p[1] && p[1] == p[2] && p[3] == 255));
    }
}
