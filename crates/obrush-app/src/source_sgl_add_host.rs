//! Source File Add appends SGL or a mesh without replacing the document.
//! The request captures import options, native word order and document revision.
use super::*;
impl OBrushApp {
    pub fn cancel_source_sgl_file_request(&mut self, id: u32) {
        self.source_sgl_import.cancel(id);
    }
    /// Continue one picker selection with a fresh revision-bound request after
    /// the preceding file has committed. Each file remains an independent
    /// Source Add/Undo step, matching the reference's per-file read loop.
    pub fn begin_next_source_sgl_file_request(
        &mut self,
        native: obrush_core::SglByteOrder,
    ) -> Result<u32, String> {
        if self.source_sgl_import.pending.is_some() {
            return Err("The preceding Source Add file is still pending".into());
        }
        Ok(self
            .source_sgl_import
            .begin_captured(self.revision, native)?
            .id)
    }
    pub(super) fn pick_source_sgl(&mut self) {
        self.modal_cancel("Source Add file command");
        self.cancel_startup_model();
        self.finish_stroke();
        if self.stroke_rejected {
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        let native = crate::source_sgl_import::native_byte_order();
        #[cfg(target_arch = "wasm32")]
        let native = if source_sgl_native_little() {
            obrush_core::SglByteOrder::LittleEndian
        } else {
            obrush_core::SglByteOrder::BigEndian
        };
        let request = match self.source_sgl_import.begin_captured(self.revision, native) {
            Ok(r) => r,
            Err(error) => {
                self.message = error;
                return;
            }
        };
        #[cfg(target_arch = "wasm32")]
        {
            pick_source_sgl_file(request.id);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let Some(paths) = rfd::FileDialog::new()
                .add_filter("SculptGL files", &["sgl", "obj", "ply", "stl"])
                .pick_files()
            else {
                self.source_sgl_import.cancel(request.id);
                return;
            };
            if paths.is_empty() {
                self.source_sgl_import.cancel(request.id);
                return;
            }
            self.add_source_paths(&paths, Some(request.id));
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn add_dropped_source_files(&mut self, files: Vec<egui::DroppedFile>) {
        let paths: Vec<_> = files
            .into_iter()
            .filter_map(|file| file.path)
            .filter(|path| {
                path.extension().is_some_and(|extension| {
                    matches!(
                        extension.to_string_lossy().to_ascii_lowercase().as_str(),
                        "sgl" | "obj" | "ply" | "stl"
                    )
                })
            })
            .collect();
        if paths.is_empty() {
            return;
        }
        self.modal_cancel("Source dropped-file command");
        self.cancel_startup_model();
        self.finish_stroke();
        if !self.stroke_rejected {
            self.add_source_paths(&paths, None);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn add_source_paths(&mut self, paths: &[std::path::PathBuf], first_request: Option<u32>) {
        let native = crate::source_sgl_import::native_byte_order();
        let mut id = first_request;
        let mut failures = Vec::new();
        for (index, path) in paths.iter().enumerate() {
            let result = (|| {
                if id.is_none() || index > 0 {
                    id = Some(self.begin_next_source_sgl_file_request(native)?);
                }
                let request_id = id.ok_or("Source Add request was not established")?;
                if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > 128 * 1024 * 1024 {
                    return Err("Source Add file exceeds the 128 MiB import limit".into());
                }
                let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
                self.load_source_sgl_for_request(
                    request_id,
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .as_ref(),
                    &bytes,
                    native,
                )
            })();
            if let Err(error) = result {
                if let Some(request_id) = id {
                    self.source_sgl_import.cancel(request_id);
                }
                failures.push(format!("file {}: {error}", index + 1));
            }
        }
        if !failures.is_empty() {
            self.message = format!("Source Add rejected {}", failures.join("; "));
        }
    }
    pub fn load_source_sgl_for_request(
        &mut self,
        id: u32,
        name: &str,
        bytes: &[u8],
        native: obrush_core::SglByteOrder,
    ) -> Result<(), String> {
        let request = self.source_sgl_import.accept_request(id, self.revision)?;
        if name.rsplit_once('.').is_some_and(|(_, extension)| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "obj" | "ply" | "stl"
            )
        }) {
            return self.load_source_mesh_add(request, name, bytes);
        }
        let mut prepared = crate::source_sgl_import::prepare_request(
            request,
            name,
            bytes,
            native,
            &self.viewport_style,
        )?;
        self.finish_stroke();
        if self.stroke_rejected {
            return Err(
                "The preceding shape edit was rejected; retry on the restored scene".into(),
            );
        }
        // If finishing the earlier gesture changed the receiving revision, reject
        // rather than installing into a different state than the captured request.
        if self.revision != request.revision {
            return Err("The document changed before SGL Add completed".into());
        }
        let options_report = prepared.options_report.take();
        self.modal_cancel("SGL installation");
        let (report, order) = crate::source_sgl_import::install(&mut self.document, prepared)?;
        // Modern globals apply even when no new mesh was returned. The global
        // display8 adapter is a separately declared integration dependency.
        self.source_sgl_import.apply_header_and_display(
            &report.input_header,
            &mut self.camera,
            &mut self.viewport_style,
        );
        self.source_sgl_import.last_order = Some(order);
        self.source_sgl_import.options_report = options_report;
        let changed = report.document_changed;
        let ids = report.added_ids.clone();
        self.source_sgl_import.report = Some(report);
        if !changed {
            self.message="SGL contains no new parts. Modern view settings applied; document and history kept.".into();
            self.egui_ctx.request_repaint();
            return Ok(());
        }
        self.cancel_topology();
        self.last_extract_report = None;
        self.last_mask_filter_report = None;
        self.last_mask_region_report = None;
        // Source frames new meshes only. This uses OBrush's existing fitting math;
        // reference camera producers, timed resets and animations remain separate.
        let snapshots: Vec<_> = self
            .document
            .subtool_render_snapshots()
            .into_iter()
            .filter(|o| ids.contains(&o.id))
            .collect();
        self.camera.frame_scene(&snapshots);
        self.changed();
        self.message = format!(
            "Added {} SGL parts. Undo restores the previous scene.",
            ids.len()
        );
        self.egui_ctx.request_repaint();
        Ok(())
    }

    fn load_source_mesh_add(
        &mut self,
        request: crate::source_sgl_import::FileRequest,
        name: &str,
        bytes: &[u8],
    ) -> Result<(), String> {
        if name
            .rsplit_once('.')
            .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("obj"))
        {
            obrush_core::resource_limits::check_document_input_size(name, bytes.len())?;
            let parts = obrush_core::Mesh::import_obj_parts(bytes)?;
            if parts.len() > 1 {
                return self.load_source_obj_parts(request, name, parts);
            }
        }
        // Parse and prepare the whole source mesh before ending a pending edit
        // or touching the receiving document. The public importer enforces its
        // own format-specific allocation and topology limits.
        let prepared = crate::scene_import::prepare(name, bytes, true, &self.viewport_style)?;
        let crate::scene_import::PreparedImport::Append {
            mesh,
            report,
            title,
        } = prepared
        else {
            return Err("Source Add expected a mesh append".into());
        };
        let prepared = obrush_core::prepare_source_mesh_import(
            *mesh,
            report.imported_colors,
            request.options,
        )?;
        crate::renderer::consumer_style_for_mesh(
            Some(&prepared.metadata),
            &prepared.mesh,
            &self.viewport_style,
        )?;
        self.finish_stroke();
        if self.stroke_rejected {
            return Err(
                "The preceding shape edit was rejected; retry on the restored scene".into(),
            );
        }
        if self.revision != request.revision {
            return Err("The document changed before Source Add completed".into());
        }
        self.modal_cancel("Source mesh installation");
        let id = self.document.add_subtool_with_metadata(
            prepared.mesh,
            &title,
            report.imported_normals,
            prepared.metadata,
        )?;
        self.source_sgl_import.report = None;
        self.source_sgl_import.last_order = None;
        self.source_sgl_import.options_report = Some(prepared.report);
        self.last_import_report = Some(report.clone());
        self.viewport_style.use_vertex_color = report.imported_colors;
        self.cancel_topology();
        self.last_extract_report = None;
        self.last_mask_filter_report = None;
        self.last_mask_region_report = None;
        let snapshots: Vec<_> = self
            .document
            .subtool_render_snapshots()
            .into_iter()
            .filter(|object| object.id == id)
            .collect();
        self.camera.frame_scene(&snapshots);
        self.changed();
        self.message = format!("Added {name}. Undo restores the previous scene.");
        self.egui_ctx.request_repaint();
        Ok(())
    }

    fn load_source_obj_parts(
        &mut self,
        request: crate::source_sgl_import::FileRequest,
        name: &str,
        parts: Vec<obrush_core::MeshImport>,
    ) -> Result<(), String> {
        let count = parts.len();
        let title = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
        let mut combined = parts[0].report.clone();
        let mut inputs = Vec::new();
        let mut imported_normals = Vec::new();
        inputs
            .try_reserve_exact(count)
            .map_err(|_| "Cannot allocate bounded Source OBJ parts")?;
        imported_normals
            .try_reserve_exact(count)
            .map_err(|_| "Cannot allocate bounded Source OBJ normal flags")?;
        for (index, part) in parts.into_iter().enumerate() {
            if index > 0 {
                combined.source_vertices = combined
                    .source_vertices
                    .checked_add(part.report.source_vertices)
                    .ok_or("OBJ source vertex count overflow")?;
                combined.source_faces = combined
                    .source_faces
                    .checked_add(part.report.source_faces)
                    .ok_or("OBJ source face count overflow")?;
                combined.triangulated_faces = combined
                    .triangulated_faces
                    .checked_add(part.report.triangulated_faces)
                    .ok_or("OBJ triangle count overflow")?;
                combined.imported_colors |= part.report.imported_colors;
                combined.imported_normals |= part.report.imported_normals;
                combined.warnings.extend(part.report.warnings);
            }
            imported_normals.push(part.report.imported_normals);
            inputs.push((part.mesh, part.report.imported_colors));
        }
        combined.warnings.push(format!(
            "Retained {count} OBJ object records as separate SubTools."
        ));
        let prepared = obrush_core::prepare_source_mesh_group(inputs, request.options)?;
        let mut incoming = Vec::new();
        incoming
            .try_reserve_exact(count)
            .map_err(|_| "Cannot allocate bounded Source OBJ import list")?;
        for (index, part) in prepared.parts.into_iter().enumerate() {
            crate::renderer::consumer_style_for_mesh(
                Some(&part.metadata),
                &part.mesh,
                &self.viewport_style,
            )?;
            incoming.push(obrush_core::ImportedSubtool {
                mesh: part.mesh,
                name: format!("{title} {}", index + 1),
                preserve_normals: imported_normals[index],
                metadata: part.metadata,
            });
        }
        self.finish_stroke();
        if self.stroke_rejected {
            return Err(
                "The preceding shape edit was rejected; retry on the restored scene".into(),
            );
        }
        if self.revision != request.revision {
            return Err("The document changed before Source Add completed".into());
        }
        self.modal_cancel("Source OBJ installation");
        let ids = self.document.add_subtools_with_metadata(incoming)?;
        self.source_sgl_import.report = None;
        self.source_sgl_import.last_order = None;
        self.source_sgl_import.options_report = Some(prepared.report);
        self.last_import_report = Some(combined.clone());
        self.viewport_style.use_vertex_color = combined.imported_colors;
        self.cancel_topology();
        self.last_extract_report = None;
        self.last_mask_filter_report = None;
        self.last_mask_region_report = None;
        let snapshots: Vec<_> = self
            .document
            .subtool_render_snapshots()
            .into_iter()
            .filter(|object| ids.contains(&object.id))
            .collect();
        self.camera.frame_scene(&snapshots);
        self.changed();
        self.message =
            format!("Added {count} OBJ parts from {name}. Undo restores the previous scene.");
        self.egui_ctx.request_repaint();
        Ok(())
    }
}
