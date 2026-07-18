use std::path::PathBuf;

#[cfg(windows)]
fn env_dir(var: &str) -> PathBuf {
    std::env::var_os(var)
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

#[cfg(windows)]
pub fn config_dir() -> PathBuf {
    env_dir("APPDATA").join("Pogly").join("cli")
}

#[cfg(windows)]
pub fn local_dir() -> PathBuf {
    env_dir("LOCALAPPDATA").join("Pogly").join("cli")
}

#[cfg(not(windows))]
fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

#[cfg(not(windows))]
pub fn config_dir() -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from) {
        xdg.join("Pogly").join("cli")
    } else {
        home_dir().join(".config").join("Pogly").join("cli")
    }
}

#[cfg(not(windows))]
pub fn local_dir() -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from) {
        xdg.join("Pogly").join("cli")
    } else {
        home_dir().join(".local").join("share").join("Pogly").join("cli")
    }
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
    if cfg!(windows) {
        local_dir().join("pogly.exe")
    } else {
        local_dir().join("pogly")
    }
}
