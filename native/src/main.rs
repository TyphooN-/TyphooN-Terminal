#![recursion_limit = "256"]
//! TyphooN Terminal — Native GPU Renderer
//!
//! Pure Rust → egui + wgpu pipeline.
//! Direct memory access from SQLite cache to GPU vertex buffers.
//! Async broker integration via tokio runtime + mpsc channels.

mod app;
mod gpu_compute;

fn dirs_home() -> std::path::PathBuf {
    let mut p = if let Ok(home) = std::env::var("HOME") {
        std::path::PathBuf::from(home)
    } else {
        std::path::PathBuf::from("/tmp")
    };
    p.push(".config");
    p.push("typhoon-terminal");
    p
}

fn main() -> eframe::Result {
    // Initialize logging — suppress noisy wgpu/egl/vulkan adapter probing
    tracing_subscriber::fmt()
        .with_env_filter("typhoon=info,wgpu_hal=error,wgpu_core=error,wgpu=error,eframe=warn,naga=error")
        .init();

    tracing::info!("TyphooN Terminal v0.1.0 — Pure Rust GPU (egui + wgpu)");
    tracing::info!("Platform: {} {}", std::env::consts::OS, std::env::consts::ARCH);
    tracing::info!("Renderer: wgpu (Vulkan/Metal/DX12)");

    // Cache path
    let mut cache_path = dirs_home();
    cache_path.push("cache");
    cache_path.push("typhoon_cache.db");
    if cache_path.exists() {
        if let Ok(meta) = std::fs::metadata(&cache_path) {
            tracing::info!("Cache: {} ({:.1} MB)", cache_path.display(), meta.len() as f64 / 1024.0 / 1024.0);
        }
    } else {
        tracing::warn!("Cache not found: {}", cache_path.display());
    }

    // Session path
    let mut session_path = dirs_home();
    session_path.push("session.json");
    if session_path.exists() {
        tracing::info!("Session: {}", session_path.display());
    }

    // Start tokio runtime in background thread for async broker operations
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");
    let rt_handle = runtime.handle().clone();
    tracing::info!("Tokio runtime: 2 worker threads");

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
