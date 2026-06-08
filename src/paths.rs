use std::path::PathBuf;

pub fn bnvr_home() -> PathBuf {
    let home = dirs::home_dir().expect("Cannot determine home directory");
    home.join(".bnvr")
}

pub fn pid_file() -> PathBuf {
    bnvr_home().join("bnvr.pid")
}

pub fn db_file() -> PathBuf {
    bnvr_home().join("bnvr.db")
}

pub fn log_dir() -> PathBuf {
    bnvr_home().join("logs")
}

pub fn kernels_dir() -> PathBuf {
    bnvr_home().join("kernels")
}

pub fn active_kernel_file() -> PathBuf {
    kernels_dir().join(".active")
}

pub fn kernel_version_dir(version: &str) -> PathBuf {
    kernels_dir().join(version)
}

pub fn kernel_binary_path(version: &str) -> PathBuf {
    let dir = kernel_version_dir(version);
    if cfg!(target_os = "windows") {
        dir.join("mihomo.exe")
    } else {
        dir.join("mihomo")
    }
}

pub fn ensure_dirs() -> std::io::Result<()> {
    let home = bnvr_home();
    if !home.exists() {
        std::fs::create_dir_all(&home)?;
    }
    let logs = log_dir();
    if !logs.exists() {
        std::fs::create_dir_all(&logs)?;
    }
    let kernels = kernels_dir();
    if !kernels.exists() {
        std::fs::create_dir_all(&kernels)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bnvr_home_ends_with_dot_bnvr() {
        let home = bnvr_home();
        assert!(home.ends_with(".bnvr"));
    }

    #[test]
    fn test_pid_file_name() {
        let path = pid_file();
        assert_eq!(path.file_name().unwrap(), "bnvr.pid");
        assert!(path.parent().unwrap().ends_with(".bnvr"));
    }

    #[test]
    fn test_db_file_name() {
        let path = db_file();
        assert_eq!(path.file_name().unwrap(), "bnvr.db");
    }

    #[test]
    fn test_log_dir_name() {
        let path = log_dir();
        assert_eq!(path.file_name().unwrap(), "logs");
        assert!(path.parent().unwrap().ends_with(".bnvr"));
    }

    #[test]
    fn test_ensure_dirs_creates_directories() {
        ensure_dirs().unwrap();
        assert!(bnvr_home().exists());
        assert!(log_dir().exists());
    }

    #[test]
    fn test_ensure_dirs_is_idempotent() {
        ensure_dirs().unwrap();
        ensure_dirs().unwrap(); // second call should not fail
    }

    #[test]
    fn test_kernels_dir_name() {
        let path = kernels_dir();
        assert_eq!(path.file_name().unwrap(), "kernels");
        assert!(path.parent().unwrap().ends_with(".bnvr"));
    }

    #[test]
    fn test_active_kernel_file_name() {
        let path = active_kernel_file();
        assert_eq!(path.file_name().unwrap(), ".active");
        assert!(path.parent().unwrap().ends_with("kernels"));
    }

    #[test]
    fn test_kernel_version_dir() {
        let path = kernel_version_dir("v1.19.27");
        assert_eq!(path.file_name().unwrap(), "v1.19.27");
        assert!(path.parent().unwrap().ends_with("kernels"));
    }

    #[test]
    fn test_kernel_binary_path() {
        let path = kernel_binary_path("v1.19.27");
        if cfg!(target_os = "windows") {
            assert_eq!(path.file_name().unwrap(), "mihomo.exe");
        } else {
            assert_eq!(path.file_name().unwrap(), "mihomo");
        }
    }
}
