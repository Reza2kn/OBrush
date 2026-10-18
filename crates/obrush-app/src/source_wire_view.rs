//! Render-only original-face wires. Corner UV seams never introduce edges.
use obrush_core::{MAX_TRIANGLES, MAX_VERTICES, Mesh, TRIANGLE_CORNER};

pub(crate) fn original_wire_edges(mesh: &Mesh) -> Result<Vec<[u32; 2]>, String> {
    if mesh.positions.len() > MAX_VERTICES || mesh.triangles.len() > MAX_TRIANGLES {
        return Err("Wire geometry exceeds the bounded mesh limits".into());
    }
    let capacity = mesh
        .triangles
        .len()
        .checked_mul(3)
        .ok_or("Wire count overflow")?;
    let mut edges = Vec::new();
    edges
        .try_reserve_exact(capacity)
        .map_err(|_| "Cannot allocate bounded original-face wires")?;
    if let Some(surface) = &mesh.surface {
        surface.validate(mesh.positions.len(), &mesh.triangles)?;
        for face in &surface.faces {
            let arity = if face[3] == TRIANGLE_CORNER { 3 } else { 4 };
            for index in 0..arity {
                let a = face[index];
                let b = face[(index + 1) % arity];
                edges.push([a.min(b), a.max(b)]);
            }
        }
    } else {
        for &[a, b, c] in &mesh.triangles {
            if [a, b, c]
                .iter()
                .any(|&id| id as usize >= mesh.positions.len())
            {
                return Err("Wire triangle refers outside the original geometry".into());
            }
            for [a, b] in [[a, b], [b, c], [c, a]] {
                edges.push([a.min(b), a.max(b)]);
            }
        }
    }
    edges.sort_unstable();
    edges.dedup();
    Ok(edges)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjacent_quads_keep_seam_edge_once_without_render_diagonals() {
        let mesh = Mesh::import_obj(b"v 0 0 0\nv 1 0 0\nv 2 0 0\nv 0 1 0\nv 1 1 0\nv 2 1 0\nvt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\nvt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\nf 1/1 2/2 5/3 4/4\nf 2/5 3/6 6/7 5/8\n").unwrap().mesh;
        let before = serde_json::to_vec(&mesh).unwrap();
        assert_eq!(
            original_wire_edges(&mesh).unwrap(),
            [[0, 1], [0, 3], [1, 2], [1, 4], [2, 5], [3, 4], [4, 5]]
        );
        assert_eq!(serde_json::to_vec(&mesh).unwrap(), before);
        let mut legacy = mesh.clone();
        legacy.surface = None;
        let triangle_edges = original_wire_edges(&legacy).unwrap();
        assert_eq!(triangle_edges.len(), 9);
        assert!(triangle_edges.contains(&[0, 4]) && triangle_edges.contains(&[1, 5]));
    }

    #[test]
    fn inconsistent_original_face_metadata_rejects_without_mutation() {
        let mut mesh = Mesh::import_obj(b"v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\n")
            .unwrap()
            .mesh;
        mesh.surface.as_mut().unwrap().faces[0][3] = 99;
        let before = serde_json::to_vec(&mesh).unwrap();
        assert!(original_wire_edges(&mesh).is_err());
        assert_eq!(serde_json::to_vec(&mesh).unwrap(), before);
    }
}
