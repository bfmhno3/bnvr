use std::fs;
use std::path::PathBuf;

use crate::paths;

pub struct KernelInfo {
    pub version: String,
    pub active: bool,
    pub binary_exists: bool,
}

pub struct KernelStatus {
    pub active_version: Option<String>,
    pub binary_path: Option<PathBuf>,
    pub binary_exists: bool,
}

pub fn read_active() -> Option<String> {
    let path = paths::active_kernel_file();
    fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn set_active(version: &str) -> std::io::Result<()> {
    let dir = paths::kernel_version_dir(version);
    if !dir.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("kernel version {version} not installed"),
        ));
    }
    fs::write(paths::active_kernel_file(), version)
}

pub fn list_installed() -> Vec<KernelInfo> {
    let kernels_dir = paths::kernels_dir();
    let active = read_active();
    let mut result = Vec::new();

    let entries = match fs::read_dir(&kernels_dir) {
        Ok(e) => e,
        Err(_) => return result,
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') {
            continue;
        }
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let binary = paths::kernel_binary_path(&name_str);
        result.push(KernelInfo {
            version: name_str.to_string(),
            active: active.as_deref() == Some(&name_str),
            binary_exists: binary.exists(),
        });
    }

    result
}

pub fn kernel_status() -> KernelStatus {
    let active = read_active();
    match active {
        Some(ref version) => {
            let path = paths::kernel_binary_path(version);
            KernelStatus {
                active_version: Some(version.clone()),
                binary_exists: path.exists(),
                binary_path: Some(path),
            }
        }
        None => KernelStatus {
            active_version: None,
            binary_path: None,
            binary_exists: false,
        },
    }
}
