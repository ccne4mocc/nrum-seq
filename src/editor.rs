//! Slint GUI adapter, following the reference project's separation between pure geometry and UI glue.
use crate::globals;
use crate::keys;
use crate::midi_export;
use crate::modulation::{format_target_value, target_in_mask, LfoClock, LFO_COUNT, LFO_MAX_HZ};
use crate::edits::{EditState, EditView};
use crate::sequence::{generate_with, GenParams, Pattern, Rng, MAX_STEPS, MIN_MAX_STEPS, TRACKS};
use crate::view::{self, View};
use crate::{
    cell_count_for, scope, DrumParams, LfoPolarity, LfoRateMode, ModTarget, Mode, OscWave, Rate,
    Shared, Theme as PluginTheme,
};
use nice_plug::prelude::*;
use slint::platform::WindowEvent;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use slint_baseview::{default_gl_config, Size, SlintWindow, WindowHandle, WindowOpenOptions, WindowScalePolicy};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

slint::include_modules!();
const WIDTH: u32 = 860;
const HEIGHT: u32 = 740;

/// Built-in starting points. These are editor conveniences, not host parameters: applying one
/// writes ordinary host parameters so the result remains automatable and saveable.
struct Preset {
    name: &'static str,
    mode: Mode,
    steps: f32,
    pulses: i32,
    density: f32,
    probability: f32,
    seed: f32,
}

const PRESETS: &[Preset] = &[
    Preset { name: "HOUSE", mode: Mode::Euclidean, steps: 16.0, pulses: 4, density: 0.88, probability: 1.0, seed: 0.271 },
    Preset { name: "BREAK", mode: Mode::Syncopation, steps: 16.0, pulses: 6, density: 0.72, probability: 0.92, seed: 0.413 },
    Preset { name: "POLY", mode: Mode::Polyrhythm, steps: 16.0, pulses: 8, density: 0.80, probability: 1.0, seed: 0.127 },
    Preset { name: "DEEP", mode: Mode::Slowcat, steps: 32.0, pulses: 5, density: 0.55, probability: 1.0, seed: 0.611 },
    Preset { name: "GLITCH", mode: Mode::Genetic, steps: 16.0, pulses: 7, density: 0.45, probability: 0.78, seed: 0.337 },
];

fn apply_preset(c: &Arc<dyn GuiContext>, p: &DrumParams, index: i32) {
    let Some(preset) = PRESETS.get(index.max(0) as usize) else { return; };
    set(c, &p.mode, preset.mode);
    set(c, &p.steps, preset.steps.min(p.step_ceiling() as f32));
    set(c, &p.pulses, preset.pulses.min(p.cell_count()));
    set(c, &p.density, preset.density);
    set(c, &p.probability, preset.probability);
    set(c, &p.seed, preset.seed);
    p.variation.fetch_add(1, Relaxed);
}

/// Append one technical line to `nrum-seq.log`; GUI thread only, never the audio path.
fn log_line(text: &str) {
    use std::io::Write;
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
    let Some(home) = home else { return };
    let path = PathBuf::from(home).join("nrum-seq.log");
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{}", text);
    }
}

pub fn create(params: Arc<DrumParams>, shared: Arc<Shared>) -> Box<dyn Editor> {
    Box::new(SlintEditor {
        params, shared,
        scale: Arc::new(AtomicU32::new(0)),
    })
}
struct SlintEditor {
    params: Arc<DrumParams>,
    shared: Arc<Shared>,
    /// Host DPI factor. `0` means "not reported yet".
    scale: Arc<AtomicU32>,
}

/// Snapshot the user-global settings out of the parameters, for `globals::save`.
fn global_settings(
    p: &DrumParams,
    scale: f32,
    midi_dir: &Mutex<String>,
) -> globals::GlobalSettings {
    globals::GlobalSettings {
        theme: Some(p.theme.value().to_index()),
        scale: if scale > 0.0 { Some(scale) } else { None },
        midi_dir: midi_dir.lock().ok().map(|d| d.clone()).filter(|d| !d.trim().is_empty()),
    }
}
struct SlintEditorHandle {
    window: WindowHandle,
    params: Arc<DrumParams>,
    shared: Arc<Shared>,
    scale: Arc<AtomicU32>,
    midi_dir: Arc<Mutex<String>>,
}
unsafe impl Send for SlintEditorHandle {}
impl Drop for SlintEditorHandle {
    fn drop(&mut self) {
        // Closing is the last chance to persist, so bypass the debounce. Read theme and scale
        // from live state here: a spawn-time snapshot would revert every change made during the
        // session. There is no snapshot field left to go stale.
        globals::save_now(&global_settings(
            &self.params,
            f32::from_bits(self.scale.load(Relaxed)),
            &self.midi_dir,
        ));
        // No text field exists any more, so host keys must go back to the host.
        self.shared.text_focus.store(false, Relaxed);
        if let Ok(mut queue) = self.shared.key_queue.lock() { queue.clear(); }
        self.window.close();
    }
}

fn set<P: Param>(ctx: &Arc<dyn GuiContext>, param: &P, value: P::Plain) {
    let setter = ParamSetter::new(ctx.as_ref());
    setter.begin_set_parameter(param);
    setter.set_parameter(param, value);
    setter.end_set_parameter(param);
}

/// Shared clamp for the slider and spin-box paths.
fn set_opt(c: &Arc<dyn GuiContext>, p: &DrumParams, key: i32, value: f32) {
    let snap = p.round_steps.value();
    // Host echo is asynchronous; use the value being set so dependent limits cannot lag a tick.
    if key == 14 {
        let cap = (value.round() as i32).clamp(MIN_MAX_STEPS as i32, MAX_STEPS as i32);
        set(c, &p.max_steps, cap);
        let cells = cell_count_for(p.steps.value(), snap, cap as usize);
        if p.steps.value() > cap as f32 { set(c, &p.steps, cap as f32); }
        if p.pulses.value() > cells { set(c, &p.pulses, cells); }
        if p.rotate.value() >= cells { set(c, &p.rotate, cells - 1); }
        return;
    }
    // `p.steps.value()` lags until the host echoes the set, so derive bounds from the value being set.
    let cap = p.step_ceiling();
    let cells = if key == 0 { cell_count_for(value, snap, cap) } else { p.cell_count() };
    let max_pulses = match p.mode.value() {
        Mode::Cellular => 15,
        Mode::Sieve => 2,
        Mode::Hilbert => 5,
        _ => cells,
    };
    match key {
        0 => {
            // Snap on: store a whole number so 16.37 cannot display as "16" while staying fractional.
            let v = value.clamp(1.0, cap as f32);
            set(c, &p.steps, if snap { v.round() } else { v });
            if p.pulses.value() > max_pulses { set(c, &p.pulses, max_pulses); }
            if p.rotate.value() >= cells { set(c, &p.rotate, cells - 1); }
        },
        1 => set(c, &p.pulses, (value.round() as i32).clamp(0, max_pulses)),
        2 => set(c, &p.rotate, (value.round() as i32).clamp(0, cells - 1)),
        3 => set(c, &p.density, value.clamp(0.0, 1.0)),
        4 => set(c, &p.seed, value.clamp(0.0, 1.0)),
        5 => set(c, &p.phase, value.clamp(0.0, 1.0)),
        6 => set(c, &p.osc_rate, value.clamp(0.125, 8.0)),
        7 => set(c, &p.ca_rule, (value.round() as i32).clamp(0, 255)),
        8 => set(c, &p.probability, value.clamp(0.0, 1.0)),
        9 => set(c, &p.velocity, value.clamp(0.01, 1.0)),
        10 => set(c, &p.regenerate_every, (value.round() as i32).clamp(0, 256)),
        11 => set(c, &p.symmetry, (value.round() as i32).clamp(1, 8)),
        12 => set(c, &p.randomize_after_lengths, (value.round() as i32).clamp(0, 256)),
        13 => set(c, &p.swing, value.clamp(0.0, 0.5)),
        15 => set(c, &p.gate, value.clamp(0.02, 1.0)),
        _ => {},
    }
}

fn lane_mode_param(p: &DrumParams, lane: i32) -> &IntParam {
    match lane {
        0 => &p.lane1_mode,
        1 => &p.lane2_mode,
        2 => &p.lane3_mode,
        3 => &p.lane4_mode,
        4 => &p.lane5_mode,
        5 => &p.lane6_mode,
        6 => &p.lane7_mode,
        _ => &p.lane8_mode,
    }
}

fn lfo_enabled_param(p: &DrumParams, slot: i32) -> &BoolParam {
    match slot { 1 => &p.lfo1_enabled, 2 => &p.lfo2_enabled, 3 => &p.lfo3_enabled, _ => &p.lfo4_enabled }
}
fn lfo_wave_param(p: &DrumParams, slot: i32) -> &EnumParam<OscWave> {
    match slot { 1 => &p.lfo1_wave, 2 => &p.lfo2_wave, 3 => &p.lfo3_wave, _ => &p.lfo4_wave }
}
fn lfo_rate_mode_param(p: &DrumParams, slot: i32) -> &EnumParam<LfoRateMode> {
    match slot { 1 => &p.lfo1_rate_mode, 2 => &p.lfo2_rate_mode, 3 => &p.lfo3_rate_mode, _ => &p.lfo4_rate_mode }
}
fn lfo_freq_param(p: &DrumParams, slot: i32) -> &FloatParam {
    match slot { 1 => &p.lfo1_freq, 2 => &p.lfo2_freq, 3 => &p.lfo3_freq, _ => &p.lfo4_freq }
}
fn lfo_beat_param(p: &DrumParams, slot: i32) -> &FloatParam {
    match slot { 1 => &p.lfo1_rate, 2 => &p.lfo2_rate, 3 => &p.lfo3_rate, _ => &p.lfo4_rate }
}
fn lfo_depth_param(p: &DrumParams, slot: i32) -> &FloatParam {
    match slot { 1 => &p.lfo1_depth, 2 => &p.lfo2_depth, 3 => &p.lfo3_depth, _ => &p.lfo4_depth }
}
fn lfo_phase_param(p: &DrumParams, slot: i32) -> &FloatParam {
    match slot { 1 => &p.lfo1_phase, 2 => &p.lfo2_phase, 3 => &p.lfo3_phase, _ => &p.lfo4_phase }
}
fn lfo_polarity_param(p: &DrumParams, slot: i32) -> &EnumParam<LfoPolarity> {
    match slot { 1 => &p.lfo1_polarity, 2 => &p.lfo2_polarity, 3 => &p.lfo3_polarity, _ => &p.lfo4_polarity }
}

/// One mode-selection path for the grouped list and the mode-header wheel.
fn apply_mode(c: &Arc<dyn GuiContext>, p: &DrumParams, index: i32) {
    let last = <Mode as Enum>::variants().len() as i32 - 1;
    let mode = Mode::from_index(index.clamp(0, last) as usize);
    set(c, &p.mode, mode);
    let max_pulses = match mode {
        Mode::Cellular => 15,
        Mode::Sieve => 2,
        Mode::Hilbert => 5,
        _ => p.cell_count(),
    };
    if p.pulses.value() > max_pulses { set(c, &p.pulses, max_pulses); }
    // Mode-specific default pulse count instead of inheriting the previous mode's value.
    let default_pulses = match mode {
        Mode::Cellular => 8,
        Mode::Eulerian => 2,
        Mode::Sieve => 1,
        Mode::PebbleBox => 4,
        _ => 5,
    };
    if p.pulses.value() == 0 { set(c, &p.pulses, default_pulses.min(max_pulses)); }
    // Mode-specific default density/phase.
    let default_density = match mode {
        Mode::Bernoulli | Mode::Cantor => 0.3,
        Mode::Logistic | Mode::ReactionDiffusion => 0.7,
        _ => 0.65,
    };
    let default_phase = match mode {
        Mode::Phyllotaxis => 0.0,
        Mode::PhaseShift => 0.25,
        _ => 0.0,
    };
    if (p.density.value() - 0.65).abs() < 1e-4 { set(c, &p.density, default_density); }
    if p.phase.value().abs() < 1e-4 { set(c, &p.phase, default_phase); }
    // Long-orbit/fractal modes start above the default 64-step ceiling; the user can lower it.
    let wants_long = matches!(
        mode,
        Mode::Lorenz
            | Mode::ReactionDiffusion
            | Mode::Cantor
            | Mode::Fibonacci
            | Mode::Genetic
            | Mode::Kuramoto
            | Mode::Phyllotaxis
            | Mode::Arnold
            | Mode::Hilbert
            | Mode::Ulam
            | Mode::ThueMorse
            | Mode::RudinShapiro
            | Mode::Brownian
            | Mode::LevyFlight
            | Mode::TentMap
            | Mode::HenonMap
    );
    if wants_long && p.max_steps.value() < 256 {
        set(c, &p.max_steps, 256);
    }
}

/// Editor-local undo stack for manual step edits. Host parameter changes are deliberately not
/// part of this history; they remain owned by the host's own undo and automation.
#[derive(Default)]
struct EditHistory {
    undo: Vec<EditState>,
    redo: Vec<EditState>,
}

impl EditHistory {
    const LIMIT: usize = 64;

    fn record(&mut self, state: EditState) {
        if self.undo.last().map(|last| &last.cells) == Some(&state.cells) { return; }
        self.undo.push(state);
        if self.undo.len() > Self::LIMIT { self.undo.remove(0); }
        self.redo.clear();
    }

    fn undo(&mut self, current: EditState) -> Option<EditState> {
        let previous = self.undo.pop()?;
        self.redo.push(current);
        Some(previous)
    }

    fn redo(&mut self, current: EditState) -> Option<EditState> {
        let next = self.redo.pop()?;
        self.undo.push(current);
        Some(next)
    }
}

struct Cache {
    gen_key: Option<GenParams>,
    pattern: Pattern,
    view_key: Option<(GenParams, i32, u8, [u32; TRACKS])>,
}
fn enabled_mask(p: &DrumParams) -> u8 {
    (0..TRACKS).fold(0u8, |m, i| m | (u8::from(p.track_enabled(i)) << i))
}
fn velocity_key(p: &DrumParams) -> [u32; TRACKS] {
    std::array::from_fn(|i| p.track_velocity(i).to_bits())
}
fn rgb(c: [u8; 3]) -> slint::Color { slint::Color::from_rgb_u8(c[0], c[1], c[2]) }

/// One modulation drop target's window-space rectangle, reported by the Slint controls.
#[derive(Clone, Copy)]
struct ModRect {
    target: i32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    seq: u32,
}

/// Newest visible control under the pointer; registration order resolves overlaps after a reflow.
fn mod_target_at(rects: &[ModRect], x: f32, y: f32) -> Option<i32> {
    rects
        .iter()
        .filter(|rect| x >= rect.x && x <= rect.x + rect.w && y >= rect.y && y <= rect.y + rect.h)
        .max_by_key(|rect| rect.seq)
        .map(|rect| rect.target)
}

fn lfo_source_label(p: &DrumParams, slot: usize) -> String {
    let spec = p.lfo_spec(slot);
    if !spec.enabled {
        return format!("LFO {} · off", slot + 1);
    }
    let wave = <OscWave as Enum>::variants()[spec.wave.to_index()];
    let speed = match spec.rate_mode {
        LfoRateMode::Hz => crate::format_hz(spec.freq_hz),
        LfoRateMode::Beats => format!("{:.2} beats", spec.rate_beats),
    };
    let polarity = match spec.polarity { LfoPolarity::Bipolar => "±", LfoPolarity::Unipolar => "+" };
    format!("LFO {} · {wave} · {speed} · {polarity}", slot + 1)
}

fn matrix_rows(p: &DrumParams, clock: LfoClock) -> Vec<ModRow> {
    let signals = p.lfo_signals(clock);
    let mut rows = Vec::new();
    for slot in 0..LFO_COUNT {
        let mask = p.lfo_link_mask(slot);
        if mask == 0 { continue; }
        let source = lfo_source_label(p, slot);
        for target in ModTarget::ALL {
            if !target_in_mask(mask, target) { continue; }
            let base = p.target_value(target, &[0.0; LFO_COUNT]);
            let value = p.target_value(target, &signals);
            rows.push(ModRow {
                lfo: slot as i32 + 1,
                target_id: target.index() as i32,
                source: SharedString::from(source.as_str()),
                target: SharedString::from(target.label()),
                base: SharedString::from(format_target_value(target, base)),
                value: SharedString::from(format_target_value(target, value)),
                delta: SharedString::from(format!("{:+.2}", value - base)),
            });
        }
    }
    rows
}

/// Per-target overlay data: base/effective values, depth bands and the hover summary.
fn target_views(p: &DrumParams, clock: LfoClock) -> Vec<ModTargetView> {
    let signals = p.lfo_signals(clock);
    ModTarget::ALL
        .iter()
        .map(|target| {
            let target = *target;
            let base = p.target_value(target, &[0.0; LFO_COUNT]);
            let value = p.target_value(target, &signals);
            let (min, max) = p.target_range(target);
            let span = (max - min).max(1e-9);
            let base01 = ((base - min) / span).clamp(0.0, 1.0);
            let value01 = ((value - min) / span).clamp(0.0, 1.0);
            let mut parts = Vec::new();
            let mut depth_up = 0.0f32;
            let mut depth_down = 0.0f32;
            for slot in 0..LFO_COUNT {
                if !target_in_mask(p.lfo_link_mask(slot), target) { continue; }
                let spec = p.lfo_spec(slot);
                if !spec.enabled {
                    parts.push(format!("LFO{} off", slot + 1));
                    continue;
                }
                let depth = spec.depth.clamp(0.0, 1.0) * 0.5;
                depth_up += depth;
                if spec.polarity == LfoPolarity::Bipolar { depth_down += depth; }
                let speed = match spec.rate_mode {
                    LfoRateMode::Hz if spec.freq_hz >= 10.0 => format!("{:.0}Hz", spec.freq_hz),
                    LfoRateMode::Hz if spec.freq_hz >= 1.0 => format!("{:.1}Hz", spec.freq_hz),
                    LfoRateMode::Hz => format!("{:.2}Hz", spec.freq_hz),
                    LfoRateMode::Beats => format!("{:.1}b", spec.rate_beats),
                };
                let percent = signals[slot] * 50.0;
                parts.push(format!(
                    "LFO{} {speed} {}{:.0}%",
                    slot + 1,
                    if percent >= 0.0 { "+" } else { "" },
                    percent
                ));
            }
            ModTargetView {
                target: target.index() as i32,
                linked: !parts.is_empty(),
                base,
                value,
                min,
                max,
                base01,
                value01,
                depth_up: depth_up.min(1.0),
                depth_down: depth_down.min(1.0),
                lfo_text: SharedString::from(parts.join(" · ")),
                count: parts.len() as i32,
            }
        })
        .collect()
}

fn lfo_dot_y(p: &DrumParams, slot: usize, phase: f32) -> f32 {
    let wave = p.lfo_spec(slot).wave;
    4.0 + (1.0 - crate::sequence::oscillator_value(wave.to_index(), phase)) * 32.0
}

static RANDOMIZE_NONCE: AtomicU32 = AtomicU32::new(1);
static MOD_REG_SEQ: AtomicU32 = AtomicU32::new(1);

/// True when the RANDOMIZE scope allows this group bit.
fn scope_enabled(p: &DrumParams, bit: i32) -> bool { p.randomize_scope.value() & bit != 0 }

/// Randomize through nice-plug's parameter setter so host, UI and generator agree.
fn randomize_parameters(p: &DrumParams, ctx: &Arc<dyn GuiContext>) {
    let nonce = RANDOMIZE_NONCE.fetch_add(1, Relaxed);
    let variation = p.variation.fetch_add(1, Relaxed).wrapping_add(nonce);
    let mut rng = Rng::new(p.seed.value(), variation, 0xA1_60_D2_0A_5EED);

    // Draw the whole set in fixed order, then apply only allowed groups: enabling a group must
    // not change what another group received. Bounds follow the mode and length that survive.
    let mode = Mode::from_index(rng.below(<Mode as Enum>::variants().len()));
    let ceiling = p.step_ceiling();
    let cells = 8 + rng.below(ceiling - 7); // 8..=ceiling
    // Keep randomize on the grid; fractional spacing stays a deliberate user choice.
    let steps = cells as f32;
    // Bounds follow the values in force after the scope, so written counts stay in range.
    let mode_after = if scope_enabled(p, scope::MODE) { mode } else { p.mode.value() };
    let cells_after = if scope_enabled(p, scope::LENGTH) { cells } else { p.cell_count().max(1) as usize };
    let pulse_limit = match mode_after {
        Mode::Cellular => 15,
        Mode::Sieve => 2,
        Mode::Hilbert => 5,
        _ => cells_after,
    };
    let pulses = rng.below(pulse_limit + 1) as i32;
    let rotate = rng.below(cells_after) as i32;
    let density = 0.15 + 0.85 * rng.unit();
    let seed = rng.unit();
    let phase = rng.unit();
    let osc_rate = 0.125 + (8.0 - 0.125) * rng.unit();
    let wave = OscWave::from_index(rng.below(crate::sequence::oscillator_wave_count));
    let ca_rule = rng.below(256) as i32;
    let symmetry = 1 + rng.below(8) as i32;
    let probability = 0.35 + 0.65 * rng.unit();
    let velocity = 0.45 + 0.55 * rng.unit();
    let rate = Rate::from_index(rng.below(14));
    // Leave swing alone: it is a feel setting, not a generator setting.

    if scope_enabled(p, scope::MODE) { set(ctx, &p.mode, mode); }
    if scope_enabled(p, scope::LENGTH) { set(ctx, &p.steps, steps); }
    if scope_enabled(p, scope::RHYTHM) {
        set(ctx, &p.pulses, pulses);
        set(ctx, &p.rotate, rotate);
        set(ctx, &p.density, density.clamp(0.0, 1.0));
    }
    if scope_enabled(p, scope::TONE) { set(ctx, &p.seed, seed); }
    if scope_enabled(p, scope::PHASE) { set(ctx, &p.phase, phase); }
    if scope_enabled(p, scope::ACID) {
        set(ctx, &p.osc_rate, osc_rate);
        set(ctx, &p.osc_wave, wave);
        set(ctx, &p.ca_rule, ca_rule);
        set(ctx, &p.symmetry, symmetry);
    }
    if scope_enabled(p, scope::CLOCK) { set(ctx, &p.rate, rate); }
    if scope_enabled(p, scope::FEEL) {
        set(ctx, &p.probability, probability.clamp(0.0, 1.0));
        set(ctx, &p.velocity, velocity.clamp(0.01, 1.0));
    }
}

/// Write one generated loop to `folder` and return the file path. Editor thread only.
fn write_midi_file(p: &DrumParams, bpm: f64, folder: &str) -> Result<PathBuf, String> {
    let folder = folder.trim();
    if folder.is_empty() {
        return Err("set an export folder first".to_string());
    }
    let folder = PathBuf::from(folder);
    std::fs::create_dir_all(&folder)
        .map_err(|e| format!("create {}: {e}", folder.display()))?;

    let gen = p.gen_params();
    let mut edits = EditView::default();
    p.edit_overlay.fill(&mut edits);
    let pattern = generate_with(&gen, &edits);
    let lanes = std::array::from_fn(|tr| midi_export::Lane {
        enabled: p.track_enabled(tr),
        note: p.track_note(tr),
        velocity: p.track_velocity(tr),
    });
    let settings = midi_export::ExportSettings {
        bpm,
        steps_per_beat: p.rate.value().steps_per_beat(),
        loop_steps: gen.length as f64,
        gate: p.gate.value() as f64,
        swing: p.swing.value() as f64,
        velocity: p.velocity.value(),
        offsets: std::array::from_fn(|step| edits.offset_at(0, step).max(0.0)),
        lanes,
    };
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let path = folder.join(format!("nrum-seq-{stamp}.mid"));
    std::fs::write(&path, midi_export::smf(&pattern, &settings))
        .map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(path)
}

fn open_folder(path: &Path) {
    let opener = if cfg!(target_os = "windows") {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let _ = std::process::Command::new(opener).arg(path).spawn();
}

fn push_view(ui: &AppWindow, v: &View) {
    // Update rows in place when the count is unchanged: replacing the model resets the repeater
    // and breaks Pebble Box's `animate x, y`.
    let model = ui.get_view_nodes();
    if model.row_count() == v.nodes.len() {
        for (i, n) in v.nodes.iter().enumerate() {
            let node = ViewNode {
                x: n.x, y: n.y, r: n.r, on: n.on, cur: n.cur, label: SharedString::from(n.label.as_str()),
            };
            if model.row_data(i).as_ref() != Some(&node) { model.set_row_data(i, node); }
        }
    } else {
        ui.set_view_nodes(ModelRc::new(VecModel::from(v.nodes.iter().map(|n| ViewNode {
            x: n.x, y: n.y, r: n.r, on: n.on, cur: n.cur, label: SharedString::from(n.label.as_str()),
        }).collect::<Vec<_>>())));
    }
    ui.set_view_edges(SharedString::from(v.edges.as_str()));
    ui.set_view_lines(SharedString::from(v.lines.as_str()));
    ui.set_view_hand(SharedString::from(v.hand.as_str()));
}

impl Editor for SlintEditor {
    fn spawn(&self, parent: ParentWindowHandle, context: Arc<dyn GuiContext>) -> Box<dyn std::any::Any + Send> {
        // Load user-global preferences before showing project state so the theme follows the user.
        let stored = globals::load();
        // A previous editor instance may have left a key or a focus flag behind.
        self.shared.text_focus.store(false, Relaxed);
        if let Ok(mut queue) = self.shared.key_queue.lock() { queue.clear(); }
        let scale = stored.scale.filter(|s| *s > 0.0).unwrap_or_else(|| f32::from_bits(self.scale.load(Relaxed)));
        // No host API for the project path; default to the working directory, which the user can edit.
        let midi_dir = Arc::new(Mutex::new(
            stored
                .midi_dir
                .clone()
                .filter(|d| !d.trim().is_empty())
                .unwrap_or_else(|| {
                    std::env::current_dir()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default()
                }),
        ));
        let setup_midi_dir = midi_dir.clone();
        let update_midi_dir = midi_dir.clone();
        let mod_rects = Arc::new(Mutex::new(Vec::<ModRect>::new()));
        let edit_history = Arc::new(Mutex::new(EditHistory::default()));
        let edit_history_step = edit_history.clone();
        let edit_history_clear = edit_history.clone();
        let edit_history_undo = edit_history.clone();
        let edit_history_redo = edit_history.clone();
        let setup_mod_rects = mod_rects.clone();
        // Keep the host factor even when `set_scale_factor` is never called; the close path reads it.
        if scale > 0.0 && self.scale.load(Relaxed) == 0 { self.scale.store(scale.to_bits(), Relaxed); }
        if let Some(theme) = stored.theme {
            // `from_index` falls back to the first variant, so a corrupt file cannot panic.
            let theme = PluginTheme::from_index(theme);
            let setter = ParamSetter::new(context.as_ref());
            setter.begin_set_parameter(&self.params.theme);
            setter.set_parameter(&self.params.theme, theme);
            setter.end_set_parameter(&self.params.theme);
        }
        let options = WindowOpenOptions {
            title: "nrum seq".to_string(), size: Size::new(WIDTH as f64, HEIGHT as f64),
            scale: if scale > 0.0 { WindowScalePolicy::ScaleFactor(scale as f64) } else { WindowScalePolicy::SystemScaleFactor },
            gl_config: Some(default_gl_config()),
        };
        let (setup_params, setup_ctx) = (self.params.clone(), context.clone());
        let setup_shared = self.shared.clone();
        let setup_scale = self.scale.clone();
        // `spawn` returns before closures run and they must be `'static`, so clone shared state here.
        let setup = move |h: &slint_baseview::SlintWindowHandler<AppWindow>, _w: &mut slint_baseview::Window| {
            let ui = h.component();
            let folder = setup_midi_dir.lock().map(|d| d.clone()).unwrap_or_default();
            ui.set_midi_folder(SharedString::from(folder.as_str()));
            {
                let (p, c) = (setup_params.clone(), setup_ctx.clone());
                ui.on_set_mode(move |v| apply_mode(&c, &p, v));
            }
            {
                let (p, c) = (setup_params.clone(), setup_ctx.clone());
                ui.on_step_mode(move |delta| {
                    // Step the order the list is drawn in, not the raw host index.
                    apply_mode(&c, &p, p.mode.value().step_in_display_order(delta).to_index() as i32);
                });
            }
            {
                let (p, c) = (setup_params.clone(), setup_ctx.clone());
                ui.on_set_rate(move |v| set(&c, &p.rate, Rate::from_index(v.clamp(0, 13) as usize)));
            }
            {
                let (p, c) = (setup_params.clone(), setup_ctx.clone());
                ui.on_apply_preset(move |index| apply_preset(&c, &p, index));
            }
            {
                let (p, c) = (setup_params.clone(), setup_ctx.clone());
                ui.on_set_markov_bias(move |v| set(&c, &p.markov_bias, (v.round() as i32).clamp(0, 255)));
                let (p, c) = (setup_params.clone(), setup_ctx.clone());
                ui.on_set_osc_wave(move |v| {
                    set(&c, &p.osc_wave, OscWave::from_index(v.clamp(0, OscWave::variants().len() as i32 - 1) as usize));
                });
            }
            {
                let (p, c) = (setup_params.clone(), setup_ctx.clone());
                ui.on_set_lfo_enabled(move |slot, enabled| {
                    set(&c, lfo_enabled_param(&p, slot), enabled);
                });
                let (p, c) = (setup_params.clone(), setup_ctx.clone());
                ui.on_set_lfo_wave(move |slot, v| {
                    let wave = OscWave::from_index(v.clamp(0, OscWave::variants().len() as i32 - 1) as usize);
                    set(&c, lfo_wave_param(&p, slot), wave);
                });
                let (p, c) = (setup_params.clone(), setup_ctx.clone());
                ui.on_set_lfo_rate_mode(move |slot, v| {
                    let mode = LfoRateMode::from_index(v.clamp(0, 1) as usize);
                    set(&c, lfo_rate_mode_param(&p, slot), mode);
                });
                let (p, c) = (setup_params.clone(), setup_ctx.clone());
                ui.on_set_lfo_speed(move |slot, value| {
                    match lfo_rate_mode_param(&p, slot).value() {
                        LfoRateMode::Hz => set(&c, lfo_freq_param(&p, slot), value.clamp(0.01, LFO_MAX_HZ)),
                        LfoRateMode::Beats => set(&c, lfo_beat_param(&p, slot), value.clamp(0.25, 16.0)),
                    }
                });
                let (p, c) = (setup_params.clone(), setup_ctx.clone());
                ui.on_set_lfo_depth(move |slot, value| set(&c, lfo_depth_param(&p, slot), value.clamp(0.0, 1.0)));
                let (p, c) = (setup_params.clone(), setup_ctx.clone());
                ui.on_set_lfo_phase(move |slot, value| set(&c, lfo_phase_param(&p, slot), value.clamp(0.0, 1.0)));
                let (p, c) = (setup_params.clone(), setup_ctx.clone());
                ui.on_set_lfo_polarity(move |slot, v| {
                    let polarity = LfoPolarity::from_index(v.clamp(0, 1) as usize);
                    set(&c, lfo_polarity_param(&p, slot), polarity);
                });
            }
            {
                let rects = setup_mod_rects.clone();
                ui.global::<ModRegistry>().on_register(move |target, visible, x, y, w, h| {
                    if target < 0 { return; }
                    let mut rects = rects.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                    rects.retain(|rect| rect.target != target);
                    if visible {
                        rects.push(ModRect { target, x, y, w, h, seq: MOD_REG_SEQ.fetch_add(1, Relaxed) });
                    }
                });
            }
            {
                let (rects, p, weak) = (setup_mod_rects.clone(), setup_params.clone(), ui.as_weak());
                ui.global::<ModDrag>().on_dropped(move |lfo, x, y| {
                    let target = { let rects = rects.lock().unwrap_or_else(|poisoned| poisoned.into_inner()); mod_target_at(&rects, x, y) };
                    let Some(target) = target.and_then(|i| ModTarget::from_index(i.max(0) as usize)) else { return; };
                    let slot = lfo.round().clamp(1.0, 4.0) as usize;
                    let linked = target_in_mask(p.lfo_link_mask(slot - 1), target);
                    p.set_lfo_link(slot - 1, target, !linked);
                    if let Some(ui) = weak.upgrade() {
                        ui.set_mod_status(SharedString::from(format!(
                            "LFO {slot} → {} {}",
                            target.label(),
                            if linked { "unlinked" } else { "linked" }
                        )));
                    }
                });
            }
            {
                let (p, weak) = (setup_params.clone(), ui.as_weak());
                ui.on_delete_mod(move |lfo, target_index| {
                    let slot = lfo.clamp(1, 4) as usize;
                    let Some(target) = ModTarget::from_index(target_index.max(0) as usize) else { return; };
                    p.set_lfo_link(slot - 1, target, false);
                    if let Some(ui) = weak.upgrade() {
                        ui.set_mod_status(SharedString::from(format!("LFO {slot} → {} removed", target.label())));
                    }
                });
            }
            {
                let (p, c) = (setup_params.clone(), setup_ctx.clone());
                ui.on_toggle_scope(move |bit| {
                    let mask = p.randomize_scope.value() ^ (1 << bit.clamp(0, 7));
                    set(&c, &p.randomize_scope, mask);
                });
            }
            {
                let (p, c) = (setup_params.clone(), setup_ctx.clone());
                ui.on_set_theme(move |v| set(&c, &p.theme, PluginTheme::from_index(v.max(0) as usize)));
            }
            {
                let (p, c) = (setup_params.clone(), setup_ctx.clone());
                ui.on_set_opt(move |key, value| set_opt(&c, &p, key, value));
            }
            {
                let (p, c) = (setup_params.clone(), setup_ctx.clone());
                ui.on_toggle_track(move |index| {
                    match index {
                        0 => set(&c, &p.kick_on, !p.kick_on.value()),
                        1 => set(&c, &p.snare_on, !p.snare_on.value()),
                        2 => set(&c, &p.closed_hat_on, !p.closed_hat_on.value()),
                        3 => set(&c, &p.open_hat_on, !p.open_hat_on.value()),
                        4 => set(&c, &p.clap_on, !p.clap_on.value()),
                        5 => set(&c, &p.rim_on, !p.rim_on.value()),
                        6 => set(&c, &p.low_tom_on, !p.low_tom_on.value()),
                        7 => set(&c, &p.perc_on, !p.perc_on.value()),
                        _ => {},
                    }
                });
            }
            {
                let (p, c) = (setup_params.clone(), setup_ctx.clone());
                ui.on_set_lane_mode(move |lane, value| {
                    let lane = lane.clamp(0, TRACKS as i32 - 1);
                    set(&c, lane_mode_param(&p, lane), value.clamp(0, 39));
                });
            }
            {
                let (p, history) = (setup_params.clone(), edit_history_step.clone());
                ui.on_edit_step(move |track, step, mode| {
                    let lane = track.clamp(0, TRACKS as i32 - 1) as usize;
                    let last = (p.cell_count().max(1) - 1) as i32;
                    let step = step.clamp(0, last) as usize;
                    if let Ok(mut history) = history.lock() {
                        history.record(p.edit_overlay.snapshot());
                    }
                    let store = &p.edit_overlay;
                    match mode {
                        1 => store.set_hit(lane, step, match store.hit(lane, step) {
                            0 => 1,
                            1 => 2,
                            _ => 0,
                        }),
                        2 => store.set_probability(lane, step, match store.probability(lane, step) {
                            255 => 254,
                            254 => 191,
                            191 => 127,
                            127 => 64,
                            _ => 255,
                        }),
                        3 => store.set_accent(lane, step, match store.accent(lane, step) {
                            0 => 255,
                            255 => 191,
                            191 => 128,
                            _ => 0,
                        }),
                        4 => store.clear_cell(lane, step),
                        5 => store.set_ratchet(lane, step, match store.ratchet(lane, step) {
                            1 => 2,
                            2 => 3,
                            3 => 4,
                            _ => 1,
                        }),
                        6 => {
                            // Microtiming is step-wide; write the same offset to every lane so a
                            // later scheduler can stay one-event-per-step.
                            let next = match store.offset(0, step).max(0) {
                                0 => 16,
                                16 => 32,
                                32 => 64,
                                _ => 0,
                            };
                            for tr in 0..TRACKS { store.set_offset(tr, step, next); }
                        }
                        _ => {}
                    }
                });
            }
            {
                let (p, history) = (setup_params.clone(), edit_history_clear.clone());
                ui.on_clear_edits(move || {
                    if let Ok(mut history) = history.lock() {
                        history.record(p.edit_overlay.snapshot());
                    }
                    p.edit_overlay.clear_all();
                });
            }
            {
                let (p, history) = (setup_params.clone(), edit_history_undo.clone());
                ui.on_undo_edit(move || {
                    let current = p.edit_overlay.snapshot();
                    if let Ok(mut history) = history.lock() {
                        if let Some(previous) = history.undo(current) {
                            p.edit_overlay.apply(&previous);
                        }
                    }
                });
            }
            {
                let (p, history) = (setup_params.clone(), edit_history_redo.clone());
                ui.on_redo_edit(move || {
                    let current = p.edit_overlay.snapshot();
                    if let Ok(mut history) = history.lock() {
                        if let Some(next) = history.redo(current) {
                            p.edit_overlay.apply(&next);
                        }
                    }
                });
            }
            {
                let (p, c) = (setup_params.clone(), setup_ctx.clone());
                ui.on_toggle_free_run(move || set(&c, &p.free_run, !p.free_run.value()));
                let (p, c) = (setup_params.clone(), setup_ctx.clone());
                ui.on_toggle_invert(move || set(&c, &p.invert, !p.invert.value()));
                let (p, c) = (setup_params.clone(), setup_ctx.clone());
                ui.on_toggle_mirror(move || set(&c, &p.mirror, !p.mirror.value()));
                let (p, c) = (setup_params.clone(), setup_ctx.clone());
                ui.on_toggle_logging(move || set(&c, &p.logging, !p.logging.value()));
            }
            {
                let (p, c) = (setup_params.clone(), setup_ctx.clone());
                ui.on_generate_pressed(move || set(&c, &p.generate, !p.generate.value()));
            }
            {
                let (p, c) = (setup_params.clone(), setup_ctx.clone());
                ui.on_randomize_pressed(move || randomize_parameters(&p, &c));
            }
            {
                let (p, c) = (setup_params.clone(), setup_ctx.clone());
                ui.on_toggle_round_steps(move || set(&c, &p.round_steps, !p.round_steps.value()));
            }
            {
                let (p, dir, scale) = (setup_params.clone(), setup_midi_dir.clone(), setup_scale.clone());
                ui.on_set_midi_folder(move |value| {
                    let text = value.trim().to_string();
                    if let Ok(mut current) = dir.lock() {
                        *current = text;
                    }
                    globals::save(&global_settings(
                        &p,
                        f32::from_bits(scale.load(Relaxed)),
                        &dir,
                    ));
                });
            }
            {
                let (p, shared, dir) = (setup_params.clone(), setup_shared.clone(), setup_midi_dir.clone());
                let weak = ui.as_weak();
                ui.on_export_midi(move || {
                    let folder = dir.lock().map(|d| d.clone()).unwrap_or_default();
                    let bpm = match shared.tempo_bits.load(Relaxed) {
                        0 => 120.0,
                        bits => f32::from_bits(bits) as f64,
                    };
                    let status = match write_midi_file(&p, bpm, &folder) {
                        Ok(path) => {
                            open_folder(Path::new(&folder));
                            let name = path
                                .file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_else(|| path.display().to_string());
                            format!("Exported {name}")
                        }
                        Err(e) => format!("Export failed: {e}"),
                    };
                    if let Some(ui) = weak.upgrade() {
                        ui.set_export_status(SharedString::from(status));
                    }
                });
            }
            // Waveform names and contact details come from Rust, so they cannot drift from metadata.
            ui.set_wave_names(ModelRc::new(VecModel::from(
                OscWave::variants().iter().map(|name| SharedString::from(*name)).collect::<Vec<_>>(),
            )));
            ui.set_wave_paths(ModelRc::new(VecModel::from(
                view::wave_icon_paths().into_iter().map(SharedString::from).collect::<Vec<_>>(),
            )));
            ui.set_mod_rows(ModelRc::new(VecModel::from(Vec::<ModRow>::new())));
            ui.set_mod_status(SharedString::new());
            ui.set_rate_names(ModelRc::new(VecModel::from(
                Rate::variants().iter().map(|name| SharedString::from(*name)).collect::<Vec<_>>(),
            )));
            ui.set_preset_names(ModelRc::new(VecModel::from(
                PRESETS.iter().map(|preset| SharedString::from(preset.name)).collect::<Vec<_>>(),
            )));
            ui.set_version(SharedString::from(env!("CARGO_PKG_VERSION")));
            // One grouped order shared with the header wheel; names are pushed from the enum.
            ui.set_mode_names(ModelRc::new(VecModel::from(
                Mode::variants().iter().map(|name| SharedString::from(*name)).collect::<Vec<_>>(),
            )));
            let mut entries: Vec<ModeEntry> = Vec::new();
            for (g, modes) in crate::modes::GROUPS {
                entries.push(ModeEntry { group: SharedString::from(*g), index: -1, header: true });
                for mode in *modes {
                    entries.push(ModeEntry { group: SharedString::from(*g), index: mode.to_index() as i32, header: false });
                }
            }
            ui.set_mode_entries(ModelRc::new(VecModel::from(entries.clone())));
            // Build paired rows so the two columns stay on the same height/row.
            let mut rows: Vec<ModeRow> = Vec::new();
            let mut i = 0;
            while i < entries.len() {
                if entries[i].header {
                    rows.push(ModeRow { header: true, group: entries[i].group.clone(), left: -1, right: -1, has_right: false });
                    i += 1;
                    let mut items: Vec<i32> = Vec::new();
                    while i < entries.len() && !entries[i].header {
                        items.push(entries[i].index);
                        i += 1;
                    }
                    let mut j = 0;
                    while j < items.len() {
                        let left = items[j];
                        let right = if j + 1 < items.len() { items[j + 1] } else { -1 };
                        let has_right = j + 1 < items.len();
                        rows.push(ModeRow { header: false, group: SharedString::default(), left, right, has_right });
                        j += 2;
                    }
                } else {
                    i += 1;
                }
            }
            ui.set_mode_rows(ModelRc::new(VecModel::from(rows)));
            ui.set_settings_path(SharedString::from(crate::globals::config_path_display()));
            ui.set_credit_author(SharedString::from(crate::contact::AUTHOR));
            ui.set_credit_github(SharedString::from(crate::contact::GITHUB));
            ui.set_credit_support(SharedString::from(crate::contact::SUPPORT));
            ui.set_credit_email(SharedString::from(crate::contact::EMAIL));
        };

        let cache = Arc::new(Mutex::new(Cache { gen_key: None, pattern: Pattern::default(), view_key: None }));
        let (update_params, update_shared, update_cache, update_context) = (self.params.clone(), self.shared.clone(), cache, context.clone());
        // The update closure outlives `&self`, so it reads the scale through the atomic.
        let update_scale_cell = self.scale.clone();
        let last_randomize_request = AtomicU32::new(update_shared.randomize_request.load(Relaxed));
        // `u32::MAX` forces the first tick to publish whatever track name is already known.
        let last_track_rev = AtomicU32::new(u32::MAX);
        let update = move |h: &slint_baseview::SlintWindowHandler<AppWindow>, _window: &mut slint_baseview::Window| {
            let requested = update_shared.randomize_request.load(Relaxed);
            let previous = last_randomize_request.swap(requested, Relaxed);
            let pending = requested.wrapping_sub(previous).min(16);
            if pending > 0 {
                for _ in 0..pending {
                    randomize_parameters(&update_params, &update_context);
                }
            }
            if update_shared.regen_pulse.swap(false, Relaxed) {
                h.component().set_regen_pulse(true);
            }
            let p = &update_params;
            let ui = h.component();
            // Replay editing keys the host would otherwise have kept for itself.
            let forwarded = update_shared.key_queue.lock().map(|mut q| std::mem::take(&mut *q)).unwrap_or_default();
            for (key, down) in forwarded {
                let text: SharedString = key.slint_key().into();
                h.window().dispatch_event(if down { WindowEvent::KeyPressed { text } } else { WindowEvent::KeyReleased { text } });
            }
            let track_rev = update_shared.track_rev.load(Relaxed);
            if last_track_rev.swap(track_rev, Relaxed) != track_rev {
                let name = update_shared.track_name.lock().map(|n| n.clone()).unwrap_or_default();
                ui.set_track_name(SharedString::from(name.as_str()));
            }
            ui.set_mode(p.mode.value() as i32);
            {
                let lane_modes = [
                    p.lane1_mode.value().clamp(0, 39),
                    p.lane2_mode.value().clamp(0, 39),
                    p.lane3_mode.value().clamp(0, 39),
                    p.lane4_mode.value().clamp(0, 39),
                    p.lane5_mode.value().clamp(0, 39),
                    p.lane6_mode.value().clamp(0, 39),
                    p.lane7_mode.value().clamp(0, 39),
                    p.lane8_mode.value().clamp(0, 39),
                ];
                let model = ui.get_lane_modes();
                if model.row_count() == lane_modes.len() {
                    for (i, value) in lane_modes.iter().enumerate() {
                        if model.row_data(i) != Some(*value) { model.set_row_data(i, *value); }
                    }
                } else {
                    ui.set_lane_modes(ModelRc::new(VecModel::from(lane_modes.to_vec())));
                }
            }
            let selected_theme = p.theme.value();
            let theme_index = selected_theme.to_index() as i32;
            let update_scale = f32::from_bits(update_scale_cell.load(Relaxed));
            if ui.get_theme_index() != theme_index {
                let colors = selected_theme.colors();
                let palette = ui.global::<Theme>();
                palette.set_c1(rgb(colors[0])); palette.set_c2(rgb(colors[1])); palette.set_c3(rgb(colors[2]));
                palette.set_hi(rgb(colors[3])); palette.set_bg1(rgb(colors[4])); palette.set_bg2(rgb(colors[5])); palette.set_bg3(rgb(colors[6]));
                palette.set_surface(rgb(colors[7])); palette.set_control(rgb(colors[8])); palette.set_control_hover(rgb(colors[9]));
                palette.set_text(rgb(colors[10])); palette.set_text_muted(rgb(colors[11])); palette.set_border(rgb(colors[12]));
                palette.set_cell_off(rgb(colors[13])); palette.set_on_accent(rgb(colors[14]));
                ui.set_theme_index(theme_index);
                // Theme is user-global; persist it outside the project as it changes.
                globals::save(&global_settings(p, update_scale, &update_midi_dir));
            }
            let ceiling = p.step_ceiling() as f32;
            ui.set_max_steps(ceiling);
            // Show the Steps value the generator will use, even if the project stored one above the ceiling.
            ui.set_steps(p.steps.value().min(ceiling));
            ui.set_round_steps(p.round_steps.value());
            ui.set_pulses(p.pulses.value() as f32);
            ui.set_rotate(p.rotate.value() as f32);
            ui.set_density(p.density.value());
            ui.set_probability(p.probability.value());
            ui.set_velocity(p.velocity.value());
            ui.set_gate(p.gate.value());
            ui.set_regenerate_every(p.regenerate_every.value() as f32);
            ui.set_randomize_after_lengths(p.randomize_after_lengths.value() as f32);
            ui.set_swing(p.swing.value());
            ui.set_scope_mode(scope_enabled(p, scope::MODE));
            ui.set_scope_length(scope_enabled(p, scope::LENGTH));
            ui.set_scope_rhythm(scope_enabled(p, scope::RHYTHM));
            ui.set_scope_tone(scope_enabled(p, scope::TONE));
            ui.set_scope_clock(scope_enabled(p, scope::CLOCK));
            ui.set_scope_phase(scope_enabled(p, scope::PHASE));
            ui.set_scope_acid(scope_enabled(p, scope::ACID));
            ui.set_scope_feel(scope_enabled(p, scope::FEEL));
            ui.set_seed(p.seed.value());
            ui.set_phase(p.phase.value());
            ui.set_osc_rate(p.osc_rate.value());
            ui.set_osc_wave(p.osc_wave.value().to_index() as i32);
            ui.set_ca_rule(p.ca_rule.value() as f32);
            ui.set_markov_bias(p.markov_bias.value() as f32);
            ui.set_symmetry(p.symmetry.value() as f32);
            ui.set_rate(p.rate.value() as i32);
            let live_clock = update_shared.lfo_clock();
            let views = target_views(p, live_clock);
            let mod_model = ui.global::<ModView>().get_targets();
            if mod_model.row_count() == views.len() {
                for (i, view) in views.iter().enumerate() {
                    if mod_model.row_data(i).as_ref() != Some(view) {
                        mod_model.set_row_data(i, view.clone());
                    }
                }
            } else {
                ui.global::<ModView>().set_targets(ModelRc::new(VecModel::from(views)));
            }
            ui.set_lfo_running(update_shared.lfo_running.load(Relaxed));
            for slot in 0..LFO_COUNT {
                let phase = f32::from_bits(update_shared.lfo_phase[slot].load(Relaxed));
                let dot_y = lfo_dot_y(p, slot, phase);
                match slot {
                    0 => {
                        ui.set_lfo1_enabled(p.lfo1_enabled.value());
                        ui.set_lfo1_wave(p.lfo1_wave.value().to_index() as i32);
                        ui.set_lfo1_rate_mode(p.lfo1_rate_mode.value().to_index() as i32);
                        ui.set_lfo1_freq(p.lfo1_freq.value());
                        ui.set_lfo1_rate(p.lfo1_rate.value());
                        ui.set_lfo1_depth(p.lfo1_depth.value());
                        ui.set_lfo1_phase(p.lfo1_phase.value());
                        ui.set_lfo1_polarity(p.lfo1_polarity.value().to_index() as i32);
                        ui.set_lfo1_anim_phase(phase);
                        ui.set_lfo1_dot_y(dot_y);
                    }
                    1 => {
                        ui.set_lfo2_enabled(p.lfo2_enabled.value());
                        ui.set_lfo2_wave(p.lfo2_wave.value().to_index() as i32);
                        ui.set_lfo2_rate_mode(p.lfo2_rate_mode.value().to_index() as i32);
                        ui.set_lfo2_freq(p.lfo2_freq.value());
                        ui.set_lfo2_rate(p.lfo2_rate.value());
                        ui.set_lfo2_depth(p.lfo2_depth.value());
                        ui.set_lfo2_phase(p.lfo2_phase.value());
                        ui.set_lfo2_polarity(p.lfo2_polarity.value().to_index() as i32);
                        ui.set_lfo2_anim_phase(phase);
                        ui.set_lfo2_dot_y(dot_y);
                    }
                    2 => {
                        ui.set_lfo3_enabled(p.lfo3_enabled.value());
                        ui.set_lfo3_wave(p.lfo3_wave.value().to_index() as i32);
                        ui.set_lfo3_rate_mode(p.lfo3_rate_mode.value().to_index() as i32);
                        ui.set_lfo3_freq(p.lfo3_freq.value());
                        ui.set_lfo3_rate(p.lfo3_rate.value());
                        ui.set_lfo3_depth(p.lfo3_depth.value());
                        ui.set_lfo3_phase(p.lfo3_phase.value());
                        ui.set_lfo3_polarity(p.lfo3_polarity.value().to_index() as i32);
                        ui.set_lfo3_anim_phase(phase);
                        ui.set_lfo3_dot_y(dot_y);
                    }
                    _ => {
                        ui.set_lfo4_enabled(p.lfo4_enabled.value());
                        ui.set_lfo4_wave(p.lfo4_wave.value().to_index() as i32);
                        ui.set_lfo4_rate_mode(p.lfo4_rate_mode.value().to_index() as i32);
                        ui.set_lfo4_freq(p.lfo4_freq.value());
                        ui.set_lfo4_rate(p.lfo4_rate.value());
                        ui.set_lfo4_depth(p.lfo4_depth.value());
                        ui.set_lfo4_phase(p.lfo4_phase.value());
                        ui.set_lfo4_polarity(p.lfo4_polarity.value().to_index() as i32);
                        ui.set_lfo4_anim_phase(phase);
                        ui.set_lfo4_dot_y(dot_y);
                    }
                }
            }
            if ui.get_matrix_open() {
                let rows = matrix_rows(p, live_clock);
                let model = ui.get_mod_rows();
                if model.row_count() == rows.len() {
                    for (i, row) in rows.iter().enumerate() {
                        if model.row_data(i).as_ref() != Some(row) {
                            model.set_row_data(i, row.clone());
                        }
                    }
                } else {
                    ui.set_mod_rows(ModelRc::new(VecModel::from(rows)));
                }
            }
            ui.set_free_run(p.free_run.value());
            ui.set_invert_on(p.invert.value());
            ui.set_mirror_on(p.mirror.value());
            ui.set_logging_on(p.logging.value());
            ui.set_generate_on(p.generate.value());
            let gen_key = p.gen_params_at(update_shared.pattern_clock());
            let cur = update_shared.current_step.load(Relaxed);
            ui.set_current_step(cur);
            let mask = enabled_mask(p);
            let vel_key = velocity_key(p);
            let mut c = update_cache.lock().unwrap_or_else(|poisoned| {
                log_line("WARNING: update cache mutex was poisoned; recovering its inner state");
                poisoned.into_inner()
            });
            if c.gen_key != Some(gen_key) {
                let mut edits = EditView::default();
                p.edit_overlay.fill(&mut edits);
                c.pattern = generate_with(&gen_key, &edits);
                ui.set_mode_math(SharedString::from(crate::help::math(&gen_key)));
                if p.logging.value() {
                    log_line(&format!(
                        "mode={:?} steps={} density={:.2} probability={:.2} seed={:.3} rotate={} invert={} mirror={}",
                        gen_key.kind, gen_key.steps, gen_key.density, gen_key.probability,
                        gen_key.seed, gen_key.rotate, gen_key.invert, gen_key.mirror,
                    ));
                }
                c.gen_key = Some(gen_key);
            }
            // Read after the replayed keys were dispatched, so the host hook sees fresh focus.
            update_shared.text_focus.store(ui.get_text_focused(), Relaxed);
            let view_key = (gen_key, cur, mask, vel_key);
            if c.view_key != Some(view_key) {
                let v = view::build(&gen_key, &c.pattern, cur);
                push_view(&ui, &v);
                const LANE_NAMES: [&str; TRACKS] = ["KICK", "SNARE", "CLOSED HAT", "OPEN HAT", "CLAP", "RIM", "LOW TOM", "PERC"];
                let mut rows = Vec::with_capacity(TRACKS);
                for tr in 0..TRACKS {
                    let enabled = p.track_enabled(tr);
                    let cells = (0..c.pattern.steps).map(|i| {
                        let hit_override = p.edit_overlay.hit(tr, i);
                        let probability_override = p.edit_overlay.probability(tr, i);
                        let edited = hit_override != 0
                            || p.edit_overlay.accent(tr, i) != 0
                            || probability_override != 255
                            || p.edit_overlay.ratchet(tr, i) != 1
                            || p.edit_overlay.offset(tr, i) != 0;
                        StepCell {
                            track: tr as i32,
                            step: i as i32,
                            hit: enabled && c.pattern.hit[tr][i],
                            playing: cur == i as i32,
                            velocity: (p.track_velocity(tr) * gen_key.velocity * c.pattern.accent[tr][i]).clamp(0.0, 1.0),
                            probability: if probability_override == 255 {
                                gen_key.probability
                            } else {
                                probability_override as f32 / 254.0
                            },
                            hit_override: hit_override as i32,
                            edited,
                            ratchet: p.edit_overlay.ratchet(tr, i) as i32,
                            offset: p.edit_overlay.offset(0, i).max(0) as i32,
                        }
                    }).collect::<Vec<_>>();
                    rows.push(TrackRow {
                        index: tr as i32,
                        name: SharedString::from(LANE_NAMES[tr]),
                        enabled,
                        note: p.track_note(tr) as i32,
                        cells: ModelRc::new(VecModel::from(cells)),
                    });
                }
                ui.set_tracks(ModelRc::new(VecModel::from(rows)));
                c.view_key = Some(view_key);
            }
        };
        let factory = move || {
            let ui = AppWindow::new()?;
            Ok(ui)
        };
        // The window never changes size: the editor reports a fixed 860x740 logical size.
        log_line("editor spawn: opening parented Slint/baseview window");
        let window = SlintWindow::open_parented(&parent, options, factory, setup, update, |_, _| {});
        log_line("editor spawn: open_parented returned; native initialization may continue asynchronously");
        Box::new(SlintEditorHandle {
            window,
            params: self.params.clone(),
            shared: self.shared.clone(),
            scale: self.scale.clone(),
            midi_dir,
        })
    }
    fn size(&self) -> (u32, u32) { (WIDTH, HEIGHT) }
    fn set_scale_factor(&self, factor: f32) -> bool {
        self.scale.store(factor.to_bits(), Relaxed);
        // Editor scale is a user preference, not project state: persist it globally.
        let mut s = globals::load();
        s.scale = if factor > 0.0 { Some(factor) } else { None };
        globals::save(&s);
        true
    }
    /// Claim a host-stolen editing key only while a text field has focus; leave host shortcuts alone.
    fn on_virtual_key_from_host(&self, key_code: VirtualKeyCode, is_down: bool, modifiers: Modifiers) -> bool {
        if !self.shared.text_focus.load(Relaxed) { return false; }
        let Some(key) = keys::edit_key(key_code, modifiers) else { return false };
        if let Ok(mut queue) = self.shared.key_queue.lock() {
            // A held key repeats a few dozen times a second; the cap guards a stalled tick.
            if queue.len() < 64 { queue.push((key, is_down)); }
        }
        true
    }
    fn param_value_changed(&self, _id: &str, _normalized_value: f32) {}
    fn param_modulation_changed(&self, _id: &str, _modulation_offset: f32) {}
    fn param_values_changed(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn built_in_presets_are_valid_for_the_current_bounds() {
        assert!(!PRESETS.is_empty());
        for preset in PRESETS {
            assert!(!preset.name.is_empty());
            assert!(preset.steps >= 1.0 && preset.steps <= MAX_STEPS as f32, "{}", preset.name);
            assert!(preset.pulses >= 0 && preset.pulses <= preset.steps as i32, "{}", preset.name);
            assert!((0.0..=1.0).contains(&preset.density), "{}", preset.name);
            assert!((0.0..=1.0).contains(&preset.probability), "{}", preset.name);
            assert!((0.0..=1.0).contains(&preset.seed), "{}", preset.name);
        }
    }

    #[test]
    fn edit_history_undoes_and_redoes_in_order() {
        use crate::edits::EditCell;
        let state = |hit: u8| crate::edits::EditState {
            version: 1,
            cells: vec![EditCell {
                lane: 0, step: 3, hit, accent: 0, probability: 255, ratchet: 1, offset: 0,
            }],
        };
        let mut history = EditHistory::default();
        let first = state(1);
        let second = state(2);
        history.record(first.clone());
        history.record(second.clone());
        let current = state(0);
        assert_eq!(history.undo(current.clone()).unwrap().cells[0].hit, 2);
        assert_eq!(history.undo(second.clone()).unwrap().cells[0].hit, 1);
        assert_eq!(history.redo(first.clone()).unwrap().cells[0].hit, 2);
        assert_eq!(history.redo(second).unwrap().cells[0].hit, 0);
    }

    #[test]
    fn edit_history_does_not_duplicate_an_unchanged_snapshot() {
        use crate::edits::EditCell;
        let state = crate::edits::EditState {
            version: 1,
            cells: vec![EditCell {
                lane: 1, step: 2, hit: 1, accent: 0, probability: 255, ratchet: 1, offset: 0,
            }],
        };
        let mut history = EditHistory::default();
        history.record(state.clone());
        history.record(state);
        assert_eq!(history.undo.len(), 1);
    }

    #[test]
    fn track_mask_covers_eight_lanes() {
        let p = DrumParams::default();
        assert_eq!(enabled_mask(&p) & 0b0000_0111, 0b0000_0111);
    }

    #[test]
    fn modulation_hit_test_prefers_the_newest_overlapping_control() {
        let rects = [
            ModRect { target: 0, x: 0.0, y: 0.0, w: 100.0, h: 20.0, seq: 1 },
            ModRect { target: 3, x: 0.0, y: 0.0, w: 100.0, h: 20.0, seq: 2 },
        ];
        assert_eq!(mod_target_at(&rects, 50.0, 10.0), Some(3));
        assert_eq!(mod_target_at(&rects, 150.0, 10.0), None);
    }

    #[test]
    fn matrix_rows_follow_links_and_show_modulated_values() {
        let p = DrumParams {
            lfo1_links: std::sync::atomic::AtomicU64::new(crate::modulation::target_bit(ModTarget::Steps)),
            lfo1_wave: EnumParam::new("LFO 1 Wave", OscWave::Sine),
            lfo1_rate_mode: EnumParam::new("LFO 1 Rate Mode", LfoRateMode::Hz),
            lfo1_freq: FloatParam::new("LFO 1 Frequency", 0.5, FloatRange::Skewed { min: 0.01, max: LFO_MAX_HZ, factor: 0.25 }),
            lfo1_depth: FloatParam::new("LFO 1 Depth", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            ..DrumParams::default()
        };
        let rows = matrix_rows(&p, LfoClock { beat: 0.5, seconds: 0.5 });
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].target.as_str(), "Steps");
        assert_eq!(rows[0].target_id, ModTarget::Steps.index() as i32);
        assert_ne!(rows[0].value.as_str(), rows[0].base.as_str());
    }

    #[test]
    fn target_views_expose_base_and_modulated_positions() {
        let p = DrumParams {
            lfo1_links: std::sync::atomic::AtomicU64::new(crate::modulation::target_bit(ModTarget::Steps)),
            lfo1_wave: EnumParam::new("LFO 1 Wave", OscWave::Sine),
            lfo1_rate_mode: EnumParam::new("LFO 1 Rate Mode", LfoRateMode::Hz),
            lfo1_freq: FloatParam::new("LFO 1 Frequency", 0.5, FloatRange::Skewed { min: 0.01, max: LFO_MAX_HZ, factor: 0.25 }),
            lfo1_depth: FloatParam::new("LFO 1 Depth", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            ..DrumParams::default()
        };
        let views = target_views(&p, LfoClock { beat: 0.5, seconds: 0.5 });
        let steps = &views[ModTarget::Steps.index()];
        assert!(steps.linked);
        assert!(steps.value01 > steps.base01);
        assert!(steps.lfo_text.as_str().contains("LFO1"));
        assert!(!views[ModTarget::Density.index()].linked);
    }
}
