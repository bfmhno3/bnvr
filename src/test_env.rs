#[cfg(test)]
use std::fs;
#[cfg(test)]
use std::path::PathBuf;
#[cfg(test)]
use std::sync::{Mutex, MutexGuard};

#[cfg(test)]
static ENV_LOCK: Mutex<()> = Mutex::new(());

#[cfg(test)]
pub fn setup_profile(test_name: &str) -> (PathBuf, MutexGuard<'static, ()>) {
    let guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = std::env::temp_dir().join(format!("bnvr-test-profile-{test_name}"));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(tmp.join("profile")).unwrap();
    // SAFETY: serialized by ENV_LOCK, no concurrent env access
    unsafe { std::env::set_var("BNVR_HOME", &tmp) };
    (tmp, guard)
}

#[cfg(test)]
pub fn cleanup(tmp: &PathBuf) {
    let _ = fs::remove_dir_all(tmp);
}
