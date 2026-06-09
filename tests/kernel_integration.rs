use bnvr::kernel::manage;
use bnvr::paths;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

// Serialize tests that mutate BNVR_HOME since env vars are process-global.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn setup(test_name: &str) -> (PathBuf, std::sync::MutexGuard<'static, ()>) {
    let guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = std::env::temp_dir().join(format!("bnvr-test-{test_name}"));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(tmp.join("kernels")).unwrap();
    // SAFETY: serialized by ENV_LOCK, no concurrent env access
    unsafe { std::env::set_var("BNVR_HOME", &tmp) };
    (tmp, guard)
}

fn cleanup(tmp: &PathBuf) {
    let _ = fs::remove_dir_all(tmp);
}

#[test]
fn test_list_installed_empty() {
    let (tmp, _guard) = setup("list-empty");
    let list = manage::list_installed();
    assert!(list.is_empty());
    cleanup(&tmp);
}

#[test]
fn test_set_active_and_read() {
    let (tmp, _guard) = setup("set-active");
    let version = "v0.0.0-test-set-active";

    let dir = paths::kernel_version_dir(version);
    fs::create_dir_all(&dir).unwrap();

    manage::set_active(version).unwrap();

    let active = manage::read_active();
    assert_eq!(active.as_deref(), Some(version));

    cleanup(&tmp);
}

#[test]
fn test_set_active_rejects_missing_version() {
    let (tmp, _guard) = setup("reject-missing");
    let result = manage::set_active("v99.99.99-nonexistent");
    assert!(result.is_err());
    cleanup(&tmp);
}

#[test]
fn test_kernel_status_no_active() {
    let (tmp, _guard) = setup("status-no-active");

    let s = manage::kernel_status();
    assert!(s.active_version.is_none());
    assert!(!s.binary_exists);
    assert!(s.binary_path.is_none());

    cleanup(&tmp);
}

#[test]
fn test_kernel_status_with_active() {
    let (tmp, _guard) = setup("status-with-active");
    let version = "v0.0.0-test-status";

    let dir = paths::kernel_version_dir(version);
    fs::create_dir_all(&dir).unwrap();
    let binary = paths::kernel_binary_path(version);
    fs::write(&binary, b"fake").unwrap();

    manage::set_active(version).unwrap();

    let s = manage::kernel_status();
    assert_eq!(s.active_version.as_deref(), Some(version));
    assert!(s.binary_exists);
    assert!(s.binary_path.is_some());

    cleanup(&tmp);
}

#[test]
fn test_list_installed_shows_version() {
    let (tmp, _guard) = setup("list-shows");
    let version = "v0.0.0-test-list";

    let dir = paths::kernel_version_dir(version);
    fs::create_dir_all(&dir).unwrap();
    let binary = paths::kernel_binary_path(version);
    fs::write(&binary, b"fake").unwrap();

    manage::set_active(version).unwrap();

    let list = manage::list_installed();
    let found = list.iter().find(|k| k.version == version);
    assert!(found.is_some());
    let k = found.unwrap();
    assert!(k.active);
    assert!(k.binary_exists);

    cleanup(&tmp);
}
