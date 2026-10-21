use crate::{Mesh, RasterAlpha, Vec3, DEFAULT_VERTEX_MATERIAL, MAX_TRIANGLES, MAX_VERTICES};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[path = "draw_cage_solver.rs"]
mod draw_cage_solver;

#[cfg(test)]
#[path = "draw_anchor_regression.rs"]
mod draw_anchor_regression;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrushKind {
    Draw,
    Clay,
    Inflate,
    Smooth,
    Flatten,
    Crease,
    Move,
    Mask,
    Paint,
    Morph,
    Pinch,
    Twist,
    LocalScale,
    Drag,
}

impl BrushKind {
    pub const ALL: [Self; 14] = [
        Self::Draw,
        Self::Clay,
        Self::Inflate,
        Self::Smooth,
        Self::Flatten,
        Self::Crease,
        Self::Move,
        Self::Mask,
        Self::Paint,
        Self::Morph,
        Self::Pinch,
        Self::Twist,
        Self::LocalScale,
        Self::Drag,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Draw => "Draw",
            Self::Clay => "Clay",
            Self::Inflate => "Inflate",
            Self::Smooth => "Smooth",
            Self::Flatten => "Flatten",
            Self::Crease => "Crease",
            Self::Move => "Move",
            Self::Mask => "Mask",
            Self::Paint => "Paint",
            Self::Morph => "Morph",
            Self::Pinch => "Pinch",
            Self::Twist => "Twist",
            Self::LocalScale => "Local Scale",
            Self::Drag => "Drag",
        }
    }
}

impl std::fmt::Display for BrushKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BrushSettings {
    pub kind: BrushKind,
    /// World-space radius.
    pub radius: f32,
    /// Normalized brush strength, clamped to 0–1 when used.
    pub strength: f32,
    pub invert: bool,
    pub symmetry_x: bool,
    pub color: [f32; 3],
    /// Independently enabled Paint channels. Legacy input keeps RGB only.
    #[serde(default = "pressure_enabled")]
    pub paint_rgb: bool,
    #[serde(default)]
    pub paint_roughness: bool,
    #[serde(default)]
    pub paint_metallic: bool,
    /// Normalized material targets, validated even while their channel is off.
    #[serde(default = "default_roughness")]
    pub roughness: f32,
    #[serde(default = "default_metallic")]
    pub metallic: f32,
    /// Per-input normalized pressure. Mouse and absent pressure use one. A
    /// zero-pressure sample never changes the mesh, including color and masks.
    #[serde(default = "full_pressure")]
    pub pressure: f32,
    #[serde(default = "pressure_enabled")]
    pub pressure_strength: bool,
    #[serde(default)]
    pub pressure_radius: bool,
    #[serde(default)]
    pub alpha: Option<RasterAlpha>,
    /// Counterclockwise stamp rotation about the surface normal, in radians.
    #[serde(default)]
    pub alpha_rotation: f32,
    /// Optional surface tangent defining image-right before rotation. Hosts
    /// may use camera-right. Absent or parallel tangents use a world-axis basis.
    #[serde(default)]
    pub alpha_tangent: Option<Vec3>,
    /// Automatically add local geometry detail after an accepted surface dab.
    /// Persistent subdivision hosts disable this to keep level correspondence.
    #[serde(default = "adaptive_detail_enabled")]
    pub adaptive_detail: bool,
    /// Reject rear-facing vertices. Defaults preserve the established OBrush
    /// front-facing policy; disabling it permits all vertices in the footprint.
    #[serde(default = "pressure_enabled")]
    pub backface_culling: bool,
    /// Optional direction toward the camera/eye for culling. Hosts use the
    /// negative pointer-ray direction. None preserves surface-normal facing.
    /// A supplied axis must be finite and nonzero; invalid axes never fall back.
    #[serde(default)]
    pub culling_axis: Option<Vec3>,
    /// Radial profile hardness, clamped to 0–1. Zero preserves the original
    /// compact kernel; one gives constant radial influence inside the radius.
    #[serde(default)]
    pub hardness: f32,
    /// Smooth only within each vertex's current tangent plane.
    #[serde(default)]
    pub smooth_tangent: bool,
    /// Use planar Clay mathematics while retaining the Draw tool identity.
    #[serde(default)]
    pub draw_clay: bool,
    /// Measure Draw's footprint on current geometry. False uses the per-stroke
    /// proxy captured by DrawStroke, including interpolated refinement points.
    #[serde(default = "pressure_enabled")]
    pub draw_accumulate: bool,
    /// Restrict Move to the seed's radius-connected material patch.
    #[serde(default)]
    pub move_topology_check: bool,
    /// Constrain Move to its captured surface normal. Gesture hosts can supply
    /// signed pointer distance through MoveStroke::apply_normal_distance.
    #[serde(default)]
    pub move_along_normal: bool,
    /// The X reflection plane is x = symmetry_offset, in mesh/world units.
    #[serde(default)]
    pub symmetry_offset: f32,
}

impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            kind: BrushKind::Draw,
            radius: 0.25,
            strength: 0.5,
            invert: false,
            symmetry_x: true,
            color: [0.77, 0.34, 0.19],
            paint_rgb: true,
            paint_roughness: false,
            paint_metallic: false,
            roughness: DEFAULT_VERTEX_MATERIAL[0],
            metallic: DEFAULT_VERTEX_MATERIAL[1],
            pressure: 1.0,
            pressure_strength: true,
            pressure_radius: false,
            alpha: None,
            alpha_rotation: 0.0,
            alpha_tangent: None,
            adaptive_detail: true,
            backface_culling: true,
            culling_axis: None,
            hardness: 0.0,
            smooth_tangent: false,
            draw_clay: false,
            draw_accumulate: true,
            move_topology_check: false,
            move_along_normal: false,
            symmetry_offset: 0.0,
        }
    }
}

fn full_pressure() -> f32 {
    1.0
}
fn default_roughness() -> f32 {
    DEFAULT_VERTEX_MATERIAL[0]
}
fn default_metallic() -> f32 {
    DEFAULT_VERTEX_MATERIAL[1]
}

fn pressure_enabled() -> bool {
    true
}

fn adaptive_detail_enabled() -> bool {
    true
}

impl BrushSettings {
    pub(crate) fn paint_targets_are_valid(&self) -> bool {
        self.roughness.is_finite()
            && self.metallic.is_finite()
            && (0.0..=1.0).contains(&self.roughness)
            && (0.0..=1.0).contains(&self.metallic)
    }
    /// The radius actually used by a sample, or None for invalid/no-contact
    /// input. The host can use the same radius for cursor and dab spacing.
    pub fn effective_radius(&self) -> Option<f32> {
        if !self.pressure.is_finite() || !self.radius.is_finite() {
            return None;
        }
        let pressure = self.pressure.clamp(0.0, 1.0);
        if pressure == 0.0 {
            return None;
        }
        let radius = self.radius * if self.pressure_radius { pressure } else { 1.0 };
        (radius > 1e-6 && radius.is_finite()).then_some(radius)
    }
}

/// Actual committed effect of a dab. Geometry can take a reduced step to keep
/// the affected triangles from collapsing or reversing during this update.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BrushResult {
    pub changed_vertices: usize,
    /// Accepted fraction of the proposed deformation; zero for a rejected dab.
    /// This is one for an accepted color/mask dab.
    pub displacement_scale: f32,
    /// New editable points added after an accepted surface-sculpting dab.
    pub refined_vertices: usize,
    /// Refinement hit a work/allocation/topology limit; accepted geometry stays.
    pub detail_limited: bool,
}

/// A Move gesture samples its influence once on the resting material surface.
/// Hosts supply total world displacement from pointer-down, not an event delta.
/// Radius, alpha, symmetry, protection and facing are frozen; current strength,
/// pressure-strength and invert scale the absolute target. Hosts should restart
/// on semantic changes such as invert or switching brushes.
///
/// Preparation is non-mutating. Local linear refinement commits only with a
/// useful accepted drag, and returning to zero total displacement restores the
/// exact original topology and attributes. This is a bounded deformation, not
/// remeshing or global self-intersection repair.
pub struct MoveStroke {
    original: Mesh,
    prepared: Mesh,
    rest_positions: Vec<Vec3>,
    influences: Vec<(usize, Vec3)>,
    refinement: crate::PatchRefinement,
    at_origin: bool,
    normal: Vec3,
    along_normal: bool,
}

impl MoveStroke {
    /// Apply an absolute target atomically. Stale geometry/attributes, invalid
    /// input and zero contact leave the current mesh unchanged. A single shared
    /// step is tested against the rest surface, so guard acceptance does not
    /// weaken as a drag is split into more events.
    pub fn apply(
        &mut self,
        mesh: &mut Mesh,
        total_delta: Vec3,
        settings: &BrushSettings,
    ) -> BrushResult {
        let expected = if self.at_origin {
            &self.original
        } else {
            &self.prepared
        };
        if !mesh.same_surface(expected)
            || mesh.normals != expected.normals
            || !total_delta.is_finite()
            || !settings.strength.is_finite()
            || !settings.pressure.is_finite()
            || settings.pressure <= 0.0
            || settings.kind != BrushKind::Move
            || settings.move_along_normal != self.along_normal
            || !valid_brush_options(settings)
        {
            return BrushResult::default();
        }
        let strength = settings.strength.clamp(0.0, 1.0)
            * if settings.pressure_strength {
                settings.pressure.clamp(0.0, 1.0)
            } else {
                1.0
            };
        if strength == 0.0 {
            return BrushResult::default();
        }
        if total_delta == Vec3::ZERO {
            return self.restore_origin(mesh);
        }
        if self.influences.is_empty() {
            return BrushResult::default();
        }
        let sign = if settings.invert { -1.0 } else { 1.0 };
        let total_delta = if self.along_normal {
            self.normal * total_delta.dot(self.normal)
        } else {
            total_delta
        };
        // A tangential grab across a tiny footprint can turn neighboring
        // faces into a sharp wedge even though the old 90-degree fold guard
        // accepts it. Keep the lateral limit while the tangent component
        // dominates. Blend continuously toward the established extrusion
        // latitude only once the normal component dominates instead of
        // mistaking a screen-space pull on a curved surface for extrusion.
        let normal_fraction = total_delta.normalize_or_zero().dot(self.normal).abs();
        let tangent_normal_boundary = std::f32::consts::FRAC_1_SQRT_2;
        let extrusion_blend = ((normal_fraction - tangent_normal_boundary)
            / (1.0 - tangent_normal_boundary))
            .clamp(0.0, 1.0);
        let fold_cosine = 0.866_025_403_784_438_6 * (1.0 - f64::from(extrusion_blend));
        self.prepared
            .positions
            .copy_from_slice(&self.rest_positions);
        for &(vertex, influence) in &self.influences {
            let next = self.rest_positions[vertex] + total_delta * influence * strength * sign;
            if !next.is_finite() || next.abs().max_element() > 1_000_000.0 {
                self.restore_working_positions(mesh);
                return BrushResult::default();
            }
            self.prepared.positions[vertex] = next;
        }
        if self.prepared.positions == self.rest_positions {
            return self.restore_origin(mesh);
        }
        // Guard the complete target against rest, rather than allowing each
        // small event to accumulate a thinner or sharper surface.
        let scale = self.prepared.accept_deformation_with_limits(
            &self.rest_positions,
            0.1,
            fold_cosine,
            true,
        );
        if scale == 0.0 {
            self.restore_working_positions(mesh);
            return BrushResult::default();
        }
        if self.prepared.positions == self.rest_positions {
            return self.restore_origin(mesh);
        }
        let changed = changed_position_count(&mesh.positions, &self.prepared.positions);
        if changed == 0 {
            return BrushResult::default();
        }
        let dirty: Vec<_> = self
            .prepared
            .positions
            .iter()
            .enumerate()
            .filter_map(|(vertex, position)| {
                (mesh.positions.get(vertex) != Some(position)).then_some(vertex)
            })
            .collect();
        self.prepared.recompute_normals_near_vertices_cached(&dirty);
        let refined_vertices = if self.at_origin {
            self.refinement.vertices_added
        } else {
            0
        };
        if self.at_origin {
            *mesh = self.prepared.snapshot();
        } else {
            // Gesture topology and attributes are fixed. Reuse the caller's
            // allocations and caches instead of copying faces/masks/RGB again.
            mesh.positions.clone_from(&self.prepared.positions);
            mesh.normals.clone_from(&self.prepared.normals);
        }
        self.at_origin = false;
        BrushResult {
            changed_vertices: changed,
            displacement_scale: scale,
            refined_vertices,
            detail_limited: self.refinement.limited,
        }
    }

    /// An along-normal host supplies signed drag distance, typically pointer
    /// travel signed by horizontal direction. The captured normal and mirrored
    /// influences determine the displacement; arbitrary input event splitting
    /// does not change the final absolute target.
    pub fn apply_normal_distance(
        &mut self,
        mesh: &mut Mesh,
        signed_distance: f32,
        settings: &BrushSettings,
    ) -> BrushResult {
        if !self.along_normal || !signed_distance.is_finite() {
            return BrushResult::default();
        }
        self.apply(mesh, self.normal * signed_distance, settings)
    }

    fn restore_working_positions(&mut self, mesh: &Mesh) {
        if self.at_origin {
            self.prepared
                .positions
                .copy_from_slice(&self.rest_positions);
        } else {
            self.prepared.positions.copy_from_slice(&mesh.positions);
        }
    }

    fn restore_origin(&mut self, mesh: &mut Mesh) -> BrushResult {
        if self.at_origin {
            return BrushResult::default();
        }
        let changed = changed_position_count(&mesh.positions, &self.original.positions);
        *mesh = self.original.snapshot();
        self.prepared.positions.clone_from(&self.rest_positions);
        self.prepared.recompute_normals();
        self.at_origin = true;
        BrushResult {
            changed_vertices: changed,
            displacement_scale: 1.0,
            ..BrushResult::default()
        }
    }
}

/// Draw's optional non-accumulating footprint proxy. The shape continues to
/// receive dabs, while radial distances read this stroke's original material
/// positions instead of feeding deformed positions back into the footprint.
/// Adaptive points inherit their proxy through validated refinement ancestry.
pub struct DrawStroke {
    working: Mesh,
    proxy: Vec<Vec3>,
    accumulate: bool,
    clay: bool,
}

impl DrawStroke {
    pub fn proxy_positions(&self) -> &[Vec3] {
        &self.proxy
    }

    pub fn apply(
        &mut self,
        mesh: &mut Mesh,
        center: Vec3,
        normal: Vec3,
        settings: &BrushSettings,
    ) -> BrushResult {
        if settings.kind != BrushKind::Draw
            || settings.draw_accumulate != self.accumulate
            || settings.draw_clay != self.clay
            || !mesh.same_surface(&self.working)
            || mesh.normals != self.working.normals
        {
            return BrushResult::default();
        }
        let (result, ancestry) = self.working.apply_brush_dab(
            center,
            normal,
            Vec3::ZERO,
            settings,
            (!self.accumulate).then_some(self.proxy.as_slice()),
            false,
            false,
        );
        if result.changed_vertices == 0 {
            return result;
        }
        let additional = self
            .working
            .positions
            .len()
            .saturating_sub(self.proxy.len());
        if self.proxy.try_reserve(additional).is_err() {
            self.working = mesh.clone();
            return BrushResult::default();
        }
        for parents in &ancestry {
            if parents.vertex != self.proxy.len()
                || parents
                    .parents
                    .iter()
                    .any(|&parent| parent >= parents.vertex)
            {
                self.proxy.truncate(mesh.positions.len());
                self.working = mesh.clone();
                return BrushResult::default();
            }
            let point = parents
                .parents
                .iter()
                .zip(parents.weights)
                .fold(Vec3::ZERO, |sum, (&parent, weight)| {
                    sum + self.proxy[parent] * weight
                });
            if !point.is_finite() {
                self.proxy.truncate(mesh.positions.len());
                self.working = mesh.clone();
                return BrushResult::default();
            }
            self.proxy.push(point);
        }
        if self.proxy.len() != self.working.positions.len() {
            self.proxy.truncate(mesh.positions.len());
            self.working = mesh.clone();
            return BrushResult::default();
        }
        if mesh.positions.len() == self.working.positions.len() {
            mesh.positions.clone_from(&self.working.positions);
            mesh.normals.clone_from(&self.working.normals);
        } else {
            *mesh = self.working.snapshot();
        }
        result
    }
}

/// An anchored Twist or Local Scale gesture. The original footprint, masks,
/// normal-facing policy, alpha and mirror planes remain fixed throughout the
/// gesture. Hosts pass an absolute angle in radians for Twist, or a natural-log
/// scale for Local Scale, rather than summing accepted event displacements.
/// These gestures retain their original topology and restore its exact surface
/// when the absolute parameter returns to zero.
pub struct LocalTransformStroke {
    original: Mesh,
    working: Mesh,
    kind: BrushKind,
    centers: [Vec3; 2],
    axes: [Vec3; 2],
    influences: Vec<LocalTransformInfluence>,
    // Source Twist refreshes display normals for every picked vertex, even a
    // fully protected vertex whose position did not move.
    normal_selection: Vec<usize>,
    guard: Option<DeformationGuard>,
    seam_epsilon: Option<f32>,
    seam_offset: f32,
    at_origin: bool,
    accepted_parameter: f32,
}

struct LocalTransformInfluence {
    vertex: usize,
    blend: [f32; 2],
    amount: f32,
}

impl LocalTransformStroke {
    pub fn normal_selection(&self) -> &[usize] {
        &self.normal_selection
    }
    /// The accepted effective angle or log-scale, after strength and invert but
    /// before each point's frozen falloff and mask. A rejected/no-contact sample
    /// leaves this value and the mesh unchanged.
    pub fn accepted_parameter(&self) -> f32 {
        self.accepted_parameter
    }

    pub fn kind(&self) -> BrushKind {
        self.kind
    }

    /// Apply an absolute target against the original surface. Geometry guards
    /// reduce the parameter itself, then rebuild the corresponding rotation or
    /// scale; the result is never a linear interpolation of a rotated shape.
    /// `displacement_scale` reports the accepted parameter fraction. No local
    /// refinement occurs, so hierarchy/layer correspondence remains intact.
    pub fn apply(
        &mut self,
        mesh: &mut Mesh,
        total_parameter: f32,
        settings: &BrushSettings,
    ) -> BrushResult {
        let expected = if self.at_origin {
            &self.original
        } else {
            &self.working
        };
        if !mesh.same_surface(expected)
            || mesh.normals != expected.normals
            || settings.kind != self.kind
            || !total_parameter.is_finite()
            || !settings.strength.is_finite()
            || !settings.pressure.is_finite()
            || settings.pressure <= 0.0
            || !valid_brush_options(settings)
        {
            return BrushResult::default();
        }
        let strength = settings.strength.clamp(0.0, 1.0)
            * if settings.pressure_strength {
                settings.pressure.clamp(0.0, 1.0)
            } else {
                1.0
            };
        if strength == 0.0 {
            return BrushResult::default();
        }
        if total_parameter == 0.0 {
            return self.restore_origin(mesh);
        }
        let requested = total_parameter * strength * if settings.invert { -1.0 } else { 1.0 };
        if requested == 0.0 || self.influences.is_empty() {
            return BrushResult::default();
        }
        if self.guard.is_none() {
            return BrushResult::default();
        }
        let mut parameter = requested;
        if !self.safe_target(parameter) {
            // Search the transform parameter, not a chord through transformed
            // positions. An exponential scale's chord fraction badly
            // underestimates useful log-scale for large pointer requests.
            let mut safe = 0.0_f32;
            let mut unsafe_parameter = requested;
            for _ in 0..24 {
                let candidate = (safe + unsafe_parameter) * 0.5;
                if candidate == safe || candidate == unsafe_parameter {
                    break;
                }
                if self.safe_target(candidate) {
                    safe = candidate;
                } else {
                    unsafe_parameter = candidate;
                }
            }
            parameter = safe;
        }
        if parameter == 0.0 || !self.safe_target(parameter) {
            self.working.positions.clone_from(&mesh.positions);
            return BrushResult::default();
        }
        if self.working.positions == self.original.positions {
            return self.restore_origin(mesh);
        }
        let dirty: Vec<_> = mesh
            .positions
            .iter()
            .zip(&self.working.positions)
            .enumerate()
            .filter_map(|(vertex, (before, after))| (before != after).then_some(vertex))
            .collect();
        self.accepted_parameter = parameter;
        if dirty.is_empty() {
            return BrushResult::default();
        }
        self.working.recompute_normals_near_vertices_cached(&dirty);
        mesh.positions.clone_from(&self.working.positions);
        mesh.normals.clone_from(&self.working.normals);
        self.at_origin = false;
        BrushResult {
            changed_vertices: dirty.len(),
            displacement_scale: parameter / requested,
            ..BrushResult::default()
        }
    }

    fn safe_target(&mut self, parameter: f32) -> bool {
        self.working.positions.clone_from(&self.original.positions);
        self.set_target(parameter)
            && self
                .guard
                .as_ref()
                .is_some_and(|guard| guard.safe(&self.working.positions))
    }

    fn set_target(&mut self, parameter: f32) -> bool {
        for influence in &self.influences {
            let position = self.original.positions[influence.vertex];
            let amount = f64::from(parameter) * f64::from(influence.amount);
            let mut displacement = glam::DVec3::ZERO;
            for side in 0..2 {
                if influence.blend[side] == 0.0 {
                    continue;
                }
                let offset = (position - self.centers[side]).as_dvec3();
                let next_offset = match self.kind {
                    BrushKind::Twist => {
                        let axis = self.axes[side].as_dvec3();
                        let (sine, cosine) = amount.sin_cos();
                        offset * cosine
                            + axis.cross(offset) * sine
                            + axis * axis.dot(offset) * (1.0 - cosine)
                    }
                    BrushKind::LocalScale => offset * amount.exp(),
                    _ => unreachable!(),
                };
                displacement += (next_offset - offset) * f64::from(influence.blend[side]);
            }
            if self
                .seam_epsilon
                .is_some_and(|epsilon| (position.x - self.seam_offset).abs() <= epsilon)
            {
                displacement.x = 0.0;
            }
            let next = (position.as_dvec3() + displacement).as_vec3();
            if !next.is_finite() || next.abs().max_element() > 1_000_000.0 {
                return false;
            }
            self.working.positions[influence.vertex] = next;
        }
        true
    }

    fn restore_origin(&mut self, mesh: &mut Mesh) -> BrushResult {
        self.accepted_parameter = 0.0;
        if self.at_origin {
            return BrushResult::default();
        }
        let changed = changed_position_count(&mesh.positions, &self.original.positions);
        *mesh = self.original.snapshot();
        self.working.positions.clone_from(&self.original.positions);
        self.working.normals.clone_from(&self.original.normals);
        self.at_origin = true;
        BrushResult {
            changed_vertices: changed,
            displacement_scale: 1.0,
            ..BrushResult::default()
        }
    }
}

fn changed_position_count(first: &[Vec3], second: &[Vec3]) -> usize {
    first.iter().zip(second).filter(|(a, b)| a != b).count() + first.len().abs_diff(second.len())
}

impl Mesh {
    fn radius_connected_patch(
        &self,
        seed: usize,
        center: Vec3,
        radius_squared: f32,
    ) -> Result<Vec<bool>, String> {
        let count = self.positions.len();
        if seed >= count {
            return Err("Move topology seed references an invalid vertex".into());
        }
        let mut selected = Vec::new();
        selected
            .try_reserve_exact(count)
            .map_err(|_| "Cannot allocate Move topology footprint")?;
        selected.resize(count, false);
        let mut queue = Vec::new();
        queue
            .try_reserve_exact(count)
            .map_err(|_| "Cannot allocate Move topology traversal")?;
        let within =
            |vertex: usize| self.positions[vertex].distance_squared(center) < radius_squared;
        let entry = if within(seed) {
            Some(seed)
        } else {
            // A picked coarse face can have no original vertex in a tiny
            // footprint. Refinement may add one on its connected component.
            let mut component = vec![false; count];
            component[seed] = true;
            queue.push(seed);
            let mut cursor = 0;
            let mut entry = None;
            let mut nearest = f32::INFINITY;
            while cursor < queue.len() {
                let vertex = queue[cursor];
                cursor += 1;
                let distance = self.positions[vertex].distance_squared(center);
                if distance < radius_squared && distance < nearest {
                    nearest = distance;
                    entry = Some(vertex);
                }
                for &neighbor in self.cached_neighbors(vertex) {
                    if !component[neighbor] {
                        component[neighbor] = true;
                        queue.push(neighbor);
                    }
                }
            }
            entry
        };
        queue.clear();
        if let Some(entry) = entry {
            selected[entry] = true;
            queue.push(entry);
            let mut cursor = 0;
            while cursor < queue.len() {
                let vertex = queue[cursor];
                cursor += 1;
                for &neighbor in self.cached_neighbors(vertex) {
                    if !selected[neighbor] && within(neighbor) {
                        selected[neighbor] = true;
                        queue.push(neighbor);
                    }
                }
            }
        }
        Ok(selected)
    }

    /// Capture a Draw proxy without editing the mesh or its history. A single
    /// direct Mesh dab has no previous material proxy; hosts that expose the
    /// Accumulate option must keep this object for the complete gesture.
    pub fn prepare_draw_stroke(&self, settings: &BrushSettings) -> Result<DrawStroke, String> {
        self.validate()?;
        if settings.kind != BrushKind::Draw || !valid_brush_options(settings) {
            return Err("A Draw stroke requires valid Draw settings".into());
        }
        Ok(DrawStroke {
            working: self.clone(),
            proxy: self.positions.clone(),
            accumulate: settings.draw_accumulate,
            clay: settings.draw_clay,
        })
    }

    /// Prepare a non-mutating anchored local transformation. Surface normal is
    /// used only for facing and the alpha frame; Twist's separately supplied
    /// axis can be the captured camera-facing direction. Local Scale uniformly
    /// scales 3D offsets about the surface center. Both use a spherical radial
    /// footprint and maximum-strength, smoothly blended mirrored influences.
    pub fn prepare_local_transform_stroke(
        &self,
        center: Vec3,
        surface_normal: Vec3,
        rotation_axis: Vec3,
        settings: &BrushSettings,
    ) -> Result<LocalTransformStroke, String> {
        self.validate()?;
        if !matches!(settings.kind, BrushKind::Twist | BrushKind::LocalScale)
            || !center.is_finite()
            || !surface_normal.is_finite()
            || !rotation_axis.is_finite()
            || surface_normal.normalize_or_zero() == Vec3::ZERO
            || rotation_axis.normalize_or_zero() == Vec3::ZERO
            || !settings.radius.is_finite()
            || settings.radius <= 1e-6
            || !settings.strength.is_finite()
            || !settings.pressure.is_finite()
            || !settings.alpha_rotation.is_finite()
            || settings.alpha_tangent.is_some_and(|t| !t.is_finite())
            || !valid_brush_options(settings)
        {
            return Err("Local transforms require finite input, a positive radius, a surface normal and an axis".into());
        }
        let radius = settings.effective_radius().unwrap_or(settings.radius);
        let radius_squared = radius * radius;
        if !radius_squared.is_finite() {
            return Err("Local transform radius exceeds the representable footprint".into());
        }
        let normal = surface_normal.normalize_or_zero();
        let culling_axis = brush_culling_axis(settings, normal);
        let axis = rotation_axis.normalize_or_zero();
        let reflect = |v: Vec3| Vec3::new(-v.x, v.y, v.z);
        let alpha_frame =
            surface_alpha_frame(normal, settings.alpha_tangent, settings.alpha_rotation);
        let mut influences = Vec::new();
        let mut normal_selection = Vec::new();
        for (vertex, &position) in self.positions.iter().enumerate() {
            let protection = self.mask_influence(vertex);
            let vertex_normal = self.normals[vertex].normalize_or_zero();
            let mut weights = [0.0_f32; 2];
            let mut picked = false;
            for (side, weight) in weights.iter_mut().enumerate() {
                if side == 1 && !settings.symmetry_x {
                    continue;
                }
                let mirror = |v| if side == 1 { reflect(v) } else { v };
                let stamp_center = if side == 1 {
                    reflect_point(center, settings.symmetry_offset)
                } else {
                    center
                };
                let offset = position - stamp_center;
                let distance_squared = offset.length_squared();
                let facing = vertex_normal.dot(mirror(culling_axis));
                if distance_squared >= radius_squared
                    || (settings.backface_culling && facing <= 0.0)
                {
                    continue;
                }
                picked = true;
                if protection == 0.0 {
                    continue;
                }
                *weight = radial_influence(distance_squared, radius_squared, settings, false);
                if settings.backface_culling {
                    let front = (facing / 0.35).clamp(0.0, 1.0);
                    // Preserve the anchored transform's established default
                    // floating-point multiplication order.
                    *weight = *weight * front * front * (3.0 - 2.0 * front);
                }
                if let Some(alpha) = &settings.alpha {
                    let (tangent, bitangent) = alpha_frame;
                    *weight *= alpha.sample(
                        0.5 + offset.dot(mirror(tangent)) / (2.0 * radius),
                        0.5 - offset.dot(mirror(bitangent)) / (2.0 * radius),
                    );
                }
            }
            if picked {
                normal_selection.push(vertex);
            }
            let maximum = weights[0].max(weights[1]) * protection;
            let sum = weights[0] + weights[1];
            if maximum > 0.0 && sum > 0.0 {
                influences.push(LocalTransformInfluence {
                    vertex,
                    blend: weights.map(|weight| weight / sum),
                    amount: maximum,
                });
            }
        }
        let mut working = self.snapshot();
        working.ensure_adjacency();
        let selected: Vec<_> = influences
            .iter()
            .map(|influence| influence.vertex)
            .collect();
        let guard = working.deformation_guard(&self.positions, &selected, 0.1, 0.0, false);
        Ok(LocalTransformStroke {
            original: self.snapshot(),
            working,
            kind: settings.kind,
            centers: [center, reflect_point(center, settings.symmetry_offset)],
            // A rotation axis is an axial vector. Reflection reverses its
            // handedness, yielding exactly reflected Twist displacements.
            axes: [axis, -reflect(axis)],
            influences,
            normal_selection,
            guard,
            seam_epsilon: settings.symmetry_x.then_some(radius * 1e-5),
            seam_offset: settings.symmetry_offset,
            at_origin: true,
            accepted_parameter: 0.0,
        })
    }

    /// Capture a Move gesture without changing the document. At most four local
    /// rounds of 512 new vertices sample the undeformed falloff. Hierarchy hosts
    /// set adaptive_detail=false, retaining exact vertex/face correspondence.
    /// Zero pressure at pointer-down uses the nominal radius for selection;
    /// later positive contact can activate the gesture. Pressure-radius hosts
    /// may defer preparation until contact to capture its pressure-scaled radius.
    pub fn prepare_move_stroke(
        &self,
        center: Vec3,
        normal: Vec3,
        settings: &BrushSettings,
    ) -> Result<MoveStroke, String> {
        self.prepare_move_stroke_with_seed(center, normal, settings, None)
    }

    /// A picked triangle disambiguates nearby disconnected surfaces when
    /// topology checking is enabled. With no triangle, the closest source
    /// vertex is the explicit fallback. Connectivity is then restricted to
    /// paths that stay inside each resting spherical footprint.
    pub fn prepare_move_stroke_with_seed(
        &self,
        center: Vec3,
        normal: Vec3,
        settings: &BrushSettings,
        seed_triangle: Option<[u32; 3]>,
    ) -> Result<MoveStroke, String> {
        self.validate()?;
        if settings.kind != BrushKind::Move
            || !center.is_finite()
            || !normal.is_finite()
            || normal.normalize_or_zero() == Vec3::ZERO
            || !settings.radius.is_finite()
            || settings.radius <= 1e-6
            || !settings.strength.is_finite()
            || !settings.pressure.is_finite()
            || !settings.alpha_rotation.is_finite()
            || settings.alpha_tangent.is_some_and(|t| !t.is_finite())
            || !valid_brush_options(settings)
        {
            return Err(
                "Move requires finite input, a positive radius and a surface normal".into(),
            );
        }
        let radius = settings.effective_radius().unwrap_or(settings.radius);
        let radius_squared = radius * radius;
        if !radius_squared.is_finite() {
            return Err("Move radius exceeds the representable footprint".into());
        }
        let original = self.snapshot();
        let mut prepared = self.snapshot();
        let refinement = if settings.adaptive_detail
            && settings.alpha.as_ref().is_none_or(RasterAlpha::has_signal)
        {
            let refinement = if settings.symmetry_offset == 0.0 {
                prepared.refine_move_patch(center, radius, settings.symmetry_x)
            } else {
                prepared.refine_move_patch_at_offset(
                    center,
                    radius,
                    settings.symmetry_x,
                    settings.symmetry_offset,
                )
            };
            match refinement {
                Ok(result) => result,
                // Refinement is atomic. A valid mesh at its capacity may still
                // be deformed safely with its existing topology; malformed
                // selected geometry continues to be an explicit error.
                Err(error)
                    if error.starts_with("Patch refinement exceeds")
                        || error.starts_with("Cannot allocate refinement") =>
                {
                    crate::PatchRefinement {
                        limited: true,
                        ..crate::PatchRefinement::default()
                    }
                }
                Err(error) => return Err(error),
            }
        } else {
            crate::PatchRefinement::default()
        };
        let normal = normal.normalize_or_zero();
        let culling_axis = brush_culling_axis(settings, normal);
        let reflect = |v: Vec3| Vec3::new(-v.x, v.y, v.z);
        let alpha_frame =
            surface_alpha_frame(normal, settings.alpha_tangent, settings.alpha_rotation);
        prepared.ensure_adjacency();
        let seed = match seed_triangle {
            Some(vertices) => {
                if vertices
                    .iter()
                    .any(|&vertex| vertex as usize >= self.positions.len())
                {
                    return Err("Move topology seed references an invalid vertex".into());
                }
                vertices
                    .into_iter()
                    .min_by(|&a, &b| {
                        self.positions[a as usize]
                            .distance_squared(center)
                            .total_cmp(&self.positions[b as usize].distance_squared(center))
                    })
                    .map(|vertex| vertex as usize)
            }
            None if settings.move_topology_check => nearest_vertex(&self.positions, center),
            None => None,
        };
        let topology = if settings.move_topology_check {
            let Some(seed) = seed else {
                return Err("Move topology checking requires a source vertex".into());
            };
            let primary = prepared.radius_connected_patch(seed, center, radius_squared)?;
            let mirrored = if settings.symmetry_x {
                let source_seed = self.positions[seed];
                let mirrored_seed = nearest_vertex(
                    &prepared.positions,
                    reflect_point(source_seed, settings.symmetry_offset),
                )
                .unwrap_or(seed);
                prepared.radius_connected_patch(
                    mirrored_seed,
                    reflect_point(center, settings.symmetry_offset),
                    radius_squared,
                )?
            } else {
                vec![false; prepared.positions.len()]
            };
            Some([primary, mirrored])
        } else {
            None
        };
        let mut influences = Vec::new();
        for (vertex, &position) in prepared.positions.iter().enumerate() {
            let protection = prepared.mask_influence(vertex);
            if protection == 0.0 {
                continue;
            }
            let vertex_normal = prepared.normals[vertex].normalize_or_zero();
            let mut weights = [0.0_f32; 2];
            for (mirrored, weight) in weights.iter_mut().enumerate() {
                if mirrored == 1 && !settings.symmetry_x {
                    continue;
                }
                if topology
                    .as_ref()
                    .is_some_and(|patches| !patches[mirrored][vertex])
                {
                    continue;
                }
                let mirror = |v| if mirrored == 1 { reflect(v) } else { v };
                let stamp_center = if mirrored == 1 {
                    reflect_point(center, settings.symmetry_offset)
                } else {
                    center
                };
                let offset = position - stamp_center;
                let distance_squared = offset.length_squared();
                let facing = vertex_normal.dot(mirror(culling_axis));
                if distance_squared >= radius_squared
                    || (settings.backface_culling && facing <= 0.0)
                {
                    continue;
                }
                // C2 compact radial field, sampled on the resting geometry.
                *weight = radial_influence(distance_squared, radius_squared, settings, true)
                    * facing_influence(facing, settings);
                if let Some(alpha) = &settings.alpha {
                    let (tangent, bitangent) = alpha_frame;
                    *weight *= alpha.sample(
                        0.5 + offset.dot(mirror(tangent)) / (2.0 * radius),
                        0.5 - offset.dot(mirror(bitangent)) / (2.0 * radius),
                    );
                }
            }
            let maximum = weights[0].max(weights[1]) * protection;
            let sum = weights[0] + weights[1];
            if maximum == 0.0 || sum == 0.0 {
                continue;
            }
            let mut x = maximum * (weights[0] - weights[1]) / sum;
            if settings.symmetry_x && (position.x - settings.symmetry_offset).abs() <= radius * 1e-5
            {
                x = 0.0;
            }
            influences.push((vertex, Vec3::new(x, maximum, maximum)));
        }
        let rest_positions = prepared.positions.clone();
        // Private gesture topology stays fixed. Build the cache once; public
        // mesh edits are rejected by the exact expected-surface comparison.
        prepared.ensure_adjacency();
        Ok(MoveStroke {
            original,
            prepared,
            rest_positions,
            influences,
            refinement,
            at_origin: true,
            normal,
            along_normal: settings.move_along_normal,
        })
    }

    /// Apply one brush dab. A stroke is a sequence of dabs grouped by `Document`.
    ///
    /// Distances and smoothing read the surface before the dab, making behavior
    /// independent of vertex ordering. Fully masked vertices are protected from
    /// every brush except the mask brush. Mirrored influences are combined using
    /// their maximum, so the symmetry seam never receives a doubled dab.
    pub fn apply_brush(
        &mut self,
        center: Vec3,
        normal: Vec3,
        delta: Vec3,
        settings: &BrushSettings,
    ) -> usize {
        self.apply_brush_with_result(center, normal, delta, settings)
            .changed_vertices
    }

    /// Apply a dab and report its accepted step, so a Move anchor can track the
    /// committed surface rather than the full, possibly unsafe pointer delta.
    pub fn apply_brush_with_result(
        &mut self,
        center: Vec3,
        normal: Vec3,
        delta: Vec3,
        settings: &BrushSettings,
    ) -> BrushResult {
        if let Some(result) = self.try_local_draw_window(center, normal, delta, settings) {
            return result;
        }
        self.apply_brush_dab(center, normal, delta, settings, None, true, true)
            .0
    }

    /// Only the exclusive validated-contact owner may use this entry point.
    pub(crate) fn apply_brush_at_validated_draw_hit(
        &mut self,
        hit: crate::Hit,
        settings: &BrushSettings,
        validated_source: crate::validated_draw_contact::ValidatedDrawSource,
    ) -> BrushResult {
        if let Some(result) = self.try_local_draw_window_from_validated_source(
            hit.position,
            hit.normal,
            Vec3::ZERO,
            settings,
            validated_source,
        ) {
            return result;
        }
        self.apply_brush_dab(
            hit.position,
            hit.normal,
            Vec3::ZERO,
            settings,
            None,
            true,
            true,
        )
        .0
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply_brush_dab(
        &mut self,
        center: Vec3,
        normal: Vec3,
        delta: Vec3,
        settings: &BrushSettings,
        proxy_positions: Option<&[Vec3]>,
        allow_constraint_repair: bool,
        allow_interior_prepare: bool,
    ) -> (BrushResult, Vec<crate::refinement::RefinementParents>) {
        if settings.kind == BrushKind::Draw && settings.draw_clay {
            let mut clay = settings.clone();
            clay.kind = BrushKind::Clay;
            clay.draw_clay = false;
            return self.apply_brush_dab(
                center,
                normal,
                delta,
                &clay,
                proxy_positions,
                false,
                false,
            );
        }
        // A target belongs to a Document and its subdivision correspondence.
        // Calling the generic mesh API cannot silently invent that target.
        if settings.kind == BrushKind::Morph {
            return (BrushResult::default(), Vec::new());
        }
        // Gesture hosts should keep the prepared stroke and pass absolute
        // targets. The direct dab API also supports an explicit angular or
        // log-scale increment in delta.x about the supplied surface normal.
        if matches!(settings.kind, BrushKind::Twist | BrushKind::LocalScale) {
            if !delta.is_finite() {
                return (BrushResult::default(), Vec::new());
            }
            return (
                self.prepare_local_transform_stroke(center, normal, normal, settings)
                    .map(|mut stroke| stroke.apply(self, delta.x, settings))
                    .unwrap_or_default(),
                Vec::new(),
            );
        }
        let count = self.positions.len();
        if count == 0
            || count > MAX_VERTICES
            || self.triangles.len() > MAX_TRIANGLES
            || self.normals.len() != count
            || self.masks.len() != count
            || self.colors.len() != count
            || proxy_positions.is_some_and(|proxy| proxy.len() != count)
        {
            return (BrushResult::default(), Vec::new());
        }
        if !center.is_finite()
            || !normal.is_finite()
            || !delta.is_finite()
            || !settings.strength.is_finite()
            || !settings.alpha_rotation.is_finite()
            || settings
                .alpha_tangent
                .is_some_and(|tangent| !tangent.is_finite())
            || !valid_brush_options(settings)
        {
            return (BrushResult::default(), Vec::new());
        }
        let Some(radius) = settings.effective_radius() else {
            return (BrushResult::default(), Vec::new());
        };
        if settings
            .alpha
            .as_ref()
            .is_some_and(|alpha| !alpha.has_signal())
        {
            return (BrushResult::default(), Vec::new());
        }
        if settings.color.iter().any(|c| !c.is_finite()) {
            return (BrushResult::default(), Vec::new());
        }
        let material_paint = settings.kind == BrushKind::Paint
            && (settings.paint_roughness || settings.paint_metallic);
        if settings.kind == BrushKind::Paint
            && ((!settings.paint_rgb && !material_paint)
                || (material_paint
                    && (self.positions.len() > MAX_VERTICES
                        || self.colors.len() != self.positions.len()
                        || self.masks.len() != self.positions.len()
                        || self.normals.len() != self.positions.len()
                        || crate::attributes::validate_materials(
                            self.materials.as_deref(),
                            self.positions.len(),
                        )
                        .is_err())))
        {
            return (BrushResult::default(), Vec::new());
        }
        let normal = normal.normalize_or_zero();
        if normal == Vec3::ZERO {
            return (BrushResult::default(), Vec::new());
        }
        let strength = settings.strength.clamp(0.0, 1.0)
            * if settings.pressure_strength {
                settings.pressure.clamp(0.0, 1.0)
            } else {
                1.0
            };
        if strength == 0.0 {
            return (BrushResult::default(), Vec::new());
        }
        let radius_squared = radius * radius;
        if !radius_squared.is_finite() {
            return (BrushResult::default(), Vec::new());
        }
        let normal = if matches!(settings.kind, BrushKind::Draw | BrushKind::Crease) {
            self.draw_patch_normal(center, normal, radius)
        } else {
            normal
        };
        let culling_axis = brush_culling_axis(settings, normal);
        let sign = if settings.invert { -1.0 } else { 1.0 };
        let reflect = |v: Vec3| Vec3::new(-v.x, v.y, v.z);
        let alpha_frame = settings
            .alpha
            .as_ref()
            .map(|_| surface_alpha_frame(normal, settings.alpha_tangent, settings.alpha_rotation));
        // A first real material change may need fallible storage allocation.
        // Keep earlier RGB writes recoverable until that allocation succeeds.
        // Implicit-default targets cannot materialize a None channel at all.
        let may_allocate_material = material_paint
            && self.materials.is_none()
            && !settings.invert
            && ((settings.paint_roughness && settings.roughness != DEFAULT_VERTEX_MATERIAL[0])
                || (settings.paint_metallic && settings.metallic != DEFAULT_VERTEX_MATERIAL[1]));
        if may_allocate_material
            && self
                .positions
                .len()
                .checked_mul(std::mem::size_of::<[f32; 2]>())
                .and_then(|bytes| self.approximate_bytes().checked_add(bytes))
                .is_none_or(|bytes| bytes > crate::decode_limits::MAX_SCENE_STORAGE_BYTES)
        {
            return (BrushResult::default(), Vec::new());
        }
        let color_backup = if may_allocate_material && settings.paint_rgb {
            let mut backup = Vec::new();
            if backup.try_reserve_exact(self.colors.len()).is_err() {
                return (BrushResult::default(), Vec::new());
            }
            backup.extend_from_slice(&self.colors);
            Some(backup)
        } else {
            None
        };
        let old_positions = self.positions.clone();
        let sparse_draw = allow_constraint_repair
            && settings.kind == BrushKind::Draw
            && settings.draw_accumulate
            && settings.adaptive_detail
            && !settings.invert
            && settings.alpha.is_none()
            && settings.hardness == 0.0
            && proxy_positions.is_none();
        if sparse_draw {
            self.ensure_local_incidence();
        } else if uses_surface_detail(settings.kind)
            || settings.kind == BrushKind::Smooth
            || (settings.kind == BrushKind::Move && settings.move_topology_check)
        {
            self.ensure_adjacency();
        }
        let move_topology = if settings.kind == BrushKind::Move && settings.move_topology_check {
            let centers = [center, reflect_point(center, settings.symmetry_offset)];
            let patches = centers.map(|center| {
                let seed = nearest_vertex(&self.positions, center).unwrap_or(0);
                self.radius_connected_patch(seed, center, radius_squared)
            });
            let [Ok(first), Ok(second)] = patches else {
                return (BrushResult::default(), Vec::new());
            };
            Some([first, second])
        } else {
            None
        };
        let mut changed = 0;
        let mut geometry_changed = false;
        for (i, &position) in old_positions.iter().enumerate() {
            let footprint_position = proxy_positions.map_or(position, |proxy| proxy[i]);
            // The footprint test is cheaper than normal normalization and
            // attribute checks. Almost all vertices lie outside a local dab.
            if footprint_position.distance_squared(center) >= radius_squared
                && (!settings.symmetry_x
                    || footprint_position
                        .distance_squared(reflect_point(center, settings.symmetry_offset))
                        >= radius_squared)
            {
                continue;
            }
            if !position.is_finite()
                || !self.normals[i].is_finite()
                || !self.masks[i].is_finite()
                || self.colors[i].iter().any(|c| !c.is_finite())
            {
                continue;
            }
            let vertex_normal = self.normals[i].normalize_or_zero();
            let mut influence = 0.0;
            let mut brush_delta = delta;
            let mut draw_direction = Vec3::ZERO;
            let mut coherent_displacement = Vec3::ZERO;
            let mut coherent_weight = 0.0;
            for mirrored in [false, true] {
                if mirrored && !settings.symmetry_x {
                    continue;
                }
                if move_topology
                    .as_ref()
                    .is_some_and(|patches| !patches[usize::from(mirrored)][i])
                {
                    continue;
                }
                let candidate_center = if mirrored {
                    reflect_point(center, settings.symmetry_offset)
                } else {
                    center
                };
                let candidate_normal = if mirrored { reflect(normal) } else { normal };
                let candidate_culling_axis = if mirrored {
                    reflect(culling_axis)
                } else {
                    culling_axis
                };
                let facing = vertex_normal.dot(candidate_culling_axis);
                if settings.backface_culling && facing <= 0.0 {
                    continue;
                }
                let distance_squared = footprint_position.distance_squared(candidate_center);
                if !distance_squared.is_finite() || distance_squared >= radius_squared {
                    continue;
                }
                let mut falloff =
                    radial_influence(distance_squared, radius_squared, settings, false);
                if let (Some(alpha), Some((tangent, bitangent))) = (&settings.alpha, alpha_frame) {
                    // Reflection changes handedness. Reflect both axes from
                    // the original frame rather than rebuilding a cross
                    // product on the opposite side; asymmetric pixels then
                    // appear at exactly mirrored world positions.
                    let tangent = if mirrored { reflect(tangent) } else { tangent };
                    let bitangent = if mirrored {
                        reflect(bitangent)
                    } else {
                        bitangent
                    };
                    let offset = position - candidate_center;
                    let u = 0.5 + offset.dot(tangent) / (2.0 * radius);
                    let v = 0.5 - offset.dot(bitangent) / (2.0 * radius);
                    falloff *= alpha.sample(u, v);
                }
                if settings.backface_culling
                    && !matches!(settings.kind, BrushKind::Paint | BrushKind::Mask)
                {
                    // Fade grazing vertices instead of switching full-strength
                    // deformation off abruptly when a crease turns a normal.
                    let front = (facing / 0.35).clamp(0.0, 1.0);
                    falloff *= front * front * (3.0 - 2.0 * front);
                }
                if matches!(settings.kind, BrushKind::Draw | BrushKind::Flatten) {
                    // Maximum influence avoids double-strength overlap, but
                    // choosing only one mirrored direction creates a jump at
                    // x=0. Blend directions across the overlapping footprints.
                    draw_direction += candidate_normal * falloff;
                }
                if matches!(
                    settings.kind,
                    BrushKind::Crease
                        | BrushKind::Clay
                        | BrushKind::Flatten
                        | BrushKind::Pinch
                        | BrushKind::Drag
                ) {
                    // A maximum scalar footprint avoids doubled symmetry
                    // strength, while weighted vectors avoid switching one
                    // whole center/normal abruptly at overlapping footprints.
                    let unit = match settings.kind {
                        BrushKind::Crease => {
                            let tangent_normal = (vertex_normal * 0.35 + candidate_normal * 0.65)
                                .normalize_or_zero();
                            let toward = candidate_center - position;
                            let tangent = toward - tangent_normal * toward.dot(tangent_normal);
                            tangent * 0.08 - candidate_normal * radius * 0.035
                        }
                        BrushKind::Clay => {
                            candidate_normal
                                * (radius * 0.07
                                    - (position - candidate_center).dot(candidate_normal) * 0.35)
                        }
                        BrushKind::Flatten => {
                            -candidate_normal
                                * (position - candidate_center).dot(candidate_normal)
                                * 0.65
                        }
                        BrushKind::Pinch => (candidate_center - position) * 0.05,
                        BrushKind::Drag => {
                            if mirrored {
                                reflect(delta)
                            } else {
                                delta
                            }
                        }
                        _ => unreachable!(),
                    };
                    coherent_displacement += unit * falloff;
                    coherent_weight += falloff;
                }
                let tie_on_mirrored_side = mirrored
                    && position.x < settings.symmetry_offset
                    && falloff == influence
                    && center.x == settings.symmetry_offset;
                if falloff > influence || tie_on_mirrored_side {
                    influence = falloff;
                    brush_delta = if mirrored { reflect(delta) } else { delta };
                }
            }
            if influence <= 0.0 {
                continue;
            }
            let protection = if settings.kind == BrushKind::Mask {
                1.0
            } else {
                self.mask_influence(i)
            };
            let amount = strength * influence * protection;
            if amount <= 0.0 {
                continue;
            }
            match settings.kind {
                BrushKind::Mask => {
                    let next = self
                        .mask_convention
                        .add_protection(self.masks[i], amount * 0.18 * sign);
                    if next != self.masks[i] {
                        self.masks[i] = next;
                        changed += 1;
                    }
                }
                BrushKind::Paint => {
                    let mut point_changed = false;
                    if settings.paint_rgb {
                        // Preserve the established RGB arithmetic and inverse
                        // target exactly, including multiplication order.
                        let target = if settings.invert {
                            [0.65, 0.69, 0.74]
                        } else {
                            settings.color.map(|c| c.clamp(0.0, 1.0))
                        };
                        let next = std::array::from_fn(|channel| {
                            self.colors[i][channel]
                                + (target[channel] - self.colors[i][channel]) * amount * 0.25
                        });
                        if next != self.colors[i] {
                            self.colors[i] = next;
                            point_changed = true;
                        }
                    }
                    if material_paint {
                        let previous = self
                            .materials
                            .as_ref()
                            .map_or(DEFAULT_VERTEX_MATERIAL, |values| values[i]);
                        let target = if settings.invert {
                            DEFAULT_VERTEX_MATERIAL
                        } else {
                            [settings.roughness, settings.metallic]
                        };
                        let flags = [settings.paint_roughness, settings.paint_metallic];
                        let next = std::array::from_fn(|channel| {
                            if !flags[channel] {
                                return previous[channel];
                            }
                            let proposed = previous[channel]
                                + (target[channel] - previous[channel]) * amount * 0.25;
                            // Unchanged scalar values retain their original
                            // signed-zero bits even when another channel edits.
                            if proposed == previous[channel] {
                                previous[channel]
                            } else {
                                proposed
                            }
                        });
                        if next != previous {
                            if self.materials.is_none() {
                                let mut values = Vec::new();
                                if values.try_reserve_exact(self.positions.len()).is_err() {
                                    if let Some(backup) = color_backup {
                                        self.colors = backup;
                                    }
                                    return (BrushResult::default(), Vec::new());
                                }
                                values.resize(self.positions.len(), DEFAULT_VERTEX_MATERIAL);
                                self.materials = Some(values);
                            }
                            self.materials.as_mut().unwrap()[i] = next;
                            point_changed = true;
                        }
                    }
                    changed += usize::from(point_changed);
                }
                kind => {
                    let mut displacement = match kind {
                        BrushKind::Draw => {
                            // Draw raises a coherent patch. Per-vertex inflation
                            // feeds a ridge's changing normals back into lateral
                            // motion; that behavior belongs to Inflate.
                            let direction = draw_direction.normalize_or_zero();
                            let fairing =
                                self.smooth_target(i, &old_positions, sparse_draw) - position;
                            direction * radius * amount * 0.075 * sign + fairing * amount * 0.3
                        }
                        BrushKind::Clay => {
                            // A planar clay stamp levels peaks while adding volume.
                            coherent_displacement / coherent_weight * amount * sign
                                + self.surface_relaxation(
                                    i,
                                    &old_positions,
                                    vertex_normal,
                                    amount,
                                    0.25,
                                    0.08,
                                )
                        }
                        BrushKind::Inflate => {
                            vertex_normal * radius * amount * 0.075 * sign
                                + self.surface_relaxation(
                                    i,
                                    &old_positions,
                                    vertex_normal,
                                    amount,
                                    0.25,
                                    0.08,
                                )
                        }
                        BrushKind::Smooth => {
                            let laplacian = self.smooth_target(i, &old_positions, false) - position;
                            let movement = if settings.smooth_tangent {
                                laplacian - vertex_normal * laplacian.dot(vertex_normal)
                            } else {
                                laplacian
                            };
                            movement * amount * 0.5 * sign
                        }
                        BrushKind::Flatten => {
                            coherent_displacement / coherent_weight * amount * sign
                                + self.surface_relaxation(
                                    i,
                                    &old_positions,
                                    draw_direction.normalize_or_zero(),
                                    amount,
                                    0.25,
                                    0.0,
                                )
                        }
                        BrushKind::Crease => {
                            coherent_displacement / coherent_weight * amount * sign
                                + self.surface_relaxation(
                                    i,
                                    &old_positions,
                                    vertex_normal,
                                    amount,
                                    0.4,
                                    0.15,
                                )
                        }
                        BrushKind::Move => {
                            let movement = if settings.move_along_normal {
                                let axis = if position.x < settings.symmetry_offset
                                    && settings.symmetry_x
                                {
                                    reflect(normal)
                                } else {
                                    normal
                                };
                                axis * brush_delta.dot(axis)
                            } else {
                                brush_delta
                            };
                            movement * amount * sign
                        }
                        BrushKind::Pinch | BrushKind::Drag => {
                            coherent_displacement / coherent_weight * amount * sign
                        }
                        _ => unreachable!(),
                    };
                    if settings.symmetry_x
                        && (position.x - settings.symmetry_offset).abs() <= radius * 1e-5
                    {
                        displacement.x = 0.0;
                    }
                    let next = position + displacement;
                    if next.is_finite()
                        && next.abs().max_element() <= 1_000_000.0
                        && next != position
                    {
                        self.positions[i] = next;
                        geometry_changed = true;
                        changed += 1;
                    }
                }
            }
        }
        if allow_interior_prepare
            && changed == 0
            && settings.kind == BrushKind::Draw
            && settings.draw_accumulate
            && settings.adaptive_detail
            && !settings.invert
            && settings.alpha.is_none()
            && settings.hardness == 0.0
            && proxy_positions.is_none()
        {
            if let Ok(Some((mut prepared, mut preparation_ancestry))) = self
                .prepare_draw_interior_contact(
                    center,
                    normal,
                    radius,
                    settings.symmetry_x,
                    settings.symmetry_offset,
                )
            {
                let prepared_vertices = prepared.positions.len() - self.positions.len();
                let (mut result, ancestry) = prepared.apply_brush_dab(
                    center,
                    normal,
                    delta,
                    settings,
                    None,
                    allow_constraint_repair,
                    false,
                );
                if result.changed_vertices > 0
                    && result.refined_vertices + prepared_vertices <= 1_024
                {
                    result.refined_vertices += prepared_vertices;
                    preparation_ancestry.extend(ancestry);
                    *self = prepared;
                    return (result, preparation_ancestry);
                }
            }
        }
        let mut dirty_vertices = Vec::new();
        let mut deferred_draw = None;
        let mut displacement_scale = if geometry_changed {
            if uses_surface_detail(settings.kind) && settings.symmetry_x {
                self.preserve_exact_sculpt_symmetry(&old_positions, settings.symmetry_offset);
            }
            // The direct default Draw path may preserve existing fragile
            // triangles as coherent local components if its full proposal is
            // clipped. DrawStroke retains its proxy/ancestry semantics.
            let draw_repair = sparse_draw.then_some((
                settings.symmetry_x,
                settings.symmetry_offset,
                center,
                radius,
                normal,
                brush_culling_axis(settings, normal),
            ));
            let scale = self.accept_deformation(
                &old_positions,
                settings.kind,
                draw_repair,
                Some(&mut deferred_draw),
            );
            dirty_vertices = self
                .positions
                .iter()
                .zip(&old_positions)
                .enumerate()
                .filter_map(|(vertex, (current, old))| (current != old).then_some(vertex))
                .collect();
            changed = dirty_vertices.len();
            if changed > 0 {
                scale
            } else {
                0.0
            }
        } else if changed > 0 {
            1.0
        } else {
            0.0
        };
        // Keep the accepted pre-refinement surface only when the picked ray has
        // not advanced. An alternate cage may be compared after the ordinary
        // refinement, without changing a productive baseline dab.
        let cage_pre_refinement = deferred_draw.as_ref().and_then(|pending| {
            if changed == 0 || !settings.adaptive_detail {
                return None;
            }
            let old = draw_local_ray_lift(
                &old_positions,
                &pending.guard,
                pending.anchor,
                pending.center,
                pending.camera_axis,
                pending.radius,
            )?;
            let current = draw_local_ray_lift(
                &self.positions,
                &pending.guard,
                pending.anchor,
                pending.center,
                pending.camera_axis,
                pending.radius,
            )?;
            (current - old <= 0.0)
                .then(|| self.try_snapshot().ok())
                .flatten()
        });
        let refinement =
            if changed > 0 && uses_surface_detail(settings.kind) && settings.adaptive_detail {
                if proxy_positions.is_none()
                    && !matches!(settings.kind, BrushKind::Draw | BrushKind::Clay)
                    && settings.symmetry_offset == 0.0
                {
                    self.refine_brush_patch_with_dirty_normals(
                        center,
                        radius,
                        settings.symmetry_x,
                        &dirty_vertices,
                    )
                    .map(|report| (report, Vec::new()))
                } else {
                    self.refine_brush_patch_with_options(
                        center,
                        radius,
                        settings.symmetry_x,
                        settings.symmetry_offset,
                        &dirty_vertices,
                        settings.kind == BrushKind::Draw,
                    )
                }
            } else {
                Ok((crate::PatchRefinement::default(), Vec::new()))
            };
        let (mut refined_vertices, mut detail_limited, mut ancestry) = match refinement {
            Ok((result, ancestry)) => (result.vertices_added, result.limited, ancestry),
            Err(_) => (0, true, Vec::new()),
        };
        if let (Some(pending), Some(mut alternate)) = (deferred_draw.as_ref(), cage_pre_refinement)
        {
            if refined_vertices > 0 {
                let old_lift = draw_local_ray_lift(
                    &old_positions,
                    &pending.guard,
                    pending.anchor,
                    pending.center,
                    pending.camera_axis,
                    pending.radius,
                );
                let baseline_lift = draw_local_ray_lift(
                    &self.positions,
                    &pending.guard,
                    pending.anchor,
                    pending.center,
                    pending.camera_axis,
                    pending.radius,
                );
                if baseline_lift
                    .zip(old_lift)
                    .is_some_and(|(current, old)| current - old <= 0.0)
                {
                    let ray = crate::Ray {
                        origin: pending.center + pending.camera_axis * (2.0 * radius),
                        direction: -pending.camera_axis,
                    };
                    let accepted_positions = alternate.positions.clone();
                    alternate.positions.copy_from_slice(&old_positions);
                    let old_hit = alternate.raycast(ray);

                    let cage = alternate.solve_draw_one_ring_cage(
                        &old_positions,
                        &pending.proposed,
                        &pending.guard,
                        pending.anchor,
                        pending.center,
                        pending.camera_axis,
                        pending.radius,
                        pending.symmetry_x,
                        pending.symmetry_offset,
                        &accepted_positions,
                    );
                    if let Some(cage) = cage {
                        alternate.positions.copy_from_slice(if pending.symmetry_x {
                            &accepted_positions
                        } else {
                            &old_positions
                        });
                        for (id, position) in cage {
                            alternate.positions[id] = position;
                        }
                        if pending.guard.safe(&alternate.positions) {
                            let candidate_dirty: Vec<_> = alternate
                                .positions
                                .iter()
                                .zip(&old_positions)
                                .enumerate()
                                .filter_map(|(id, (a, b))| (a != b).then_some(id))
                                .collect();
                            let candidate_refinement = alternate.refine_brush_patch_with_options(
                                center,
                                radius,
                                settings.symmetry_x,
                                settings.symmetry_offset,
                                &candidate_dirty,
                                true,
                            );
                            if let Ok((report, candidate_ancestry)) = candidate_refinement {
                                if let (Some(old_hit), Some(baseline_hit), Some(candidate_hit)) =
                                    (old_hit, self.raycast(ray), alternate.raycast(ray))
                                {
                                    let required_lift =
                                        radius * if settings.symmetry_x { 0.001 } else { 0.002 };
                                    if baseline_hit.distance - candidate_hit.distance
                                        >= required_lift
                                        && old_hit.distance - candidate_hit.distance
                                            >= required_lift
                                    {
                                        if report.vertices_added == 0 {
                                            alternate.recompute_normals_near_vertices_cached(
                                                &candidate_dirty,
                                            );
                                        }
                                        *self = alternate;
                                        dirty_vertices = candidate_dirty;
                                        changed = dirty_vertices.len();
                                        displacement_scale = 1.0;
                                        refined_vertices = report.vertices_added;
                                        detail_limited |= report.limited;
                                        ancestry = candidate_ancestry;
                                        deferred_draw = None;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Some(pending) = deferred_draw {
            if refined_vertices == 0 && self.positions.len() == old_positions.len() {
                let old_ray_lift = draw_local_ray_lift(
                    &old_positions,
                    &pending.guard,
                    pending.anchor,
                    pending.center,
                    pending.camera_axis,
                    pending.radius,
                );
                let baseline_ray_lift = draw_local_ray_lift(
                    &self.positions,
                    &pending.guard,
                    pending.anchor,
                    pending.center,
                    pending.camera_axis,
                    pending.radius,
                );
                let baseline_progress = baseline_ray_lift
                    .zip(old_ray_lift)
                    .map(|(current, old)| current - old);

                if baseline_progress.is_some_and(|progress| progress <= 0.0) {
                    let baseline_changed = changed;
                    let baseline_scale = displacement_scale;
                    let baseline_detail_limited = detail_limited;
                    let baseline_dirty = dirty_vertices.clone();
                    let baseline_ancestry = ancestry.clone();
                    let baseline_mesh = self.snapshot();
                    let baseline = self.positions.clone();
                    let ray = crate::Ray {
                        origin: pending.center + pending.camera_axis * (2.0 * pending.radius),
                        direction: -pending.camera_axis,
                    };
                    let baseline_hit = self.raycast(ray);
                    self.positions.copy_from_slice(&old_positions);
                    let old_hit = self.raycast(ray);
                    self.positions.copy_from_slice(&baseline);
                    self.positions.copy_from_slice(&pending.proposed);
                    let mut repaired = self.repair_draw_constraint_components(
                        &pending.guard,
                        &old_positions,
                        &pending.proposed,
                        pending.symmetry_x,
                        pending.symmetry_offset,
                    );
                    let mut picked_ray_lift = draw_local_ray_lift(
                        &self.positions,
                        &pending.guard,
                        pending.anchor,
                        pending.center,
                        pending.camera_axis,
                        pending.radius,
                    );
                    let mut picked_progress = picked_ray_lift
                        .zip(old_ray_lift)
                        .map(|(current, old)| current - old);
                    let required_lift =
                        f64::from(radius) * if pending.symmetry_x { 0.001 } else { 0.002 };
                    if !repaired
                        || !pending.guard.safe(&self.positions)
                        || !picked_progress.is_some_and(|lift| lift >= required_lift)
                    {
                        let cage = self.solve_draw_one_ring_cage(
                            &old_positions,
                            &pending.proposed,
                            &pending.guard,
                            pending.anchor,
                            pending.center,
                            pending.camera_axis,
                            pending.radius,
                            pending.symmetry_x,
                            pending.symmetry_offset,
                            &baseline,
                        );
                        if let Some(cage) = cage {
                            self.positions.copy_from_slice(if pending.symmetry_x {
                                &baseline
                            } else {
                                &old_positions
                            });
                            for (id, position) in cage {
                                self.positions[id] = position;
                            }
                            repaired = true;
                            picked_ray_lift = draw_local_ray_lift(
                                &self.positions,
                                &pending.guard,
                                pending.anchor,
                                pending.center,
                                pending.camera_axis,
                                pending.radius,
                            );
                            picked_progress = picked_ray_lift
                                .zip(old_ray_lift)
                                .map(|(current, old)| current - old);
                        }
                    }

                    let candidate_hit = self.raycast(ray);
                    let full_ray_growth = old_hit.zip(baseline_hit).zip(candidate_hit).is_some_and(
                        |((old, baseline), candidate)| {
                            old.distance - candidate.distance >= required_lift as f32
                                && baseline.distance - candidate.distance >= required_lift as f32
                        },
                    );
                    if repaired
                        && pending.guard.safe(&self.positions)
                        && picked_progress.is_some_and(|lift| lift >= required_lift)
                        && full_ray_growth
                    {
                        dirty_vertices = self
                            .positions
                            .iter()
                            .zip(&old_positions)
                            .enumerate()
                            .filter_map(|(vertex, (current, previous))| {
                                (current != previous).then_some(vertex)
                            })
                            .collect();
                        changed = dirty_vertices.len();
                        displacement_scale = 1.0;
                        if settings.adaptive_detail {
                            match self.refine_brush_patch_with_options(
                                center,
                                radius,
                                settings.symmetry_x,
                                settings.symmetry_offset,
                                &dirty_vertices,
                                true,
                            ) {
                                Ok((result, new_ancestry)) => {
                                    refined_vertices = result.vertices_added;
                                    detail_limited |= result.limited;
                                    ancestry = new_ancestry;
                                }
                                Err(_) => detail_limited = true,
                            }
                        }
                        let final_hit = self.raycast(ray);
                        if !old_hit.zip(baseline_hit).zip(final_hit).is_some_and(
                            |((old, baseline), final_hit)| {
                                old.distance - final_hit.distance >= required_lift as f32
                                    && baseline.distance - final_hit.distance
                                        >= required_lift as f32
                            },
                        ) {
                            *self = baseline_mesh;
                            dirty_vertices = baseline_dirty;
                            changed = baseline_changed;
                            displacement_scale = baseline_scale;
                            refined_vertices = 0;
                            detail_limited = baseline_detail_limited;
                            ancestry = baseline_ancestry;
                        }
                    } else {
                        self.positions.copy_from_slice(&baseline);
                    }
                }
            }
        }
        // Refinement rebuilds normals once after its complete atomic commit.
        // Avoid an unnecessary full normal pass immediately before that work.
        if !dirty_vertices.is_empty() && refined_vertices == 0 {
            self.recompute_normals_near_vertices_cached(&dirty_vertices);
        }
        (
            BrushResult {
                changed_vertices: changed,
                displacement_scale,
                refined_vertices,
                detail_limited,
            },
            ancestry,
        )
    }

    fn smooth_target(&self, vertex: usize, old: &[Vec3], local_incidence: bool) -> Vec3 {
        let local_neighbors = local_incidence.then(|| self.local_neighbors(vertex));
        let neighbors = local_neighbors
            .as_deref()
            .unwrap_or_else(|| self.cached_neighbors(vertex));
        if neighbors.is_empty() {
            old[vertex]
        } else {
            neighbors
                .iter()
                .fold(Vec3::ZERO, |sum, &other| sum + old[other])
                / neighbors.len() as f32
        }
    }

    fn surface_relaxation(
        &self,
        vertex: usize,
        old: &[Vec3],
        normal: Vec3,
        amount: f32,
        tangent_weight: f32,
        normal_weight: f32,
    ) -> Vec3 {
        let laplacian = self.smooth_target(vertex, old, false) - old[vertex];
        let normal_component = normal * laplacian.dot(normal);
        let neighbors = self.cached_neighbors(vertex);
        let mean_edge_squared = if neighbors.is_empty() {
            0.0
        } else {
            neighbors
                .iter()
                .map(|&other| old[vertex].distance_squared(old[other]))
                .sum::<f32>()
                / neighbors.len() as f32
        };
        // Redistribute the surface tangentially on every dab. Normal smoothing
        // addresses concentrated high-frequency curvature rather than shrinking
        // an already smooth spherical starting surface.
        let roughness = if mean_edge_squared > 0.0 {
            laplacian.dot(normal).abs() / mean_edge_squared.sqrt()
        } else {
            0.0
        };
        let normal_blend = ((roughness - 0.18) / 0.18).clamp(0.0, 1.0);
        ((laplacian - normal_component) * tangent_weight
            + normal_component * normal_weight * normal_blend)
            * amount
    }

    /// The field is symmetric mathematically, but different neighbor summation
    /// orders can leave mirrored f32 points a few bits apart. Keep originally
    /// exact pairs exact before refinement groups them, without forcing an
    /// asymmetric mesh or differently protected pair into symmetry.
    fn preserve_exact_sculpt_symmetry(&mut self, old: &[Vec3], offset: f32) {
        let key = |position: Vec3| {
            position
                .to_array()
                .map(|value| if value == 0.0 { 0 } else { value.to_bits() })
        };
        let mut changed = HashMap::<[u32; 3], Option<usize>>::new();
        let mut changed_vertices = Vec::new();
        for (vertex, (&before, &after)) in old.iter().zip(&self.positions).enumerate() {
            if before != after {
                changed_vertices.push(vertex);
                changed
                    .entry(key(before))
                    .and_modify(|entry| *entry = None)
                    .or_insert(Some(vertex));
            }
        }
        // Keep the original ascending order and duplicate-position rule, but
        // do not hash every unchanged vertex on a dense imported model.
        for vertex in changed_vertices {
            let before = old[vertex];
            if before.x <= offset || changed.get(&key(before)) != Some(&Some(vertex)) {
                continue;
            }
            let mirror = key(reflect_point(before, offset));
            let Some(other) = changed.get(&mirror).copied().flatten() else {
                continue;
            };
            if self.masks[vertex] == self.masks[other] {
                let a = self.positions[vertex];
                let b = self.positions[other];
                let average = (a + reflect_point(b, offset)) * 0.5;
                self.positions[vertex] = average;
                self.positions[other] = reflect_point(average, offset);
            }
        }
    }

    /// Area-weighted patch direction, independent of vertex sampling density.
    /// A smooth footprint limits jumps when picking an already raised stroke.
    fn draw_patch_normal(&self, center: Vec3, fallback: Vec3, radius: f32) -> Vec3 {
        let center = center.as_dvec3();
        let radius_squared = f64::from(radius).powi(2);
        let fallback_precise = fallback.as_dvec3();
        let mut average = glam::DVec3::ZERO;
        for &triangle in &self.triangles {
            let [a, b, c] = triangle.map(|vertex| vertex as usize);
            let (Some(a), Some(b), Some(c)) = (
                self.positions.get(a),
                self.positions.get(b),
                self.positions.get(c),
            ) else {
                continue;
            };
            let [a, b, c] = [a, b, c].map(|point| point.as_dvec3());
            let distance_squared = ((a + b + c) / 3.0).distance_squared(center);
            if distance_squared >= radius_squared {
                continue;
            }
            let cross = (b - a).cross(c - a);
            let facing = cross.normalize_or_zero().dot(fallback_precise);
            if facing <= 0.0 || !cross.is_finite() {
                continue;
            }
            let protection = triangle
                .iter()
                .map(|&vertex| self.mask_influence_f64(vertex as usize))
                .sum::<f64>()
                / 3.0;
            let footprint = (1.0 - distance_squared / radius_squared).powi(2);
            average += cross * footprint * facing * protection;
        }
        let direction = average.normalize_or_zero().as_vec3();
        if direction.is_finite() && direction != Vec3::ZERO {
            direction
        } else {
            fallback
        }
    }

    /// Gather affected faces once and try a bounded, shared step. Shared scaling
    /// preserves mirror pairs and zero displacement at protected vertices.
    /// This is a local per-dab check, not global self-intersection prevention.
    pub(crate) fn apply_morph_positions_with_result(
        &mut self,
        target: &[Vec3],
        center: Vec3,
        normal: Vec3,
        settings: &BrushSettings,
    ) -> BrushResult {
        if target.len() != self.positions.len()
            || settings.kind != BrushKind::Morph
            || !center.is_finite()
            || !normal.is_finite()
            || !settings.strength.is_finite()
            || !settings.alpha_rotation.is_finite()
            || settings.alpha_tangent.is_some_and(|t| !t.is_finite())
            || !valid_brush_options(settings)
        {
            return BrushResult::default();
        }
        let Some(radius) = settings.effective_radius() else {
            return BrushResult::default();
        };
        let radius_squared = radius * radius;
        let normal = normal.normalize_or_zero();
        let culling_axis = brush_culling_axis(settings, normal);
        let strength = settings.strength.clamp(0.0, 1.0)
            * if settings.pressure_strength {
                settings.pressure.clamp(0.0, 1.0)
            } else {
                1.0
            };
        if strength == 0.0
            || !radius_squared.is_finite()
            || normal == Vec3::ZERO
            || settings.alpha.as_ref().is_some_and(|a| !a.has_signal())
        {
            return BrushResult::default();
        }
        let old = self.positions.clone();
        let reflect = |p: Vec3| Vec3::new(-p.x, p.y, p.z);
        let alpha_frame =
            surface_alpha_frame(normal, settings.alpha_tangent, settings.alpha_rotation);
        let sign = if settings.invert { -1.0_f64 } else { 1.0_f64 };
        for (vertex, (&position, &destination)) in old.iter().zip(target).enumerate() {
            if position.to_array().map(f32::to_bits) == destination.to_array().map(f32::to_bits) {
                continue;
            }
            let vertex_normal = self.normals[vertex].normalize_or_zero();
            let mut influence = 0.0_f32;
            for mirrored in [false, true] {
                if mirrored && !settings.symmetry_x {
                    continue;
                }
                let stamp_center = if mirrored {
                    reflect_point(center, settings.symmetry_offset)
                } else {
                    center
                };
                let distance_squared = position.distance_squared(stamp_center);
                let facing = vertex_normal.dot(if mirrored {
                    reflect(culling_axis)
                } else {
                    culling_axis
                });
                if (settings.backface_culling && facing <= 0.0)
                    || !distance_squared.is_finite()
                    || distance_squared >= radius_squared
                {
                    continue;
                }
                let mut weight =
                    radial_influence(distance_squared, radius_squared, settings, false)
                        * facing_influence(facing, settings);
                if let Some(alpha) = &settings.alpha {
                    let (tangent, bitangent) = alpha_frame;
                    let tangent = if mirrored { reflect(tangent) } else { tangent };
                    let bitangent = if mirrored {
                        reflect(bitangent)
                    } else {
                        bitangent
                    };
                    let offset = position - stamp_center;
                    weight *= alpha.sample(
                        0.5 + offset.dot(tangent) / (2.0 * radius),
                        0.5 - offset.dot(bitangent) / (2.0 * radius),
                    );
                }
                influence = influence.max(weight);
            }
            let amount = strength * influence * (self.mask_influence(vertex));
            if amount <= 0.0 {
                continue;
            }
            // Preserve exact endpoint bits, including signed zero. A component
            // already at its target is never recomputed through adding zero.
            let mut next = position;
            for component in 0..3 {
                if position[component].to_bits() == destination[component].to_bits() {
                    continue;
                }
                next[component] = if amount == 1.0 && !settings.invert {
                    destination[component]
                } else {
                    (f64::from(position[component])
                        + (f64::from(destination[component]) - f64::from(position[component]))
                            * f64::from(amount)
                            * sign) as f32
                };
            }
            if next.is_finite() && next.abs().max_element() <= 1_000_000.0 {
                self.positions[vertex] = next;
            }
        }
        if crate::morph::positions_equal_bits(&old, &self.positions) {
            return BrushResult::default();
        }
        self.ensure_adjacency();
        let proposed = self.positions.clone();
        // Existing smooth-tool policy: quality >= min(old quality, 0.1),
        // folds <= max(old folds, 60 degrees), with shared bounded step scaling.
        let scale = self.accept_deformation(&old, BrushKind::Morph, None, None);
        // The common guard interpolates a whole array on clipping. Restore
        // untouched component bits so protection also preserves signed zeros.
        for ((position, original), proposal) in self.positions.iter_mut().zip(&old).zip(&proposed) {
            for component in 0..3 {
                if proposal[component].to_bits() == original[component].to_bits() {
                    position[component] = original[component];
                }
            }
        }
        let dirty: Vec<_> = self
            .positions
            .iter()
            .zip(&old)
            .enumerate()
            .filter_map(|(i, (a, b))| {
                (a.to_array().map(f32::to_bits) != b.to_array().map(f32::to_bits)).then_some(i)
            })
            .collect();
        if dirty.is_empty() {
            return BrushResult::default();
        }
        self.recompute_normals_near_vertices_cached(&dirty);
        BrushResult {
            changed_vertices: dirty.len(),
            displacement_scale: scale,
            refined_vertices: 0,
            detail_limited: false,
        }
    }

    /// Separate Source geometry caller; Legacy tool kernels and their guard
    /// settings are unchanged. Untouched component words remain protected.
    #[cfg(test)]
    pub(crate) fn accept_source_geometry_candidate(&mut self, old: &[Vec3]) -> f32 {
        self.ensure_adjacency();
        let proposed = self.positions.clone();
        let scale = self.accept_deformation_with_limits(old, 0.1, 0.5, false);
        for ((point, previous), candidate) in self.positions.iter_mut().zip(old).zip(proposed) {
            for component in 0..3 {
                if candidate[component].to_bits() == previous[component].to_bits() {
                    point[component] = previous[component];
                }
            }
        }
        scale
    }

    pub(crate) fn accept_source_geometry_candidate_sparse(
        &mut self,
        old: &[Vec3],
        proposals: &[(usize, Vec3)],
    ) -> f32 {
        self.ensure_adjacency();
        // The Source producer has already written only these candidate rows.
        // Preserve the generic guard's ascending changed-vertex order while
        // avoiding another full candidate copy and full-array scale pass.
        let mut changed_vertices: Vec<_> = proposals
            .iter()
            .filter_map(|&(vertex, _)| (self.positions[vertex] != old[vertex]).then_some(vertex))
            .collect();
        changed_vertices.sort_unstable();
        changed_vertices.dedup();
        let scale = match self.deformation_guard(old, &changed_vertices, 0.1, 0.5, false) {
            None => {
                self.positions.copy_from_slice(old);
                0.0
            }
            Some(guard) if guard.safe(&self.positions) => 1.0,
            Some(guard) => {
                let mut scale = 1.0;
                let mut accepted = 0.0;
                for _ in 0..12 {
                    scale *= 0.5;
                    for &(vertex, candidate) in proposals {
                        let original = old[vertex];
                        self.positions[vertex] = original + (candidate - original) * scale;
                    }
                    if guard.safe(&self.positions) {
                        accepted = scale;
                        break;
                    }
                }
                if accepted == 0.0 {
                    self.positions.copy_from_slice(old);
                }
                accepted
            }
        };
        for &(vertex, candidate) in proposals {
            let point = &mut self.positions[vertex];
            let previous = old[vertex];
            for component in 0..3 {
                if candidate[component].to_bits() == previous[component].to_bits() {
                    point[component] = previous[component];
                }
            }
        }
        scale
    }

    fn accept_deformation(
        &mut self,
        old: &[Vec3],
        kind: BrushKind,
        draw_repair: Option<(bool, f32, Vec3, f32, Vec3, Vec3)>,
        deferred: Option<&mut Option<DrawDeferred>>,
    ) -> f32 {
        let (quality_floor, fold_cosine) = match kind {
            BrushKind::Crease | BrushKind::Drag => (0.1, 0.0), // At most 90 degrees.
            BrushKind::Move => (0.02, -0.866_025_403_784_438_6), // Large rotations, 150 degrees.
            _ => (0.1, 0.5),                                   // Smooth surface tools, 60 degrees.
        };
        if kind == BrushKind::Move {
            self.ensure_adjacency();
        }
        self.accept_deformation_with_limits_and_draw_repair(
            old,
            quality_floor,
            fold_cosine,
            false,
            draw_repair,
            deferred,
        )
    }

    fn deformation_guard(
        &mut self,
        old: &[Vec3],
        changed_vertices: &[usize],
        quality_floor: f64,
        fold_cosine: f64,
        local_incidence: bool,
    ) -> Option<DeformationGuard> {
        if self.cached_invalid_indices() {
            return None;
        }
        let affected = self.cached_faces_for_vertices(changed_vertices);
        let mut faces = Vec::new();
        for &face in &affected {
            let indices = self.triangles[face];
            let points = indices.map(|vertex| old[vertex as usize].as_dvec3());
            let cross = (points[1] - points[0]).cross(points[2] - points[0]);
            let area_squared = cross.length_squared();
            let longest_edge_squared = longest_edge_squared(points);
            // A collapsed face has no previous orientation to preserve. Do not
            // silently guess a repair; unchanged/rejected dabs preserve undo.
            if area_squared <= 0.0 || !area_squared.is_finite() {
                return None;
            }
            let minimum_quality = (area_squared.sqrt() / longest_edge_squared).min(quality_floor);
            faces.push(FaceGuard {
                indices,
                cross,
                minimum_projection: area_squared * 0.05,
                minimum_quality,
            });
        }
        // Three or more faces on an edge have no unique adjacent fold. Avoid
        // quadratic pair allocation or silently ignoring an unchecked pair.
        if !local_incidence
            && self
                .cached_nonmanifold_faces()
                .iter()
                .any(|&face| affected.binary_search(&(face as usize)).is_ok())
        {
            return None;
        }
        let mut folds = Vec::new();
        let pairs = if local_incidence {
            self.local_face_pairs_for_faces(&affected).ok()?
        } else {
            self.cached_pairs_for_faces(&affected)
                .into_iter()
                .map(|pair| self.cached_face_pairs()[pair])
                .collect()
        };
        for [a, b] in pairs {
            let indices = [self.triangles[a as usize], self.triangles[b as usize]];
            let normals = indices.map(|triangle| face_cross(old, triangle).normalize_or_zero());
            if normals.contains(&glam::DVec3::ZERO) {
                return None;
            }
            folds.push(FoldGuard {
                indices,
                // A narrow groove is useful, but newly back-to-back triangles
                // create the visible folded slivers. Existing sharp geometry
                // is allowed to improve without demanding an automatic repair.
                minimum_cosine: normals[0].dot(normals[1]).min(fold_cosine),
            });
        }
        Some(DeformationGuard { faces, folds })
    }

    fn accept_deformation_with_limits(
        &mut self,
        old: &[Vec3],
        quality_floor: f64,
        fold_cosine: f64,
        refine_scale: bool,
    ) -> f32 {
        self.accept_deformation_with_limits_and_draw_repair(
            old,
            quality_floor,
            fold_cosine,
            refine_scale,
            None,
            None,
        )
    }

    fn accept_deformation_with_limits_and_draw_repair(
        &mut self,
        old: &[Vec3],
        quality_floor: f64,
        fold_cosine: f64,
        refine_scale: bool,
        draw_repair: Option<(bool, f32, Vec3, f32, Vec3, Vec3)>,
        mut deferred: Option<&mut Option<DrawDeferred>>,
    ) -> f32 {
        let changed_vertices: Vec<_> = self
            .positions
            .iter()
            .zip(old)
            .enumerate()
            .filter_map(|(vertex, (current, previous))| (current != previous).then_some(vertex))
            .collect();
        let Some(guard) = self.deformation_guard(
            old,
            &changed_vertices,
            quality_floor,
            fold_cosine,
            draw_repair.is_some(),
        ) else {
            self.positions.copy_from_slice(old);
            return 0.0;
        };
        if guard.safe(&self.positions) {
            return 1.0;
        }
        let proposed = self.positions.clone();
        let mut scale = 1.0;
        let mut unsafe_scale = 1.0;
        let mut baseline_found = false;
        if let Some((symmetry_x, symmetry_offset, _center, _radius, _normal, _camera_axis)) =
            draw_repair
        {
            if self.repair_draw_constraint_components(
                &guard,
                old,
                &proposed,
                symmetry_x,
                symmetry_offset,
            ) {
                baseline_found = true;
            } else {
                self.positions.copy_from_slice(&proposed);
            }
        }
        for _ in 0..12 {
            if baseline_found {
                break;
            }
            scale *= 0.5;
            for ((position, &original), &candidate) in
                self.positions.iter_mut().zip(old).zip(&proposed)
            {
                *position = original + (candidate - original) * scale;
            }
            if guard.safe(&self.positions) {
                if refine_scale {
                    // A quantized halving would make an absolute Move target
                    // suddenly shrink when the cursor advances across a guard
                    // limit. Refine the safe/unsafe bracket with bounded work.
                    for _ in 0..12 {
                        let candidate_scale = (scale + unsafe_scale) * 0.5;
                        for ((position, &original), &candidate) in
                            self.positions.iter_mut().zip(old).zip(&proposed)
                        {
                            *position = original + (candidate - original) * candidate_scale;
                        }
                        if guard.safe(&self.positions) {
                            scale = candidate_scale;
                        } else {
                            unsafe_scale = candidate_scale;
                        }
                    }
                    for ((position, &original), &candidate) in
                        self.positions.iter_mut().zip(old).zip(&proposed)
                    {
                        *position = original + (candidate - original) * scale;
                    }
                }
                baseline_found = true;
                break;
            }
            unsafe_scale = scale;
        }
        if !baseline_found {
            self.positions.copy_from_slice(old);
            scale = 0.0;
        }
        if let (
            Some(output),
            Some((symmetry_x, symmetry_offset, center, radius, _normal, camera_axis)),
        ) = (deferred.as_mut(), draw_repair)
        {
            if let Some(anchor) = self.draw_contact_anchor(old, center, camera_axis, radius) {
                **output = Some(DrawDeferred {
                    guard,
                    proposed,
                    anchor,
                    symmetry_x,
                    symmetry_offset,
                    center,
                    radius,
                    camera_axis,
                });
            }
        }
        scale
    }

    /// Keep pre-existing skinny triangles and sharp folds coherent instead of
    /// scaling an entire Draw dab down to zero. Only vertices of faces that
    /// actually reject the proposed dab are grouped. Each group receives one
    /// exactly representable translation; its edge vectors therefore remain
    /// unchanged despite f32 storage. Every candidate still passes the
    /// original face and fold guard, including the transition around groups.
    fn repair_draw_constraint_components(
        &mut self,
        guard: &DeformationGuard,
        old: &[Vec3],
        proposed: &[Vec3],
        symmetry_x: bool,
        symmetry_offset: f32,
    ) -> bool {
        use std::collections::{BTreeMap, BTreeSet};

        fn root(parents: &BTreeMap<usize, usize>, mut vertex: usize) -> usize {
            while let Some(&parent) = parents.get(&vertex) {
                if parent == vertex {
                    break;
                }
                vertex = parent;
            }
            vertex
        }

        fn union(parents: &mut BTreeMap<usize, usize>, a: usize, b: usize) {
            parents.entry(a).or_insert(a);
            parents.entry(b).or_insert(b);
            let a = root(parents, a);
            let b = root(parents, b);
            if a != b {
                parents.insert(b, a);
            }
        }

        const MAX_CONSTRAINED: usize = 512;
        const MAX_SWEEPS: usize = 12;
        let mut parents = BTreeMap::<usize, usize>::new();
        let mut constrained = BTreeSet::<usize>::new();
        let mut previous_groups = usize::MAX;

        for _ in 0..MAX_SWEEPS {
            let previous_vertices = constrained.len();
            for face in guard
                .faces
                .iter()
                .filter(|face| !face.safe(&self.positions))
            {
                let [a, b, c] = face.indices.map(|v| v as usize);
                union(&mut parents, a, b);
                union(&mut parents, a, c);
                constrained.extend([a, b, c]);
            }
            for fold in guard
                .folds
                .iter()
                .filter(|fold| !fold.safe(&self.positions))
            {
                let ids = fold.indices.map(|face| face.map(|v| v as usize));
                let anchor = ids[0][0];
                for &vertex in ids.iter().flatten() {
                    union(&mut parents, anchor, vertex);
                    constrained.insert(vertex);
                }
            }
            if constrained.is_empty() || constrained.len() > MAX_CONSTRAINED {
                return false;
            }

            let mut groups = BTreeMap::<usize, (glam::DVec3, usize, bool, Vec<usize>)>::new();
            for &vertex in &constrained {
                let group = root(&parents, vertex);
                let entry =
                    groups
                        .entry(group)
                        .or_insert((glam::DVec3::ZERO, 0, false, Vec::new()));
                entry.0 += proposed[vertex].as_dvec3() - old[vertex].as_dvec3();
                entry.1 += 1;
                // A group touching an unchanged or protected vertex is fixed.
                // In particular, an edge-of-brush or masked vertex cannot be
                // drawn beyond its authored support/paint protection.
                entry.2 |= proposed[vertex] == old[vertex]
                    || self.mask_influence(vertex) < 1.0 - f32::EPSILON;
                entry.3.push(vertex);
            }

            let translations: BTreeMap<usize, Vec3> = groups
                .iter()
                .map(|(&group, (sum, count, fixed, members))| {
                    let mut translation = Vec3::ZERO;
                    if !*fixed {
                        let minimum_authored_motion = members
                            .iter()
                            .map(|&vertex| {
                                (proposed[vertex].as_dvec3() - old[vertex].as_dvec3()).length()
                            })
                            .fold(f64::INFINITY, f64::min);
                        let mut requested = *sum / *count as f64;
                        let requested_length = requested.length();
                        if requested_length > minimum_authored_motion {
                            requested *= minimum_authored_motion / requested_length;
                        }
                        for _ in 0..4 {
                            translation = Vec3::ZERO;
                            for axis in 0..3 {
                                let ulp = members
                                    .iter()
                                    .map(|&vertex| old[vertex][axis].next_up() - old[vertex][axis])
                                    .fold(0.0_f32, f32::max);
                                if ulp == 0.0 || !ulp.is_finite() {
                                    continue;
                                }
                                let snapped = ((requested[axis] / f64::from(ulp)).trunc()
                                    * f64::from(ulp))
                                    as f32;
                                if !snapped.is_finite() {
                                    continue;
                                }
                                let first = f64::from(old[members[0]][axis] + snapped)
                                    - f64::from(old[members[0]][axis]);
                                if members.iter().all(|&vertex| {
                                    f64::from(old[vertex][axis] + snapped)
                                        - f64::from(old[vertex][axis])
                                        == first
                                }) {
                                    translation[axis] = snapped;
                                }
                            }
                            let actual = (old[members[0]] + translation).as_dvec3()
                                - old[members[0]].as_dvec3();
                            if actual.length() <= minimum_authored_motion {
                                break;
                            }
                            requested *= 0.5;
                        }
                        let actual =
                            (old[members[0]] + translation).as_dvec3() - old[members[0]].as_dvec3();
                        if actual.length() > minimum_authored_motion {
                            translation = Vec3::ZERO;
                        }
                    }
                    (group, translation)
                })
                .collect();

            self.positions.copy_from_slice(proposed);
            for &vertex in &constrained {
                let translation = translations[&root(&parents, vertex)];
                for axis in 0..3 {
                    self.positions[vertex][axis] = if translation[axis] == 0.0 {
                        old[vertex][axis]
                    } else {
                        old[vertex][axis] + translation[axis]
                    };
                }
            }
            if symmetry_x {
                self.preserve_exact_sculpt_symmetry(old, symmetry_offset);
            }
            // The exact-mirror pass can round a paired displacement once more.
            // Keep the authored per-vertex motion as a hard upper bound even
            // after that final f32 realization.
            if constrained.iter().any(|&vertex| {
                let actual = (self.positions[vertex].as_dvec3() - old[vertex].as_dvec3()).length();
                let authored = (proposed[vertex].as_dvec3() - old[vertex].as_dvec3()).length();
                actual > authored
            }) {
                return false;
            }
            if guard.safe(&self.positions) && self.positions != old {
                return true;
            }

            if constrained.len() == previous_vertices && groups.len() == previous_groups {
                break;
            }
            previous_groups = groups.len();
        }
        false
    }

    #[allow(clippy::too_many_arguments)]
    fn solve_draw_one_ring_cage(
        &self,
        old: &[Vec3],
        proposed: &[Vec3],
        guard: &DeformationGuard,
        anchor: DrawContactAnchor,
        center: Vec3,
        camera_axis: Vec3,
        radius: f32,
        symmetry_x: bool,
        symmetry_offset: f32,
        baseline: &[Vec3],
    ) -> Option<Vec<(usize, Vec3)>> {
        let anchor_faces: Vec<_> = guard
            .faces
            .iter()
            .filter(|face| {
                face.indices
                    .iter()
                    .any(|&vertex| anchor.ids.contains(&(vertex as usize)))
            })
            .collect();
        let mut ids: Vec<_> = anchor_faces
            .iter()
            .flat_map(|face| face.indices)
            .map(|id| id as usize)
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids.retain(|&id| {
            proposed[id] != old[id]
                && self.mask_influence(id) >= 1.0 - f32::EPSILON
                && old[id].distance(center) < radius
        });

        if ids.len() < 4 || ids.len() > 16 || !anchor.ids.iter().all(|id| ids.contains(id)) {
            return None;
        }
        let mirror_ids = if symmetry_x {
            let key = |position: Vec3| {
                position
                    .to_array()
                    .map(|value| if value == 0.0 { 0 } else { value.to_bits() })
            };
            let mut old_lookup = HashMap::<[u32; 3], Option<usize>>::new();
            for (id, &position) in old.iter().enumerate() {
                old_lookup
                    .entry(key(position))
                    .and_modify(|entry| *entry = None)
                    .or_insert(Some(id));
            }
            let mirror_center = reflect_point(center, symmetry_offset);
            // If both ends of a unique old-surface mirror pair are selected by
            // the same overlapping brush patch, solve one coupled DOF. Leaving
            // both in `ids` would let the optimizer break exact X reflection.
            let selected = ids.clone();
            ids.retain(|&id| {
                let partner = old_lookup
                    .get(&key(reflect_point(old[id], symmetry_offset)))
                    .copied()
                    .flatten();
                !partner.is_some_and(|other| {
                    other < id
                        && selected.binary_search(&other).is_ok()
                        && self.masks[id] == self.masks[other]
                        && old[other].distance(mirror_center) < radius
                })
            });
            if ids.len() < 3 {
                return None;
            }
            let mut mirrors = Vec::with_capacity(ids.len());
            for &id in &ids {
                let other = old_lookup
                    .get(&key(reflect_point(old[id], symmetry_offset)))
                    .copied()
                    .flatten()
                    .filter(|&other| {
                        other != id
                            && !ids.contains(&other)
                            && proposed[other] != old[other]
                            && self.mask_influence(other) >= 1.0 - f32::EPSILON
                            && self.masks[id] == self.masks[other]
                            && old[other].distance(mirror_center) < radius
                    });
                mirrors.push(other);
            }
            Some(mirrors)
        } else {
            None
        };
        let faces: Vec<_> = guard
            .faces
            .iter()
            .filter(|face| {
                face.indices.iter().any(|id| {
                    ids.binary_search(&(*id as usize)).is_ok()
                        || mirror_ids
                            .as_ref()
                            .is_some_and(|mirrors| mirrors.contains(&Some(*id as usize)))
                })
            })
            .collect();
        let folds: Vec<_> = guard
            .folds
            .iter()
            .filter(|fold| {
                fold.indices.iter().flatten().any(|id| {
                    ids.binary_search(&(*id as usize)).is_ok()
                        || mirror_ids
                            .as_ref()
                            .is_some_and(|mirrors| mirrors.contains(&Some(*id as usize)))
                })
            })
            .collect();
        if faces.len() > if symmetry_x { 256 } else { 128 }
            || folds.len() > if symmetry_x { 512 } else { 256 }
        {
            return None;
        }

        let old_distance = draw_cage_ray_distance(old, anchor, center, camera_axis, radius, None)?;
        let scale = f64::from(radius) / 180.0;
        if symmetry_x {
            return self.solve_draw_slsqp_cage(
                old,
                proposed,
                baseline,
                guard,
                &faces,
                &folds,
                &ids,
                mirror_ids.as_deref(),
                symmetry_offset,
                anchor,
                center,
                camera_axis,
                radius,
                old_distance,
                scale,
            );
        }

        let mut candidate = if symmetry_x {
            baseline.to_vec()
        } else {
            old.to_vec()
        };
        let mut x = vec![0.0_f64; ids.len() * 3];
        for pad_scale in [1.0, 0.5, 0.25, 0.125, 0.0625, 0.03125, 0.0] {
            let (quality_pad, fold_pad, bary_pad) =
                (0.0001 * pad_scale, 0.0005 * pad_scale, 0.03 * pad_scale);

            let mut quality_rounding = vec![0.0_f64; faces.len()];
            let mut orientation_rounding = vec![0.0_f64; faces.len()];
            let mut fold_rounding = vec![0.0_f64; folds.len()];
            let evaluate = |x: &[f64], q_round: &[f64], o_round: &[f64], f_round: &[f64]| {
                draw_cage_values_local(
                    old,
                    proposed,
                    if symmetry_x { baseline } else { old },
                    &faces,
                    &folds,
                    &ids,
                    mirror_ids.as_deref(),
                    symmetry_offset,
                    anchor,
                    center,
                    camera_axis,
                    radius,
                    scale,
                    old_distance,
                    quality_pad,
                    fold_pad,
                    bary_pad,
                    q_round,
                    o_round,
                    f_round,
                    x,
                )
            };
            for _ in 0..16 {
                let values =
                    evaluate(&x, &quality_rounding, &orientation_rounding, &fold_rounding)?;
                let mut columns = Vec::with_capacity(x.len());
                for coordinate in 0..x.len() {
                    let mut plus = x.clone();
                    let mut minus = x.clone();
                    plus[coordinate] += 0.0001;
                    minus[coordinate] -= 0.0001;
                    let upper = evaluate(
                        &plus,
                        &quality_rounding,
                        &orientation_rounding,
                        &fold_rounding,
                    )?;
                    let lower = evaluate(
                        &minus,
                        &quality_rounding,
                        &orientation_rounding,
                        &fold_rounding,
                    )?;
                    columns.push(
                        upper
                            .iter()
                            .zip(lower)
                            .map(|(a, b)| (a - b) / 0.0002)
                            .collect::<Vec<_>>(),
                    );
                }
                let Some(next) = solve_draw_cage_linearized(&x, &values, &columns) else {
                    break;
                };
                let old_merit: f64 = values.iter().map(|v| v.min(0.0).powi(2)).sum();
                let mut advanced = false;
                for fraction in [1.0, 0.5, 0.25, 0.125, 0.0625] {
                    let trial: Vec<_> = x
                        .iter()
                        .zip(&next)
                        .map(|(a, b)| a + (b - a) * fraction)
                        .collect();
                    let Some(new_values) = evaluate(
                        &trial,
                        &quality_rounding,
                        &orientation_rounding,
                        &fold_rounding,
                    ) else {
                        continue;
                    };
                    let merit: f64 = new_values.iter().map(|v| v.min(0.0).powi(2)).sum();
                    if merit < old_merit || merit <= 1e-16 {
                        x = trial;
                        advanced = true;
                        break;
                    }
                }
                if !advanced {
                    break;
                }
                for (slot, &id) in ids.iter().enumerate() {
                    candidate[id] = (old[id].as_dvec3()
                        + glam::DVec3::new(x[3 * slot], x[3 * slot + 1], x[3 * slot + 2]) * scale)
                        .as_vec3();
                    if symmetry_x && old[id].x == symmetry_offset {
                        candidate[id].x = symmetry_offset;
                    }
                    if let Some(Some(other)) = mirror_ids.as_ref().map(|mirrors| mirrors[slot]) {
                        candidate[other] = reflect_point(candidate[id], symmetry_offset);
                    }
                }
                if ids.iter().any(|&id| {
                    (candidate[id].as_dvec3() - old[id].as_dvec3()).length()
                        > (proposed[id].as_dvec3() - old[id].as_dvec3()).length()
                }) || mirror_ids.as_ref().is_some_and(|mirrors| {
                    mirrors.iter().flatten().any(|&id| {
                        (candidate[id].as_dvec3() - old[id].as_dvec3()).length()
                            > (proposed[id].as_dvec3() - old[id].as_dvec3()).length()
                    })
                }) {
                    continue;
                }
                let progressed = draw_cage_ray_distance(
                    &candidate,
                    anchor,
                    center,
                    camera_axis,
                    radius,
                    Some(0.0),
                )
                .is_some_and(|new_distance| {
                    old_distance - new_distance >= f64::from(radius) * 0.002
                });

                // The continuous SQP rows can be safe while the same vertex
                // positions rounded to f32 cross a skinny face or fold limit.
                // Tighten only rows that actually failed after realization.
                if !guard.safe(&candidate) {
                    for (index, face) in faces.iter().enumerate() {
                        let [a, b, c] = face.indices.map(|id| candidate[id as usize].as_dvec3());
                        let cross = (b - a).cross(c - a);
                        let longest = a
                            .distance_squared(b)
                            .max(b.distance_squared(c))
                            .max(c.distance_squared(a));
                        if longest > 0.0 {
                            let deficit = face.minimum_quality - cross.length() / longest;
                            if deficit > 0.0 {
                                quality_rounding[index] =
                                    quality_rounding[index].max(deficit + 0.000002);
                            }
                        }
                        let projection_deficit = (face.minimum_projection - cross.dot(face.cross))
                            / face.cross.length_squared();
                        if projection_deficit > 0.0 {
                            orientation_rounding[index] =
                                orientation_rounding[index].max(projection_deficit + 0.000002);
                        }
                    }
                    for (index, fold) in folds.iter().enumerate() {
                        let [a, b] = fold
                            .indices
                            .map(|triangle| face_cross(&candidate, triangle));
                        let product = a.length() * b.length();
                        if product > 0.0 {
                            let deficit = fold.minimum_cosine - a.dot(b) / product;
                            if deficit > 0.0 {
                                fold_rounding[index] = fold_rounding[index].max(deficit + 0.00001);
                            }
                        }
                    }
                }
                if progressed
                    && guard.safe(&candidate)
                    && draw_local_ray_lift(&candidate, guard, anchor, center, camera_axis, radius)
                        .zip(draw_local_ray_lift(
                            old,
                            guard,
                            anchor,
                            center,
                            camera_axis,
                            radius,
                        ))
                        .is_some_and(|(after, before)| after - before >= f64::from(radius) * 0.002)
                {
                    let mut result: Vec<_> = ids.iter().map(|&id| (id, candidate[id])).collect();
                    if let Some(mirrors) = &mirror_ids {
                        result.extend(mirrors.iter().flatten().map(|&id| (id, candidate[id])));
                    }
                    return Some(result);
                }
            }
        }
        None
    }

    #[allow(clippy::too_many_arguments)]
    fn solve_draw_slsqp_cage(
        &self,
        old: &[Vec3],
        proposed: &[Vec3],
        baseline: &[Vec3],
        guard: &DeformationGuard,
        faces: &[&FaceGuard],
        folds: &[&FoldGuard],
        ids: &[usize],
        mirrors: Option<&[Option<usize>]>,
        symmetry_offset: f32,
        anchor: DrawContactAnchor,
        center: Vec3,
        camera_axis: Vec3,
        radius: f32,
        old_distance: f64,
        scale: f64,
    ) -> Option<Vec<(usize, Vec3)>> {
        use slsqp_rssl::{
            fmin_slsqp_inequalities_with_derivatives, AnalyticDerivatives, Inequality,
        };

        let mirror_ids: Vec<_> = mirrors.into_iter().flatten().copied().flatten().collect();
        let rows = faces.len() * 2 + folds.len() + ids.len() + mirror_ids.len() + 4;
        if rows > 1024 || ids.len() > 16 {
            return None;
        }
        let n = ids.len() * 3;
        let bounds = vec![(-10.0, 10.0); n];
        for (initial_quality_pad, initial_fold_pad, initial_bary_pad, lift_factor) in
            [(0.00005_f64, 0.0002, 0.005, 0.0027), (0.0, 0.0, 0.0, 0.002)]
        {
            let initial_bary_pad: f64 = initial_bary_pad;
            let bary_pad = initial_bary_pad
                .min(anchor.bary.iter().copied().fold(f64::INFINITY, f64::min) * 0.25);
            let mut quality_pad = initial_quality_pad;
            let mut fold_pad = initial_fold_pad;
            for _rounding_attempt in 0..4 {
                let mirror_ids_ref = &mirror_ids;
                let constraints: Vec<Inequality<'_>> = (0..rows)
                    .map(|row| {
                        Box::new(move |x: &[f64]| {
                            draw_slsqp_row(
                                old,
                                proposed,
                                baseline,
                                faces,
                                folds,
                                ids,
                                mirrors,
                                mirror_ids_ref,
                                symmetry_offset,
                                anchor,
                                center,
                                camera_axis,
                                radius,
                                old_distance,
                                scale,
                                quality_pad,
                                fold_pad,
                                bary_pad,
                                lift_factor,
                                row,
                                x,
                            )
                        }) as Inequality<'_>
                    })
                    .collect();
                let jacobian = draw_cage_solver::DrawCageJacobianContext {
                    old,
                    baseline,
                    faces,
                    folds,
                    ids,
                    mirrors,
                    mirror_ids: &mirror_ids,
                    symmetry_offset,
                    anchor,
                    center,
                    camera_axis,
                    radius,
                    scale,
                };
                debug_assert_eq!(jacobian.rows(), rows);
                let result = fmin_slsqp_inequalities_with_derivatives(
                    |x| x.iter().map(|value| value * value).sum(),
                    &vec![0.0; n],
                    &bounds,
                    constraints,
                    AnalyticDerivatives {
                        objective: Box::new(|x, out| {
                            for (derivative, value) in out.iter_mut().zip(x) {
                                *derivative = 2.0 * value;
                            }
                        }),
                        constraints: Box::new(|x, out| jacobian.fill(x, out)),
                    },
                    96,
                    1e-10,
                );

                if result.x.len() != n || result.x.iter().any(|value| !value.is_finite()) {
                    continue;
                }
                let mut candidate = baseline.to_vec();
                for (slot, &id) in ids.iter().enumerate() {
                    candidate[id] = (old[id].as_dvec3()
                        + glam::DVec3::new(
                            result.x[3 * slot],
                            result.x[3 * slot + 1],
                            result.x[3 * slot + 2],
                        ) * scale)
                        .as_vec3();
                    if old[id].x == symmetry_offset {
                        candidate[id].x = symmetry_offset;
                    }
                    if let Some(Some(other)) = mirrors.map(|items| items[slot]) {
                        candidate[other] = reflect_point(candidate[id], symmetry_offset);
                    }
                }
                if guard.safe(&candidate) {
                    repair_draw_rounded_caps(
                        &mut candidate,
                        old,
                        proposed,
                        ids,
                        mirrors,
                        symmetry_offset,
                        guard,
                        anchor,
                        center,
                        camera_axis,
                        radius,
                    );
                }

                let cap_violated = ids.iter().chain(&mirror_ids).any(|&id| {
                    (candidate[id].as_dvec3() - old[id].as_dvec3()).length()
                        > (proposed[id].as_dvec3() - old[id].as_dvec3()).length()
                });
                let guard_safe = guard.safe(&candidate);
                let old_lift =
                    draw_local_ray_lift(old, guard, anchor, center, camera_axis, radius)?;
                let new_lift =
                    draw_local_ray_lift(&candidate, guard, anchor, center, camera_axis, radius);

                if !guard_safe && result.status == 0 {
                    let quality_deficit = faces
                        .iter()
                        .map(|face| {
                            let points = face.indices.map(|id| candidate[id as usize].as_dvec3());
                            let cross = (points[1] - points[0]).cross(points[2] - points[0]);
                            (face.minimum_quality - cross.length() / longest_edge_squared(points))
                                .max(0.0)
                        })
                        .fold(0.0_f64, f64::max);
                    let fold_deficit = folds
                        .iter()
                        .map(|fold| {
                            let [a, b] = fold
                                .indices
                                .map(|triangle| face_cross(&candidate, triangle));
                            (fold.minimum_cosine - a.dot(b) / (a.length() * b.length())).max(0.0)
                        })
                        .fold(0.0_f64, f64::max);
                    if quality_deficit > 0.0 || fold_deficit > 0.0 {
                        quality_pad +=
                            quality_deficit + (quality_deficit > 0.0) as u8 as f64 * 1e-5;
                        fold_pad += fold_deficit + (fold_deficit > 0.0) as u8 as f64 * 2e-5;
                        continue;
                    }
                }
                if cap_violated || !guard_safe {
                    break;
                }
                let Some(new_lift) = new_lift else {
                    break;
                };
                if new_lift - old_lift < f64::from(radius) * 0.001 {
                    break;
                }
                let mut result: Vec<_> = ids.iter().map(|&id| (id, candidate[id])).collect();
                result.extend(mirror_ids.iter().map(|&id| (id, candidate[id])));
                return Some(result);
            }
        }
        None
    }

    /// Experimental local constrained field for a rigid component that reaches
    /// fixed brush support. This only edits already-authored, unmasked members;
    /// the original face/fold guard still decides whether the result may stay.
    fn draw_contact_anchor(
        &self,
        old: &[Vec3],
        center: Vec3,
        camera_axis: Vec3,
        radius: f32,
    ) -> Option<DrawContactAnchor> {
        let near_radius_squared = (radius * 0.1).powi(2);
        let near: Vec<_> = old
            .iter()
            .enumerate()
            .filter_map(|(vertex, &p)| {
                (p.distance_squared(center) < near_radius_squared).then_some(vertex)
            })
            .collect();
        if near.is_empty() || near.len() > 512 {
            return None;
        }
        let candidates = self.cached_faces_for_vertices(&near);
        if candidates.len() > 4096 {
            return None;
        }
        let mut best: Option<(f64, usize, DrawContactAnchor)> = None;
        for face in candidates {
            let ids = self.triangles[face].map(|v| v as usize);
            let Some((distance, bary)) = draw_cage_ray_intersection(
                ids.map(|id| old[id].as_dvec3()),
                center,
                camera_axis,
                radius,
            ) else {
                continue;
            };
            if !(distance >= 1.5 * f64::from(radius)
                && distance <= 2.5 * f64::from(radius)
                && bary.iter().all(|&weight| (0.0..=1.0).contains(&weight)))
            {
                continue;
            }
            if best.as_ref().is_none_or(|(previous, row, _)| {
                distance < *previous || (distance == *previous && face < *row)
            }) {
                best = Some((distance, face, DrawContactAnchor { ids, bary }));
            }
        }
        Some(best?.2)
    }
}

struct DeformationGuard {
    faces: Vec<FaceGuard>,
    folds: Vec<FoldGuard>,
}

#[derive(Clone, Copy)]
struct DrawContactAnchor {
    ids: [usize; 3],
    bary: [f64; 3],
}

struct DrawDeferred {
    guard: DeformationGuard,
    proposed: Vec<Vec3>,
    anchor: DrawContactAnchor,
    symmetry_x: bool,
    symmetry_offset: f32,
    center: Vec3,
    radius: f32,
    camera_axis: Vec3,
}

fn draw_local_ray_lift(
    positions: &[Vec3],
    guard: &DeformationGuard,
    anchor: DrawContactAnchor,
    center: Vec3,
    camera_axis: Vec3,
    radius: f32,
) -> Option<f64> {
    let axis = camera_axis.as_dvec3().normalize_or_zero();
    if axis == glam::DVec3::ZERO {
        return None;
    }
    let start = center.as_dvec3() + axis * (2.0 * f64::from(radius));
    let direction = -axis;
    let mut closest = f64::INFINITY;
    let faces = std::iter::once(anchor.ids.map(|v| v as u32))
        .chain(guard.faces.iter().map(|face| face.indices));
    for ids in faces {
        let [a, b, c] = ids.map(|v| positions[v as usize].as_dvec3());
        let e1 = b - a;
        let e2 = c - a;
        let cross = e1.cross(e2);
        if cross.dot(axis) <= cross.length() * 0.05 {
            continue;
        }
        let p = direction.cross(e2);
        let determinant = e1.dot(p);
        if determinant.abs() <= 1e-24 {
            continue;
        }
        let t = start - a;
        let u = t.dot(p) / determinant;
        if !(0.0..=1.0).contains(&u) {
            continue;
        }
        let q = t.cross(e1);
        let v = direction.dot(q) / determinant;
        if v < 0.0 || u + v > 1.0 {
            continue;
        }
        let distance = e2.dot(q) / determinant;
        if distance.is_finite()
            && distance >= 1.5 * f64::from(radius)
            && distance <= 2.5 * f64::from(radius)
            && distance < closest
        {
            closest = distance;
        }
    }
    closest
        .is_finite()
        .then_some(2.0 * f64::from(radius) - closest)
}

fn draw_cage_ray_intersection(
    corners: [glam::DVec3; 3],
    center: Vec3,
    camera_axis: Vec3,
    radius: f32,
) -> Option<(f64, [f64; 3])> {
    let axis = camera_axis.as_dvec3().normalize_or_zero();
    if axis == glam::DVec3::ZERO {
        return None;
    }
    let origin = center.as_dvec3() + axis * (2.0 * f64::from(radius));
    let direction = -axis;
    let [a, b, c] = corners;
    let e1 = b - a;
    let e2 = c - a;
    let cross = e1.cross(e2);
    if cross.dot(axis) <= cross.length() * 0.05 {
        return None;
    }
    let p = direction.cross(e2);
    let det = e1.dot(p);
    if det.abs() <= 1e-24 {
        return None;
    }
    let t = origin - a;
    let u = t.dot(p) / det;
    let q = t.cross(e1);
    let v = direction.dot(q) / det;
    let distance = e2.dot(q) / det;
    (distance.is_finite() && u.is_finite() && v.is_finite())
        .then_some((distance, [1.0 - u - v, u, v]))
}

fn draw_cage_ray_distance(
    positions: &[Vec3],
    anchor: DrawContactAnchor,
    center: Vec3,
    camera_axis: Vec3,
    radius: f32,
    minimum_bary: Option<f64>,
) -> Option<f64> {
    let (distance, bary) = draw_cage_ray_intersection(
        anchor.ids.map(|id| positions[id].as_dvec3()),
        center,
        camera_axis,
        radius,
    )?;
    (distance >= 1.5 * f64::from(radius)
        && distance <= 2.5 * f64::from(radius)
        && bary
            .iter()
            .all(|&weight| weight >= minimum_bary.unwrap_or(0.0)))
    .then_some(distance)
}

#[allow(clippy::too_many_arguments)]
/// Minimum-motion projection onto the current linearized cage constraints.
/// The bounded contact cage keeps the KKT system small; all nonlinear and
/// rounded geometry is checked separately before a candidate can be accepted.
fn solve_draw_cage_linearized(
    current: &[f64],
    margins: &[f64],
    columns: &[Vec<f64>],
) -> Option<Vec<f64>> {
    let count = margins.len();
    let dimensions = current.len();
    if dimensions == 0
        || dimensions > 48
        || columns.len() != dimensions
        || columns.iter().any(|column| column.len() != count)
    {
        return None;
    }
    let rhs: Vec<_> = (0..count)
        .map(|row| {
            (0..dimensions)
                .map(|coordinate| columns[coordinate][row] * current[coordinate])
                .sum::<f64>()
                - margins[row]
        })
        .collect();
    let mut active = Vec::<usize>::new();
    let mut result = vec![0.0_f64; dimensions];
    for _ in 0..count.min(96) {
        let (worst, violation) = (0..count)
            .map(|row| {
                let projection = (0..dimensions)
                    .map(|coordinate| columns[coordinate][row] * result[coordinate])
                    .sum::<f64>();
                (row, rhs[row] - projection)
            })
            .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0)))?;
        if violation < 1e-9 {
            return Some(result);
        }
        if !active.contains(&worst) {
            if active.len() == dimensions {
                return None;
            }
            active.push(worst);
        }
        let mut solved = false;
        for _ in 0..10 {
            let size = active.len();
            if size == 0 {
                return None;
            }
            let mut gram = vec![vec![0.0_f64; size]; size];
            let mut target = vec![0.0_f64; size];
            for i in 0..size {
                target[i] = rhs[active[i]];
                for j in 0..size {
                    gram[i][j] = (0..dimensions)
                        .map(|coordinate| {
                            columns[coordinate][active[i]] * columns[coordinate][active[j]]
                        })
                        .sum();
                }
                gram[i][i] += 1e-11 * gram[i][i];
            }
            let multipliers = solve_draw_cage_kkt(gram, target)?;
            if let Some((remove, _)) = multipliers
                .iter()
                .enumerate()
                .min_by(|a, b| a.1.total_cmp(b.1))
                .filter(|(_, value)| **value < -1e-8)
            {
                active.remove(remove);
                continue;
            }
            result = (0..dimensions)
                .map(|coordinate| {
                    active
                        .iter()
                        .zip(&multipliers)
                        .map(|(&row, &weight)| columns[coordinate][row] * weight)
                        .sum()
                })
                .collect();
            solved = result.iter().all(|value| value.is_finite());
            break;
        }
        if !solved {
            return None;
        }
    }
    None
}

fn solve_draw_cage_kkt(mut matrix: Vec<Vec<f64>>, mut rhs: Vec<f64>) -> Option<Vec<f64>> {
    let size = rhs.len();
    for column in 0..size {
        let pivot = (column..size)
            .max_by(|&a, &b| matrix[a][column].abs().total_cmp(&matrix[b][column].abs()))?;
        if matrix[pivot][column].abs() < 1e-20 {
            return None;
        }
        matrix.swap(column, pivot);
        rhs.swap(column, pivot);
        let inverse = 1.0 / matrix[column][column];
        for entry in matrix[column].iter_mut().take(size).skip(column) {
            *entry *= inverse;
        }
        rhs[column] *= inverse;
        for row in 0..size {
            if row == column {
                continue;
            }
            let factor = matrix[row][column];
            // Indexed access avoids allocating a pivot-row copy for each elimination.
            #[allow(clippy::needless_range_loop)]
            for entry in column..size {
                matrix[row][entry] -= factor * matrix[column][entry];
            }
            rhs[row] -= factor * rhs[column];
        }
    }
    rhs.iter().all(|value| value.is_finite()).then_some(rhs)
}

#[allow(clippy::too_many_arguments)]
fn repair_draw_rounded_caps(
    candidate: &mut [Vec3],
    old: &[Vec3],
    proposed: &[Vec3],
    ids: &[usize],
    mirrors: Option<&[Option<usize>]>,
    symmetry_offset: f32,
    guard: &DeformationGuard,
    anchor: DrawContactAnchor,
    center: Vec3,
    camera_axis: Vec3,
    radius: f32,
) {
    let excess = |positions: &[Vec3]| -> f64 {
        ids.iter()
            .copied()
            .chain(mirrors.into_iter().flatten().flatten().copied())
            .map(|id| {
                let actual = (positions[id].as_dvec3() - old[id].as_dvec3()).length();
                let authored = (proposed[id].as_dvec3() - old[id].as_dvec3()).length();
                (actual - authored).max(0.0)
            })
            .sum()
    };
    let Some(old_lift) = draw_local_ray_lift(old, guard, anchor, center, camera_axis, radius)
    else {
        return;
    };
    let minimum_lift = f64::from(radius) * 0.001;
    for _ in 0..8 {
        let current_excess = excess(candidate);
        if current_excess == 0.0 {
            return;
        }
        type InwardStep = (f64, usize, Vec3, Option<(usize, Vec3)>);
        let mut best: Option<InwardStep> = None;
        for (slot, &id) in ids.iter().enumerate() {
            let partner = mirrors.and_then(|items| items[slot]);
            for axis in 0..3 {
                let previous = candidate[id][axis];
                let target = old[id][axis];
                let inward = if previous > target {
                    previous.next_down()
                } else if previous < target {
                    previous.next_up()
                } else {
                    continue;
                };
                if inward == previous || !inward.is_finite() {
                    continue;
                }
                candidate[id][axis] = inward;
                let previous_partner = partner.map(|other| (other, candidate[other]));
                if let Some(other) = partner {
                    candidate[other] = reflect_point(candidate[id], symmetry_offset);
                }
                let trial_excess = excess(candidate);
                if trial_excess < current_excess
                    && guard.safe(candidate)
                    && draw_local_ray_lift(candidate, guard, anchor, center, camera_axis, radius)
                        .is_some_and(|lift| lift - old_lift >= minimum_lift)
                    && best
                        .as_ref()
                        .is_none_or(|(best_excess, _, _, _)| trial_excess < *best_excess)
                {
                    best = Some((
                        trial_excess,
                        id,
                        candidate[id],
                        partner.map(|other| (other, candidate[other])),
                    ));
                }
                candidate[id][axis] = previous;
                if let Some((other, original)) = previous_partner {
                    candidate[other] = original;
                }
            }
        }
        let Some((_, id, position, partner)) = best else {
            return;
        };
        candidate[id] = position;
        if let Some((other, position)) = partner {
            candidate[other] = position;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_slsqp_row(
    old: &[Vec3],
    proposed: &[Vec3],
    baseline: &[Vec3],
    faces: &[&FaceGuard],
    folds: &[&FoldGuard],
    ids: &[usize],
    mirrors: Option<&[Option<usize>]>,
    mirror_ids: &[usize],
    symmetry_offset: f32,
    anchor: DrawContactAnchor,
    center: Vec3,
    camera_axis: Vec3,
    radius: f32,
    old_distance: f64,
    scale: f64,
    quality_pad: f64,
    fold_pad: f64,
    bary_pad: f64,
    lift_factor: f64,
    row: usize,
    x: &[f64],
) -> f64 {
    if x.len() != ids.len() * 3 || x.iter().any(|v| !v.is_finite()) {
        return -1e12;
    }
    let point = |id: usize| -> glam::DVec3 {
        if let Ok(slot) = ids.binary_search(&id) {
            let mut p = old[id].as_dvec3()
                + glam::DVec3::new(x[3 * slot], x[3 * slot + 1], x[3 * slot + 2]) * scale;
            if old[id].x == symmetry_offset {
                p.x = f64::from(symmetry_offset);
            }
            p
        } else if let Some((slot, _)) = mirrors.and_then(|items| {
            items
                .iter()
                .enumerate()
                .find(|(_, other)| **other == Some(id))
        }) {
            let p = old[ids[slot]].as_dvec3()
                + glam::DVec3::new(x[3 * slot], x[3 * slot + 1], x[3 * slot + 2]) * scale;
            glam::DVec3::new(2.0 * f64::from(symmetry_offset) - p.x, p.y, p.z)
        } else {
            baseline[id].as_dvec3()
        }
    };
    let mut r = row;
    let result = if r < faces.len() {
        let face = faces[r];
        let [a, b, c] = face.indices.map(|id| point(id as usize));
        let cross = (b - a).cross(c - a);
        let longest = a
            .distance_squared(b)
            .max(b.distance_squared(c))
            .max(c.distance_squared(a));
        cross.length() / longest - face.minimum_quality - quality_pad
    } else {
        r -= faces.len();
        if r < faces.len() {
            let face = faces[r];
            let [a, b, c] = face.indices.map(|id| point(id as usize));
            let cross = (b - a).cross(c - a);
            (cross.dot(face.cross) - face.minimum_projection) / face.cross.length_squared() - 1e-6
        } else {
            r -= faces.len();
            if r < folds.len() {
                let fold = folds[r];
                let [[a, b, c], [d, e, f]] = fold
                    .indices
                    .map(|triangle| triangle.map(|id| point(id as usize)));
                let first = (b - a).cross(c - a);
                let second = (e - d).cross(f - d);
                first.dot(second) / (first.length() * second.length())
                    - fold.minimum_cosine
                    - fold_pad
            } else {
                r -= folds.len();
                if r < ids.len() + mirror_ids.len() {
                    let id = if r < ids.len() {
                        ids[r]
                    } else {
                        mirror_ids[r - ids.len()]
                    };
                    let allowed = (proposed[id].as_dvec3() - old[id].as_dvec3()).length();
                    let interior = 1e-6_f64.min(allowed * 0.01);
                    (allowed - (point(id) - old[id].as_dvec3()).length() - interior) * 100.0
                } else {
                    r -= ids.len() + mirror_ids.len();
                    let corners = anchor.ids.map(point);
                    match draw_cage_ray_intersection(corners, center, camera_axis, radius) {
                        Some((_, bary)) if r < 3 => bary[r] - bary_pad,
                        Some((distance, _)) if r == 3 => {
                            (old_distance - distance - f64::from(radius) * lift_factor) * 100.0
                        }
                        _ => -1e12,
                    }
                }
            }
        }
    };
    if result.is_finite() {
        result
    } else {
        -1e12
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_cage_values_local(
    old: &[Vec3],
    proposed: &[Vec3],
    baseline: &[Vec3],
    faces: &[&FaceGuard],
    folds: &[&FoldGuard],
    ids: &[usize],
    mirror_ids: Option<&[Option<usize>]>,
    symmetry_offset: f32,
    anchor: DrawContactAnchor,
    center: Vec3,
    camera_axis: Vec3,
    radius: f32,
    scale: f64,
    old_distance: f64,
    quality_pad: f64,
    fold_pad: f64,
    bary_pad: f64,
    quality_rounding: &[f64],
    orientation_rounding: &[f64],
    fold_rounding: &[f64],
    x: &[f64],
) -> Option<Vec<f64>> {
    if x.len() != ids.len() * 3 {
        return None;
    }
    let point = |id: u32| {
        let id = id as usize;
        if let Ok(slot) = ids.binary_search(&id) {
            let position = old[id].as_dvec3()
                + glam::DVec3::new(x[3 * slot], x[3 * slot + 1], x[3 * slot + 2]) * scale;
            if mirror_ids.is_some() && old[id].x == symmetry_offset {
                glam::DVec3::new(f64::from(symmetry_offset), position.y, position.z)
            } else {
                position
            }
        } else if let Some((slot, _)) = mirror_ids.and_then(|mirrors| {
            mirrors
                .iter()
                .enumerate()
                .find(|(_, &mirror)| mirror == Some(id))
        }) {
            let position = old[ids[slot]].as_dvec3()
                + glam::DVec3::new(x[3 * slot], x[3 * slot + 1], x[3 * slot + 2]) * scale;
            glam::DVec3::new(
                2.0 * f64::from(symmetry_offset) - position.x,
                position.y,
                position.z,
            )
        } else {
            baseline[id].as_dvec3()
        }
    };
    let mut values = Vec::with_capacity(faces.len() * 2 + folds.len() + ids.len() + 4);
    for (index, face) in faces.iter().enumerate() {
        let [a, b, c] = face.indices.map(point);
        let cross = (b - a).cross(c - a);
        let longest = a
            .distance_squared(b)
            .max(b.distance_squared(c))
            .max(c.distance_squared(a));
        if longest <= 0.0 || !longest.is_finite() {
            return None;
        }
        values.push(
            cross.length() / longest - face.minimum_quality - quality_pad - quality_rounding[index],
        );
    }
    for (index, face) in faces.iter().enumerate() {
        let [a, b, c] = face.indices.map(point);
        let cross = (b - a).cross(c - a);
        values.push(
            (cross.dot(face.cross) - face.minimum_projection) / face.cross.length_squared()
                - orientation_rounding[index],
        );
    }
    for (index, fold) in folds.iter().enumerate() {
        let [[a, b, c], [d, e, f]] = fold.indices.map(|triangle| triangle.map(point));
        let first = (b - a).cross(c - a);
        let second = (e - d).cross(f - d);
        let product = first.length() * second.length();
        if product <= 0.0 || !product.is_finite() {
            return None;
        }
        values.push(
            first.dot(second) / product - fold.minimum_cosine - fold_pad - fold_rounding[index],
        );
    }
    for &id in ids {
        let allowed = (proposed[id].as_dvec3() - old[id].as_dvec3()).length();
        let actual = (point(id as u32) - old[id].as_dvec3()).length();
        let rounding_margin =
            (f64::from(f32::EPSILON) * f64::from(old[id].abs().max_element()) * 0.5)
                .min(allowed * 0.25);
        values.push((allowed - rounding_margin - actual) * 100.0);
    }
    if let Some(mirrors) = mirror_ids {
        for &id in mirrors.iter().flatten() {
            let allowed = (proposed[id].as_dvec3() - old[id].as_dvec3()).length();
            let actual = (point(id as u32) - old[id].as_dvec3()).length();
            let rounding_margin =
                (f64::from(f32::EPSILON) * f64::from(old[id].abs().max_element()) * 0.5)
                    .min(allowed * 0.25);
            values.push((allowed - rounding_margin - actual) * 100.0);
        }
    }
    let corners = anchor.ids.map(|id| point(id as u32));
    let (distance, bary) = draw_cage_ray_intersection(corners, center, camera_axis, radius)?;
    values.extend(bary.map(|weight| weight - bary_pad));
    values.push((old_distance - distance - f64::from(radius) / 400.0) * 100.0);
    values
        .iter()
        .all(|value| value.is_finite())
        .then_some(values)
}

impl DeformationGuard {
    fn safe(&self, positions: &[Vec3]) -> bool {
        self.faces.iter().all(|face| face.safe(positions))
            && self.folds.iter().all(|fold| fold.safe(positions))
    }
}

struct FaceGuard {
    indices: [u32; 3],
    cross: glam::DVec3,
    minimum_projection: f64,
    minimum_quality: f64,
}

struct FoldGuard {
    indices: [[u32; 3]; 2],
    minimum_cosine: f64,
}

impl FaceGuard {
    fn safe(&self, positions: &[Vec3]) -> bool {
        let points = self
            .indices
            .map(|vertex| positions[vertex as usize].as_dvec3());
        let cross = (points[1] - points[0]).cross(points[2] - points[0]);
        let edge_squared = longest_edge_squared(points);
        let relative_tolerance = 32.0 * f64::EPSILON;
        cross.is_finite()
            && cross.dot(self.cross) >= self.minimum_projection
            && cross.length_squared() * (1.0 + relative_tolerance)
                >= (self.minimum_quality * edge_squared).powi(2)
    }
}

impl FoldGuard {
    fn safe(&self, positions: &[Vec3]) -> bool {
        let [a, b] = self.indices.map(|triangle| face_cross(positions, triangle));
        let product = a.length() * b.length();
        a.dot(b) + 32.0 * f64::EPSILON * product >= self.minimum_cosine * product
    }
}

fn surface_alpha_frame(normal: Vec3, tangent: Option<Vec3>, rotation: f32) -> (Vec3, Vec3) {
    let fallback = if normal.x.abs() < 0.9 {
        Vec3::X
    } else {
        Vec3::Y
    };
    let project =
        |direction: Vec3| (direction - normal * direction.dot(normal)).normalize_or_zero();
    let mut tangent = project(tangent.unwrap_or(fallback));
    if tangent == Vec3::ZERO {
        tangent = project(fallback);
    }
    let bitangent = normal.cross(tangent).normalize_or_zero();
    let (sine, cosine) = rotation.sin_cos();
    (
        tangent * cosine + bitangent * sine,
        bitangent * cosine - tangent * sine,
    )
}

fn valid_brush_options(settings: &BrushSettings) -> bool {
    settings.paint_targets_are_valid()
        && settings.hardness.is_finite()
        && settings.symmetry_offset.is_finite()
        && settings.symmetry_offset.abs() <= 1_000_000.0
        && settings
            .culling_axis
            .is_none_or(|axis| axis.is_finite() && axis != Vec3::ZERO)
}

fn brush_culling_axis(settings: &BrushSettings, normal: Vec3) -> Vec3 {
    settings
        .culling_axis
        .map_or(normal, |axis| axis.as_dvec3().normalize().as_vec3())
}

fn reflect_point(point: Vec3, offset: f32) -> Vec3 {
    Vec3::new(
        if offset == 0.0 {
            -point.x
        } else {
            (2.0 * f64::from(offset) - f64::from(point.x)) as f32
        },
        point.y,
        point.z,
    )
}

fn nearest_vertex(positions: &[Vec3], center: Vec3) -> Option<usize> {
    positions
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            a.distance_squared(center)
                .total_cmp(&b.distance_squared(center))
        })
        .map(|(vertex, _)| vertex)
}

fn radial_influence(
    distance_squared: f32,
    radius_squared: f32,
    settings: &BrushSettings,
    move_kernel: bool,
) -> f32 {
    let kernel = if settings.kind == BrushKind::Twist {
        // SculptGL Twist uses a distinct smooth radial angle profile. Keep
        // this scoped to Twist; Local Scale and ordinary stamps retain theirs.
        let distance = (distance_squared / radius_squared).sqrt();
        let distance_squared = distance * distance;
        1.0 - 4.0 * distance_squared * distance + 3.0 * distance_squared * distance_squared
    } else if move_kernel {
        let distance = (distance_squared / radius_squared).sqrt();
        (1.0 - distance).powi(4) * (1.0 + 4.0 * distance)
    } else {
        (1.0 - distance_squared / radius_squared).powi(2)
    };
    let hardness = settings.hardness.clamp(0.0, 1.0);
    if hardness == 0.0 {
        kernel
    } else if hardness == 1.0 {
        1.0
    } else {
        kernel.powf(1.0 - hardness)
    }
}

fn facing_influence(facing: f32, settings: &BrushSettings) -> f32 {
    if !settings.backface_culling {
        return 1.0;
    }
    if facing <= 0.0 {
        return 0.0;
    }
    if matches!(settings.kind, BrushKind::Mask | BrushKind::Paint) {
        return 1.0;
    }
    let front = (facing / 0.35).clamp(0.0, 1.0);
    front * front * (3.0 - 2.0 * front)
}

fn face_cross(positions: &[Vec3], triangle: [u32; 3]) -> glam::DVec3 {
    let [a, b, c] = triangle.map(|vertex| positions[vertex as usize].as_dvec3());
    (b - a).cross(c - a)
}

fn longest_edge_squared([a, b, c]: [glam::DVec3; 3]) -> f64 {
    a.distance_squared(b)
        .max(b.distance_squared(c))
        .max(c.distance_squared(a))
}

fn uses_surface_detail(kind: BrushKind) -> bool {
    matches!(
        kind,
        BrushKind::Draw
            | BrushKind::Clay
            | BrushKind::Inflate
            | BrushKind::Flatten
            | BrushKind::Crease
            | BrushKind::Pinch
            | BrushKind::Drag
    )
}

#[cfg(test)]
mod draw_constraint_component_tests {
    use super::*;

    fn skinny_patch() -> Mesh {
        Mesh::new(
            vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.2, 0.05, 0.0),
                Vec3::new(1.0, 1.0, 0.0),
            ],
            vec![[0, 1, 2], [1, 3, 2]],
        )
    }

    #[test]
    fn coherent_draw_component_moves_despite_preexisting_skinny_face() {
        let mut mesh = skinny_patch();
        mesh.ensure_adjacency();
        let old = mesh.positions.clone();
        for position in &mut mesh.positions {
            position.z += 0.02;
        }
        mesh.positions[2].y -= 0.005;
        let proposed = mesh.positions.clone();
        let mut clipped = mesh.snapshot();
        clipped.ensure_adjacency();
        let clipped_scale = clipped.accept_deformation(&old, BrushKind::Draw, None, None);
        assert!(clipped_scale < 1.0, "fixture must clip the full proposal");

        let scale = mesh.accept_deformation(
            &old,
            BrushKind::Draw,
            Some((false, 0.0, Vec3::ZERO, 1.0, Vec3::Z, Vec3::Z)),
            None,
        );
        assert_eq!(scale, 1.0);
        assert_ne!(mesh.positions, old);
        assert_ne!(mesh.positions, proposed);
        let changed: Vec<_> = mesh
            .positions
            .iter()
            .zip(&old)
            .enumerate()
            .filter_map(|(i, (a, b))| (a != b).then_some(i))
            .collect();
        let guard = mesh
            .deformation_guard(&old, &changed, 0.1, 0.5, false)
            .unwrap();
        assert!(guard.safe(&mesh.positions));
        mesh.validate().unwrap();
    }

    #[test]
    fn zero_area_rest_face_is_not_repaired_or_moved() {
        let mut mesh = Mesh::new(vec![Vec3::ZERO, Vec3::X, Vec3::X * 0.5], vec![[0, 1, 2]]);
        mesh.ensure_adjacency();
        let old = mesh.positions.clone();
        mesh.positions[2].z = 0.1;
        assert_eq!(
            mesh.accept_deformation(
                &old,
                BrushKind::Draw,
                Some((false, 0.0, Vec3::ZERO, 1.0, Vec3::Z, Vec3::Z)),
                None
            ),
            0.0
        );
        assert_eq!(mesh.positions, old);
    }

    #[test]
    fn mirrored_skinny_components_keep_exact_x_pairs() {
        let right = [
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(1.2, 0.05, 0.0),
        ];
        for offset in [0.0, 0.25] {
            let mut positions = right.to_vec();
            positions.extend(right.map(|p| reflect_point(p, offset)));
            let mut mesh = Mesh::new(positions, vec![[0, 1, 2], [3, 5, 4]]);
            mesh.ensure_adjacency();
            let old = mesh.positions.clone();
            for position in &mut mesh.positions {
                position.z += 0.02;
            }
            mesh.positions[2].y -= 0.005;
            mesh.positions[5].y -= 0.005;
            assert_eq!(
                mesh.accept_deformation(
                    &old,
                    BrushKind::Draw,
                    Some((true, offset, Vec3::ZERO, 1.0, Vec3::Z, Vec3::Z)),
                    None
                ),
                1.0
            );
            for index in 0..3 {
                assert_eq!(
                    mesh.positions[index],
                    reflect_point(mesh.positions[index + 3], offset)
                );
            }
            mesh.validate().unwrap();
        }
    }

    #[test]
    fn protected_vertex_remains_fixed_on_constraint_retry() {
        let mut mesh = skinny_patch();
        mesh.ensure_adjacency();
        mesh.masks[0] = 1.0;
        let old = mesh.positions.clone();
        for position in &mut mesh.positions[1..] {
            position.z += 0.02;
        }
        mesh.positions[2].y -= 0.005;
        let _ = mesh.accept_deformation(
            &old,
            BrushKind::Draw,
            Some((false, 0.0, Vec3::ZERO, 1.0, Vec3::Z, Vec3::Z)),
            None,
        );
        assert_eq!(
            mesh.positions[0].to_array().map(f32::to_bits),
            old[0].to_array().map(f32::to_bits)
        );
        assert_eq!(mesh.masks[0], 1.0);
        mesh.validate().unwrap();
    }
}

#[cfg(test)]
mod sparse_source_guard_tests {
    use super::*;

    fn old_guard(mesh: &mut Mesh, old: &[Vec3]) -> f32 {
        mesh.ensure_adjacency();
        let proposed = mesh.positions.clone();
        let scale = mesh.accept_deformation_with_limits(old, 0.1, 0.5, false);
        for ((point, previous), candidate) in mesh.positions.iter_mut().zip(old).zip(proposed) {
            for component in 0..3 {
                if candidate[component].to_bits() == previous[component].to_bits() {
                    point[component] = previous[component];
                }
            }
        }
        scale
    }

    #[test]
    fn sparse_source_guard_matches_full_guard_on_local_and_folded_candidates() {
        let original = Mesh::sphere(2);
        let mut reduced_scale = false;
        for amplitude in [0.0, 0.1, 0.5, 1.0, 2.0, 5.0, -0.5, -2.0] {
            for count in [1, 4, 16] {
                let old = original.positions.clone();
                let mut sparse = original.clone();
                let mut full = original.clone();
                let mut proposals = Vec::new();
                for step in 0..count {
                    // Deliberately reverse the producer order. The guard must
                    // still see the generic path's ascending changed IDs.
                    let vertex = 47 - step;
                    let candidate = old[vertex]
                        + Vec3::new(0.4, -0.7, 1.0) * amplitude * (1.0 + step as f32 * 0.04);
                    proposals.push((vertex, candidate));
                    sparse.positions[vertex] = candidate;
                    full.positions[vertex] = candidate;
                }
                let actual_scale = sparse.accept_source_geometry_candidate_sparse(&old, &proposals);
                let expected_scale = old_guard(&mut full, &old);
                reduced_scale |= expected_scale > 0.0 && expected_scale < 1.0;
                assert_eq!(actual_scale.to_bits(), expected_scale.to_bits());
                assert_eq!(
                    sparse
                        .positions
                        .iter()
                        .map(|v| v.to_array().map(f32::to_bits))
                        .collect::<Vec<_>>(),
                    full.positions
                        .iter()
                        .map(|v| v.to_array().map(f32::to_bits))
                        .collect::<Vec<_>>()
                );
            }
        }
        assert!(reduced_scale, "fixture must exercise guarded rescaling");
    }
}

#[cfg(test)]
mod small_radius_move_quality_tests {
    use super::*;

    fn maximum_adjacent_fold(mesh: &mut Mesh) -> f64 {
        mesh.ensure_adjacency();
        mesh.cached_face_pairs()
            .iter()
            .map(|&[a, b]| {
                let first =
                    face_cross(&mesh.positions, mesh.triangles[a as usize]).normalize_or_zero();
                let second =
                    face_cross(&mesh.positions, mesh.triangles[b as usize]).normalize_or_zero();
                first.dot(second).clamp(-1.0, 1.0).acos().to_degrees()
            })
            .fold(0.0, f64::max)
    }

    #[test]
    fn small_symmetric_move_keeps_visible_fold_bounded_and_still_moves() {
        // Independently authored analogue of the reported .037/.79 symmetric
        // pull on a level-six sphere. The rest surface is deliberately dense;
        // the small selected footprint still needs local refinement.
        let mut mesh = Mesh::sphere(6);
        let original = mesh.snapshot();
        let center = Vec3::new(-0.133, 0.245, 0.963).normalize();
        let settings = BrushSettings {
            kind: BrushKind::Move,
            radius: 0.037,
            strength: 0.79,
            symmetry_x: true,
            ..BrushSettings::default()
        };
        let mut stroke = mesh.prepare_move_stroke(center, center, &settings).unwrap();
        let result = stroke.apply(&mut mesh, Vec3::new(-0.06, 0.021, -0.026), &settings);
        assert!(result.changed_vertices > 0, "Move became an inert brush");
        assert!(
            result.refined_vertices > 0,
            "small footprint was not resolved"
        );
        let maximum_displacement = mesh
            .positions
            .iter()
            .zip(&original.positions)
            .map(|(&after, &before)| after.distance(before))
            .fold(0.0_f32, f32::max);
        assert!(
            maximum_displacement >= 0.002,
            "Move was over-clipped: {maximum_displacement}"
        );
        let fold = maximum_adjacent_fold(&mut mesh);
        eprintln!(
            "small Move: changed={}, refined={}, scale={:.6}, displacement={maximum_displacement:.6}, max_fold={fold:.3}",
            result.changed_vertices, result.refined_vertices, result.displacement_scale
        );
        assert!(fold <= 30.01, "Move made a visible sharp fold: {fold}");
        mesh.validate().unwrap();
    }
}
