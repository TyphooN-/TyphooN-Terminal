//! TyphooN Terminal — Native GPU Renderer
//!
//! Pure Rust → wgpu pipeline. Zero WebKit, zero JS, zero JSON serialization.
//! Direct memory access from SQLite cache to GPU vertex buffers.

mod app;

fn main() -> eframe::Result {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("typhoon=info,wgpu=warn,eframe=warn")
        .init();

    tracing::info!("TyphooN Terminal (native GPU) starting...");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("TyphooN Terminal")
            .with_inner_size([1920.0, 1080.0])
            .with_min_inner_size([800.0, 600.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "TyphooN Terminal",
        options,
        Box::new(|cc| Ok(Box::new(app::TyphooNApp::new(cc)))),
    )
}
