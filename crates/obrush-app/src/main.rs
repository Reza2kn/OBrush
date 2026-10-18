#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let startup = obrush_app::startup_options::StartupOptions::capture_native(&args)
        .map_err(|error| eframe::Error::AppCreation(Box::new(std::io::Error::other(error))))?;
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("OBrush · Open sculpting studio")
            .with_inner_size([1440.0, 960.0])
            .with_min_inner_size([900.0, 620.0]),
        renderer: eframe::Renderer::Glow,
        depth_buffer: 24,
        ..Default::default()
    };
    eframe::run_native(
        "OBrush",
        options,
        Box::new(move |cc| {
            Ok(Box::new(obrush_app::OBrushApp::new_with_startup(
                cc, startup,
            )?))
        }),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {}
