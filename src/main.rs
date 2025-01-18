use std::process::Command;

fn main() {
    let output = Command::new("cargo")
        .arg("version")
        .output()
        .expect("Failed to execute cargo --version");

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout);
        println!("Cargo version: {}", version.trim());
    } else {
        eprintln!("Failed to get Cargo version");
    }
}