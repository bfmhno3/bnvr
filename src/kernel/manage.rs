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
    pub pid: Option<u32>,
}

pub fn read_active() -> Option<String> {
    let path = paths::active_kernel_file();
    fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn set_active(version: &str) -> std::io::Result<()> {
    paths::validate_component(version, "kernel version")
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?;
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

/// Find the PID of a running mihomo process by scanning for its binary name.
pub fn running_pid() -> Option<u32> {
    #[cfg(windows)]
    {
        // Use Windows toolhelp32 to enumerate processes
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, PROCESSENTRY32, Process32First, Process32Next,
        };
        const TH32CS_SNAPPROCESS: u32 = 0x00000002;

        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snapshot.is_null() {
                return None;
            }

            let mut entry: PROCESSENTRY32 = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;

            if Process32First(snapshot, &mut entry) != 0 {
                loop {
                    let name = std::ffi::CStr::from_ptr(
                        entry.szExeFile.as_ptr() as *const std::ffi::c_char
                    );
                    if name.to_string_lossy().contains("mihomo") {
                        let _ = CloseHandle(snapshot);
                        return Some(entry.th32ProcessID);
                    }
                    if Process32Next(snapshot, &mut entry) == 0 {
                        break;
                    }
                }
            }

            let _ = CloseHandle(snapshot);
            None
        }
    }

    #[cfg(unix)]
    {
        // Scan /proc for mihomo process
        let proc_dir = std::path::Path::new("/proc");
        let entries = fs::read_dir(proc_dir).ok()?;

        for entry in entries.flatten() {
            let name = entry.file_name();
            let pid_str = name.to_string_lossy();
            let pid: u32 = match pid_str.parse() {
                Ok(p) => p,
                Err(_) => continue,
            };

            let exe_link = entry.path().join("exe");
            if let Ok(exe_path) = fs::read_link(&exe_link) {
                if exe_path
                    .file_name()
                    .map(|n| n.to_string_lossy().contains("mihomo"))
                    .unwrap_or(false)
                {
                    return Some(pid);
                }
            }
        }
        None
    }
}

pub fn kernel_status() -> KernelStatus {
    let active = read_active();
    let pid = running_pid();
    match active {
        Some(ref version) => {
            let path = paths::kernel_binary_path(version);
            KernelStatus {
                active_version: Some(version.clone()),
                binary_exists: path.exists(),
                binary_path: Some(path),
                pid,
            }
        }
        None => KernelStatus {
            active_version: None,
            binary_path: None,
            binary_exists: false,
            pid,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_running_pid_returns_option() {
        // running_pid() should return Some if mihomo is running, None otherwise
        let pid = running_pid();
        // We can't assert a specific value since it depends on system state
        // but we can verify it doesn't panic
        if let Some(p) = pid {
            assert!(p > 0);
        }
    }

    #[test]
    fn test_kernel_status_includes_pid() {
        let status = kernel_status();
        // pid field should be populated (either Some or None)
        let _ = status.pid;
    }

    #[test]
    fn test_set_active_rejects_invalid_version_without_writing_active() {
        let temp = std::env::temp_dir().join("bnvr_test_kernel_invalid_active");
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(temp.join("kernels")).unwrap();
        let old_home = std::env::var("BNVR_HOME").ok();
        unsafe { std::env::set_var("BNVR_HOME", &temp) };

        let result = set_active("../escape");
        assert!(result.is_err());
        assert!(!paths::active_kernel_file().exists());

        match old_home {
            Some(value) => unsafe { std::env::set_var("BNVR_HOME", value) },
            None => unsafe { std::env::remove_var("BNVR_HOME") },
        }
        let _ = std::fs::remove_dir_all(&temp);
    }
}
