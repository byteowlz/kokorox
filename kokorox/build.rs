use std::path::{Path, PathBuf};
use std::process::Command;

enum LibLink {
    ByName { dir: PathBuf },
    ByPath { path: PathBuf },
}

fn find_ldconfig_lib(name: &str) -> Option<LibLink> {
    let output = Command::new("ldconfig").arg("-p").output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut versioned: Option<PathBuf> = None;

    for line in stdout.lines() {
        let (left, right) = match line.split_once("=>") {
            Some((left, right)) => (left.trim(), right.trim()),
            None => continue,
        };

        let filename = left.split_whitespace().next().unwrap_or("");
        if filename == format!("lib{}.so", name) {
            let path = PathBuf::from(right);
            if let Some(dir) = path.parent() {
                return Some(LibLink::ByName { dir: dir.to_path_buf() });
            }
        }

        let version_prefix = format!("lib{}.so.", name);
        if filename.starts_with(&version_prefix) {
            let path = PathBuf::from(right);
            if path.is_file() {
                versioned = Some(path);
            }
        }
    }

    versioned.map(|path| LibLink::ByPath { path })
}

fn find_in_dirs(name: &str, dirs: &[&str]) -> Option<LibLink> {
    let so_name = format!("lib{}.so", name);
    let static_name = format!("lib{}.a", name);
    let version_prefix = format!("lib{}.so.", name);

    for dir in dirs {
        let dir_path = Path::new(dir);
        let so_path = dir_path.join(&so_name);
        if so_path.is_file() {
            return Some(LibLink::ByName {
                dir: dir_path.to_path_buf(),
            });
        }
        let static_path = dir_path.join(&static_name);
        if static_path.is_file() {
            return Some(LibLink::ByName {
                dir: dir_path.to_path_buf(),
            });
        }
        if let Ok(entries) = dir_path.read_dir() {
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                if let Some(file_name) = file_name.to_str() {
                    if file_name.starts_with(&version_prefix) {
                        let path = entry.path();
                        if path.is_file() {
                            return Some(LibLink::ByPath { path });
                        }
                    }
                }
            }
        }
    }

    None
}

fn find_library(name: &str) -> Option<LibLink> {
    if let Some(link) = find_ldconfig_lib(name) {
        return Some(link);
    }

    let search_paths = [
        "/lib",
        "/lib64",
        "/usr/lib",
        "/usr/lib64",
        "/usr/local/lib",
        "/usr/lib/x86_64-linux-gnu",
    ];

    find_in_dirs(name, &search_paths)
}

fn link_library(name: &str) {
    match find_library(name) {
        Some(LibLink::ByName { dir }) => {
            println!("cargo:rustc-link-search=native={}", dir.display());
            println!("cargo:rustc-link-lib={}", name);
        }
        Some(LibLink::ByPath { path }) => {
            println!("cargo:rustc-link-arg={}", path.display());
        }
        None => {}
    }
}

fn main() {
    if cfg!(target_os = "linux") {
        link_library("sonic");
        link_library("pcaudio");
    }
}
