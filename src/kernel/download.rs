use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use serde::Deserialize;

use crate::paths;

const REPO: &str = "MetaCubeX/mihomo";

fn api_client() -> Result<reqwest::Client, Box<dyn std::error::Error>> {
    let mut builder = reqwest::Client::builder().user_agent("bnvr");
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {token}").parse()?,
        );
        builder = builder.default_headers(headers);
    }
    Ok(builder.build()?)
}

pub fn detect_platform() -> (&'static str, &'static str) {
    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        panic!("unsupported OS: only Windows and Linux are supported")
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "amd64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        panic!("unsupported architecture: only x86_64 and aarch64 are supported")
    };

    (os, arch)
}

pub fn asset_name(version: &str) -> String {
    let (os, arch) = detect_platform();
    let ext = if os == "windows" { "zip" } else { "gz" };
    format!("mihomo-{os}-{arch}-{version}.{ext}")
}

pub fn download_url(version: &str) -> String {
    format!(
        "https://github.com/{REPO}/releases/download/{version}/{}",
        asset_name(version)
    )
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
}

pub async fn latest_version() -> Result<String, Box<dyn std::error::Error>> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let release: Release = api_client()?
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(release.tag_name)
}

pub async fn download_and_extract(version: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if version != "latest" {
        paths::validate_component(version, "kernel version")?;
    }
    let resolved = if version == "latest" {
        latest_version().await?
    } else {
        version.to_string()
    };
    paths::validate_component(&resolved, "kernel version")?;

    let binary_path = paths::kernel_binary_path(&resolved);
    if binary_path.exists() {
        println!("kernel {} already installed", resolved);
        return Ok(binary_path);
    }

    let kernels_dir = paths::kernels_dir();
    fs::create_dir_all(&kernels_dir)?;
    let staging_dir = kernels_dir.join(format!(".install-{}-{resolved}", std::process::id()));
    remove_dir_if_exists(&staging_dir)?;
    fs::create_dir_all(&staging_dir)?;

    match download_and_stage(&resolved, &staging_dir).await {
        Ok(()) => {
            let install_result = finalize_staged_install(&resolved, &staging_dir);
            if install_result.is_err() {
                remove_dir_if_exists(&staging_dir)?;
            }
            install_result?;
            Ok(binary_path)
        }
        Err(e) => {
            remove_dir_if_exists(&staging_dir)?;
            Err(e)
        }
    }
}

async fn download_and_stage(
    resolved: &str,
    staging_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = download_url(resolved);
    println!("downloading {} ...", url);

    let resp = api_client()?.get(&url).send().await?;

    let status = resp.status();
    if !status.is_success() {
        return Err(format!("download failed: HTTP {status}").into());
    }

    let bytes = resp.bytes().await?;
    let (os, _) = detect_platform();
    if os == "windows" {
        extract_zip(&bytes, staging_dir)?;
    } else {
        extract_gz(&bytes, &kernel_binary_in_dir(staging_dir))?;
    }
    Ok(())
}

fn finalize_staged_install(
    resolved: &str,
    staging_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let staged_binary = kernel_binary_in_dir(staging_dir);
    if !staged_binary.exists() {
        return Err("binary not found after extraction".into());
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&staged_binary, fs::Permissions::from_mode(0o755))?;
    }

    let version_dir = paths::kernel_version_dir(resolved);
    fs::rename(staging_dir, &version_dir)?;
    println!("installed kernel {} to {}", resolved, version_dir.display());
    Ok(())
}

fn kernel_binary_in_dir(dir: &Path) -> PathBuf {
    if cfg!(target_os = "windows") {
        dir.join("mihomo.exe")
    } else {
        dir.join("mihomo")
    }
}

fn remove_dir_if_exists(dir: &Path) -> std::io::Result<()> {
    if dir.exists() {
        fs::remove_dir_all(dir)?;
    }
    Ok(())
}

fn extract_gz(data: &[u8], dest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut decoder = GzDecoder::new(data);
    let mut file = fs::File::create(dest)?;
    io::copy(&mut decoder, &mut file)?;
    Ok(())
}

fn extract_zip(data: &[u8], dest_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let reader = std::io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(reader)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();
        if name.ends_with(".exe")
            || (!name.contains('/') && !name.contains('.') && name.contains("mihomo"))
        {
            let dest = if name.ends_with(".exe") {
                dest_dir.join("mihomo.exe")
            } else {
                dest_dir.join("mihomo")
            };
            let mut out = fs::File::create(&dest)?;
            io::copy(&mut file, &mut out)?;
            return Ok(());
        }
    }

    Err("no mihomo binary found in zip archive".into())
}
