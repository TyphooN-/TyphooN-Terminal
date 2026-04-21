#![recursion_limit = "256"]
//! TyphooN Terminal — Native GPU Renderer
//!
//! Pure Rust → egui + wgpu pipeline.
//! Direct memory access from SQLite cache to GPU vertex buffers.
//! Async broker integration via tokio runtime + mpsc channels.

// PERF: mimalloc is 5-15% faster than the system allocator on small-allocation
// heavy workloads (per-frame Strings, Vecs, HashMaps in the render loop).
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod app;
mod gpu_compute;
mod metrics;

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
        .with_env_filter(
            "typhoon=info,wgpu_hal=error,wgpu_core=error,wgpu=error,eframe=warn,naga=error",
        )
        .init();

    tracing::info!("TyphooN Terminal v0.1.0 — Pure Rust GPU (egui + wgpu)");
    tracing::info!(
        "Platform: {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    tracing::info!("Renderer: wgpu (Vulkan/Metal/DX12)");

    // Resolve custom cache dir (user may have moved it to a NAS / faster drive).
    // The setting is stored in `~/.config/typhoon-terminal/cache_location.txt`
    // so it's readable before session.json is parsed. If set but the target
    // directory no longer exists (unmounted share, removed drive), `cache_dir`
    // falls back to the default — but we log a WARN so the user sees it, and
    // the UI shows a red banner from `is_custom_cache_missing()`.
    let configured_custom = app::read_custom_cache_dir();
    if let Some(ref p) = configured_custom {
        if !p.is_dir() {
            tracing::warn!(
                "Custom cache directory is configured but UNAVAILABLE: {} — falling back to default. \
                 Mount the drive / restart the NAS and restart the terminal to restore.",
                p.display()
            );
        } else {
            tracing::info!("Custom cache directory: {}", p.display());
        }
    }
    app::init_custom_cache_dir(configured_custom);

    let cache_path = app::cache_db_path();
    if cache_path.exists() {
        if let Ok(meta) = std::fs::metadata(&cache_path) {
            tracing::info!(
                "Cache: {} ({:.1} MB)",
                cache_path.display(),
                meta.len() as f64 / 1024.0 / 1024.0
            );
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

    // Start tokio runtime in background thread for async broker operations.
    // If this fails we cannot continue — the whole app depends on tokio.
    // We print a user-visible error instead of an anonymous panic.
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("Fatal: failed to create tokio runtime: {e}");
            eprintln!("This usually means the OS refused to create worker threads.");
            std::process::exit(1);
        }
    };
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
