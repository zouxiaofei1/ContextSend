use std::fs;
use std::path::Path;

fn main() {
    // 每次 build.rs 执行时构建号 +1
    let counter_path = Path::new("build-counter.txt");

    let current: u32 = fs::read_to_string(counter_path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);

    let next = current.wrapping_add(1);

    fs::write(counter_path, next.to_string()).expect("Failed to write build-counter.txt");

    println!("cargo:rustc-env=BUILD_NUMBER={next}");

    tauri_build::build()
}
