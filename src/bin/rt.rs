#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    app_core::run_raytracing()
}

#[cfg(any(target_arch = "wasm32", target_os = "android"))]
fn main() {
    panic!("The hardware ray tracing demo is desktop-only.");
}
