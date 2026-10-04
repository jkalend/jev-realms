//! Runtime display options with persistence (E12 accessibility surface).
//!
//! Three post-stack effects can be killed in-window without a restart:
//! `B` bloom, `V` vignette, `L` light-flicker (the combat-pulse term).
//! Zoom rides along so a comfortable px/tile survives reboots.
//!
//! Defaults reproduce the E0/E4-ratified frame exactly; a missing or corrupt
//! `saves/view-options.json` silently falls back to them — the file is a
//! preference, never a gate.

use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::camera::{CamZoom, MainCam};

pub const DEFAULT_PATH: &str = "saves/view-options.json";

pub fn default_path() -> PathBuf {
    PathBuf::from(DEFAULT_PATH)
}

#[derive(Resource, Serialize, Deserialize)]
pub struct ViewOptions {
    #[serde(default = "on")]
    pub bloom: bool,
    #[serde(default = "on")]
    pub vignette: bool,
    #[serde(default = "on")]
    pub flicker: bool,
    #[serde(default = "default_zoom")]
    pub zoom: f32,
    /// Set when a mutation hasn't been flushed yet; not a user preference.
    #[serde(skip)]
    dirty: Option<Instant>,
}

fn on() -> bool {
    true
}

fn default_zoom() -> f32 {
    crate::camera::ZOOM_DEFAULT
}

impl Default for ViewOptions {
    fn default() -> Self {
        Self {
            bloom: true,
            vignette: true,
            flicker: true,
            zoom: crate::camera::ZOOM_DEFAULT,
            dirty: None,
        }
    }
}

impl ViewOptions {
    /// Tolerant load: any read/parse failure yields the ratified defaults.
    pub fn load_from(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn load() -> Self {
        Self::load_from(&default_path())
    }

    /// Best-effort write; a readonly working dir must not cost the frame.
    fn save_to(&self, path: &Path) {
        if let Some(parent) = path.parent() {
            if std::fs::create_dir_all(parent).is_err() {
                return;
            }
        }
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, text);
        }
    }

    pub fn save(&self) {
        self.save_to(&default_path());
    }

    fn mark_dirty(&mut self) {
        self.dirty = Some(Instant::now());
    }
}

/// Debounce window: wheel zoom can fire at event rate; the preference file
/// only needs to settle after the last adjustment.
const SAVE_DEBOUNCE: Duration = Duration::from_millis(500);

/// Key bind in one place: B bloom, V vignette, L light-flicker.
/// (F was the natural flicker mnemonic but already means Defend.)
const KEY_BLOOM: KeyCode = KeyCode::KeyB;
const KEY_VIGNETTE: KeyCode = KeyCode::KeyV;
const KEY_FLICKER: KeyCode = KeyCode::KeyL;

pub fn toggles(
    keys: Res<ButtonInput<KeyCode>>,
    mut opts: ResMut<ViewOptions>,
    zoom: ResMut<CamZoom>,
    mut blooms: Query<&mut Bloom, With<MainCam>>,
) {
    let mut changed = false;
    if keys.just_pressed(KEY_BLOOM) {
        opts.bloom = !opts.bloom;
        for mut bloom in blooms.iter_mut() {
            // Component stays attached both ways — intensity is the gate,
            // so no structural churn on the post stack.
            bloom.intensity = if opts.bloom { 0.08 } else { 0.0 };
        }
        changed = true;
    }
    if keys.just_pressed(KEY_VIGNETTE) {
        opts.vignette = !opts.vignette;
        changed = true;
    }
    if keys.just_pressed(KEY_FLICKER) {
        opts.flicker = !opts.flicker;
        changed = true;
    }
    // Wheel zoom updates CamZoom directly; fold it back into the preference.
    if opts.zoom != zoom.target {
        opts.zoom = zoom.target;
        changed = true;
    }
    if changed {
        opts.mark_dirty();
    }
    let flush = opts
        .dirty
        .is_some_and(|since| since.elapsed() >= SAVE_DEBOUNCE);
    if flush {
        opts.dirty = None;
        opts.save();
    }
}

pub fn bloom_intensity(enabled: bool) -> f32 {
    if enabled {
        0.08
    } else {
        0.0
    }
}

pub fn vignette_intensity(opts: &ViewOptions, in_combat: bool, elapsed_secs: f32) -> f32 {
    if !opts.vignette {
        return 0.0;
    }
    if in_combat && opts.flicker {
        0.35 + 0.10 * (0.5 + 0.5 * (elapsed_secs * 4.0).sin())
    } else {
        0.35
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("laya-view-options-{name}-{stamp}.json"))
    }

    #[test]
    fn defaults_reproduce_ratified_frame() {
        let opts = ViewOptions::default();
        assert!(opts.bloom && opts.vignette && opts.flicker);
        assert_eq!(opts.zoom, crate::camera::ZOOM_DEFAULT);
        assert_eq!(bloom_intensity(true), 0.08);
        assert_eq!(vignette_intensity(&opts, false, 0.0), 0.35);
    }

    #[test]
    fn missing_file_falls_back_to_defaults() {
        let path = temp_path("missing");
        let opts = ViewOptions::load_from(&path);
        assert!(opts.bloom && opts.zoom == crate::camera::ZOOM_DEFAULT);
    }

    #[test]
    fn corrupt_file_falls_back_to_defaults() {
        let path = temp_path("corrupt");
        std::fs::write(&path, "{ not json at all ").unwrap();
        let opts = ViewOptions::load_from(&path);
        assert!(opts.bloom && opts.vignette && opts.flicker);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn partial_file_keeps_defaults_for_absent_fields() {
        let path = temp_path("partial");
        std::fs::write(&path, r#"{ "bloom": false }"#).unwrap();
        let opts = ViewOptions::load_from(&path);
        assert!(!opts.bloom);
        assert!(opts.vignette && opts.flicker);
        assert_eq!(opts.zoom, crate::camera::ZOOM_DEFAULT);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn roundtrip_preserves_values() {
        let path = temp_path("roundtrip");
        let mut opts = ViewOptions {
            bloom: false,
            vignette: false,
            flicker: false,
            zoom: 32.0,
            dirty: None,
        };
        opts.mark_dirty();
        opts.save_to(&path);
        let loaded = ViewOptions::load_from(&path);
        assert!(!loaded.bloom && !loaded.vignette && !loaded.flicker);
        assert_eq!(loaded.zoom, 32.0);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn vignette_combines_gates() {
        let mut opts = ViewOptions::default();
        opts.vignette = false;
        assert_eq!(vignette_intensity(&opts, true, 1.0), 0.0);
        opts.vignette = true;
        opts.flicker = false;
        assert_eq!(vignette_intensity(&opts, true, 1.0), 0.35);
        opts.flicker = true;
        let pulsed = vignette_intensity(&opts, true, 0.9);
        assert!(pulsed > 0.35 && pulsed <= 0.45001);
    }
}
