//! User-global settings, stored outside the DAW project.
//!
//! nice-plug already persists every parameter plus `#[persist]` fields with the host's
//! project, so this file only holds preferences that should follow the user across every
//! project: the colour theme, the host scale factor and the MIDI export folder.
//! Everything musical stays in the project state.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Written on a debounce so dragging a slider does not hit the disk every frame. The value
/// is an epoch-second timestamp compared with `now`.
static LAST_WRITE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

const DEBOUNCE_MS: u64 = 750;

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct GlobalSettings {
    /// Index into `Theme`. Absent or out of range falls back to the host default.
    pub theme: Option<usize>,
    /// Editor scale factor as the host reports it; `None` means "follow the system".
    pub scale: Option<f32>,
    /// Last folder used for MIDI export. `None` means the host process working directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub midi_dir: Option<String>,
}

/// `$XDG_CONFIG_HOME/nrum-sequencer/settings.json`, falling back to
/// `$HOME/.config/nrum-sequencer/settings.json`, or `%APPDATA%` on Windows. Returns `None`
/// when no config directory can be determined — a host launched with neither `HOME` nor
/// `XDG_CONFIG_HOME`, e.g. under a bare systemd unit. Settings then simply do not persist
/// across sessions rather than failing the editor.
fn config_path() -> Option<PathBuf> {
    #[cfg(windows)]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
    Some(base?.join("nrum-sequencer").join("settings.json"))
}

/// Human-readable location shown in Settings; uneditable so users can find the file without
/// guessing where the host's environment points.
pub(crate) fn config_path_display() -> String {
    config_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "unavailable (no HOME/XDG_CONFIG_HOME)".to_string())
}

pub(crate) fn load() -> GlobalSettings {
    config_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Write only if `DEBOUNCE_MS` has passed since the last *write*. Failures are silent:
/// a read-only home directory should not break editing a pattern.
pub(crate) fn save(settings: &GlobalSettings) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    // Load first, store only when actually writing. Storing before the check made this a
    // throttle: every call restarted the window, so a burst of changes wrote on a fixed
    // cadence instead of once the user stopped changing things.
    if now.saturating_sub(LAST_WRITE.load(std::sync::atomic::Ordering::Relaxed)) < DEBOUNCE_MS {
        return;
    }
    LAST_WRITE.store(now, std::sync::atomic::Ordering::Relaxed);
    let path = match config_path() { Some(path) => path, None => return };
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write(path, json);
    }
}

/// Force a write on editor close, bypassing the debounce so the last change is not lost.
pub(crate) fn save_now(settings: &GlobalSettings) {
    LAST_WRITE.store(0, std::sync::atomic::Ordering::Relaxed);
    save(settings);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Theme as PluginTheme;
    use nice_plug::prelude::Enum;

    #[test]
    fn settings_round_trip_through_json() {
        let s = GlobalSettings { theme: Some(2), scale: Some(1.25), midi_dir: None };
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, r#"{"theme":2,"scale":1.25}"#);
        let back: GlobalSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(back.theme, Some(2));
        assert_eq!(back.scale, Some(1.25));
        assert_eq!(back.midi_dir, None);
        // A settings file written while the sequencer-size setting still existed must load.
        assert_eq!(serde_json::from_str::<GlobalSettings>(r#"{"theme":0,"sequencer_scale":1}"#).unwrap().theme, Some(0));
    }

    #[test]
    fn missing_and_corrupt_files_fall_back_to_defaults() {
        assert_eq!(serde_json::from_str::<GlobalSettings>("{}").unwrap().theme, None);
        // An unknown extra field must not break loading, and neither must garbage.
        assert_eq!(serde_json::from_str::<GlobalSettings>(r#"{"theme":1,"who":42}"#).unwrap().theme, Some(1));
        assert!(serde_json::from_str::<GlobalSettings>("not json").is_err());
        assert_eq!(GlobalSettings::default().theme, None);
    }

    #[test]
    fn an_out_of_range_theme_cannot_panic() {
        // `Theme::from_index` falls back to the first variant, which is Sketchbook (host id
        // `aurora`).
        assert_eq!(PluginTheme::from_index(9), PluginTheme::Sketchbook);
        assert_eq!(PluginTheme::from_index(0), PluginTheme::Sketchbook);
    }

    #[test]
    fn config_path_is_exactly_the_expected_file_under_the_config_home() {
        // Only meaningful when a config home is actually resolvable.
        let Some(path) = config_path() else { return };
        // The old assertion only checked the path *contained* the app name, which any path
        // would pass; pin the actual location. On Linux that is `$XDG_CONFIG_HOME` or
        // `$HOME/.config`, so this fails loudly if settings ever start landing elsewhere.
        let expected_home: Option<PathBuf> = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|p| !p.as_os_str().is_empty())
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
        // No config home resolvable at all; there is nothing to compare against.
        let Some(home) = expected_home else { return };
        assert_eq!(path, home.join("nrum-sequencer").join("settings.json"));
    }
}