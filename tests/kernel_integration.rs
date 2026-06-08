use bnvr::kernel::manage;
use bnvr::paths;
use std::fs;

// These tests share ~/.bnvr/kernels/.active and must run serially.
// Run with: cargo test --test kernel_integration -- --test-threads=1

fn setup() {
    paths::ensure_dirs().unwrap();
}

fn cleanup_version(version: &str) {
    let dir = paths::kernel_version_dir(version);
    let _ = fs::remove_dir_all(dir);
}

fn cleanup_active() {
    let _ = fs::remove_file(paths::active_kernel_file());
}

#[test]
fn test_list_installed_empty() {
    setup();
    // list_installed should not panic even if kernels dir is empty or has no versions
    let list = manage::list_installed();
    // We can't assert it's empty because other tests may have left versions
    // but it should not panic
    let _ = list;
}

#[test]
fn test_set_active_and_read() {
    setup();
    let version = "v0.0.0-test-set-active";

    // Create a fake version dir
    let dir = paths::kernel_version_dir(version);
    fs::create_dir_all(&dir).unwrap();

    // Set active
    manage::set_active(version).unwrap();

    // Read back
    let active = manage::read_active();
    assert_eq!(active.as_deref(), Some(version));

    cleanup_version(version);
    cleanup_active();
}

#[test]
fn test_set_active_rejects_missing_version() {
    setup();
    let result = manage::set_active("v99.99.99-nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_kernel_status_no_active() {
    setup();
    cleanup_active();

    let s = manage::kernel_status();
    assert!(s.active_version.is_none());
    assert!(!s.binary_exists);
    assert!(s.binary_path.is_none());
}

#[test]
fn test_kernel_status_with_active() {
    setup();
    let version = "v0.0.0-test-status";

    // Create fake version dir with binary
    let dir = paths::kernel_version_dir(version);
    fs::create_dir_all(&dir).unwrap();
    let binary = paths::kernel_binary_path(version);
    fs::write(&binary, b"fake").unwrap();

    manage::set_active(version).unwrap();

    let s = manage::kernel_status();
    assert_eq!(s.active_version.as_deref(), Some(version));
    assert!(s.binary_exists);
    assert!(s.binary_path.is_some());

    cleanup_version(version);
    cleanup_active();
}

#[test]
fn test_list_installed_shows_version() {
    setup();
    let version = "v0.0.0-test-list";

    // Create fake version dir with binary
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

    cleanup_version(version);
    cleanup_active();
}
