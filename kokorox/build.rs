use std::path::Path;
use std::process::Command;

fn library_exists(name: &str) -> bool {
    let soname = format!("lib{}.so", name);

    if let Ok(output) = Command::new("ldconfig").arg("-p").output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains(&soname) {
                return true;
            }
        }
    }

    let search_paths = [
        "/lib",
        "/lib64",
        "/usr/lib",
        "/usr/lib64",
        "/usr/local/lib",
        "/usr/lib/x86_64-linux-gnu",
    ];

    search_paths
        .iter()
        .map(|dir| Path::new(dir).join(&soname))
        .any(|path| path.exists())
}

fn main() {
    if cfg!(target_os = "linux") {
        if library_exists("sonic") {
            println!("cargo:rustc-link-lib=sonic");
        }
        if library_exists("pcaudio") {
            println!("cargo:rustc-link-lib=pcaudio");
        }
    }
}
