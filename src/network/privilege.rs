use std::error::Error;

#[cfg(target_os = "windows")]
pub fn check_privileges() -> Result<(), Box<dyn Error>> {
    use windows_sys::Win32::UI::Shell::IsUserAnAdmin;

    let is_admin = unsafe { IsUserAnAdmin() } != 0;
    if is_admin {
        Ok(())
    } else {
        Err("TUN device creation requires Administrator privileges. Run Command Prompt as Administrator, then execute 'bnvr daemon start'.".into())
    }
}

#[cfg(target_os = "linux")]
pub fn check_privileges() -> Result<(), Box<dyn Error>> {
    let is_root = unsafe { libc::geteuid() } == 0;
    if is_root
        || caps::has_cap(
            None,
            caps::CapSet::Effective,
            caps::Capability::CAP_NET_ADMIN,
        )?
    {
        Ok(())
    } else {
        Err("TUN device creation requires root or CAP_NET_ADMIN. Run 'sudo bnvr daemon start' or grant capabilities with 'sudo setcap cap_net_admin+eip /path/to/bnvr'.".into())
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn check_privileges() -> Result<(), Box<dyn Error>> {
    Err("TUN device creation is supported only on Windows and Linux".into())
}
