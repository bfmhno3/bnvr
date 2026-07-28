/// Check if a process with the given PID is alive.
pub fn is_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // SAFETY: kill with signal 0 checks existence without sending a signal
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        };
        // SAFETY: We only query process info, then immediately close the handle
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle.is_null() {
                return false;
            }
            let _ = windows_sys::Win32::Foundation::CloseHandle(handle);
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_process_is_alive() {
        let pid = std::process::id();
        assert!(is_alive(pid));
    }

    #[test]
    fn test_invalid_pid_is_not_alive() {
        // PID 0 is the idle process on Windows, so use a very high unlikely PID
        assert!(!is_alive(u32::MAX - 1));
    }
}
