//! XDG Base Directory resolution (with a fallback for non-Linux dev hosts).

use std::path::PathBuf;

const APP_DIR: &str = "bigify";

fn env_dir(var: &str) -> Option<PathBuf> {
    std::env::var_os(var)
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
}

fn home() -> PathBuf {
    env_dir("HOME")
        .or_else(|| env_dir("USERPROFILE"))
        .unwrap_or_else(std::env::temp_dir)
}

fn xdg(var: &str, fallback: &[&str]) -> PathBuf {
    env_dir(var)
        .unwrap_or_else(|| fallback.iter().fold(home(), |p, seg| p.join(seg)))
        .join(APP_DIR)
}

/// `$XDG_CONFIG_HOME/bigify` — settings.toml.
pub fn config_dir() -> PathBuf {
    xdg("XDG_CONFIG_HOME", &[".config"])
}

/// `$XDG_CACHE_HOME/bigify` — audio and metadata caches (safe to delete).
pub fn cache_dir() -> PathBuf {
    xdg("XDG_CACHE_HOME", &[".cache"])
}

/// `$XDG_STATE_HOME/bigify` — logs and reusable Spotify credentials.
pub fn state_dir() -> PathBuf {
    xdg("XDG_STATE_HOME", &[".local", "state"])
}

pub fn settings_file() -> PathBuf {
    config_dir().join("settings.toml")
}

pub fn log_file() -> PathBuf {
    state_dir().join("bigify.log")
}
