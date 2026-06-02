#![recursion_limit = "512"]
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

#[cfg(target_os = "linux")]
fn ac_power_available() -> bool {
    let supplies = match std::fs::read_dir("/sys/class/power_supply") {
        Ok(entries) => entries,
        Err(_) => return true,
    };

    let mut saw_battery = false;
    let mut saw_discharging_battery = false;

    for entry in supplies.flatten() {
        let path = entry.path();
        let supply_type = std::fs::read_to_string(path.join("type"))
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        match supply_type.as_str() {
            "mains" | "usb" | "usb_c" | "usb_pd" => {
                if std::fs::read_to_string(path.join("online"))
                    .map(|s| s.trim() == "1")
                    .unwrap_or(false)
                {
                    return true;
                }
            }
            "battery" => {
                saw_battery = true;
                let status = std::fs::read_to_string(path.join("status"))
                    .unwrap_or_default()
                    .trim()
                    .to_ascii_lowercase();
                if status == "discharging" {
                    saw_discharging_battery = true;
                }
            }
            _ => {}
        }
    }

    // Desktop/VM/unknown power topology: fail open so sync/maintenance is not
    // silently throttled forever. Real laptop on battery reports Discharging.
    !saw_battery || !saw_discharging_battery
}

#[cfg(not(target_os = "linux"))]
fn ac_power_available() -> bool {
    true
}

fn main() -> eframe::Result {
    // Initialize logging — suppress noisy wgpu/egl/vulkan adapter probing
    tracing_subscriber::fmt()
        .with_env_filter(
            "typhoon=info,wgpu_hal=error,wgpu_core=error,wgpu=error,eframe=warn,naga=error",
        )
        .init();

    tracing::info!("TyphooN Terminal v0.1.0 (egui + wgpu)");
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
    // Do not hand almost every logical CPU to background sync. Release/max-LTO
    // runs can keep Tokio busy with HTTP, JSON, SQLite, zstd, SEC/fundamentals,
    // and WS decode work for minutes; if the async pool is 42/44 CPUs, egui/wgpu
    // and the compositor lose scheduling headroom and ordinary pointer motion
    // stutters. Full-tilt must mean continuous bounded pressure, not UI starvation.
    // TYPHOON_TOKIO_WORKERS can override this for profiling.
    let detected_cpus = std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .unwrap_or(4);
    let on_ac_power = ac_power_available();
    let default_workers = if on_ac_power {
        // Cap high-core machines as well as reserving cores: beyond ~24 async
        // workers this app becomes scheduler/cache-contention bound before it
        // becomes provider-throughput bound.
        detected_cpus.saturating_sub(8).clamp(4, 24)
    } else {
        (detected_cpus / 2).clamp(2, 8)
    };
    let worker_threads = std::env::var("TYPHOON_TOKIO_WORKERS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(default_workers);

    // If this fails we cannot continue — the whole app depends on tokio.
    // We print a user-visible error instead of an anonymous panic.
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .thread_name("typhoon-async")
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
    tracing::info!(
        "Tokio runtime: {worker_threads} worker threads ({detected_cpus} logical CPUs detected, power={})",
        if on_ac_power {
            "AC/max"
        } else {
            "battery/balanced"
        }
    );

    // Keep runtime alive for the lifetime of the app
    let _rt_guard = runtime;

    let mut wgpu_options = eframe::egui_wgpu::WgpuConfiguration::default();
    wgpu_options.present_mode = eframe::wgpu::PresentMode::AutoVsync;
    wgpu_options.desired_maximum_frame_latency = Some(1);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("TyphooN Terminal")
            .with_inner_size([1920.0, 1080.0])
            .with_min_inner_size([800.0, 600.0]),
        renderer: eframe::Renderer::Wgpu,
        vsync: true,
        hardware_acceleration: eframe::HardwareAcceleration::Required,
        wgpu_options,
        ..Default::default()
    };

    eframe::run_native(
        "TyphooN Terminal",
        options,
        Box::new(move |cc| Ok(Box::new(app::TyphooNApp::new(cc, rt_handle)))),
    )
}
