//! Through-depth screen-region vertex masking, with frozen caller projection.
//! This deliberately has no GPU, front-face, visibility or occlusion query.
use crate::{Mesh, Vec3};
use serde::de::{Error as _, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

pub const MAX_MASK_REGION_POINTS: usize = 512;
pub const MAX_MASK_REGION_WORK: u64 = 32_000_000;
/// Absolute distance in the caller's screen units, for lasso boundaries only.
pub const MASK_REGION_BOUNDARY_EPSILON: f64 = 1e-7;
const MAX_SCREEN_COORDINATE: f64 = 1_000_000.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaskRegionKind {
    Rectangle,
    Lasso,
}

impl MaskRegionKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Rectangle => "Rectangle",
            Self::Lasso => "Lasso",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MaskRegion {
    Rectangle {
        start: [f64; 2],
        end: [f64; 2],
    },
    Lasso {
        #[serde(deserialize_with = "deserialize_points")]
        points: Vec<[f64; 2]>,
    },
}

impl MaskRegion {
    pub fn kind(&self) -> MaskRegionKind {
        match self {
            Self::Rectangle { .. } => MaskRegionKind::Rectangle,
            Self::Lasso { .. } => MaskRegionKind::Lasso,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaskRegionSettings {
    pub region: MaskRegion,
    /// Column-major OpenGL homogeneous matrix, e.g. Mat4::to_cols_array().
    pub view_projection: [f32; 16],
    /// Top-left origin x/y, then positive width/height in caller screen units.
    pub viewport: [f64; 4],
    pub erase: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaskRegionReport {
    pub kind: MaskRegionKind,
    pub erase: bool,
    pub vertices: usize,
    /// All clip-eligible vertices, even for a zero-extent rectangle.
    pub projected_vertices: usize,
    pub selected_vertices: usize,
    pub changed_vertices: usize,
    /// Rectangle has four conceptual corners; lasso uses raw input count.
    pub polygon_points: usize,
    /// Conservative preflight vertices*raw_lasso_points, rectangle zero.
    /// This is a work estimate, not measured comparisons or latency.
    pub point_edge_work: u64,
    pub max_abs_change: f64,
}

impl Mesh {
    /// Nonmutating hard vertex selection in a frozen top-left screen space.
    /// f32 coordinates/matrix coefficients are evaluated in f64, requiring
    /// finite clip values, w>0 and inclusive -w<=x/y/z<=w before division.
    /// Rectangle bounds are exact inclusive and reversed corners work; zero
    /// extent selects nothing. Lasso uses even-odd crossings and includes the
    /// absolute 1e-7 distance boundary band with an expanded segment bbox.
    /// Ordered repeated/closing points are retained, so repeated traces retain
    /// their even-odd meaning. Selected masks become 1 (erase 0), with exact old
    /// bits retained when numerically equal, including signed zero.
    pub fn region_masks(
        &self,
        settings: &MaskRegionSettings,
    ) -> Result<(Vec<f32>, MaskRegionReport), String> {
        let region = PreparedRegion::new(settings)?;
        self.validate()?;
        let (polygon_points, point_edge_work) = match &settings.region {
            MaskRegion::Rectangle { .. } => (4, 0),
            MaskRegion::Lasso { points } => {
                let work = (self.positions.len() as u64)
                    .checked_mul(points.len() as u64)
                    .filter(|&work| work <= MAX_MASK_REGION_WORK)
                    .ok_or("Region mask exceeds the 32000000 point-edge work estimate limit")?;
                (points.len(), work)
            }
        };
        // Counts/settings/work are checked before mask scratch allocation.
        let mut masks = Vec::new();
        masks
            .try_reserve_exact(self.masks.len())
            .map_err(|_| "Cannot allocate region masks")?;
        masks.extend_from_slice(&self.masks);
        let matrix = settings.view_projection.map(f64::from);
        let target = if settings.erase {
            self.mask_convention.clear_value()
        } else {
            self.mask_convention.full_value()
        };
        let mut report = MaskRegionReport {
            kind: settings.region.kind(),
            erase: settings.erase,
            vertices: self.positions.len(),
            projected_vertices: 0,
            selected_vertices: 0,
            changed_vertices: 0,
            polygon_points,
            point_edge_work,
            max_abs_change: 0.0,
        };
        for (vertex, &position) in self.positions.iter().enumerate() {
            let Some(screen) = project(position, &matrix, settings.viewport)? else {
                continue;
            };
            report.projected_vertices += 1;
            if region.contains(screen) {
                report.selected_vertices += 1;
                if masks[vertex] != target {
                    report.changed_vertices += 1;
                    report.max_abs_change = report
                        .max_abs_change
                        .max((f64::from(masks[vertex]) - f64::from(target)).abs());
                    masks[vertex] = target;
                }
            }
        }
        Ok((masks, report))
    }
}

fn project(
    point: Vec3,
    matrix: &[f64; 16],
    viewport: [f64; 4],
) -> Result<Option<[f64; 2]>, String> {
    let [x, y, z] = point.to_array().map(f64::from);
    let clip: [f64; 4] = std::array::from_fn(|row| {
        matrix[row] * x + matrix[row + 4] * y + matrix[row + 8] * z + matrix[row + 12]
    });
    if clip.iter().any(|v| !v.is_finite()) {
        return Err("Region projection produced nonfinite homogeneous coordinates".into());
    }
    let w = clip[3];
    if w <= 0.0 || clip[..3].iter().any(|coordinate| coordinate.abs() > w) {
        return Ok(None);
    }
    let screen = [
        viewport[0] + (clip[0] / w + 1.0) * 0.5 * viewport[2],
        viewport[1] + (1.0 - clip[1] / w) * 0.5 * viewport[3],
    ];
    if screen.iter().any(|v| !v.is_finite()) {
        return Err("Region projection produced nonfinite screen coordinates".into());
    }
    Ok(Some(screen))
}

enum PreparedRegion<'a> {
    Rectangle { min: [f64; 2], max: [f64; 2] },
    Lasso { points: &'a [[f64; 2]] },
}

impl<'a> PreparedRegion<'a> {
    fn new(settings: &'a MaskRegionSettings) -> Result<Self, String> {
        if settings.view_projection.iter().any(|v| !v.is_finite()) {
            return Err("Region projection matrix entries must be finite".into());
        }
        let [x, y, width, height] = settings.viewport;
        if !screen_point([x, y])
            || !width.is_finite()
            || !height.is_finite()
            || width <= 0.0
            || height <= 0.0
            || width > MAX_SCREEN_COORDINATE
            || height > MAX_SCREEN_COORDINATE
            || !screen_point([x + width, y + height])
        {
            return Err(
                "Region viewport needs finite positive extents and bounded ±1M screen corners"
                    .into(),
            );
        }
        match &settings.region {
            MaskRegion::Rectangle { start, end } => {
                if !screen_point(*start) || !screen_point(*end) {
                    return Err("Rectangle coordinates must be finite within ±1M".into());
                }
                Ok(Self::Rectangle {
                    min: [start[0].min(end[0]), start[1].min(end[1])],
                    max: [start[0].max(end[0]), start[1].max(end[1])],
                })
            }
            MaskRegion::Lasso { points } => {
                if !(3..=MAX_MASK_REGION_POINTS).contains(&points.len()) {
                    return Err("Lasso requires 3–512 stored points".into());
                }
                if points.iter().any(|&point| !screen_point(point)) {
                    return Err("Lasso coordinates must be finite within ±1M".into());
                }
                let first = points[0];
                let second = points.iter().copied().find(|&p| p != first);
                if !second.is_some_and(|second| {
                    points
                        .iter()
                        .any(|&point| noncollinear(first, second, point))
                }) {
                    return Err("Lasso requires three non-collinear distinct stored points".into());
                }
                Ok(Self::Lasso { points })
            }
        }
    }

    fn contains(&self, point: [f64; 2]) -> bool {
        match self {
            Self::Rectangle { min, max } => {
                min[0] < max[0]
                    && min[1] < max[1]
                    && point[0] >= min[0]
                    && point[0] <= max[0]
                    && point[1] >= min[1]
                    && point[1] <= max[1]
            }
            Self::Lasso { points } => {
                let mut inside = false;
                let mut previous = points[points.len() - 1];
                for &next in *points {
                    if on_segment(point, previous, next) {
                        return true;
                    }
                    if (previous[1] > point[1]) != (next[1] > point[1]) {
                        let crossing = previous[0]
                            + (point[1] - previous[1]) * (next[0] - previous[0])
                                / (next[1] - previous[1]);
                        if point[0] < crossing {
                            inside = !inside;
                        }
                    }
                    previous = next;
                }
                inside
            }
        }
    }
}

fn screen_point(point: [f64; 2]) -> bool {
    point
        .iter()
        .all(|v| v.is_finite() && v.abs() <= MAX_SCREEN_COORDINATE)
}

fn noncollinear(first: [f64; 2], second: [f64; 2], point: [f64; 2]) -> bool {
    let a = [second[0] - first[0], second[1] - first[1]];
    let b = [point[0] - first[0], point[1] - first[1]];
    if a[0] * b[1] - a[1] * b[0] != 0.0 {
        return true;
    }
    // Normalize directions independently: valid tiny finite triangles can
    // otherwise lose both products to underflow. No minimum extent/tolerance
    // is imposed by this validity check, and ordered points stay untouched.
    let a_scale = a[0].abs().max(a[1].abs());
    let b_scale = b[0].abs().max(b[1].abs());
    a_scale != 0.0
        && b_scale != 0.0
        && (a[0] / a_scale) * (b[1] / b_scale) - (a[1] / a_scale) * (b[0] / b_scale) != 0.0
}

fn on_segment(point: [f64; 2], a: [f64; 2], b: [f64; 2]) -> bool {
    let epsilon = MASK_REGION_BOUNDARY_EPSILON;
    if (0..2).any(|axis| {
        point[axis] < a[axis].min(b[axis]) - epsilon || point[axis] > a[axis].max(b[axis]) + epsilon
    }) {
        return false;
    }
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let px = point[0] - a[0];
    let py = point[1] - a[1];
    let length = dx.hypot(dy);
    if length == 0.0 {
        return px.hypot(py) <= epsilon;
    }
    let direction = [dx / length, dy / length];
    let along = px * direction[0] + py * direction[1];
    if along <= 0.0 {
        px.hypot(py) <= epsilon
    } else if along >= length {
        (point[0] - b[0]).hypot(point[1] - b[1]) <= epsilon
    } else {
        (px * direction[1] - py * direction[0]).abs() <= epsilon
    }
}

fn deserialize_points<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<[f64; 2]>, D::Error> {
    struct Points;
    impl<'de> Visitor<'de> for Points {
        type Value = Vec<[f64; 2]>;
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("at most 512 screen points")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            if seq.size_hint().is_some_and(|n| n > MAX_MASK_REGION_POINTS) {
                return Err(A::Error::custom("Lasso exceeds 512 stored points"));
            }
            let mut points = Vec::new();
            while let Some(point) = seq.next_element()? {
                if points.len() >= MAX_MASK_REGION_POINTS {
                    return Err(A::Error::custom("Lasso exceeds 512 stored points"));
                }
                if points.len() == points.capacity() {
                    points
                        .try_reserve_exact((MAX_MASK_REGION_POINTS - points.len()).min(32))
                        .map_err(|_| A::Error::custom("Cannot allocate lasso points"))?;
                }
                points.push(point);
            }
            Ok(points)
        }
    }
    deserializer.deserialize_seq(Points)
}
