//! TyphooN Terminal — Native GPU Renderer
//!
//! Pure Rust → egui + wgpu pipeline.
//! Direct memory access from SQLite cache to GPU vertex buffers.
//! Async broker integration via tokio runtime + mpsc channels.

mod app;

fn main() -> eframe::Result {
    // Initialize logging — suppress noisy wgpu/egl/vulkan adapter probing
    tracing_subscriber::fmt()
        .with_env_filter("typhoon=info,wgpu_hal=error,wgpu_core=error,wgpu=error,eframe=warn,naga=error")
        .init();

    tracing::info!("TyphooN Terminal (native GPU) starting...");

    // Start tokio runtime in background thread for async broker operations
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");
    let rt_handle = runtime.handle().clone();

    // Keep runtime alive for the lifetime of the app
    let _rt_guard = runtime;

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
        Box::new(move |cc| Ok(Box::new(app::TyphooNApp::new(cc, rt_handle)))),
    )
}
