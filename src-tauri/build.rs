fn main() {
    // Declare custom cfg keys so rustc's check-cfg lint doesn't fire.
    println!("cargo::rustc-check-cfg=cfg(voca_metal_available)");
    println!("cargo::rustc-check-cfg=cfg(voca_cuda_available)");

    detect_gpu();
    tauri_build::build();
}

/// Probe the build host for GPU toolkits and emit `cargo:rustc-cfg` flags that
/// the source code uses to gate GPU-specific behaviour at compile time.
///
/// Metal   — Apple Silicon macOS.  No external toolkit needed; presence is
///           inferred from target triple (`aarch64-apple-darwin`).
///
/// CUDA    — Windows / Linux.  We check for an installed CUDA toolkit via the
///           `CUDA_PATH` (Windows installer default) or `CUDA_HOME` (Linux)
///           environment variables, or by finding `nvcc` on `PATH`.
///           Emits `voca_cuda_available` when detected.
fn detect_gpu() {
    // Metal — always available on Apple Silicon; nothing extra to probe.
    let target_os   = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    if target_os == "macos" && target_arch == "aarch64" {
        println!("cargo:rustc-cfg=voca_metal_available");
        println!("cargo:warning=VOCA: Apple Silicon target — Metal GPU path active");
    }

    // CUDA — Windows / Linux only.
    if target_os != "macos" {
        let toolkit_env = std::env::var("CUDA_PATH")
            .or_else(|_| std::env::var("CUDA_HOME"))
            .is_ok();

        let nvcc_in_path = std::process::Command::new("nvcc")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if toolkit_env || nvcc_in_path {
            println!("cargo:rustc-cfg=voca_cuda_available");
            println!("cargo:warning=VOCA: CUDA toolkit detected — rebuild with `--features cuda` to enable GPU acceleration");
        }
    }
}

