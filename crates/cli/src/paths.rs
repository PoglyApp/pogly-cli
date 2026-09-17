use std::path::PathBuf;

#[cfg(windows)]
fn config_base_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("USERPROFILE")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir)
        })
}

#[cfg(not(windows))]
fn config_base_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir)
                .join(".config")
        })
}

#[cfg(windows)]
fn local_base_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("USERPROFILE")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir)
        })
}

#[cfg(not(windows))]
fn local_base_dir() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir)
                .join(".local")
                .join("share")
        })
}

pub fn config_dir() -> PathBuf {
    config_base_dir().join("Pogly").join("cli")
}

pub fn local_dir() -> PathBuf {
    local_base_dir().join("Pogly").join("cli")
}

pub fn config_file() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn state_file() -> PathBuf {
    local_dir().join("state.toml")
}

pub fn version_pointer() -> PathBuf {
    local_dir().join("version")
}

pub fn bin_dir() -> PathBuf {
    local_dir().join("bin")
}

pub fn launcher_path() -> PathBuf {
    local_dir().join(if cfg!(windows) { "pogly.exe" } else { "pogly" })
}
