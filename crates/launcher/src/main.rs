use std::path::{Path, PathBuf};
use std::process::Command;

fn local_dir() -> PathBuf {
    #[cfg(windows)]
    {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("Pogly")
            .join("cli")
    }

    #[cfg(not(windows))]
    {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(std::env::temp_dir)
                    .join(".local")
                    .join("share")
            })
            .join("Pogly")
            .join("cli")
    }
}

fn cli_binary_name() -> &'static str {
    if cfg!(windows) {
        "pogly-cli.exe"
    } else {
        "pogly-cli"
    }
}

fn launcher_old_name() -> &'static str {
    if cfg!(windows) {
        "pogly.exe.old"
    } else {
        "pogly.old"
    }
}

fn selected_version(dir: &Path) -> Option<String> {
    let pointer = std::fs::read_to_string(dir.join("version"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    if pointer.is_some() {
        return pointer;
    }

    // No pointer file: fall back to the highest installed version.
    let binary_name = cli_binary_name();
    let mut versions: Vec<String> = std::fs::read_dir(dir.join("bin"))
        .ok()?
        .flatten()
        .filter(|e| e.path().join(binary_name).is_file())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    versions.sort();
    versions.pop()
}

fn main() {
    let dir = local_dir();

    // Leftover from a launcher self-upgrade; it can't delete itself while running.
    if let Some(parent) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
    {
        let _ = std::fs::remove_file(parent.join(launcher_old_name()));
    }

    let Some(version) = selected_version(&dir) else {
        eprintln!("pogly-cli is not installed.");
        eprintln!("Download it from https://github.com/PoglyApp/pogly-cli/releases");
        std::process::exit(1);
    };

    let exe = dir.join("bin").join(&version).join(cli_binary_name());
    if !exe.is_file() {
        eprintln!("pogly-cli {version} is not installed at {}", exe.display());
        eprintln!(
            "Run `pogly version list` from an installed version, \
             or reinstall from https://github.com/PoglyApp/pogly-cli/releases"
        );
        std::process::exit(1);
    }

    // Spawn + wait rather than exec: Windows has no execvp, and Ctrl+C reaches
    // both processes through the shared console.
    let status = Command::new(&exe)
        .args(std::env::args_os().skip(1))
        .status();
    match status {
        Ok(s) => std::process::exit(s.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("failed to run {}: {e}", exe.display());
            std::process::exit(1);
        }
    }
}
