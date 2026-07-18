use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde_json::Value;
use ureq::Agent;

use crate::paths;

pub fn installed_versions() -> Vec<String> {
    let mut versions: Vec<String> = std::fs::read_dir(paths::bin_dir())
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.path().join("pogly-cli.exe").is_file())
                .filter_map(|e| e.file_name().into_string().ok())
                .collect()
        })
        .unwrap_or_default();
    versions.sort_by(
        |a, b| match (semver::Version::parse(a), semver::Version::parse(b)) {
            (Ok(a), Ok(b)) => a.cmp(&b),
            _ => a.cmp(b),
        },
    );
    versions
}

pub fn pointer() -> Option<String> {
    std::fs::read_to_string(paths::version_pointer())
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn set_pointer(version: &str) -> Result<()> {
    std::fs::create_dir_all(paths::local_dir())?;
    std::fs::write(paths::version_pointer(), version).context("failed to write version pointer")
}

fn download_agent() -> Agent {
    Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(120)))
        .user_agent(concat!("pogly-cli/", env!("CARGO_PKG_VERSION")))
        .build()
        .new_agent()
}

fn asset_url<'a>(release: &'a Value, name: &str) -> Option<&'a str> {
    release
        .get("assets")?
        .as_array()?
        .iter()
        .find(|a| a.get("name").and_then(Value::as_str) == Some(name))?
        .get("browser_download_url")?
        .as_str()
}

fn download(url: &str) -> Result<Vec<u8>> {
    let mut response = download_agent()
        .get(url)
        .call()
        .with_context(|| format!("download failed: {url}"))?;
    let bytes = response
        .body_mut()
        .with_config()
        .limit(200 * 1024 * 1024)
        .read_to_vec()
        .context("failed to read download")?;
    Ok(bytes)
}

pub fn install_version(version: &str, release: &Value) -> Result<()> {
    let binary_name = if cfg!(windows) { "pogly-cli.exe" } else { "pogly-cli" };
    let url = asset_url(release, binary_name)
        .or_else(|| asset_url(release, "pogly-cli.exe"))
        .context("release has no pogly-cli asset")?;
    let dir = paths::bin_dir().join(version);
    std::fs::create_dir_all(&dir)?;
    let target = dir.join(binary_name);
    let tmp = dir.join(format!("{binary_name}.tmp"));
    std::fs::write(&tmp, download(url)?)?;
    std::fs::rename(&tmp, &target)
        .with_context(|| format!("failed to install {}", target.display()))?;
    Ok(())
}

// The launcher stays a running parent process during upgrades, so it can't be
// overwritten — but Windows/Unix allows renaming a running image out of the way.
// The launcher deletes the .old file on its next start.
pub fn replace_launcher(release: &Value) -> Result<()> {
    let path = paths::launcher_path();
    if !path.is_file() {
        return Ok(());
    }
    let launcher_name = if cfg!(windows) { "pogly.exe" } else { "pogly" };
    let Some(url) = asset_url(release, launcher_name).or_else(|| asset_url(release, "pogly.exe")) else {
        return Ok(());
    };
    let bytes = download(url)?;
    let launcher_old_name = if cfg!(windows) { "pogly.exe.old" } else { "pogly.old" };
    let old = paths::local_dir().join(launcher_old_name);
    let _ = std::fs::remove_file(&old);
    std::fs::rename(&path, &old).context("failed to move the current launcher aside")?;
    if let Err(e) = std::fs::write(&path, bytes) {
        let _ = std::fs::rename(&old, &path);
        bail!("failed to write new launcher: {e}");
    }
    Ok(())
}
