//! nDrum: algorithmic multichannel percussion pattern generation, emitted as MIDI.
//! The plug-in intentionally has no audio renderer: route it to a drum sampler / rack.
mod editor;
mod globals;
mod help;
mod midi_export;
mod modes;
mod sequence;
mod view;

pub use modes::Mode;

use nih_plug::prelude::*;
use sequence::{
    generate, GenParams, Pattern, DEFAULT_MAX_STEPS, MAX_STEPS, MIN_MAX_STEPS, TRACKS,
};
use std::sync::atomic::{AtomicI32, AtomicU32, Ordering::Relaxed};
use std::sync::Arc;

#[derive(Enum, PartialEq, Eq, Clone, Copy, Debug)]
pub enum Theme {
    // The host id stays `aurora` so existing projects and automation keep their palette slot;
    // only the display name and the colours changed.
    #[id = "aurora"] #[name = "Sketchbook"] Sketchbook,
    #[id = "neon"] #[name = "Neon"] Neon,
    #[id = "lava"] #[name = "Lava"] Lava,
    #[id = "forest"] #[name = "Forest"] Forest,
}
impl Theme {
    /// Accent1/accent2/accent3/playhead/background1/background2/background3, then the
    /// surface/control/hover/text/muted/border/cell-off/on-accent UI tokens.
    fn colors(self) -> [[u8; 3]; 15] {
        match self {
            Self::Sketchbook => [
                [0xa6,0x7c,0x00],[0x3f,0x65,0x91],[0x9c,0x4f,0x43],[0xa9,0x32,0x26],
                [0xf3,0xee,0xe2],[0xee,0xe7,0xd8],[0xe8,0xdf,0xcd],
                [0xfa,0xf5,0xe9],[0xe5,0xdc,0xc9],[0xd7,0xcc,0xb5],
                [0x2f,0x2b,0x25],[0x7a,0x70,0x61],[0xc7,0xbc,0xa8],
                [0xde,0xd5,0xc3],[0xfd,0xf9,0xf0],
            ],
            Self::Neon => [
                [0x22,0xd3,0xee],[0xa3,0xe6,0x35],[0xf4,0x72,0xb6],[0xff,0xff,0xff],
                [0x0b,0x10,0x20],[0x11,0x11,0x27],[0x16,0x0f,0x24],
                [0x11,0x15,0x2a],[0x1c,0x22,0x40],[0x2b,0x31,0x58],
                [0xe5,0xe7,0xeb],[0x9a,0xa3,0xb5],[0x34,0x3a,0x5c],
                [0x1a,0x20,0x38],[0x0b,0x0f,0x14],
            ],
            Self::Lava => [
                [0xfd,0xe0,0x47],[0xf9,0x73,0x16],[0xdc,0x26,0x26],[0x38,0xbd,0xf8],
                [0x1b,0x10,0x10],[0x20,0x11,0x13],[0x18,0x0e,0x13],
                [0x22,0x14,0x16],[0x33,0x20,0x24],[0x4a,0x2c,0x2f],
                [0xf3,0xe8,0xe0],[0xb0,0x9a,0x94],[0x4a,0x31,0x34],
                [0x2a,0x1a,0x1c],[0x1a,0x0d,0x0d],
            ],
            Self::Forest => [
                [0xbe,0xf2,0x64],[0x22,0xc5,0x5e],[0x0d,0x94,0x88],[0xfb,0x92,0x3c],
                [0x0d,0x17,0x12],[0x11,0x1b,0x15],[0x0e,0x1b,0x19],
                [0x10,0x1e,0x17],[0x1b,0x2c,0x23],[0x2b,0x40,0x33],
                [0xe3,0xec,0xe5],[0x93,0xa8,0x9b],[0x35,0x4b,0x3e],
                [0x16,0x26,0x1d],[0x0b,0x0f,0x14],
            ],
        }
    }
}

#[derive(Enum, PartialEq, Eq, Clone, Copy, Debug)]
pub enum Rate {
    #[id = "1_4"] #[name = "1/4"] Quarter,
    #[id = "1_8"] #[name = "1/8"] Eighth,
    #[id = "1_8t"] #[name = "1/8T"] EighthTriplet,
    #[id = "1_16"] #[name = "1/16"] Sixteenth,
    #[id = "1_16t"] #[name = "1/16T"] SixteenthTriplet,
    #[id = "1_32"] #[name = "1/32"] ThirtySecond,
}
impl Rate {
    fn steps_per_beat(self) -> f64 {
        match self { Self::Quarter => 1.0, Self::Eighth => 2.0, Self::EighthTriplet => 3.0, Self::Sixteenth => 4.0, Self::SixteenthTriplet => 6.0, Self::ThirtySecond => 8.0 }
    }
}

/// Oscillator waveform. The ids are stable host state; new shapes are appended, never renumbered.
#[derive(Enum, PartialEq, Eq, Clone, Copy, Debug)]
pub enum OscWave {
    #[id = "sine"] #[name = "Sine"] Sine,
    #[id = "triangle"] #[name = "Triangle"] Triangle,
    #[id = "saw"] #[name = "Saw"] Saw,
    #[id = "square"] #[name = "Square"] Square,
    #[id = "pulse"] #[name = "Pulse"] Pulse,
    #[id = "decay"] #[name = "Decay"] Decay,
    #[id = "half_sine"] #[name = "Half-Sine"] HalfSine,
    #[id = "noise"] #[name = "Noise"] Noise,
}

/// Which parameter groups RANDOMIZE is allowed to touch. One host parameter, five bits.
pub mod scope {
    pub const MODE: i32 = 1 << 0;
    pub const LENGTH: i32 = 1 << 1;
    pub const RHYTHM: i32 = 1 << 2;
    pub const TONE: i32 = 1 << 3;
    pub const FEEL: i32 = 1 << 4;
    /// Everything: the default, and what RANDOMIZE did before the scope existed.
    pub const ALL: i32 = MODE | LENGTH | RHYTHM | TONE | FEEL;
}


#[derive(Params)]
pub struct DrumParams {
    /// Persisted generation nonce. Generate switches it indirectly; pattern snapshots include it.
    #[persist = "variation"]
    pub variation: AtomicU32,
    #[id = "mode"] pub mode: EnumParam<Mode>,
    /// Pattern length in steps. Whole-number when `round_steps` is on; fractional values
    /// create a loop that no longer aligns with the step grid.
    #[id = "steps"] pub steps: FloatParam,
    /// User-selected ceiling for `steps`/`pulses`/`rotate`, 64–1024. The Pattern arrays always
    /// have room for the highest value, so changing this takes effect immediately.
    #[id = "max_steps"] pub max_steps: IntParam,
    /// Snap the pattern length to whole steps. Off allows fractional step spacing.
    #[id = "round_steps"] pub round_steps: BoolParam,
    #[id = "pulses"] pub pulses: IntParam,
    #[id = "rotate"] pub rotate: IntParam,
    #[id = "density"] pub density: FloatParam,
    #[id = "probability"] pub probability: FloatParam,
    #[id = "velocity"] pub velocity: FloatParam,
    #[id = "regenerate_every"] pub regenerate_every: IntParam,
    #[id = "randomize_after_lengths"] pub randomize_after_lengths: IntParam,
    /// Which groups RANDOMIZE may rewrite: Mode, Length, Rhythm, Tone, Feel. One bit each.
    #[id = "randomize_scope"] pub randomize_scope: IntParam,
    #[id = "seed"] pub seed: FloatParam,
    #[id = "phase"] pub phase: FloatParam,
    #[id = "osc_rate"] pub osc_rate: FloatParam,
    #[id = "osc_wave"] pub osc_wave: EnumParam<OscWave>,
    #[id = "ca_rule"] pub ca_rule: IntParam,
    #[id = "symmetry"] pub symmetry: IntParam,
    #[id = "theme"] pub theme: EnumParam<Theme>,
    #[id = "rate"] pub rate: EnumParam<Rate>,
    #[id = "gate"] pub gate: FloatParam,
    /// Delays every second step by this fraction of a step. 0 is straight; 1/3 gives
    /// triplet feel. Applied in the beat domain so NoteOff follows the onset.
    #[id = "swing"] pub swing: FloatParam,
    /// A change in either direction requests a fresh deterministic variation.
    #[id = "generate"] pub generate: BoolParam,
    #[id = "freerun"] pub free_run: BoolParam,

    #[id = "kick_on"] pub kick_on: BoolParam,
    #[id = "snare_on"] pub snare_on: BoolParam,
    #[id = "closed_hat_on"] pub closed_hat_on: BoolParam,
    #[id = "open_hat_on"] pub open_hat_on: BoolParam,
    #[id = "clap_on"] pub clap_on: BoolParam,
    #[id = "rim_on"] pub rim_on: BoolParam,
    #[id = "low_tom_on"] pub low_tom_on: BoolParam,
    #[id = "perc_on"] pub perc_on: BoolParam,

    #[id = "kick_note"] pub kick_note: IntParam,
    #[id = "snare_note"] pub snare_note: IntParam,
    #[id = "closed_hat_note"] pub closed_hat_note: IntParam,
    #[id = "open_hat_note"] pub open_hat_note: IntParam,
    #[id = "clap_note"] pub clap_note: IntParam,
    #[id = "rim_note"] pub rim_note: IntParam,
    #[id = "low_tom_note"] pub low_tom_note: IntParam,
    #[id = "perc_note"] pub perc_note: IntParam,

    #[id = "kick_velocity"] pub kick_velocity: FloatParam,
    #[id = "snare_velocity"] pub snare_velocity: FloatParam,
    #[id = "closed_hat_velocity"] pub closed_hat_velocity: FloatParam,
    #[id = "open_hat_velocity"] pub open_hat_velocity: FloatParam,
    #[id = "clap_velocity"] pub clap_velocity: FloatParam,
    #[id = "rim_velocity"] pub rim_velocity: FloatParam,
    #[id = "low_tom_velocity"] pub low_tom_velocity: FloatParam,
    #[id = "perc_velocity"] pub perc_velocity: FloatParam,
}

fn percentage_param(name: &'static str, value: f32) -> FloatParam {
    FloatParam::new(name, value, FloatRange::Linear { min: 0.01, max: 1.0 })
        .with_value_to_string(formatters::v2s_f32_percentage(0))
        .with_string_to_value(formatters::s2v_f32_percentage())
}
fn midi_note_param(name: &'static str, value: i32) -> IntParam {
    IntParam::new(name, value, IntRange::Linear { min: 0, max: 127 })
}

impl Default for DrumParams {
    fn default() -> Self {
        Self {
            variation: AtomicU32::new(0),
            mode: EnumParam::new("Generation Mode", Mode::Euclidean),
            steps: FloatParam::new("Steps", 16.0, FloatRange::Linear { min: 1.0, max: MAX_STEPS as f32 }),
            max_steps: IntParam::new(
                "Max Steps",
                DEFAULT_MAX_STEPS as i32,
                IntRange::Linear { min: MIN_MAX_STEPS as i32, max: MAX_STEPS as i32 },
            ),
            round_steps: BoolParam::new("Snap Steps To Whole Numbers", true),
            pulses: IntParam::new("Pulses / Generation", 5, IntRange::Linear { min: 0, max: MAX_STEPS as i32 }),
            rotate: IntParam::new("Rotate", 0, IntRange::Linear { min: 0, max: MAX_STEPS as i32 - 1 }),
            density: percentage_param("Density", 0.65),
            probability: FloatParam::new("Hit Probability", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            velocity: percentage_param("Global Velocity", 1.0),
            regenerate_every: IntParam::new("Regenerate After (lengths; 0 = Off, Chaos = Live)", 0, IntRange::Linear { min: 0, max: 256 }),
            randomize_after_lengths: IntParam::new("Randomize After (lengths; 0 = Off)", 0, IntRange::Linear { min: 0, max: 256 }),
            randomize_scope: IntParam::new("Randomize Scope (Mode|Length|Rhythm|Tone|Feel)", scope::ALL, IntRange::Linear { min: 0, max: scope::ALL }),
            seed: FloatParam::new("Seed", 0.271, FloatRange::Linear { min: 0.0, max: 1.0 }),
            phase: FloatParam::new("Phase", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            osc_rate: FloatParam::new("Oscillator Cycles", 2.0, FloatRange::Linear { min: 0.125, max: 8.0 }),
            osc_wave: EnumParam::new("Oscillator Wave", OscWave::Sine),
            ca_rule: IntParam::new("Cellular Automaton Rule", 30, IntRange::Linear { min: 0, max: 255 }),
            symmetry: IntParam::new("Crystallographic Symmetry", 4, IntRange::Linear { min: 1, max: 8 }),
            theme: EnumParam::new("Theme", Theme::Sketchbook),
            rate: EnumParam::new("Rate", Rate::Sixteenth),
            gate: FloatParam::new("Gate", 0.12, FloatRange::Linear { min: 0.02, max: 1.0 }),
            swing: FloatParam::new("Swing", 0.0, FloatRange::Linear { min: 0.0, max: 0.5 }),
            generate: BoolParam::new("Generate", false),
            free_run: BoolParam::new("Free Run", false),

            kick_on: BoolParam::new("Kick Enable", true),
            snare_on: BoolParam::new("Snare Enable", true),
            closed_hat_on: BoolParam::new("Closed Hat Enable", true),
            open_hat_on: BoolParam::new("Open Hat Enable", true),
            clap_on: BoolParam::new("Clap Enable", false),
            rim_on: BoolParam::new("Rim Enable", false),
            low_tom_on: BoolParam::new("Low Tom Enable", false),
            perc_on: BoolParam::new("Percussion Enable", true),

            kick_note: midi_note_param("Kick MIDI Note", 36),
            snare_note: midi_note_param("Snare MIDI Note", 38),
            closed_hat_note: midi_note_param("Closed Hat MIDI Note", 42),
            open_hat_note: midi_note_param("Open Hat MIDI Note", 46),
            clap_note: midi_note_param("Clap MIDI Note", 39),
            rim_note: midi_note_param("Rim MIDI Note", 37),
            low_tom_note: midi_note_param("Low Tom MIDI Note", 45),
            perc_note: midi_note_param("Percussion MIDI Note", 50),

            kick_velocity: percentage_param("Kick Velocity", 0.92),
            snare_velocity: percentage_param("Snare Velocity", 0.86),
            closed_hat_velocity: percentage_param("Closed Hat Velocity", 0.60),
            open_hat_velocity: percentage_param("Open Hat Velocity", 0.68),
            clap_velocity: percentage_param("Clap Velocity", 0.74),
            rim_velocity: percentage_param("Rim Velocity", 0.62),
            low_tom_velocity: percentage_param("Low Tom Velocity", 0.72),
            perc_velocity: percentage_param("Percussion Velocity", 0.70),
        }
    }
}

impl DrumParams {
    /// Effective ceiling from the host parameter. The range already enforces it, but host state
    /// from an older build can be out of range, and the audio thread should not trust either.
    fn step_ceiling(&self) -> usize {
        self.max_steps.value().clamp(MIN_MAX_STEPS as i32, MAX_STEPS as i32) as usize
    }

    /// Grid cells the current `steps` value covers, matching what `gen_params` produces.
    fn cell_count(&self) -> i32 {
        cell_count_for(self.steps.value(), self.round_steps.value(), self.step_ceiling())
    }

    fn gen_params(&self) -> GenParams {
        let cap = self.step_ceiling();
        let raw = self.steps.value().clamp(1.0, cap as f32);
        // Rounding off leaves `length` fractional, so the loop wraps between grid cells.
        let cells = cell_count_for(raw, self.round_steps.value(), cap) as f32;
        GenParams {
            kind: self.mode.value(),
            steps: cells as usize,
            // With snapping on the loop is exactly `cells` long. Passing the raw value through
            // made a slider drag to e.g. 16.37 (displayed as "16") wrap mid-cell and drift.
            length: if self.round_steps.value() { cells } else { raw },
            pulses: self.pulses.value() as usize,
            rotate: self.rotate.value() as usize,
            density: self.density.value(),
            probability: self.probability.value(),
            velocity: self.velocity.value(),
            seed: self.seed.value(),
            phase: self.phase.value(),
            osc_rate: self.osc_rate.value(),
            osc_wave: self.osc_wave.value().to_index(),
            ca_rule: self.ca_rule.value().clamp(0, 255) as u8,
            symmetry: self.symmetry.value().clamp(1, 8) as usize,
            variation: self.variation.load(Relaxed),
        }
    }


    fn track_enabled(&self, t: usize) -> bool {
        match t { 0 => self.kick_on.value(), 1 => self.snare_on.value(), 2 => self.closed_hat_on.value(), 3 => self.open_hat_on.value(), 4 => self.clap_on.value(), 5 => self.rim_on.value(), 6 => self.low_tom_on.value(), _ => self.perc_on.value() }
    }
    fn track_note(&self, t: usize) -> u8 {
        let n = match t { 0 => self.kick_note.value(), 1 => self.snare_note.value(), 2 => self.closed_hat_note.value(), 3 => self.open_hat_note.value(), 4 => self.clap_note.value(), 5 => self.rim_note.value(), 6 => self.low_tom_note.value(), _ => self.perc_note.value() };
        n.clamp(0, 127) as u8
    }
    fn track_velocity(&self, t: usize) -> f32 {
        match t { 0 => self.kick_velocity.value(), 1 => self.snare_velocity.value(), 2 => self.closed_hat_velocity.value(), 3 => self.open_hat_velocity.value(), 4 => self.clap_velocity.value(), 5 => self.rim_velocity.value(), 6 => self.low_tom_velocity.value(), _ => self.perc_velocity.value() }
    }
}

/// Grid cells a raw `steps` value covers under the user-selected ceiling. Free function so
/// tests exercise the real rule: an inlined copy inside the test silently outlived the
/// previous `MAX_STEPS` reduction, because comparing a formula against itself cannot fail.
fn cell_count_for(raw: f32, snap: bool, ceiling: usize) -> i32 {
    let cap = ceiling.clamp(MIN_MAX_STEPS, MAX_STEPS) as f32;
    let raw = raw.clamp(1.0, cap);
    (if snap { raw.round() } else { raw.ceil() }).clamp(1.0, cap) as i32
}

/// Lock-free signals shared between the audio scheduler and the editor.
/// Auto-randomization is requested on the audio thread at a beat boundary and
/// applied through NIH-plug's ParamSetter on the editor thread.
pub struct Shared {
    pub current_step: AtomicI32,
    pub randomize_request: AtomicU32,
    /// Last host tempo as `f32` bits, for the editor-side MIDI export. Zero means "not seen yet".
    pub tempo_bits: AtomicU32,
}
impl Default for Shared {
    fn default() -> Self {
        Self { current_step: AtomicI32::new(-1), randomize_request: AtomicU32::new(0), tempo_bits: AtomicU32::new(0) }
    }
}

fn regeneration_cycle_at(step_index: i64, loop_length: f64, every: i64, last: Option<i64>) -> Option<i64> {
    if !loop_length.is_finite() || loop_length <= 0.0 || every <= 0 { return None; }
    // Count complete loop lengths, not grid cells: a 16.5-step loop wraps between cells 16 and
    // 17, so its first complete cycle starts at step 17, not at step 16.
    let cycle = (step_index as f64 / loop_length).floor() as i64;
    if cycle > 0 && cycle.rem_euclid(every) == 0 && last != Some(cycle) { Some(cycle) } else { None }
}

/// Grid cell for step `k` in a loop of `length` steps. A whole-number length is the identity
/// `k.rem_euclid(n)`; a fractional one wraps mid-cell, so each cycle starts a fraction of a
/// step later and the pattern drifts against the grid.
fn step_index(k: i64, length: f64) -> i64 {
    if length >= 1.0 && length.fract() == 0.0 { k.rem_euclid(length as i64) } else { (k as f64).rem_euclid(length).floor() as i64 }
}

/// Where the step cursor resumes in this block. A carried cursor is only trusted while it is
/// plausibly "the next step" for the current block: the lower bound drops a cursor the clock has
/// already passed, and the upper bound drops one that is far ahead, which happens when `rate`
/// changes to a slower value (the cursor counts steps of the *old* rate, so e.g. step 400 at 1/16
/// is beat 100 but step 400 at 1/4 is beat 400, silencing the plug-in for ~300 beats).
fn resume_cursor(carried: Option<i64>, jumped: bool, start: f64, end: f64, spb: f64) -> i64 {
    let lo = (start * spb - 1.0).ceil() as i64;
    let hi = (end * spb + 1.0).ceil() as i64;
    match carried {
        Some(next) if !jumped && next >= lo && next <= hi => next,
        _ => (start * spb - 1e-6).ceil() as i64,
    }
}

#[derive(Clone, Copy)]
struct Active { note: u8, off_beat: f64 }

pub struct NDrum {
    params: Arc<DrumParams>,
    shared: Arc<Shared>,
    last_key: Option<GenParams>,
    pattern: Pattern,
    /// Loop length in steps, from the same snapshot as `pattern`.
    loop_length: f64,
    last_generate: Option<bool>,
    active: [Option<Active>; TRACKS],
    expected_beat: Option<f64>,
    /// Next step index to schedule, carried across blocks so swing cannot drop a step.
    next_step: Option<i64>,
    /// Set by `reset`, which has no `ProcessContext` to send NoteOffs with. The next
    /// `process` flushes them, so a host transport reset cannot leave notes hanging.
    pending_reset: bool,
    free_beat: f64,
    last_regen_cycle: Option<i64>,
    last_randomize_cycle: Option<i64>,
    sample_rate: f64,
}
impl Default for NDrum {
    fn default() -> Self {
        Self {
            params: Arc::new(DrumParams::default()), shared: Arc::new(Shared::default()),
            last_key: None, pattern: Pattern::default(), loop_length: 16.0, last_generate: None,
            active: [None; TRACKS], expected_beat: None, next_step: None, pending_reset: false, free_beat: 0.0,
            last_regen_cycle: None, last_randomize_cycle: None, sample_rate: 44100.0,
        }
    }
}

impl NDrum {
    fn stop_all(&mut self, ctx: &mut impl ProcessContext<Self>) {
        for t in 0..TRACKS {
            if let Some(a) = self.active[t].take() {
                ctx.send_event(NoteEvent::NoteOff { timing: 0, voice_id: None, channel: 9, note: a.note, velocity: 0.0 });
            }
        }
    }
}

/// Author contact, shared by the plug-in metadata and the Credits panel.
pub mod contact {
    pub const AUTHOR: &str = "ccnn4mocc";
    pub const EMAIL: &str = "";
    pub const GITHUB: &str = "https://github.com/ccnn4mocc";
    pub const SUPPORT: &str = "";
}

impl Plugin for NDrum {
    const NAME: &'static str = "nDrum";
    const VENDOR: &'static str = contact::AUTHOR;
    const URL: &'static str = contact::GITHUB;
    const EMAIL: &'static str = contact::EMAIL;
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout { main_input_channels: None, main_output_channels: None, ..AudioIOLayout::const_default() }];
    // Input is drained and discarded in `process`, so advertising CC input would only make
    // hosts send messages that are thrown away.
    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::MidiCCs;
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> { self.params.clone() }
    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        Some(editor::create(self.params.clone(), self.shared.clone()))
    }
    fn initialize(&mut self, _layout: &AudioIOLayout, config: &BufferConfig, _context: &mut impl InitContext<Self>) -> bool {
        self.sample_rate = config.sample_rate as f64;
        true
    }
    fn reset(&mut self) {
        // nih-plug hands `reset` no ProcessContext, so NoteOffs cannot be sent from here.
        // `active` is deliberately left intact and flushed at the top of the next `process`;
        // clearing it here would drop the note numbers and leave every sounding note stuck.
        self.pending_reset = self.active.iter().any(|a| a.is_some());
        self.expected_beat = None;
        self.next_step = None;
        self.free_beat = 0.0;
        self.last_regen_cycle = None;
        self.last_randomize_cycle = None;
        self.last_key = None;
        self.loop_length = 16.0;
        self.last_generate = None;
        self.shared.current_step.store(-1, Relaxed);
    }

    fn process(&mut self, buffer: &mut Buffer, _aux: &mut AuxiliaryBuffers, context: &mut impl ProcessContext<Self>) -> ProcessStatus {
        let p = self.params.clone();
        if self.pending_reset {
            self.pending_reset = false;
            self.stop_all(context);
        }
        while context.next_event().is_some() {} // This is a MIDI generator, not an input passthrough.

        let generated = p.generate.value();
        if matches!(self.last_generate, Some(prev) if prev != generated) { p.variation.fetch_add(1, Relaxed); }
        self.last_generate = Some(generated);
        let initial_key = p.gen_params();
        if self.last_key != Some(initial_key) {
            self.pattern = generate(&initial_key);
            self.loop_length = initial_key.length as f64;
            self.last_key = Some(initial_key);
            // Cycle numbers are counted in pattern lengths, so changing the length renumbers
            // them; the remembered cycle would otherwise suppress or trigger the next
            // regeneration against the wrong count.
            self.last_regen_cycle = None;
            self.last_randomize_cycle = None;
        }

        let n_samples = buffer.samples();
        let transport = context.transport();
        let playing = transport.playing;
        // The beat clock must track the host's tempo exactly. Clamping it to a musical range
        // would desynchronise our `expected_beat` prediction from the authoritative
        // `pos_beats`, and the transport-jump detector below would then fire on our own drift
        // rather than on a real seek. Only reject degenerate values that would divide by zero.
        let bpm = transport.tempo.filter(|t| t.is_finite() && *t > 0.0).unwrap_or(120.0);
        self.shared.tempo_bits.store((bpm as f32).to_bits(), Relaxed);
        let beats_per_sample = (bpm / 60.0) / self.sample_rate.max(1.0);
        let block_beats = n_samples as f64 * beats_per_sample;
        let start = if playing {
            self.free_beat = 0.0;
            // Hosts that omit the position would otherwise pin every block to beat 0, replaying
            // step 0 forever and tripping the jump detector; keep counting on our own clock.
            Some(transport.pos_beats().or(self.expected_beat).unwrap_or(0.0))
        } else if p.free_run.value() { Some(self.free_beat) } else { None };
        let Some(start) = start else {
            self.stop_all(context);
            self.expected_beat = None;
            self.next_step = None;
            self.shared.current_step.store(-1, Relaxed);
            return ProcessStatus::Normal;
        };

        let end = start + block_beats;
        // A jump is a discontinuity, so tolerance is block-relative: hosts that report
        // pos_beats quantised to the sample grid, or that resync between blocks, are not jumps.
        let jump_tolerance = 0.02_f64.max(block_beats * 2.0);
        let transport_jump = self.expected_beat.map(|e| (start - e).abs() > jump_tolerance).unwrap_or(false);
        if transport_jump { self.stop_all(context); self.last_regen_cycle = None; self.last_randomize_cycle = None; }
        self.expected_beat = Some(end);
        if !playing { self.free_beat = end; }

        let timing = |beat: f64| -> u32 {
            (((beat - start) / beats_per_sample).round().max(0.0) as usize).min(n_samples.saturating_sub(1)) as u32
        };
        let spb = p.rate.value().steps_per_beat();
        let step_len = 1.0 / spb;
        let s1 = end * spb;
        // Swing delays alternate steps past their unswung position, so a step can fall
        // beyond this block's end. The cursor is carried across blocks rather than
        // re-derived from the host position, otherwise a deferred step would be skipped.
        let swing = p.swing.value().clamp(0.0, 0.5) as f64 * step_len;
        let swung = |k: i64| k as f64 * step_len + if k.rem_euclid(2) != 0 { swing } else { 0.0 };
        let mut k = resume_cursor(self.next_step, transport_jump, start, end, spb);
        let every = p.regenerate_every.value().max(0) as i64;
        let randomize_every = p.randomize_after_lengths.value().max(0) as i64;

        loop {
            let next_step = if (k as f64) < s1 + 1.0 { swung(k) } else { f64::INFINITY };
            let next_off = self.active.iter().filter_map(|a| a.map(|v| v.off_beat)).fold(f64::INFINITY, f64::min);
            let at = next_step.min(next_off);
            if !at.is_finite() || at >= end - 1e-9 { break; }

            // Note-offs precede any retrigger at the same timeline position.
            for tr in 0..TRACKS {
                if let Some(a) = self.active[tr] {
                    if a.off_beat <= at + 1e-9 {
                        context.send_event(NoteEvent::NoteOff { timing: timing(a.off_beat), voice_id: None, channel: 9, note: a.note, velocity: 0.0 });
                        self.active[tr] = None;
                    }
                }
            }
            if next_step <= at + 1e-9 && next_step < end - 1e-9 {
                let idx = step_index(k, self.loop_length) as usize;
                // Complete pattern lengths, not host beats, like regeneration. The request is
                // lock-free here; the editor applies it through ParamSetter on its next tick.
                if let Some(cycle) = regeneration_cycle_at(k, self.loop_length, randomize_every, self.last_randomize_cycle) {
                    self.shared.randomize_request.fetch_add(1, Relaxed);
                    self.last_randomize_cycle = Some(cycle);
                }
                if let Some(cycle) = regeneration_cycle_at(k, self.loop_length, every, self.last_regen_cycle) {
                    p.variation.fetch_add(1, Relaxed);
                    let next_key = p.gen_params();
                    self.pattern = generate(&next_key);
                    self.loop_length = next_key.length as f64;
                    self.last_key = Some(next_key);
                    self.last_regen_cycle = Some(cycle);
                }
                // Regeneration can change the step count; clamp the active index to
                // the new generated length before updating the playhead or reading hits.
                let idx = idx.min(self.pattern.steps.saturating_sub(1));
                self.shared.current_step.store(idx as i32, Relaxed);
                for tr in 0..TRACKS {
                    if !p.track_enabled(tr) || !self.pattern.hit[tr][idx] { continue; }
                    let t = timing(next_step);
                    if let Some(a) = self.active[tr].take() {
                        context.send_event(NoteEvent::NoteOff { timing: t, voice_id: None, channel: 9, note: a.note, velocity: 0.0 });
                    }
                    let note = p.track_note(tr);
                    let velocity = (p.track_velocity(tr) * p.velocity.value() * self.pattern.accent[tr][idx]).clamp(0.01, 1.0);
                    context.send_event(NoteEvent::NoteOn { timing: t, voice_id: None, channel: 9, note, velocity });
                    self.active[tr] = Some(Active { note, off_beat: next_step + p.gate.value() as f64 * step_len });
                }
                k += 1;
            }
        }
        // Carry the unplayed step into the next block; a swung step may not have been
        // reached before this block ended.
        self.next_step = Some(k);
        ProcessStatus::Normal
    }
}

impl ClapPlugin for NDrum {
    const CLAP_ID: &'static str = "com.ndrum.ndrum";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Algorithmic percussion MIDI sequencer with eight drum lanes");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(contact::GITHUB);
    const CLAP_SUPPORT_URL: Option<&'static str> = Some(contact::SUPPORT);
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::NoteEffect, ClapFeature::Utility];
}
impl Vst3Plugin for NDrum {
    const VST3_CLASS_ID: [u8; 16] = *b"nDrum\0\0\0\0\0\0\0\0\0\0\0";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Instrument, Vst3SubCategory::Tools];
}
nih_export_clap!(NDrum);
nih_export_vst3!(NDrum);

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_parameter_is_host_automatable_and_visible() {
        let params = DrumParams::default();
        let map = params.param_map();
        let ids: Vec<&str> = map.iter().map(|(id, _, _)| id.as_str()).collect();
        for expected in ["mode", "steps", "round_steps", "pulses", "rotate", "density", "probability", "velocity", "regenerate_every", "randomize_after_lengths", "randomize_scope", "theme", "seed", "phase", "osc_rate", "osc_wave", "ca_rule", "symmetry", "rate", "gate", "swing", "generate", "freerun", "kick_on", "snare_on", "closed_hat_on", "open_hat_on", "clap_on", "rim_on", "low_tom_on", "perc_on", "kick_note", "snare_note", "closed_hat_note", "open_hat_note", "clap_note", "rim_note", "low_tom_note", "perc_note", "kick_velocity", "snare_velocity", "closed_hat_velocity", "open_hat_velocity", "clap_velocity", "rim_velocity", "low_tom_velocity", "perc_velocity"] {
            assert!(ids.contains(&expected), "missing parameter {expected}");
        }
        for (id, ptr, _) in &map {
            let flags = unsafe { ptr.flags() };
            assert!(!flags.contains(ParamFlags::NON_AUTOMATABLE), "{id} is not automatable");
            assert!(!flags.contains(ParamFlags::HIDDEN), "{id} is hidden");
        }
    }
    /// The UI addresses modes by index (`root.mode == 4` shows the wave selector), so the enum
    /// order is a host-visible contract. This pins the indices the UI hardcodes to the mode
    /// each one means, and checks the UI's own mode-name list against the enum length.
    #[test]
    fn mode_indices_match_the_ui() {
        // (index, mode, what the UI does at that index)
        for (index, mode, ui_meaning) in [
            (0, Mode::Euclidean, "target pulses label"),
            (1, Mode::Eulerian, "degree label"),
            (2, Mode::Phyllotaxis, "spiral phase label"),
            (3, Mode::Cellular, "generation label"),
            (4, Mode::Oscillator, "wave selector"),
            (5, Mode::Crystallographic, "symmetry slider"),
            (6, Mode::Logistic, "growth label"),
            (7, Mode::Markov, "walk activity label"),
            (8, Mode::RaisedCosine, "envelope phase label"),
            (9, Mode::Slowcat, "motif pulses label"),
            (13, Mode::PhaseShift, "phase offset label"),
            (15, Mode::Syncopation, "target onsets label"),
            (16, Mode::PebbleBox, "substeps and gravity"),
            (17, Mode::Sieve, "moduli and residue labels"),
            (18, Mode::Kuramoto, "coupling label"),
        ] {
            assert_eq!(Mode::from_index(index), mode, "mode {index} is {ui_meaning} in the UI");
        }
        // The UI's mode list must have one entry per mode.
        let ui = include_str!("../ui/app.slint");
        let names = ui.split("mode-names: [").nth(1).expect("no mode-name list in the UI");
        let count = names.split(']').next().unwrap().matches('"').count() / 2;
        assert_eq!(count, Mode::all().count(), "the UI lists {count} modes");
    }


    /// The waveform names and ids are host state: the picker reads `variants()`, the help line
    /// reads `ids()`, and the generator only knows a count. All three must agree.
    #[test]
    fn every_oscillator_wave_is_named_indexed_and_stable() {
        let names = OscWave::variants();
        let ids = OscWave::ids().expect("wave ids are part of the host contract");
        assert_eq!(names.len(), crate::sequence::oscillator_wave_count);
        assert_eq!(ids.len(), names.len());
        assert_eq!(ids, ["sine", "triangle", "saw", "square", "pulse", "decay", "half_sine", "noise"]);
        for index in 0..names.len() {
            assert_eq!(OscWave::from_index(index).to_index(), index);
        }
    }

    #[test]
    fn randomize_scope_is_one_bit_per_group() {
        let bits = [scope::MODE, scope::LENGTH, scope::RHYTHM, scope::TONE, scope::FEEL];
        for (i, a) in bits.iter().enumerate() {
            assert!((*a as u32).is_power_of_two(), "scope bit {a} is not a single bit");
            for b in &bits[i + 1..] {
                assert_eq!(a & b, 0, "scope groups {a} and {b} overlap");
            }
        }
        let p = DrumParams::default();
        assert_eq!(p.randomize_scope.value(), scope::ALL, "default must keep the old full randomize");
    }

    #[test]
    fn contact_details_are_the_published_ones() {
        assert_eq!(contact::AUTHOR, "ccnn4mocc");
        assert_eq!(contact::EMAIL, "");
        assert_eq!(contact::GITHUB, "https://github.com/ccnn4mocc");
        assert_eq!(contact::SUPPORT, "");
        assert_eq!(<NDrum as Plugin>::VENDOR, contact::AUTHOR);
    }

    #[test]
    fn the_documented_pattern_size_is_current() {
        // The architecture doc states the struct size; keep the two from drifting. The doc size
        // is for 64-bit targets, where `usize` is eight bytes.
        if cfg!(target_pointer_width = "64") {
            let size = std::mem::size_of::<crate::sequence::Pattern>();
            let docs = include_str!("../docs/ARCHITECTURE.md");
            assert!(docs.contains(&format!("{size} bytes")), "ARCHITECTURE.md does not mention the {size}-byte Pattern");
        }
    }

    #[test]
    fn theme_indices_and_palettes_are_consistent() {
        for index in 0..4 { assert_eq!(Theme::from_index(index).to_index(), index); }
        let palettes = [Theme::Sketchbook.colors(), Theme::Neon.colors(), Theme::Lava.colors(), Theme::Forest.colors()];
        for palette in palettes { assert_ne!(palette[0], palette[1]); assert_ne!(palette[1], palette[2]); assert_ne!(palette[3], palette[4]); }
        assert_ne!(palettes[0][0], palettes[1][0]);
        assert_ne!(palettes[1][0], palettes[2][0]);
        assert_ne!(palettes[2][0], palettes[3][0]);
    }

    #[test]
    fn auto_randomization_defaults_to_off() {
        let p = DrumParams::default();
        assert_eq!(p.randomize_after_lengths.value(), 0);
    }

    #[test]
    fn regeneration_and_randomization_trigger_only_on_complete_lengths() {
        assert_eq!(regeneration_cycle_at(0, 16.0, 2, None), None);
        assert_eq!(regeneration_cycle_at(16, 16.0, 2, None), None);
        assert_eq!(regeneration_cycle_at(31, 16.0, 2, None), None);
        assert_eq!(regeneration_cycle_at(32, 16.0, 2, None), Some(2));
        assert_eq!(regeneration_cycle_at(32, 16.0, 2, Some(2)), None);
        assert_eq!(regeneration_cycle_at(64, 16.0, 2, Some(2)), Some(4));
        assert_eq!(regeneration_cycle_at(512, 256.0, 2, None), Some(2));
        assert_eq!(regeneration_cycle_at(16, 16.0, 0, None), None);
        // A fractional loop must count whole loops, not grid cells: 16.5 steps per cycle and
        // regeneration every 2 loops fires at 33 and 66, not at 32 and 64.
        assert_eq!(regeneration_cycle_at(33, 16.5, 2, None), Some(2));
        assert_eq!(regeneration_cycle_at(66, 16.5, 2, Some(2)), Some(4));
        assert_eq!(regeneration_cycle_at(67, 16.5, 2, Some(4)), None);
    }

    /// A tempo the plugin used to clamp away must not desynchronise our beat clock from the
    /// host's, or every block looks like a transport jump and silences the plugin.
    #[test]
    fn extreme_tempos_do_not_trip_the_transport_jump_detector() {
        for bpm in [20.0f64, 120.0, 400.0, 425.0, 700.0, 999.0] {
            let sample_rate = 44100.0;
            let block = 2048usize;
            let beats_per_sample = (bpm / 60.0) / sample_rate;
            let block_beats = block as f64 * beats_per_sample;
            let jump_tolerance = 0.02_f64.max(block_beats * 2.0);
            let mut host_pos = 0.0f64;
            let mut expected: Option<f64> = None;
            for _ in 0..400 {
                let start = host_pos;
                if let Some(e) = expected {
                    assert!((start - e).abs() <= jump_tolerance, "spurious jump at bpm {bpm}");
                }
                expected = Some(start + block_beats);
                host_pos += block_beats;
            }
        }
    }

    #[test]
    fn a_carried_cursor_survives_normal_blocks_but_not_a_slower_rate() {
        // 1/16 at beat 100: cursor ~400. Same cursor under 1/4 (spb 1) is 300 beats in the future.
        assert_eq!(resume_cursor(Some(401), false, 100.0, 100.01, 4.0), 401);
        let after_rate_change = resume_cursor(Some(401), false, 100.0, 100.01, 1.0);
        assert_eq!(after_rate_change, 100, "stale cursor must be dropped");
        // A cursor the clock has passed is dropped, as before; a jump always resets.
        assert_eq!(resume_cursor(Some(10), false, 100.0, 100.01, 1.0), 100);
        assert_eq!(resume_cursor(Some(401), true, 100.0, 100.01, 4.0), 400);
        // A swung step deferred past the block end (cursor just beyond end*spb) is kept.
        assert_eq!(resume_cursor(Some(101), false, 99.9, 100.0, 1.0), 101);
    }

    #[test]
    fn whole_step_lengths_behave_exactly_like_the_old_modulo_wrap() {
        // The fractional path must not change any existing whole-step behaviour.
        for n in [1i64, 2, 4, 16, 17, 255, 256] {
            for k in 0..2000i64 {
                assert_eq!(step_index(k, n as f64), k.rem_euclid(n), "n={n}, k={k}");
            }
        }
    }

    #[test]
    fn fractional_step_lengths_stay_inside_the_generated_cells() {
        // `cell_count` rounds a fractional length up, so the loop may touch every cell.
        for whole in 1usize..=64 {
            for tenth in 0..10 {
                let length = whole as f64 + tenth as f64 / 10.0;
                let cells = length.ceil() as i64;
                for k in 0..500i64 {
                    let idx = step_index(k, length);
                    assert!(idx >= 0 && idx < cells, "length={length}, k={k}, idx={idx}, cells={cells}");
                }
            }
        }
    }

    #[test]
    fn fractional_lengths_reach_the_top_cell_and_drift_between_cycles() {
        for length in [1.5f64, 7.25, 16.5, 100.5] {
            let cells = length.ceil() as i64;
            let max = (0..4000i64).map(|k| step_index(k, length)).max().unwrap();
            assert_eq!(max, cells - 1, "top cell unreachable at length={length}");
        }
        // 16.5 steps per cycle. Cycle 1 starts at k=17, half a cell past the 16 that a
        // whole-step loop would reach; cycle 2 (k=33) wraps to 0.5 and lands on cell 0.
        assert_eq!(step_index(0, 16.5), 0);
        assert_eq!(step_index(16, 16.5), 16);
        assert_eq!(step_index(17, 16.5), step_index(0, 16.5));
        assert_eq!(step_index(33, 16.5), 0);
        // Over two cycles the pattern has advanced 33 steps but only 17 cells, so it drifts
        // by half a cell relative to a 16-step grid instead of repeating in phase.
        assert_ne!(step_index(33, 16.5), step_index(1, 16.5));
    }

    #[test]
    fn step_count_rounding_follows_the_snap_toggle() {
        // Calls `cell_count_for` itself rather than restating it, so the two cannot drift.
        for (raw, snap, cells) in [
            (16.0f32, true, 16i32), (16.4, true, 16), (16.6, true, 17),
            (16.4, false, 17), (16.0, false, 16), (1.0, false, 1),
            // Clamped to the selected ceiling from both ends, whichever way rounding goes.
            (64.0, false, 64), (64.4, false, 64), (100.0, true, 64),
            (MAX_STEPS as f32, false, MAX_STEPS as i32),
            (MAX_STEPS as f32 + 40.0, false, MAX_STEPS as i32),
            (MAX_STEPS as f32 + 40.0, true, MAX_STEPS as i32),
            (0.0, false, 1), (-5.0, true, 1),
        ] {
            let ceiling = if raw <= 100.0 { 64 } else { MAX_STEPS };
            assert_eq!(cell_count_for(raw, snap, ceiling), cells, "raw={raw}, snap={snap}, ceiling={ceiling}");
        }
    }

    #[test]
    fn max_steps_defaults_to_the_old_limit() {
        let p = DrumParams::default();
        assert_eq!(p.step_ceiling(), DEFAULT_MAX_STEPS);
        assert_eq!(p.max_steps.value(), DEFAULT_MAX_STEPS as i32);
        // The default project must still behave exactly like the pre-1024 build.
        assert_eq!(p.cell_count(), 16);
        assert_eq!(p.gen_params().steps, p.cell_count() as usize);
        assert!(p.gen_params().steps <= DEFAULT_MAX_STEPS);
    }

    /// Reproduce the scheduler's step walk for one block, returning (played, cursor).
fn walk_block(start: f64, block_beats: f64, swing: f64, cursor: Option<i64>) -> (Vec<i64>, i64) {
    let spb = 4.0f64; // 1/16
    let step_len = 1.0 / spb;
    let end = start + block_beats;
    let s1 = end * spb;
    let swung = |k: i64| k as f64 * step_len + if k.rem_euclid(2) != 0 { swing * step_len } else { 0.0 };
    let mut k = match cursor {
        Some(next) if next >= (start * spb - 1.0).ceil() as i64 => next,
        _ => (start * spb - 1e-6).ceil() as i64,
    };
    let mut played = Vec::new();
    loop {
        if (k as f64) >= s1 + 1.0 { break; }
        let next_step = swung(k);
        if next_step >= end - 1e-9 { break; }
        played.push(k);
        k += 1;
    }
    (played, k)
}

#[test]
    fn swing_never_drops_or_repeats_a_step_across_block_boundaries() {
        // Regression: `k` used to be re-derived from the host position each block, so a
        // swung step falling past the block end was skipped entirely.
        let sample_rate = 44100.0;
        for block in [64usize, 128, 256, 441, 512, 1024, 2048] {
            for &swing in &[0.0f64, 0.25, 1.0 / 3.0, 0.5] {
                for &bpm in &[60.0f64, 120.0, 174.0] {
                    let block_beats = block as f64 * (bpm / 60.0) / sample_rate;
                    let mut start = 0.0;
                    let mut cursor = None;
                    let mut all: Vec<i64> = Vec::new();
                    for _ in 0..3000 {
                        let (played, next) = walk_block(start, block_beats, swing, cursor);
                        all.extend(played);
                        cursor = Some(next);
                        start += block_beats;
                    }
                    for pair in all.windows(2) {
                        assert_eq!(pair[1], pair[0] + 1,
                            "block={block}, swing={swing}, bpm={bpm}: gap or repeat at {:?}", pair);
                    }
                }
            }
        }
    }

    #[test]
    fn swing_shifts_alternate_steps_without_reordering_them() {
        let step_len = 0.25f64;
        let at = |k: i64, swing: f64| k as f64 * step_len + if k.rem_euclid(2) != 0 { swing * step_len } else { 0.0 };
        // Straight, triplet, and max swing all keep strictly increasing onset order.
        for swing in [0.0f64, 0.25, 1.0 / 3.0, 0.5] {
            for k in 0..200i64 {
                assert!(at(k, swing) < at(k + 1, swing), "swing={swing} reorders at k={k}");
            }
        }
        // Even steps never move; that is what makes the odd ones read as swung.
        for k in 0..200i64 {
            let base = k as f64 * step_len;
            if k.rem_euclid(2) == 0 {
                assert_eq!(at(k, 0.5), base);
            } else {
                assert_eq!(at(k, 0.5), base + 0.5 * step_len);
                assert_eq!(at(k, 0.0), base);
            }
        }
        // A third of a step gives the triplet ratio.
        let third = 1.0 / 3.0;
        assert!((at(1, third) - at(0, third) - 4.0 / 3.0 * step_len).abs() < 1e-12);
        assert!((at(1, third) - at(2, third) + 2.0 / 3.0 * step_len).abs() < 1e-12);
    }

    #[test]
    fn drum_notes_are_gm_style_and_on_channel_ten() {
        let p = DrumParams::default();
        assert_eq!((p.track_note(0), p.track_note(1), p.track_note(2), p.track_note(3)), (36, 38, 42, 46));
        assert!(p.track_enabled(0) && p.track_enabled(1) && p.track_enabled(2));
        assert_eq!(p.gen_params().kind, Mode::Euclidean);
    }

    #[test]
    fn snapped_loop_length_is_always_whole() {
        // Regression: `length` was the raw float even with snapping on, so a slider value of
        // 16.37 gave a 16-cell pattern in a 16.37-step loop that drifted every cycle.
        let p = DrumParams::default();
        let g = p.gen_params();
        assert!(p.round_steps.value());
        assert_eq!(g.length, g.steps as f32);
        assert_eq!(g.length.fract(), 0.0);
    }
}
