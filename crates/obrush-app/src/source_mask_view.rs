//! A render-only unprotected channel. Canonical raw U words never pass through P.
use obrush_core::{MaskConvention, Mesh};

pub(crate) fn source_mask_words(
    mesh: &Mesh,
    geometric_ids: Option<&[[u32; 2]]>,
) -> Result<Vec<f32>, String> {
    if mesh.positions.len() != mesh.masks.len() || mesh.positions.len() > obrush_core::MAX_VERTICES
    {
        return Err("Source mask render channel has inconsistent bounded counts".into());
    }
    let count = geometric_ids.map_or(mesh.positions.len(), <[[u32; 2]]>::len);
    if count
        > obrush_core::MAX_TRIANGLES
            .checked_mul(3)
            .ok_or("Source mask row limit overflow")?
    {
        return Err("Source mask seam rows exceed the render allocation limit".into());
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(count)
        .map_err(|_| "Cannot allocate bounded source mask render rows")?;
    for row in 0..count {
        let id = geometric_ids.map_or(row, |ids| ids[row][0] as usize);
        let raw = *mesh
            .masks
            .get(id)
            .ok_or("Source mask seam maps outside original geometry")?;
        if !raw.is_finite() || !(0.0..=1.0).contains(&raw) {
            return Err("Source mask requires finite canonical unit values".into());
        }
        output.push(match mesh.mask_convention {
            MaskConvention::UnprotectedF32V1 => raw,
            MaskConvention::ProtectedF32V1 => (1.0 - f64::from(raw)) as f32,
        });
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cube_with_eight_geometric_vertices() -> Mesh {
        let mesh = Mesh::import_obj(b"v -1 -1 -1\nv 1 -1 -1\nv 1 1 -1\nv -1 1 -1\nv -1 -1 1\nv 1 -1 1\nv 1 1 1\nv -1 1 1\nf 1 4 3 2\nf 5 6 7 8\nf 1 2 6 5\nf 4 8 7 3\nf 1 5 8 4\nf 2 3 7 6\n").unwrap().mesh;
        assert_eq!(mesh.positions.len(), 8);
        mesh
    }
    #[test]
    fn tiny_and_signed_zero_unprotected_words_remain_exact_through_seam_mapping() {
        let mut mesh = cube_with_eight_geometric_vertices();
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        let words = [
            0x80000000, 0x00000001, 0x00800000, 0x33800000, 0x3e800000, 0x3f000000, 0x3f7fffff,
            0x3f800000,
        ];
        mesh.masks = words.map(f32::from_bits).to_vec();
        let rows = [[1, 0], [7, 1], [1, 2], [0, 3], [2, 4]];
        assert_eq!(
            source_mask_words(&mesh, None)
                .unwrap()
                .iter()
                .map(|x| x.to_bits())
                .collect::<Vec<_>>(),
            words
        );
        assert_eq!(
            source_mask_words(&mesh, Some(&rows))
                .unwrap()
                .iter()
                .map(|x| x.to_bits())
                .collect::<Vec<_>>(),
            [words[1], words[7], words[1], words[0], words[2]]
        );
    }
    #[test]
    fn protected_display_conversion_does_not_mutate_canonical_document() {
        let mut mesh = cube_with_eight_geometric_vertices();
        mesh.masks = vec![0.0, 0.25, 0.5, 0.75, 1.0, 0.0, 0.25, 1.0];
        let before = serde_json::to_vec(&mesh).unwrap();
        assert_eq!(
            source_mask_words(&mesh, None).unwrap(),
            [1.0, 0.75, 0.5, 0.25, 0.0, 1.0, 0.75, 0.0]
        );
        assert_eq!(serde_json::to_vec(&mesh).unwrap(), before);
        assert!(source_mask_words(&mesh, Some(&[[8, 0]])).is_err());
        assert_eq!(serde_json::to_vec(&mesh).unwrap(), before);
    }
}
