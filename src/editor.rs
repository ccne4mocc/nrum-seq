//! Slint GUI adapter, following the reference project's separation between pure geometry and UI glue.
use crate::globals;
use crate::keys;
use crate::midi_export;
use crate::sequence::{generate, GenParams, Pattern, Rng, MAX_STEPS, MIN_MAX_STEPS, TRACKS};
use crate::view::{self, View};
use crate::{cell_count_for, scope, DrumParams, Mode, OscWave, Rate, Shared, Theme as PluginTheme};
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
const HEIGHT: u32 = 760;

/// Append one technical line to `nDrum.log` in the user's home directory. Called from the GUI
/// update thread only; the audio path never touches the file system.
fn log_line(text: &str) {
    use std::io::Write;
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
    let Some(home) = home else { return };
    let path = PathBuf::from(home).join("nDrum.log");
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
        // Editor closing is the last chance to persist, so bypass the debounce.
        // Theme and host scale are read from live state *here*: a snapshot
        // taken at spawn is stale by the time the user closes the editor, and writing it back
        // would revert every change made during the session. No unit test covers this:
        // nice-plug's `Param` trait is read-only outside a `GuiContext`, so a test cannot
        // change the theme and observe what the close path writes. The fix is structural --
        // there is no snapshot field left to go stale.
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

/// Slider and spin-box paths for the option table share this so clamping stays identical.
fn set_opt(c: &Arc<dyn GuiContext>, p: &DrumParams, key: i32, value: f32) {
    let snap = p.round_steps.value();
    // The ceiling is a host parameter, but host echo is asynchronous. Use the value being set
    // for the same tick so the dependent steps/pulses/rotate limits cannot lag one edit behind.
    if key == 14 {
        let cap = (value.round() as i32).clamp(MIN_MAX_STEPS as i32, MAX_STEPS as i32);
        set(c, &p.max_steps, cap);
        let cells = cell_count_for(p.steps.value(), snap, cap as usize);
        if p.steps.value() > cap as f32 { set(c, &p.steps, cap as f32); }
        if p.pulses.value() > cells { set(c, &p.pulses, cells); }
        if p.rotate.value() >= cells { set(c, &p.rotate, cells - 1); }
        return;
    }
    // Setting `steps` reaches the host first and only shows up in `p.steps.value()` once the host
    // echoes it back, so the bounds for pulses/rotate must come from the value being set, not
    // from a read taken after (or before) the set.
    let cap = p.step_ceiling();
    let cells = if key == 0 { cell_count_for(value, snap, cap) } else { p.cell_count() };
    let max_pulses = if p.mode.value() == Mode::Cellular { 15 } else { cells };
    match key {
        0 => {
            // Snap on: store a whole number. A raw slider position such as 16.37 would display
            // as "16" while the stored value stayed fractional.
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
        _ => {},
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

static RANDOMIZE_NONCE: AtomicU32 = AtomicU32::new(1);

/// True when the RANDOMIZE scope allows this group bit.
fn scope_enabled(p: &DrumParams, bit: i32) -> bool { p.randomize_scope.value() & bit != 0 }

/// Randomize the active algorithm controls through nice-plug's parameter setter,
/// so the host, the custom UI, and the generator all observe the same values.
fn randomize_parameters(p: &DrumParams, ctx: &Arc<dyn GuiContext>) {
    let nonce = RANDOMIZE_NONCE.fetch_add(1, Relaxed);
    let variation = p.variation.fetch_add(1, Relaxed).wrapping_add(nonce);
    let mut rng = Rng::new(p.seed.value(), variation, 0xA1_60_D2_0A_5EED);

    // Draw the whole set in a fixed order, then apply only the groups the scope allows: the
    // stream does not skip draws when a group is disabled, so enabling a group never changes
    // what another group received. Pulse/rotate bounds still follow the mode and length that
    // survive the scope, so a written count is always in range.
    let mode = Mode::from_index(rng.below(<Mode as Enum>::variants().len()));
    let ceiling = p.step_ceiling();
    let cells = 8 + rng.below(ceiling - 7); // 8..=ceiling
    // Keep randomize on the grid; fractional spacing stays a deliberate user choice.
    let steps = cells as f32;
    // Bounds follow the values that will actually be in force, so a written pulse or rotate
    // count is never out of range for the mode and length that survive the scope.
    let mode_after = if scope_enabled(p, scope::MODE) { mode } else { p.mode.value() };
    let cells_after = if scope_enabled(p, scope::LENGTH) { cells } else { p.cell_count().max(1) as usize };
    let pulse_limit = match mode_after {
        Mode::Cellular => 15,
        Mode::Sieve => 2,
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
    let rate = Rate::from_index(rng.below(6));
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
    let pattern = generate(&gen);
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
        lanes,
    };
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let path = folder.join(format!("ndrum-{stamp}.mid"));
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
    // Update rows in place when the count is unchanged: replacing the model resets the
    // repeater, and a reset element does not tween. Pebble Box relies on `animate x, y`, and
    // in-place rows also stop every other panel from recreating its nodes each step.
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
        // User-global preferences load before the host's project state is shown, so the theme
        // follows the user rather than whatever the project last stored.
        let stored = globals::load();
        // A previous editor instance may have left a key or a focus flag behind.
        self.shared.text_focus.store(false, Relaxed);
        if let Ok(mut queue) = self.shared.key_queue.lock() { queue.clear(); }
        let scale = stored.scale.filter(|s| *s > 0.0).unwrap_or_else(|| f32::from_bits(self.scale.load(Relaxed)));
        // A plugin has no host API for the project path, so the default is the process working
        // directory. The editor shows the absolute path and the user can type another one.
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
        // Keep the host factor in the atomic even when the host never calls
        // `set_scale_factor`: the close path reads it, and an unset value used to drop it.
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
            title: "nDrum".to_string(), size: Size::new(WIDTH as f64, HEIGHT as f64),
            scale: if scale > 0.0 { WindowScalePolicy::ScaleFactor(scale as f64) } else { WindowScalePolicy::SystemScaleFactor },
            gl_config: Some(default_gl_config()),
        };
        let (setup_params, setup_ctx) = (self.params.clone(), context.clone());
        let setup_shared = self.shared.clone();
        let setup_scale = self.scale.clone();
        // `spawn` returns before any closure runs, and the closures must be `'static`, so the
        // shared state is cloned out here rather than captured through `&self`.
        let setup = move |h: &slint_baseview::SlintWindowHandler<AppWindow>, _w: &mut slint_baseview::Window| {
            let ui = h.component();
            let folder = setup_midi_dir.lock().map(|d| d.clone()).unwrap_or_default();
            ui.set_midi_folder(SharedString::from(folder.as_str()));
            {
                let (p, c) = (setup_params.clone(), setup_ctx.clone());
                ui.on_set_mode(move |v| {
                    let last = <Mode as Enum>::variants().len() as i32 - 1;
                    let mode = Mode::from_index(v.clamp(0, last) as usize);
                    set(&c, &p.mode, mode);
                    let max_pulses = match mode {
                        Mode::Cellular => 15,
                        Mode::Sieve => 2,
                        _ => p.cell_count(),
                    };
                    if p.pulses.value() > max_pulses { set(&c, &p.pulses, max_pulses); }
                    // Mode-specific default pulse count, so a freshly selected mode lands on
                    // a sensible starting point instead of inheriting the previous mode's value.
                    let default_pulses = match mode {
                        Mode::Cellular => 8,
                        Mode::Eulerian => 2,
                        Mode::Sieve => 1,
                        Mode::PebbleBox => 4,
                        _ => 5,
                    };
                    if p.pulses.value() == 0 { set(&c, &p.pulses, default_pulses.min(max_pulses)); }
                    // Modes that read a long orbit or fractal benefit from more than the
                    // default 64-step ceiling. Raise the ceiling once, and let the user lower it.
                    let wants_long = matches!(
                        mode,
                        Mode::Lorenz
                            | Mode::ReactionDiffusion
                            | Mode::Cantor
                            | Mode::Fibonacci
                            | Mode::Genetic
                            | Mode::Kuramoto
                            | Mode::Phyllotaxis
                    );
                    if wants_long && p.max_steps.value() < 256 {
                        set(&c, &p.max_steps, 256);
                    }
                });
            }
            {
                let (p, c) = (setup_params.clone(), setup_ctx.clone());
                ui.on_set_rate(move |v| set(&c, &p.rate, Rate::from_index(v.clamp(0, 5) as usize)));
            }
            {
                let (p, c) = (setup_params.clone(), setup_ctx.clone());
                ui.on_set_osc_wave(move |v| {
                    set(&c, &p.osc_wave, OscWave::from_index(v.clamp(0, OscWave::variants().len() as i32 - 1) as usize));
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
            // Constant UI data: waveform names and the author's contact details come from Rust,
            // so the picker and the Credits panel cannot drift from the metadata.
            ui.set_wave_names(ModelRc::new(VecModel::from(
                OscWave::variants().iter().map(|name| SharedString::from(*name)).collect::<Vec<_>>(),
            )));
            // Grouped mode list for the dropdown picker.
            let groups: &[( &str, &[usize] )] = &[
                ("RHYTHMIC", &[0, 19, 13, 14, 11, 9, 15, 8]),
                ("STOCHASTIC", &[7, 20, 27, 10]),
                ("PHYSICS / FRACTALS", &[6, 24, 23, 25, 16, 18]),
                ("MATHEMATICAL / FORMAL", &[2, 3, 4, 5, 17, 26, 12, 1, 21, 22]),
            ];
            let mut entries: Vec<ModeEntry> = Vec::new();
            for (g, idxs) in groups {
                entries.push(ModeEntry { group: SharedString::from(*g), index: -1, header: true });
                for i in *idxs {
                    entries.push(ModeEntry { group: SharedString::from(*g), index: *i as i32, header: false });
                }
            }
            ui.set_mode_entries(ModelRc::new(VecModel::from(entries)));
            ui.set_settings_path(SharedString::from(crate::globals::config_path_display()));
            ui.set_credit_author(SharedString::from(crate::contact::AUTHOR));
            ui.set_credit_website(SharedString::from(crate::contact::WEBSITE));
            ui.set_credit_github(SharedString::from(crate::contact::GITHUB));
            ui.set_credit_support(SharedString::from(crate::contact::SUPPORT));
            ui.set_credit_email(SharedString::from(crate::contact::EMAIL));
        };

        let cache = Arc::new(Mutex::new(Cache { gen_key: None, pattern: Pattern::default(), view_key: None }));
        let (update_params, update_shared, update_cache, update_context) = (self.params.clone(), self.shared.clone(), cache, context.clone());
        // The update closure outlives `&self`, so it reaches the scale through the atomic
        // rather than by borrowing the editor.
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
                // Theme is user-global, so persist it outside the project as it changes.
                // Both fields are known here, so no read is needed.
                globals::save(&global_settings(p, update_scale, &update_midi_dir));
            }
            let ceiling = p.step_ceiling() as f32;
            ui.set_max_steps(ceiling);
            // A project may contain a Steps value left above a newly lowered ceiling. Show the
            // value the generator will actually use instead of a number the matrix contradicts.
            ui.set_steps(p.steps.value().min(ceiling));
            ui.set_round_steps(p.round_steps.value());
            ui.set_pulses(p.pulses.value() as f32);
            ui.set_rotate(p.rotate.value() as f32);
            ui.set_density(p.density.value());
            ui.set_probability(p.probability.value());
            ui.set_velocity(p.velocity.value());
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
            ui.set_symmetry(p.symmetry.value() as f32);
            ui.set_rate(p.rate.value() as i32);
            ui.set_free_run(p.free_run.value());
            ui.set_invert_on(p.invert.value());
            ui.set_mirror_on(p.mirror.value());
            ui.set_logging_on(p.logging.value());
            ui.set_generate_on(p.generate.value());
            let gen_key = p.gen_params();
            let cur = update_shared.current_step.load(Relaxed);
            ui.set_current_step(cur);
            let mask = enabled_mask(p);
            let vel_key = velocity_key(p);
            let mut c = update_cache.lock().unwrap_or_else(|poisoned| {
                log_line("WARNING: update cache mutex was poisoned; recovering its inner state");
                poisoned.into_inner()
            });
            if c.gen_key != Some(gen_key) {
                c.pattern = generate(&gen_key);
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
                    let cells = (0..c.pattern.steps).map(|i| StepCell {
                        track: tr as i32,
                        step: i as i32,
                        hit: enabled && c.pattern.hit[tr][i],
                        playing: cur == i as i32,
                        velocity: (p.track_velocity(tr) * gen_key.velocity * c.pattern.accent[tr][i]).clamp(0.0, 1.0),
                        probability: gen_key.probability,
                    }).collect::<Vec<_>>();
                    rows.push(TrackRow {
                        index: tr as i32,
                        name: SharedString::from(LANE_NAMES[tr]),
                        enabled,
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
        // The window never changes size: the editor reports a fixed 860x760 logical size.
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
    /// Claim a host-stolen editing key only while a text field has focus; every other key, and
    /// every key outside a text field, stays with the host so its shortcuts keep working.
    fn on_virtual_key_from_host(&self, key_code: VirtualKeyCode, is_down: bool, modifiers: Modifiers) -> bool {
        if !self.shared.text_focus.load(Relaxed) { return false; }
        let Some(key) = keys::edit_key(key_code, modifiers) else { return false };
        if let Ok(mut queue) = self.shared.key_queue.lock() {
            // A held key repeats at most a few dozen times a second and the tick drains it every
            // frame; the cap only guards against a window that has stopped ticking.
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
    fn track_mask_covers_eight_lanes() {
        let p = DrumParams::default();
        assert_eq!(enabled_mask(&p) & 0b0000_0111, 0b0000_0111);
    }
}
