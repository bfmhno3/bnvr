use std::fs;
use std::io::{self, Read as _, Write as _};
use std::path::PathBuf;

use flate2::read::GzDecoder;
use serde::Deserialize;

use crate::paths;

const REPO: &str = "MetaCubeX/mihomo";

fn api_client() -> Result<reqwest::Client, Box<dyn std::error::Error>> {
    let mut builder = reqwest::Client::builder().user_agent("bnvr");
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        builder = builder.default_headers({
            let mut h = reqwest::header::HeaderMap::new();
            h.insert(
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&format!("Bearer {token}"))?,
            );
            h
        });
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
    let asset = asset_name(version);
    format!("https://github.com/{REPO}/releases/download/{version}/{asset}")
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
}

pub async fn latest_version() -> Result<String, Box<dyn std::error::Error>> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let resp = api_client()?
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("GitHub API returned {status}: {body}").into());
    }

    let release: Release = resp.json().await?;
    Ok(release.tag_name)
}

pub async fn download_and_extract(version: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let resolved = if version == "latest" {
        latest_version().await?
    } else {
        version.to_string()
    };

    let binary_path = paths::kernel_binary_path(&resolved);
    if binary_path.exists() {
        println!("kernel {} already installed", resolved);
        return Ok(binary_path);
    }

    let url = download_url(&resolved);
    println!("downloading {} ...", url);

    let resp = api_client()?
        .get(&url)
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        return Err(format!("download failed: HTTP {status}").into());
    }

    let bytes = resp.bytes().await?;
    let version_dir = paths::kernel_version_dir(&resolved);
    fs::create_dir_all(&version_dir)?;

    let (os, _) = detect_platform();
    if os == "windows" {
        extract_zip(&bytes, &version_dir)?;
    } else {
        extract_gz(&bytes, &binary_path)?;
    }

    if !binary_path.exists() {
        return Err("binary not found after extraction".into());
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&binary_path, fs::Permissions::from_mode(0o755))?;
    }

    println!("installed kernel {} to {}", resolved, version_dir.display());
    Ok(binary_path)
}

fn extract_gz(data: &[u8], dest: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut decoder = GzDecoder::new(data);
    let mut buf = Vec::new();
    decoder.read_to_end(&mut buf)?;
    let mut file = fs::File::create(dest)?;
    file.write_all(&buf)?;
    Ok(())
}

fn extract_zip(data: &[u8], dest_dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let reader = std::io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(reader)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();
        // Find the binary: look for mihomo*.exe or just mihomo
        if name.ends_with(".exe") || (!name.contains('/') && !name.contains('.') && name.contains("mihomo"))
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
