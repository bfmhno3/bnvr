use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::paths;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProfileKind {
    Remote,
    Merge,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProfileMeta {
    pub kind: ProfileKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<String>,
    pub created_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<u64>,
}

#[derive(Debug)]
pub struct ProfileInfo {
    pub name: String,
    pub meta: ProfileMeta,
    pub active: bool,
    pub has_raw: bool,
    pub has_processed: bool,
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn write_atomic(path: &Path, contents: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
    ));
    if let Err(e) = fs::write(&tmp, contents) {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    Ok(())
}

pub fn add(name: &str, url: &str, user_agent: Option<&str>) -> Result<(), Box<dyn Error>> {
    paths::validate_component(name, "profile name")?;
    let dir = paths::profile_dir(name);
    if dir.exists() {
        return Err(format!("profile already exists: {name}").into());
    }
    fs::create_dir_all(&dir)?;
    let meta = ProfileMeta {
        kind: ProfileKind::Remote,
        url: Some(url.to_string()),
        user_agent: user_agent.map(str::to_string),
        sources: Vec::new(),
        created_at: now_secs(),
        updated_at: None,
    };
    write_meta(name, &meta)?;
    Ok(())
}

pub fn del(name: &str) -> Result<(), Box<dyn Error>> {
    paths::validate_component(name, "profile name")?;
    let dir = paths::profile_dir(name);
    if !dir.exists() {
        return Err(format!("profile not found: {name}").into());
    }
    fs::remove_dir_all(dir)?;
    if get_active().as_deref() == Some(name) {
        let _ = fs::remove_file(paths::active_profile_file());
    }
    Ok(())
}

pub fn list() -> Result<Vec<ProfileInfo>, Box<dyn Error>> {
    let active = get_active();
    let mut result = Vec::new();
    let entries = match fs::read_dir(paths::profiles_dir()) {
        Ok(e) => e,
        Err(_) => return Ok(result),
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
        let meta = match read_meta(&name_str) {
            Ok(m) => m,
            Err(_) => {
                warn!(name = %name_str, "skipping profile with unreadable meta.json");
                continue;
            }
        };
        result.push(ProfileInfo {
            name: name_str.to_string(),
            meta,
            active: active.as_deref() == Some(&name_str),
            has_raw: dir.join("raw.yml").exists(),
            has_processed: dir.join("processed.yml").exists(),
        });
    }

    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}

pub fn get(name: &str) -> Result<ProfileInfo, Box<dyn Error>> {
    paths::validate_component(name, "profile name")?;
    let dir = paths::profile_dir(name);
    if !dir.exists() {
        return Err(format!("profile not found: {name}").into());
    }
    let meta = read_meta(name).map_err(|e| format!("invalid meta.json for profile {name}: {e}"))?;
    Ok(ProfileInfo {
        name: name.to_string(),
        meta,
        active: get_active().as_deref() == Some(name),
        has_raw: paths::profile_raw_file(name).exists(),
        has_processed: paths::profile_processed_file(name).exists(),
    })
}

pub fn read_meta(name: &str) -> Result<ProfileMeta, Box<dyn Error>> {
    let content = fs::read_to_string(paths::profile_meta_file(name))?;
    serde_json::from_str(&content).map_err(Into::into)
}

pub fn write_meta(name: &str, meta: &ProfileMeta) -> Result<(), Box<dyn Error>> {
    write_atomic(
        &paths::profile_meta_file(name),
        &serde_json::to_string_pretty(meta)?,
    )?;
    Ok(())
}

pub fn read_raw(name: &str) -> Result<String, Box<dyn Error>> {
    fs::read_to_string(paths::profile_raw_file(name)).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            format!("no config stored for profile {name} (run `bnvr profile sync` first)").into()
        } else {
            e.into()
        }
    })
}

pub fn read_processed(name: &str) -> Option<String> {
    fs::read_to_string(paths::profile_processed_file(name)).ok()
}

pub fn effective_config(name: &str) -> Result<String, Box<dyn Error>> {
    match read_processed(name) {
        Some(content) => Ok(content),
        None => read_raw(name),
    }
}

pub fn get_active() -> Option<String> {
    fs::read_to_string(paths::active_profile_file())
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn set_active(name: &str) -> Result<(), Box<dyn Error>> {
    paths::validate_component(name, "profile name")?;
    if !paths::profile_dir(name).exists() {
        return Err(format!("profile not found: {name}").into());
    }
    fs::write(paths::active_profile_file(), name)?;
    Ok(())
}

pub fn activate(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    set_active(name)?;
    let path = paths::mihomo_config_file();
    write_atomic(&path, &effective_config(name)?)?;
    info!(name = %name, path = %path.display(), "active profile set");
    Ok(path)
}

pub fn resolve(name: Option<&str>) -> Result<String, Box<dyn Error>> {
    if let Some(name) = name {
        return Ok(name.to_string());
    }
    get_active().ok_or_else(|| "no active profile (run `bnvr profile use <name>`)".into())
}

pub fn refresh_active_config(name: &str) -> Result<(), Box<dyn Error>> {
    if get_active().as_deref() == Some(name) {
        write_atomic(&paths::mihomo_config_file(), &effective_config(name)?)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths;
    use crate::test_env;
    use std::fs;
    use std::path::PathBuf;

    fn setup(test_name: &str) -> (PathBuf, std::sync::MutexGuard<'static, ()>) {
        test_env::setup_profile(&format!("crud-{test_name}"))
    }

    fn cleanup(tmp: &PathBuf) {
        test_env::cleanup(tmp);
    }

    #[test]
    fn test_add_get_roundtrip() {
        let (tmp, _guard) = setup("add-get");
        add("alpha", "http://example.com/a.yml", Some("ua/1")).unwrap();
        let info = get("alpha").unwrap();
        assert_eq!(info.name, "alpha");
        assert_eq!(info.meta.kind, ProfileKind::Remote);
        assert_eq!(info.meta.url.as_deref(), Some("http://example.com/a.yml"));
        assert_eq!(info.meta.user_agent.as_deref(), Some("ua/1"));
        assert!(!info.has_raw);
        cleanup(&tmp);
    }

    #[test]
    fn test_duplicate_add_rejected() {
        let (tmp, _guard) = setup("duplicate-add");
        add("alpha", "http://example.com/a.yml", None).unwrap();
        let err = add("alpha", "http://example.com/b.yml", None).unwrap_err();
        assert_eq!(err.to_string(), "profile already exists: alpha");
        cleanup(&tmp);
    }

    #[test]
    fn test_del_clears_active() {
        let (tmp, _guard) = setup("del-clears-active");
        add("alpha", "http://example.com/a.yml", None).unwrap();
        write_atomic(&paths::profile_raw_file("alpha"), "proxies: []\n").unwrap();
        activate("alpha").unwrap();
        del("alpha").unwrap();
        assert!(!paths::active_profile_file().exists());
        cleanup(&tmp);
    }

    #[test]
    fn test_list_sorts_and_skips_dotfiles() {
        let (tmp, _guard) = setup("list-sorts");
        add("beta", "http://example.com/b.yml", None).unwrap();
        add("alpha", "http://example.com/a.yml", None).unwrap();
        fs::create_dir_all(paths::profiles_dir().join(".hidden")).unwrap();
        let profiles = list().unwrap();
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].name, "alpha");
        assert_eq!(profiles[1].name, "beta");
        cleanup(&tmp);
    }

    #[test]
    fn test_effective_config_prefers_processed() {
        let (tmp, _guard) = setup("effective-config");
        add("alpha", "http://example.com/a.yml", None).unwrap();
        write_atomic(&paths::profile_raw_file("alpha"), "proxies: []\n").unwrap();
        write_atomic(
            &paths::profile_processed_file("alpha"),
            "proxies: [processed]\n",
        )
        .unwrap();
        assert_eq!(effective_config("alpha").unwrap(), "proxies: [processed]\n");
        cleanup(&tmp);
    }

    #[test]
    fn test_resolve_errors_with_no_active_profile() {
        let (tmp, _guard) = setup("resolve-no-active");
        let err = resolve(None).unwrap_err();
        assert_eq!(
            err.to_string(),
            "no active profile (run `bnvr profile use <name>`)"
        );
        cleanup(&tmp);
    }

    #[test]
    fn test_activate_writes_config_yaml() {
        let (tmp, _guard) = setup("activate");
        add("alpha", "http://example.com/a.yml", None).unwrap();
        write_atomic(&paths::profile_raw_file("alpha"), "proxies: []\n").unwrap();
        let path = activate("alpha").unwrap();
        assert_eq!(path, paths::mihomo_config_file());
        assert_eq!(fs::read_to_string(path).unwrap(), "proxies: []\n");
        cleanup(&tmp);
    }
}
