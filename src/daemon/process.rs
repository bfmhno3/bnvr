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

/// Send a graceful shutdown signal to a process.
pub fn send_shutdown_signal(pid: u32) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(unix)]
    {
        // SAFETY: SIGTERM requests graceful termination
        let ret = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
        if ret != 0 {
            return Err(format!("failed to send SIGTERM to pid {pid}").into());
        }
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Threading::{
            OpenProcess, TerminateProcess, PROCESS_TERMINATE,
        };
        // SAFETY: We open with PROCESS_TERMINATE, call TerminateProcess, then close
        unsafe {
            let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
            if handle.is_null() {
                return Err(format!("failed to open process {pid}").into());
            }
            let ret = TerminateProcess(handle, 0);
            let _ = windows_sys::Win32::Foundation::CloseHandle(handle);
            if ret == 0 {
                return Err(format!("failed to terminate process {pid}").into());
            }
        }
    }
    Ok(())
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

    #[test]
    fn test_send_shutdown_signal_invalid_pid() {
        let result = send_shutdown_signal(u32::MAX - 1);
        assert!(result.is_err());
    }
}
