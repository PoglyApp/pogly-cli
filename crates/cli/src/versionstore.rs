use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::Value;
use ureq::Agent;

use crate::paths;

fn cli_binary_name() -> &'static str {
    if cfg!(windows) {
        "pogly-cli.exe"
    } else {
        "pogly-cli"
    }
}

fn launcher_binary_name() -> &'static str {
    if cfg!(windows) {
        "pogly.exe"
    } else {
        "pogly"
    }
}

pub fn installed_versions() -> Vec<String> {
    let binary_name = cli_binary_name();
    let mut versions: Vec<String> = std::fs::read_dir(paths::bin_dir())
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.path().join(binary_name).is_file())
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
    let binary_name = cli_binary_name();
    let url = asset_url(release, binary_name)
        .with_context(|| format!("release has no {binary_name} asset"))?;
    let dir = paths::bin_dir().join(version);
    std::fs::create_dir_all(&dir)?;
    let target = dir.join(binary_name);
    let tmp = dir.join(format!("{binary_name}.tmp"));
    std::fs::write(&tmp, download(url)?)?;
    std::fs::rename(&tmp, &target)
        .with_context(|| format!("failed to install {}", target.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = std::fs::metadata(&target)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&target, permissions)?;
    }
    Ok(())
}

// The launcher stays a running parent process during upgrades, so it can't be
// overwritten — but Windows allows renaming a running image out of the way.
// The launcher deletes the .old file on its next start.
pub fn replace_launcher(release: &Value) -> Result<()> {
    let path = paths::launcher_path();
    if !path.is_file() {
        return Ok(());
    }
    let launcher_name = launcher_binary_name();
    let Some(url) = asset_url(release, launcher_name) else {
        return Ok(());
    };
    let bytes = download(url)?;
    let old = paths::local_dir().join(if cfg!(windows) {
        "pogly.exe.old"
    } else {
        "pogly.old"
    });
    let tmp = paths::local_dir().join(if cfg!(windows) {
        "pogly.exe.tmp"
    } else {
        "pogly.tmp"
    });
    let _ = std::fs::remove_file(&old);
    let _ = std::fs::remove_file(&tmp);
    std::fs::write(&tmp, bytes).context("failed to write new launcher")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = std::fs::metadata(&tmp)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&tmp, permissions)?;
    }
    std::fs::rename(&path, &old).context("failed to move the current launcher aside")?;
    if let Err(e) = std::fs::rename(&tmp, &path) {
        let _ = std::fs::rename(&old, &path);
        return Err(e).context("failed to install new launcher");
    }
    Ok(())
}
