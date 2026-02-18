use std::path::Path;
use std::process::Command;

fn main() {
    let target_dir = Path::new("frontend");

    let output = Command::new("bun")
        .args(["run", "build"])
        .current_dir(target_dir)
        .output()
        .expect("failed to execute process");

    if !output.status.success() {
        panic!(
            "Bun build failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    println!("status: {}", output.status);
}
