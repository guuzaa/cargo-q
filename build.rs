use std::process::Command;

fn main() {
    let git = |args: &[&str]| -> String {
        Command::new("git")
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|| "unknown".into())
    };

    println!(
        "cargo:rustc-env=GIT_HASH={}",
        git(&["rev-parse", "--short=9", "HEAD"])
    );
    println!(
        "cargo:rustc-env=GIT_COMMIT_DATE={}",
        git(&["log", "-1", "--format=%cd", "--date=short"])
    );

    println!("cargo:rerun-if-changed=.git/HEAD");
}
