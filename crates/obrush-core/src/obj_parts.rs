//! Source-style OBJ object splitting over the existing validated mesh parser.
use super::*;

impl Mesh {
    /// Keep one parsed part per pinned `o` boundary. Positive OBJ indices are
    /// global declarations, while negative indices are relative to the current
    /// object. Cross-object references cannot form an editable local mesh and
    /// reject before any document installation.
    pub fn import_obj_parts(source: &[u8]) -> Result<Vec<MeshImport>, String> {
        if source.is_empty() || source.len() > MAX_INPUT_BYTES {
            return Err("OBJ requires nonempty input within 128 MiB".into());
        }
        let text = std::str::from_utf8(source).map_err(|_| "OBJ must be UTF-8 text")?;
        Bounds::scan(text)?;
        let object_count = text
            .lines()
            .filter(|line| line.trim_start().starts_with("o "))
            .count();
        if object_count < 2 {
            return Ok(vec![Self::import_obj(source)?]);
        }
        if object_count > crate::MAX_SUBTOOLS {
            return Err("OBJ declares more than 32 objects".into());
        }
        let mut parts = Vec::new();
        parts
            .try_reserve_exact(object_count)
            .map_err(|_| "Cannot allocate bounded OBJ object list")?;
        let mut current = String::new();
        let mut emitted_bytes = 0_usize;
        let mut vertices = 0_usize;
        let mut coordinates = 0_usize;
        let mut normals = 0_usize;
        let mut vertex_base = 0_usize;
        let mut coordinate_base = 0_usize;
        let mut normal_base = 0_usize;
        let mut opened = false;
        for (line, raw) in text.lines().enumerate() {
            let clean = raw.split('#').next().unwrap_or("").trim();
            let kind = clean.split_whitespace().next().unwrap_or("");
            if kind == "o" && raw.trim_start().starts_with("o ") {
                if opened {
                    parts.push(Self::import_obj(current.as_bytes()).map_err(|error| {
                        failure(line, &format!("previous OBJ object is invalid: {error}"))
                    })?);
                    current.clear();
                    vertex_base = vertices;
                    coordinate_base = coordinates;
                    normal_base = normals;
                }
                opened = true;
                continue;
            }
            let output = if kind == "f" {
                let mut face = String::from("f");
                for token in clean.split_whitespace().skip(1) {
                    let mut fields = token.split('/');
                    let vertex =
                        local_obj_index(fields.next().unwrap_or(""), vertices, vertex_base)
                            .map_err(|error| failure(line, error))?;
                    face.push(' ');
                    face.push_str(&vertex);
                    if let Some(uv) = fields.next() {
                        face.push('/');
                        if !uv.is_empty() {
                            face.push_str(
                                &local_obj_index(uv, coordinates, coordinate_base)
                                    .map_err(|error| failure(line, error))?,
                            );
                        }
                    }
                    if let Some(normal) = fields.next() {
                        face.push('/');
                        if !normal.is_empty() {
                            face.push_str(
                                &local_obj_index(normal, normals, normal_base)
                                    .map_err(|error| failure(line, error))?,
                            );
                        }
                    }
                    if fields.next().is_some() {
                        return Err(failure(line, "face has more than three index components"));
                    }
                }
                face
            } else {
                raw.to_owned()
            };
            let needed = output
                .len()
                .checked_add(1)
                .ok_or("OBJ object text overflow")?;
            emitted_bytes = emitted_bytes
                .checked_add(needed)
                .ok_or("OBJ object text overflow")?;
            if emitted_bytes > MAX_INPUT_BYTES {
                return Err("OBJ object staging exceeds 128 MiB".into());
            }
            current
                .try_reserve(needed)
                .map_err(|_| "Cannot allocate bounded OBJ object text")?;
            current.push_str(&output);
            current.push('\n');
            match kind {
                "v" => vertices += 1,
                "vt" => coordinates += 1,
                "vn" => normals += 1,
                _ => {}
            }
        }
        parts.push(Self::import_obj(current.as_bytes())?);
        Ok(parts)
    }
}

fn local_obj_index(raw: &str, global_count: usize, base: usize) -> Result<String, &'static str> {
    let parsed = raw.parse::<i64>().map_err(|_| "invalid OBJ face index")?;
    if parsed == 0 {
        return Err("OBJ face index cannot be zero");
    }
    let local_count = global_count
        .checked_sub(base)
        .ok_or("OBJ object index base exceeds declarations")?;
    if parsed < 0 {
        if parsed.unsigned_abs() as usize > local_count {
            return Err("OBJ face references another object's declarations");
        }
        return Ok(raw.to_owned());
    }
    let positive = usize::try_from(parsed).map_err(|_| "OBJ face index exceeds bounds")?;
    if positive <= base || positive > global_count {
        return Err("OBJ face references another object's declarations");
    }
    Ok((positive - base).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    const TWO: &[u8] = b"o Left\nv 0 0 0 1 0 0\nv 1 0 0 1 0 0\nv 0 1 0 1 0 0\nvt 0 0\nvt 1 0\nvt 0 1\nf 1/1 2/2 3/3\no Right\nv 4 0 0 0 0 1\nv 5 0 0 0 0 1\nv 4 1 0 0 0 1\nvt 0 0\nvt 1 0\nvt 0 1\nf 4/4 5/5 6/6\n";

    #[test]
    fn two_objects_rebase_positive_indices_and_keep_uv_rgb() {
        let parts = Mesh::import_obj_parts(TWO).unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].mesh.positions.len(), 3);
        assert_eq!(parts[1].mesh.positions.len(), 3);
        assert_eq!(parts[0].mesh.triangles, vec![[0, 1, 2]]);
        assert_eq!(parts[1].mesh.triangles, vec![[0, 1, 2]]);
        assert_eq!(parts[1].mesh.positions[0], Vec3::new(4.0, 0.0, 0.0));
        assert_eq!(parts[1].mesh.colors[0], [0.0, 0.0, 1.0]);
        assert_eq!(parts[1].mesh.uv_coordinate_count(), 3);
    }

    #[test]
    fn cross_object_reference_and_invalid_later_object_reject() {
        let crossed = std::str::from_utf8(TWO)
            .unwrap()
            .replace("f 4/4 5/5 6/6", "f 1/4 5/5 6/6");
        assert!(Mesh::import_obj_parts(crossed.as_bytes()).is_err());
        let invalid = std::str::from_utf8(TWO)
            .unwrap()
            .replace("f 4/4 5/5 6/6", "f 4/4 5/5 5/6");
        assert!(Mesh::import_obj_parts(invalid.as_bytes()).is_err());
    }

    #[test]
    fn grouped_source_add_uses_one_world_scale_and_one_history_entry() {
        let parts = Mesh::import_obj_parts(TWO).unwrap();
        let prepared = crate::prepare_source_mesh_group(
            parts
                .into_iter()
                .map(|part| (part.mesh, part.report.imported_colors))
                .collect(),
            crate::SourceImportOptions {
                scale_center: true,
                vertex_srgb: true,
            },
        )
        .unwrap();
        assert_eq!(prepared.report.objects, 2);
        // Source bounds pad the zero-depth triangle by 20% of its own diagonal
        // on either side of Z before the group union.
        let padded_depth = 0.4 * 2.0_f64.sqrt();
        let expected_scale = 100.0 / (26.0 + padded_depth * padded_depth).sqrt();
        let actual_scale = prepared.parts[0].metadata.affine.local_to_world[0];
        assert!(
            (f64::from(actual_scale) - expected_scale).abs() < 1e-5,
            "actual scale {actual_scale}, expected {expected_scale}, bound {:?}",
            prepared
                .report
                .normalization
                .as_ref()
                .unwrap()
                .incoming_world_bound
        );
        assert_eq!(
            prepared.parts[0].metadata.affine.local_to_world,
            prepared.parts[1].metadata.affine.local_to_world
        );
        let mut document = crate::Document::empty();
        let incoming = prepared
            .parts
            .into_iter()
            .enumerate()
            .map(|(index, part)| crate::ImportedSubtool {
                mesh: part.mesh,
                name: format!("Part {index}"),
                preserve_normals: false,
                metadata: part.metadata,
            })
            .collect();
        let ids = document.add_subtools_with_metadata(incoming).unwrap();
        assert_eq!(ids.len(), 2);
        assert_eq!(document.subtool_count(), 2);
        assert_eq!(document.active_subtool_id(), Some(ids[1]));
        assert_eq!(document.subtool_mesh(ids[0]).unwrap().positions.len(), 3);
        assert_eq!(document.subtool_mesh(ids[1]).unwrap().positions[0].x, 4.0);
        assert!(document.undo());
        assert_eq!(document.subtool_count(), 0);
        assert!(!document.undo());
        assert!(document.redo());
        assert_eq!(document.subtool_count(), 2);
    }
}
