//! Persisted user preferences (`$XDG_CONFIG_HOME/bigify/settings.toml`).
//!
//! Only options the playback engine or UI actually honour are listed here.
//! Unknown keys are ignored and missing keys take defaults, so older and
//! newer config files both load.

use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum AudioQuality {
    /// 96 kbit/s
    Low,
    /// 160 kbit/s
    Normal,
    /// 320 kbit/s (Premium)
    #[default]
    High,
}

impl AudioQuality {
    pub fn kbps(self) -> u32 {
        match self {
            AudioQuality::Low => 96,
            AudioQuality::Normal => 160,
            AudioQuality::High => 320,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum AudioBackend {
    /// PulseAudio protocol; also served by PipeWire (pipewire-pulse), which
    /// is what SteamOS and most current desktops use.
    #[default]
    Pulseaudio,
    /// Direct ALSA, for minimal systems without a sound server.
    Alsa,
}

impl AudioBackend {
    pub fn librespot_name(self) -> &'static str {
        match self {
            AudioBackend::Pulseaudio => "pulseaudio",
            AudioBackend::Alsa => "alsa",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Name shown in Spotify Connect device lists.
    pub device_name: String,
    pub audio_quality: AudioQuality,
    pub audio_backend: AudioBackend,
    /// ReplayGain-style loudness normalisation.
    pub normalisation: bool,
    pub gapless: bool,
    /// Volume used when the session starts, 0..=100.
    pub initial_volume: u8,
    /// Advertise Bigify on the LAN so the Spotify app can sign it in.
    pub zeroconf_discovery: bool,
    pub start_fullscreen: bool,
    /// Tint the interface with colors taken from the current artwork.
    pub dynamic_color: bool,
    /// Replace movement with fades and disable decorative animation.
    pub reduced_motion: bool,
    /// Read controllers directly (disable if Steam Input already maps the
    /// controller to keyboard events, to avoid double input).
    pub controller_input: bool,
    /// UI scale multiplier on top of automatic resolution scaling.
    pub ui_scale: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            device_name: "Bigify".into(),
            audio_quality: AudioQuality::default(),
            audio_backend: AudioBackend::default(),
            normalisation: true,
            gapless: true,
            initial_volume: 60,
            zeroconf_discovery: true,
            start_fullscreen: true,
            dynamic_color: true,
            reduced_motion: false,
            controller_input: true,
            ui_scale: 1.0,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("could not read settings: {0}")]
    Io(#[from] std::io::Error),
    #[error("settings file is malformed: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("could not serialise settings: {0}")]
    Serialise(#[from] toml::ser::Error),
}

impl Settings {
    /// Clamp values into ranges the engine accepts.
    pub fn sanitised(mut self) -> Self {
        self.device_name = self.device_name.trim().chars().take(64).collect();
        if self.device_name.is_empty() {
            self.device_name = Settings::default().device_name;
        }
        self.initial_volume = self.initial_volume.min(100);
        if !self.ui_scale.is_finite() {
            self.ui_scale = 1.0;
        }
        self.ui_scale = self.ui_scale.clamp(0.75, 1.5);
        self
    }

    pub fn from_toml(text: &str) -> Result<Self, SettingsError> {
        Ok(toml::from_str::<Settings>(text)?.sanitised())
    }

    pub fn to_toml(&self) -> Result<String, SettingsError> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// Load settings; a missing file yields defaults. A malformed file is
    /// reported so the caller can warn instead of silently discarding it.
    pub fn load(path: &Path) -> Result<Self, SettingsError> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::from_toml(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.into()),
        }
    }

    /// Write atomically (temp file + rename) so a crash cannot truncate it.
    pub fn save(&self, path: &Path) -> Result<(), SettingsError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, self.to_toml()?)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Apply a single change coming from the UI (`key` uses the camelCase
    /// field name, `value` is JSON). Returns false for unknown keys or
    /// values of the wrong type.
    pub fn set_from_json(&mut self, key: &str, value: &serde_json::Value) -> bool {
        let mut obj = match serde_json::to_value(&*self) {
            Ok(serde_json::Value::Object(map)) => map,
            _ => return false,
        };
        if !obj.contains_key(key) {
            return false;
        }
        obj.insert(key.to_owned(), value.clone());
        match serde_json::from_value::<Settings>(serde_json::Value::Object(obj)) {
            Ok(updated) => {
                *self = updated.sanitised();
                true
            }
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let s = Settings {
            device_name: "Living Room".into(),
            audio_quality: AudioQuality::Normal,
            ..Settings::default()
        };
        let text = s.to_toml().unwrap();
        assert_eq!(Settings::from_toml(&text).unwrap(), s);
    }

    #[test]
    fn partial_and_unknown_keys() {
        let s = Settings::from_toml("deviceName = \"Deck\"\nsomethingNew = 3\n").unwrap();
        assert_eq!(s.device_name, "Deck");
        assert!(s.normalisation, "missing keys use defaults");
    }

    #[test]
    fn sanitises_values() {
        let s = Settings::from_toml("deviceName = \"   \"\ninitialVolume = 250\nuiScale = 9.0")
            .unwrap();
        assert_eq!(s.device_name, "Bigify");
        assert_eq!(s.initial_volume, 100);
        assert_eq!(s.ui_scale, 1.5);
    }

    #[test]
    fn set_from_json_validates() {
        let mut s = Settings::default();
        assert!(s.set_from_json("audioQuality", &serde_json::json!("low")));
        assert_eq!(s.audio_quality, AudioQuality::Low);
        assert!(!s.set_from_json("audioQuality", &serde_json::json!("ultra")));
        assert!(!s.set_from_json("nope", &serde_json::json!(true)));
        assert!(!s.set_from_json("dynamicColor", &serde_json::json!("yes")));
        assert_eq!(s.audio_quality, AudioQuality::Low, "failed set keeps old");
    }

    #[test]
    fn missing_file_gives_defaults() {
        let path = std::env::temp_dir().join("bigify-test-does-not-exist/settings.toml");
        assert_eq!(Settings::load(&path).unwrap(), Settings::default());
    }

    #[test]
    fn save_and_load() {
        let dir = std::env::temp_dir().join(format!("bigify-test-{}", std::process::id()));
        let path = dir.join("settings.toml");
        let s = Settings {
            reduced_motion: true,
            ..Settings::default()
        };
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path).unwrap(), s);
        std::fs::remove_dir_all(dir).ok();
    }
}
