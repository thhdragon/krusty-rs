// build.rs - Optional build script
use std::process::Command;

fn main() {
    // This is optional - just for build information
    println!("cargo:rustc-env=BUILD_TIME={}", 
             std::time::SystemTime::now()
                 .duration_since(std::time::UNIX_EPOCH)
                 .unwrap()
                 .as_secs());
}