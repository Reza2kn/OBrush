use super::*;
use crate::source_worker_warm_io::{Reader, Writer};

impl SourceGeometryNormalState {
    pub(crate) fn encode_warm(&self, mesh: &Mesh, out: &mut Writer) -> Result<(), String> {
        validate_mesh(mesh).map_err(|e| e.to_string())?;
        if !self.matches_mesh(mesh) {
            return Err("Source warm state is stale for mesh".into());
        }
        if self.approximate_bytes() > MAX_SOURCE_GEOMETRY_STATE_BYTES {
            return Err("Source warm state exceeds storage budget".into());
        }
        out.u64(self.generation)?;
        out.u8(u8::from(self.stored_original_faces))?;
        for word in self.interaction_center {
            out.u32(word.to_bits())?;
        }
        out.usize(self.position_words.len())?;
        for row in &self.position_words {
            for &word in row {
                out.u32(word)?;
            }
        }
        out.usize(self.face_normals.len())?;
        for normal in &self.face_normals {
            for word in normal.to_array() {
                out.u32(word.to_bits())?;
            }
        }
        out.usize(self.vertex_normals.len())?;
        for normal in &self.vertex_normals {
            for word in normal.to_array() {
                out.u32(word.to_bits())?;
            }
        }
        for chunk in self.render_selected_chunks {
            out.u64(chunk)?;
        }
        self.ordered_tree.encode_warm(out)?;
        Ok(())
    }

    pub(crate) fn decode_warm(mesh: &Mesh, input: &mut Reader<'_>) -> Result<Self, String> {
        validate_mesh(mesh).map_err(|e| e.to_string())?;
        // This also establishes canonical immutable face rows and their first
        // triangle mapping without trusting private bytes to invent topology.
        let mut state = Self::seed(mesh).map_err(|e| e.to_string())?;
        let generation = input.u64()?;
        let stored_original_faces = match input.u8()? {
            0 => false,
            1 => true,
            _ => return Err("Invalid Source warm surface tag".into()),
        };
        if stored_original_faces != mesh.surface.is_some() {
            return Err("Source warm surface topology differs from mesh".into());
        }
        let mut interaction_center = [0.0f32; 3];
        for word in &mut interaction_center {
            *word = f32::from_bits(input.u32()?);
        }
        if interaction_center.iter().any(|v| !v.is_finite()) {
            return Err("Nonfinite Source warm interaction center".into());
        }
        if input.usize()? != mesh.positions.len() {
            return Err("Source warm position count differs from mesh".into());
        }
        let mut position_words = reserved(mesh.positions.len()).map_err(|e| e.to_string())?;
        for position in &mesh.positions {
            let row = [input.u32()?, input.u32()?, input.u32()?];
            if row != position.to_array().map(f32::to_bits) {
                return Err("Source warm position words differ from mesh".into());
            }
            position_words.push(row);
        }
        if input.usize()? != state.face_normals.len() {
            return Err("Source warm face normal count differs from mesh".into());
        }
        let mut face_normals = reserved(state.face_normals.len()).map_err(|e| e.to_string())?;
        for _ in 0..state.face_normals.len() {
            let row = [input.u32()?, input.u32()?, input.u32()?].map(f32::from_bits);
            let normal = Vec3::from_array(row);
            if !normal.is_finite() {
                return Err("Nonfinite Source warm face normal".into());
            }
            face_normals.push(normal);
        }
        if input.usize()? != state.vertex_normals.len() {
            return Err("Source warm vertex normal count differs from mesh".into());
        }
        let mut vertex_normals = reserved(state.vertex_normals.len()).map_err(|e| e.to_string())?;
        for _ in 0..state.vertex_normals.len() {
            let row = [input.u32()?, input.u32()?, input.u32()?].map(f32::from_bits);
            let normal = Vec3::from_array(row);
            if !normal.is_finite() {
                return Err("Nonfinite Source warm vertex normal".into());
            }
            vertex_normals.push(normal);
        }
        let mut selected_chunks = [0u64; SOURCE_GEOMETRY_CHUNK_WORDS];
        for chunk in &mut selected_chunks {
            *chunk = input.u64()?;
        }
        let tree = OrderedTree::decode_warm(input, &mesh.positions, &state.faces)?;
        state.generation = generation;
        state.stored_original_faces = stored_original_faces;
        state.interaction_center = interaction_center;
        state.position_words = position_words;
        state.face_normals = face_normals;
        state.vertex_normals = vertex_normals;
        state.ordered_tree = tree;
        // An Arc::Weak predecessor cannot cross realms. The bits are retained,
        // but without pointer identity they cannot authorize a sparse render.
        state.render_predecessor = None;
        state.render_selected_chunks = selected_chunks;
        if state.approximate_bytes() > MAX_SOURCE_GEOMETRY_STATE_BYTES || !state.matches_mesh(mesh)
        {
            return Err("Decoded Source warm state exceeds budget or mesh binding".into());
        }
        Ok(state)
    }
}
