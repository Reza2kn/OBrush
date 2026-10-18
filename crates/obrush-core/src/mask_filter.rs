//! Deterministic mask-only graph filters. This is an OBrush scalar policy,
//! not a claim of matching another sculpting application's response curves.
use crate::Mesh;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaskFilterOperation {
    Grow,
    Shrink,
    Blur,
    Sharpen,
}

impl MaskFilterOperation {
    pub const ALL: [Self; 4] = [Self::Grow, Self::Shrink, Self::Blur, Self::Sharpen];

    pub fn name(self) -> &'static str {
        match self {
            Self::Grow => "Grow",
            Self::Shrink => "Shrink",
            Self::Blur => "Blur",
            Self::Sharpen => "Sharpen",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaskFilterSettings {
    pub operation: MaskFilterOperation,
    pub iterations: u32,
    pub strength: f32,
}

impl Default for MaskFilterSettings {
    fn default() -> Self {
        Self {
            operation: MaskFilterOperation::Blur,
            iterations: 1,
            strength: 1.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaskFilterReport {
    pub operation: MaskFilterOperation,
    pub iterations: u32,
    pub strength: f32,
    pub vertices: usize,
    /// Final f32 bit differences relative to the input, not pass totals.
    pub changed_vertices: usize,
    pub max_abs_change: f64,
}

impl Mesh {
    /// One reference-style Masking Blur or Sharpen action. SculptGL stores
    /// unprotected weight U in material.z; OBrush's alternate protected
    /// convention is converted only for this calculation.
    pub fn sculptgl_filtered_masks(
        &self,
        operation: MaskFilterOperation,
    ) -> Result<(Vec<f32>, MaskFilterReport), String> {
        if !matches!(
            operation,
            MaskFilterOperation::Blur | MaskFilterOperation::Sharpen
        ) {
            return Err("SculptGL mask response is available for Blur and Sharpen".into());
        }
        self.validate()?;
        let mut result = reserved(self.masks.len(), "SculptGL mask values")?;
        result.extend_from_slice(&self.masks);
        let settings = MaskFilterSettings {
            operation,
            iterations: 1,
            strength: 1.0,
        };
        let unprotected = |value: f32| match self.mask_convention {
            crate::MaskConvention::UnprotectedF32V1 => value,
            crate::MaskConvention::ProtectedF32V1 => 1.0 - value,
        };
        let stored = |value: f32| match self.mask_convention {
            crate::MaskConvention::UnprotectedF32V1 => value,
            crate::MaskConvention::ProtectedF32V1 => 1.0 - value,
        };
        if operation == MaskFilterOperation::Sharpen {
            for (index, output) in result.iter_mut().enumerate() {
                let value = unprotected(self.masks[index]);
                if value < 1.0 {
                    let sharpened = if value > 0.5 {
                        (f64::from(value) + 0.1).min(1.0)
                    } else {
                        (f64::from(value) - 1.0).max(0.0)
                    } as f32;
                    *output = stored(sharpened);
                }
            }
            let summary = report(self, &settings, &result);
            return Ok((result, summary));
        }
        let (graph, on_edge) = source_graph(self)?;
        let mut selected = reserved(self.masks.len(), "SculptGL mask selection")?;
        selected.resize(self.masks.len(), false);
        for (index, &raw) in self.masks.iter().enumerate() {
            if unprotected(raw) < 1.0 {
                selected[index] = true;
                for &neighbor in graph.neighbors(index) {
                    selected[neighbor as usize] = true;
                }
            }
        }
        for (index, output) in result.iter_mut().enumerate() {
            if !selected[index] {
                continue;
            }
            let neighbors = graph.neighbors(index);
            if neighbors.len() <= 2 {
                continue;
            }
            let edge_count = neighbors
                .iter()
                .filter(|&&neighbor| on_edge[neighbor as usize])
                .count();
            let use_edge = on_edge[index] && edge_count >= 2;
            let sum = neighbors
                .iter()
                .filter(|&&neighbor| !use_edge || on_edge[neighbor as usize])
                .fold(0.0, |sum, &neighbor| {
                    sum + f64::from(unprotected(self.masks[neighbor as usize]))
                });
            let count = if use_edge {
                edge_count
            } else {
                neighbors.len()
            };
            *output = stored((sum / count as f64) as f32);
        }
        let summary = report(self, &settings, &result);
        Ok((result, summary))
    }

    /// Nonmutating synchronous passes: previous f32 values are calculated in
    /// f64 and rounded to f32 after each pass. Graph filters include self and
    /// each unique indexed neighbor once, in ascending neighbor-ID order.
    /// No coordinate welding, manifold requirement or geometry-normal update.
    /// Sharpen is pointwise, including isolated vertices; the graph operations
    /// leave isolated vertices unchanged. Fixed values retain their old bits.
    pub fn filtered_masks(
        &self,
        settings: &MaskFilterSettings,
    ) -> Result<(Vec<f32>, MaskFilterReport), String> {
        if !(1..=16).contains(&settings.iterations) {
            return Err("Mask filter iterations must be in 1–16".into());
        }
        if !settings.strength.is_finite() || !(0.0..=1.0).contains(&settings.strength) {
            return Err("Mask filter strength must be finite in 0–1".into());
        }
        // Mesh validation checks general count limits before scratch buffers.
        // Repeated-index faces remain invalid. Duplicate or geometrically
        // collapsed faces with distinct valid IDs are safe graph inputs.
        self.validate()?;
        let mut current = reserved(self.masks.len(), "mask values")?;
        current.extend_from_slice(&self.masks);
        if settings.strength == 0.0 {
            return Ok((current, report(self, settings, &self.masks)));
        }
        let graph = if settings.operation == MaskFilterOperation::Sharpen {
            None
        } else {
            Some(Graph::new(self)?)
        };
        let mut next = reserved(current.len(), "next mask values")?;
        next.resize(current.len(), 0.0);
        let strength = f64::from(settings.strength);
        let operation = match (self.mask_convention, settings.operation) {
            (crate::MaskConvention::UnprotectedF32V1, MaskFilterOperation::Grow) => {
                MaskFilterOperation::Shrink
            }
            (crate::MaskConvention::UnprotectedF32V1, MaskFilterOperation::Shrink) => {
                MaskFilterOperation::Grow
            }
            (_, value) => value,
        };
        for _ in 0..settings.iterations {
            for (vertex, output) in next.iter_mut().enumerate() {
                let old = current[vertex];
                let value = if let Some(graph) = &graph {
                    let neighbors = graph.neighbors(vertex);
                    if neighbors.is_empty() {
                        *output = old;
                        continue;
                    }
                    let old = f64::from(old);
                    let target = match operation {
                        MaskFilterOperation::Grow => neighbors
                            .iter()
                            .fold(old, |m, &v| m.max(f64::from(current[v as usize]))),
                        MaskFilterOperation::Shrink => neighbors
                            .iter()
                            .fold(old, |m, &v| m.min(f64::from(current[v as usize]))),
                        MaskFilterOperation::Blur => {
                            let sum = neighbors
                                .iter()
                                .fold(old, |sum, &v| sum + f64::from(current[v as usize]));
                            sum / (neighbors.len() + 1) as f64
                        }
                        MaskFilterOperation::Sharpen => unreachable!("Sharpen has no graph"),
                    };
                    old + strength * (target - old)
                } else {
                    // Explicit fixed-point handling preserves ±zero, .5 and
                    // 1 even when the scalar formula would canonicalize zero.
                    if old == 0.0 || old == 0.5 || old == 1.0 {
                        *output = old;
                        continue;
                    }
                    (0.5 + (f64::from(old) - 0.5) * (1.0 + strength)).clamp(0.0, 1.0)
                };
                let value = value as f32;
                *output = if value == old { old } else { value };
            }
            std::mem::swap(&mut current, &mut next);
        }
        let report = report(self, settings, &current);
        Ok((current, report))
    }
}

fn source_graph(mesh: &Mesh) -> Result<(Graph, Vec<bool>), String> {
    let edge_limit = mesh
        .surface
        .as_ref()
        .map_or(mesh.triangles.len().checked_mul(3), |surface| {
            surface.faces.len().checked_mul(4)
        })
        .ok_or("SculptGL mask edge-count overflow")?;
    let mut edges = reserved(edge_limit, "SculptGL mask edges")?;
    if let Some(surface) = &mesh.surface {
        for face in &surface.faces {
            let count = if face[3] == crate::TRIANGLE_CORNER {
                3
            } else {
                4
            };
            for slot in 0..count {
                let a = face[slot];
                let b = face[(slot + 1) % count];
                edges.push([a.min(b), a.max(b)]);
            }
        }
    } else {
        for &[a, b, c] in &mesh.triangles {
            for [a, b] in [[a, b], [b, c], [c, a]] {
                edges.push([a.min(b), a.max(b)]);
            }
        }
    }
    edges.sort_unstable();
    let mut boundary = reserved(mesh.positions.len(), "SculptGL mask boundary")?;
    boundary.resize(mesh.positions.len(), false);
    let mut unique = reserved(edges.len(), "SculptGL mask unique edges")?;
    let mut start = 0;
    while start < edges.len() {
        let edge = edges[start];
        let mut end = start + 1;
        while end < edges.len() && edges[end] == edge {
            end += 1;
        }
        if end == start + 1 {
            boundary[edge[0] as usize] = true;
            boundary[edge[1] as usize] = true;
        }
        unique.push(edge);
        start = end;
    }
    let count = mesh.positions.len();
    let mut offsets = reserved(count + 1, "SculptGL mask offsets")?;
    offsets.resize(count + 1, 0_u32);
    for &[a, b] in &unique {
        offsets[a as usize + 1] += 1;
        offsets[b as usize + 1] += 1;
    }
    for index in 1..offsets.len() {
        offsets[index] = offsets[index]
            .checked_add(offsets[index - 1])
            .ok_or("SculptGL mask neighbor-count overflow")?;
    }
    let mut indices = reserved(offsets[count] as usize, "SculptGL mask neighbors")?;
    indices.resize(offsets[count] as usize, 0);
    let mut cursors = reserved(count, "SculptGL mask cursors")?;
    cursors.extend_from_slice(&offsets[..count]);
    for [a, b] in unique {
        indices[cursors[a as usize] as usize] = b;
        indices[cursors[b as usize] as usize] = a;
        cursors[a as usize] += 1;
        cursors[b as usize] += 1;
    }
    Ok((Graph { offsets, indices }, boundary))
}

fn report(mesh: &Mesh, settings: &MaskFilterSettings, result: &[f32]) -> MaskFilterReport {
    let mut changed_vertices = 0;
    let mut max_abs_change: f64 = 0.0;
    for (&old, &new) in mesh.masks.iter().zip(result) {
        if old.to_bits() != new.to_bits() {
            changed_vertices += 1;
            max_abs_change = max_abs_change.max((f64::from(new) - f64::from(old)).abs());
        }
    }
    MaskFilterReport {
        operation: settings.operation,
        iterations: settings.iterations,
        strength: settings.strength,
        vertices: result.len(),
        changed_vertices,
        max_abs_change,
    }
}

struct Graph {
    offsets: Vec<u32>,
    indices: Vec<u32>,
}

impl Graph {
    fn new(mesh: &Mesh) -> Result<Self, String> {
        let edge_count = mesh
            .triangles
            .len()
            .checked_mul(3)
            .ok_or("Mask graph edge-count overflow")?;
        let mut edges = reserved(edge_count, "mask graph edges")?;
        for &[a, b, c] in &mesh.triangles {
            for [a, b] in [[a, b], [b, c], [c, a]] {
                if a != b {
                    edges.push([a.min(b), a.max(b)]);
                }
            }
        }
        edges.sort_unstable();
        edges.dedup();
        let count = mesh.positions.len();
        let mut offsets = reserved(count + 1, "mask graph offsets")?;
        offsets.resize(count + 1, 0_u32);
        for &[a, b] in &edges {
            offsets[a as usize + 1] += 1;
            offsets[b as usize + 1] += 1;
        }
        for vertex in 1..offsets.len() {
            offsets[vertex] = offsets[vertex]
                .checked_add(offsets[vertex - 1])
                .ok_or("Mask graph neighbor-count overflow")?;
        }
        let mut indices = reserved(offsets[count] as usize, "mask graph neighbors")?;
        indices.resize(offsets[count] as usize, 0);
        let mut cursors = reserved(count, "mask graph cursors")?;
        cursors.extend_from_slice(&offsets[..count]);
        for [a, b] in edges {
            indices[cursors[a as usize] as usize] = b;
            indices[cursors[b as usize] as usize] = a;
            cursors[a as usize] += 1;
            cursors[b as usize] += 1;
        }
        Ok(Self { offsets, indices })
    }
    fn neighbors(&self, vertex: usize) -> &[u32] {
        &self.indices[self.offsets[vertex] as usize..self.offsets[vertex + 1] as usize]
    }
}

fn reserved<T>(count: usize, label: &str) -> Result<Vec<T>, String> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| format!("Cannot allocate {label}"))?;
    Ok(values)
}
