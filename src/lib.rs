//! nrum seq: algorithmic multichannel percussion pattern generation, emitted as MIDI.
//! The plug-in intentionally has no audio renderer: route it to a drum sampler / rack.
mod editor;
mod edits;
mod globals;
mod help;
mod keys;
mod midi_export;
mod modes;
mod modulation;
mod sequence;
mod view;

pub use modes::Mode;
pub use modulation::ModTarget;

use edits::EditView;
use modulation::{
    fold, target_bit, target_in_mask, LfoClock, LfoSpec, LFO_COUNT, LFO_MAX_DEPTH, LFO_MAX_HZ,
};
use nice_plug::prelude::*;
use sequence::{
    generate_with, GenParams, Pattern, DEFAULT_MAX_STEPS, MAX_STEPS, MIN_MAX_STEPS, TRACKS,
};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};

#[derive(Enum, PartialEq, Eq, Clone, Copy, Debug)]
pub enum Theme {
    // Keep the stable host id so existing projects and automation keep this palette slot.
    #[id = "aurora"] #[name = "Sketch"] Sketchbook,
    #[id = "neon"] #[name = "Neon"] Neon,
    #[id = "lava"] #[name = "Lava"] Lava,
    #[id = "forest"] #[name = "Forest"] Forest,
}
impl Theme {
    /// Accent/playhead/background colours followed by the UI surface/text/border tokens.
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

    /// Eight distinguishable lane colours for the phyllotaxis rosettes. The matrix keeps its
    /// theme accents; only per-lane drawings use this palette.
    fn lanes(self) -> [[u8; 3]; 8] {
        match self {
            Self::Sketchbook => [
                [0x3f,0x65,0x91],[0x9c,0x4f,0x43],[0xa6,0x7c,0x00],[0x4f,0x7a,0x4a],
                [0x7a,0x4f,0x91],[0x2f,0x7a,0x74],[0xa9,0x32,0x26],[0x5c,0x5c,0x5c],
            ],
            Self::Neon => [
                [0x22,0xd3,0xee],[0xa3,0xe6,0x35],[0xf4,0x72,0xb6],[0xff,0xff,0xff],
                [0xfb,0x92,0x3c],[0xa7,0x8b,0xfa],[0x2d,0xd4,0xbf],[0xfd,0xe0,0x47],
            ],
            Self::Lava => [
                [0xfd,0xe0,0x47],[0xf9,0x73,0x16],[0xdc,0x26,0x26],[0x38,0xbd,0xf8],
                [0xf4,0x72,0xb6],[0xf3,0xe8,0xe0],[0xff,0xa8,0x7a],[0xe0,0xa0,0x20],
            ],
            Self::Forest => [
                [0xbe,0xf2,0x64],[0x22,0xc5,0x5e],[0x0d,0x94,0x88],[0xfb,0x92,0x3c],
                [0x9a,0xe0,0xa0],[0x6f,0xc7,0x8f],[0xc9,0xd6,0x4a],[0x72,0xc7,0xd6],
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
    #[id = "1_4_dot"] #[name = "1/4."] QuarterDotted,
    #[id = "1_8_dot"] #[name = "1/8."] EighthDotted,
    #[id = "1_16_dot"] #[name = "1/16."] SixteenthDotted,
    #[id = "1_32t"] #[name = "1/32T"] ThirtySecondTriplet,
    #[id = "1_32_dot"] #[name = "1/32."] ThirtySecondDotted,
    #[id = "1_64"] #[name = "1/64"] SixtyFourth,
    #[id = "1_64t"] #[name = "1/64T"] SixtyFourthTriplet,
    #[id = "1_64_dot"] #[name = "1/64."] SixtyFourthDotted,
}
#[allow(dead_code)]
impl Rate {
    pub(crate) fn steps_per_beat(self) -> f64 {
        match self {
            Self::Quarter => 1.0,
            Self::QuarterDotted => 1.0 / 1.5,
            Self::Eighth => 2.0,
            Self::EighthDotted => 2.0 / 1.5,
            Self::EighthTriplet => 3.0,
            Self::Sixteenth => 4.0,
            Self::SixteenthDotted => 4.0 / 1.5,
            Self::SixteenthTriplet => 6.0,
            Self::ThirtySecond => 8.0,
            Self::ThirtySecondDotted => 8.0 / 1.5,
            Self::ThirtySecondTriplet => 12.0,
            Self::SixtyFourth => 16.0,
            Self::SixtyFourthDotted => 16.0 / 1.5,
            Self::SixtyFourthTriplet => 24.0,
        }
    }
    const RATE_ORDER: [Rate; 14] = [
        Rate::Quarter, Rate::QuarterDotted,
        Rate::Eighth, Rate::EighthDotted, Rate::EighthTriplet,
        Rate::Sixteenth, Rate::SixteenthDotted, Rate::SixteenthTriplet,
        Rate::ThirtySecond, Rate::ThirtySecondDotted, Rate::ThirtySecondTriplet,
        Rate::SixtyFourth, Rate::SixtyFourthDotted, Rate::SixtyFourthTriplet,
    ];
    pub(crate) fn stepped(self, delta: i32) -> Rate {
        let pos = Self::RATE_ORDER.iter().position(|r| *r == self).unwrap_or(0) as i32;
        let last = Self::RATE_ORDER.len() as i32 - 1;
        Self::RATE_ORDER[(pos + delta).clamp(0, last) as usize]
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

/// LFO speed domain: free-running Hz or tempo-locked beats.
#[derive(Enum, PartialEq, Eq, Clone, Copy, Debug)]
pub enum LfoRateMode {
    #[id = "hz"] #[name = "Hz"] Hz,
    #[id = "beats"] #[name = "Beats"] Beats,
}

/// Whether a modulation offset can move both ways or only upward from the base value.
#[derive(Enum, PartialEq, Eq, Clone, Copy, Debug)]
pub enum LfoPolarity {
    #[id = "bipolar"] #[name = "Bipolar"] Bipolar,
    #[id = "unipolar"] #[name = "Unipolar"] Unipolar,
    /// Downward-only unipolar: the signal spans `-depth..0`.
    #[id = "unipolar_neg"] #[name = "Unipolar Negative"] UnipolarNegative,
}

/// Which parameter groups RANDOMIZE is allowed to touch. One host parameter, five bits.
pub mod scope {
    pub const MODE: i32 = 1 << 0;
    pub const LENGTH: i32 = 1 << 1;
    pub const RHYTHM: i32 = 1 << 2;
    pub const TONE: i32 = 1 << 3;
    pub const CLOCK: i32 = 1 << 4;
    pub const PHASE: i32 = 1 << 5;
    pub const ACID: i32 = 1 << 6;
    pub const FEEL: i32 = 1 << 7;
    /// Every group; the default.
    pub const ALL: i32 = MODE | LENGTH | RHYTHM | TONE | CLOCK | PHASE | ACID | FEEL;
}


#[derive(Params)]
pub struct DrumParams {
    /// Persisted generation nonce. Generate switches it indirectly; pattern snapshots include it.
    #[persist = "variation"]
    pub variation: AtomicU32,
    /// Per-step manual edits, persisted as sparse cells and mirrored in atomics for the audio
    /// thread. Its revision is part of the generation snapshot.
    #[persist = "edit_overlay"]
    pub edit_overlay: Arc<edits::EditStore>,
    #[id = "mode"] pub mode: EnumParam<Mode>,
    /// Pattern length in steps; fractional values wrap between grid cells.
    #[id = "steps"] pub steps: FloatParam,
    /// Ceiling for `steps`/`pulses`/`rotate`, 64–1024. Pattern arrays always hold the maximum.
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
    /// Markov transition bias: 2 bits each for rest, repeat, role and novelty.
    #[id = "markov_bias"] pub markov_bias: IntParam,
    /// Per-lane generator override: 0 = follow `mode`, otherwise 1 + the mode index.
    #[id = "lane1_mode"] pub lane1_mode: IntParam,
    #[id = "lane2_mode"] pub lane2_mode: IntParam,
    #[id = "lane3_mode"] pub lane3_mode: IntParam,
    #[id = "lane4_mode"] pub lane4_mode: IntParam,
    #[id = "lane5_mode"] pub lane5_mode: IntParam,
    #[id = "lane6_mode"] pub lane6_mode: IntParam,
    #[id = "lane7_mode"] pub lane7_mode: IntParam,
    #[id = "lane8_mode"] pub lane8_mode: IntParam,
    #[id = "theme"] pub theme: EnumParam<Theme>,
    #[id = "rate"] pub rate: EnumParam<Rate>,
    #[id = "gate"] pub gate: FloatParam,
    /// Delays every second step by this fraction of a step (1/3 = triplet), in the beat domain.
    #[id = "swing"] pub swing: FloatParam,
    // --- LFO modulation (runtime overlay; links are persisted, values are host parameters) --
    #[id = "lfo1_enabled"] pub lfo1_enabled: BoolParam,
    #[id = "lfo1_wave"] pub lfo1_wave: EnumParam<OscWave>,
    #[id = "lfo1_rate_mode"] pub lfo1_rate_mode: EnumParam<LfoRateMode>,
    #[id = "lfo1_freq"] pub lfo1_freq: FloatParam,
    #[id = "lfo1_rate"] pub lfo1_rate: FloatParam,
    #[id = "lfo1_depth"] pub lfo1_depth: FloatParam,
    #[id = "lfo1_phase"] pub lfo1_phase: FloatParam,
    #[id = "lfo1_polarity"] pub lfo1_polarity: EnumParam<LfoPolarity>,
    #[persist = "lfo1_links"] pub lfo1_links: AtomicU64,
    #[id = "lfo2_enabled"] pub lfo2_enabled: BoolParam,
    #[id = "lfo2_wave"] pub lfo2_wave: EnumParam<OscWave>,
    #[id = "lfo2_rate_mode"] pub lfo2_rate_mode: EnumParam<LfoRateMode>,
    #[id = "lfo2_freq"] pub lfo2_freq: FloatParam,
    #[id = "lfo2_rate"] pub lfo2_rate: FloatParam,
    #[id = "lfo2_depth"] pub lfo2_depth: FloatParam,
    #[id = "lfo2_phase"] pub lfo2_phase: FloatParam,
    #[id = "lfo2_polarity"] pub lfo2_polarity: EnumParam<LfoPolarity>,
    #[persist = "lfo2_links"] pub lfo2_links: AtomicU64,
    #[id = "lfo3_enabled"] pub lfo3_enabled: BoolParam,
    #[id = "lfo3_wave"] pub lfo3_wave: EnumParam<OscWave>,
    #[id = "lfo3_rate_mode"] pub lfo3_rate_mode: EnumParam<LfoRateMode>,
    #[id = "lfo3_freq"] pub lfo3_freq: FloatParam,
    #[id = "lfo3_rate"] pub lfo3_rate: FloatParam,
    #[id = "lfo3_depth"] pub lfo3_depth: FloatParam,
    #[id = "lfo3_phase"] pub lfo3_phase: FloatParam,
    #[id = "lfo3_polarity"] pub lfo3_polarity: EnumParam<LfoPolarity>,
    #[persist = "lfo3_links"] pub lfo3_links: AtomicU64,
    #[id = "lfo4_enabled"] pub lfo4_enabled: BoolParam,
    #[id = "lfo4_wave"] pub lfo4_wave: EnumParam<OscWave>,
    #[id = "lfo4_rate_mode"] pub lfo4_rate_mode: EnumParam<LfoRateMode>,
    #[id = "lfo4_freq"] pub lfo4_freq: FloatParam,
    #[id = "lfo4_rate"] pub lfo4_rate: FloatParam,
    #[id = "lfo4_depth"] pub lfo4_depth: FloatParam,
    #[id = "lfo4_phase"] pub lfo4_phase: FloatParam,
    #[id = "lfo4_polarity"] pub lfo4_polarity: EnumParam<LfoPolarity>,
    #[persist = "lfo4_links"] pub lfo4_links: AtomicU64,
    /// A change in either direction requests a fresh deterministic variation.
    #[id = "generate"] pub generate: BoolParam,
    #[id = "invert"] pub invert: BoolParam,
    #[id = "mirror"] pub mirror: BoolParam,
    /// Write technical diagnostics to `nrum-seq.log` in the user's home directory.
    #[id = "logging"] pub logging: BoolParam,
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

/// Format an LFO frequency in Hz; the speed field is either Hz or a beat-synced rate.
fn format_hz(value: f32) -> String {
    if !value.is_finite() {
        return "0.00 Hz".to_string();
    }
    let value = value.clamp(0.01, LFO_MAX_HZ);
    if value >= 10.0 {
        format!("{value:.1} Hz")
    } else {
        format!("{value:.2} Hz")
    }
}

/// Accept `Hz` or a bare number (interpreted as Hz) in typed speed fields.
fn parse_hz(text: &str) -> Option<f32> {
    let text = text.trim().to_ascii_lowercase();
    let number = text.strip_suffix("hz").unwrap_or(text.as_str());
    let value = number.trim().parse::<f32>().ok()?;
    value.is_finite().then(|| value.clamp(0.01, LFO_MAX_HZ))
}

fn hz_param(name: &'static str, value: f32) -> FloatParam {
    FloatParam::new(
        name,
        value,
        FloatRange::Skewed { min: 0.01, max: LFO_MAX_HZ, factor: FloatRange::skew_factor(-2.0) },
    )
    .with_value_to_string(Arc::new(|v: f32| format_hz(v)))
    .with_string_to_value(Arc::new(|s: &str| parse_hz(s)))
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
            edit_overlay: Arc::new(edits::EditStore::default()),
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
            randomize_scope: IntParam::new("Randomize Scope (Mode|Length|Rhythm|Tone|Clock|Phase|Acid|Feel)", scope::ALL, IntRange::Linear { min: 0, max: scope::ALL }),
            seed: FloatParam::new("Seed", 0.271, FloatRange::Linear { min: 0.0, max: 1.0 }),
            phase: FloatParam::new("Phase", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            osc_rate: FloatParam::new("Oscillator Cycles", 2.0, FloatRange::Linear { min: 0.125, max: 8.0 }),
            osc_wave: EnumParam::new("Oscillator Wave", OscWave::Sine),
            ca_rule: IntParam::new("Cellular Automaton Rule", 30, IntRange::Linear { min: 0, max: 255 }),
            symmetry: IntParam::new("Crystallographic Symmetry", 4, IntRange::Linear { min: 1, max: 8 }),
            markov_bias: IntParam::new("Markov Transition Bias", 0, IntRange::Linear { min: 0, max: 255 }),
            lane1_mode: IntParam::new("Lane 1 Mode", 0, IntRange::Linear { min: 0, max: 39 }),
            lane2_mode: IntParam::new("Lane 2 Mode", 0, IntRange::Linear { min: 0, max: 39 }),
            lane3_mode: IntParam::new("Lane 3 Mode", 0, IntRange::Linear { min: 0, max: 39 }),
            lane4_mode: IntParam::new("Lane 4 Mode", 0, IntRange::Linear { min: 0, max: 39 }),
            lane5_mode: IntParam::new("Lane 5 Mode", 0, IntRange::Linear { min: 0, max: 39 }),
            lane6_mode: IntParam::new("Lane 6 Mode", 0, IntRange::Linear { min: 0, max: 39 }),
            lane7_mode: IntParam::new("Lane 7 Mode", 0, IntRange::Linear { min: 0, max: 39 }),
            lane8_mode: IntParam::new("Lane 8 Mode", 0, IntRange::Linear { min: 0, max: 39 }),
            theme: EnumParam::new("Theme", Theme::Neon),
            rate: EnumParam::new("Rate", Rate::Sixteenth),
            gate: FloatParam::new("Gate", 0.12, FloatRange::Linear { min: 0.02, max: 1.0 }),
            swing: FloatParam::new("Swing", 0.0, FloatRange::Linear { min: 0.0, max: 0.5 }),
            lfo1_enabled: BoolParam::new("LFO 1 Enabled", true),
            lfo1_wave: EnumParam::new("LFO 1 Wave", OscWave::Sine),
            lfo1_rate_mode: EnumParam::new("LFO 1 Rate Mode", LfoRateMode::Hz),
            lfo1_freq: hz_param("LFO 1 Frequency", 0.5),
            lfo1_rate: FloatParam::new("LFO 1 Rate (beats)", 4.0, FloatRange::Linear { min: 0.25, max: 16.0 }),
            lfo1_depth: FloatParam::new("LFO 1 Depth", 0.0, FloatRange::Linear { min: 0.0, max: LFO_MAX_DEPTH }),
            lfo1_phase: FloatParam::new("LFO 1 Phase", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            lfo1_polarity: EnumParam::new("LFO 1 Polarity", LfoPolarity::Bipolar),
            lfo1_links: AtomicU64::new(0),
            lfo2_enabled: BoolParam::new("LFO 2 Enabled", true),
            lfo2_wave: EnumParam::new("LFO 2 Wave", OscWave::Triangle),
            lfo2_rate_mode: EnumParam::new("LFO 2 Rate Mode", LfoRateMode::Hz),
            lfo2_freq: hz_param("LFO 2 Frequency", 0.25),
            lfo2_rate: FloatParam::new("LFO 2 Rate (beats)", 8.0, FloatRange::Linear { min: 0.25, max: 16.0 }),
            lfo2_depth: FloatParam::new("LFO 2 Depth", 0.0, FloatRange::Linear { min: 0.0, max: LFO_MAX_DEPTH }),
            lfo2_phase: FloatParam::new("LFO 2 Phase", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            lfo2_polarity: EnumParam::new("LFO 2 Polarity", LfoPolarity::Bipolar),
            lfo2_links: AtomicU64::new(0),
            lfo3_enabled: BoolParam::new("LFO 3 Enabled", true),
            lfo3_wave: EnumParam::new("LFO 3 Wave", OscWave::Saw),
            lfo3_rate_mode: EnumParam::new("LFO 3 Rate Mode", LfoRateMode::Hz),
            lfo3_freq: hz_param("LFO 3 Frequency", 1.0),
            lfo3_rate: FloatParam::new("LFO 3 Rate (beats)", 2.0, FloatRange::Linear { min: 0.25, max: 16.0 }),
            lfo3_depth: FloatParam::new("LFO 3 Depth", 0.0, FloatRange::Linear { min: 0.0, max: LFO_MAX_DEPTH }),
            lfo3_phase: FloatParam::new("LFO 3 Phase", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            lfo3_polarity: EnumParam::new("LFO 3 Polarity", LfoPolarity::Bipolar),
            lfo3_links: AtomicU64::new(0),
            lfo4_enabled: BoolParam::new("LFO 4 Enabled", true),
            lfo4_wave: EnumParam::new("LFO 4 Wave", OscWave::Square),
            lfo4_rate_mode: EnumParam::new("LFO 4 Rate Mode", LfoRateMode::Hz),
            lfo4_freq: hz_param("LFO 4 Frequency", 2.0),
            lfo4_rate: FloatParam::new("LFO 4 Rate (beats)", 1.0, FloatRange::Linear { min: 0.25, max: 16.0 }),
            lfo4_depth: FloatParam::new("LFO 4 Depth", 0.0, FloatRange::Linear { min: 0.0, max: LFO_MAX_DEPTH }),
            lfo4_phase: FloatParam::new("LFO 4 Phase", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            lfo4_polarity: EnumParam::new("LFO 4 Polarity", LfoPolarity::Bipolar),
            lfo4_links: AtomicU64::new(0),
            generate: BoolParam::new("Generate", false),
            invert: BoolParam::new("Invert Pattern", false),
            mirror: BoolParam::new("Mirror Pattern", false),
            logging: BoolParam::new("Technical Logging", false),
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

/// Effective feel/scheduling values for one note or block.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ScheduleValues {
    pub swing: f32,
    pub gate: f32,
    pub velocity: f32,
    pub regenerate_every: i32,
    pub randomize_after: i32,
}

impl DrumParams {
    /// Effective ceiling, clamped against stale host state.
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
            // Snap to `cells`; passing the raw value through would wrap mid-cell and drift.
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
            invert: self.invert.value(),
            mirror: self.mirror.value(),
            lane_modes: [
                self.lane1_mode.value().clamp(0, 39) as u8,
                self.lane2_mode.value().clamp(0, 39) as u8,
                self.lane3_mode.value().clamp(0, 39) as u8,
                self.lane4_mode.value().clamp(0, 39) as u8,
                self.lane5_mode.value().clamp(0, 39) as u8,
                self.lane6_mode.value().clamp(0, 39) as u8,
                self.lane7_mode.value().clamp(0, 39) as u8,
                self.lane8_mode.value().clamp(0, 39) as u8,
            ],
            markov_bias: self.markov_bias.value().clamp(0, 255) as u8,
            edit_rev: self.edit_overlay.revision(),
        }
    }


    /// One LFO's settings as a compact, copyable value.
    pub(crate) fn lfo_spec(&self, slot: usize) -> LfoSpec {
        match slot {
            0 => LfoSpec {
                enabled: self.lfo1_enabled.value(),
                wave: self.lfo1_wave.value(),
                rate_mode: self.lfo1_rate_mode.value(),
                freq_hz: self.lfo1_freq.value(),
                rate_beats: self.lfo1_rate.value(),
                depth: self.lfo1_depth.value(),
                phase: self.lfo1_phase.value(),
                polarity: self.lfo1_polarity.value(),
            },
            1 => LfoSpec {
                enabled: self.lfo2_enabled.value(),
                wave: self.lfo2_wave.value(),
                rate_mode: self.lfo2_rate_mode.value(),
                freq_hz: self.lfo2_freq.value(),
                rate_beats: self.lfo2_rate.value(),
                depth: self.lfo2_depth.value(),
                phase: self.lfo2_phase.value(),
                polarity: self.lfo2_polarity.value(),
            },
            2 => LfoSpec {
                enabled: self.lfo3_enabled.value(),
                wave: self.lfo3_wave.value(),
                rate_mode: self.lfo3_rate_mode.value(),
                freq_hz: self.lfo3_freq.value(),
                rate_beats: self.lfo3_rate.value(),
                depth: self.lfo3_depth.value(),
                phase: self.lfo3_phase.value(),
                polarity: self.lfo3_polarity.value(),
            },
            _ => LfoSpec {
                enabled: self.lfo4_enabled.value(),
                wave: self.lfo4_wave.value(),
                rate_mode: self.lfo4_rate_mode.value(),
                freq_hz: self.lfo4_freq.value(),
                rate_beats: self.lfo4_rate.value(),
                depth: self.lfo4_depth.value(),
                phase: self.lfo4_phase.value(),
                polarity: self.lfo4_polarity.value(),
            },
        }
    }

    pub(crate) fn lfo_signals(&self, clock: LfoClock) -> [f32; LFO_COUNT] {
        std::array::from_fn(|slot| modulation::lfo_signal(self.lfo_spec(slot), clock))
    }

    fn lfo_link_field(&self, slot: usize) -> &AtomicU64 {
        match slot {
            0 => &self.lfo1_links,
            1 => &self.lfo2_links,
            2 => &self.lfo3_links,
            _ => &self.lfo4_links,
        }
    }

    pub(crate) fn lfo_link_mask(&self, slot: usize) -> u64 {
        self.lfo_link_field(slot).load(Relaxed)
    }

    pub(crate) fn set_lfo_link(&self, slot: usize, target: ModTarget, linked: bool) {
        let field = self.lfo_link_field(slot);
        if linked {
            field.fetch_or(target_bit(target), Relaxed);
        } else {
            field.fetch_and(!target_bit(target), Relaxed);
        }
    }

    pub(crate) fn has_generation_links(&self) -> bool {
        (0..LFO_COUNT).any(|slot| {
            let mask = self.lfo_link_mask(slot);
            ModTarget::ALL
                .iter()
                .any(|target| target.affects_generation() && target_in_mask(mask, *target))
        })
    }

    fn target_base(&self, target: ModTarget) -> f32 {
        match target {
            ModTarget::Steps => self.steps.value(),
            ModTarget::Pulses => self.pulses.value() as f32,
            ModTarget::Rotate => self.rotate.value() as f32,
            ModTarget::Density => self.density.value(),
            ModTarget::Seed => self.seed.value(),
            ModTarget::OscPhase => self.phase.value(),
            ModTarget::OscRate => self.osc_rate.value(),
            ModTarget::CaRule => self.ca_rule.value() as f32,
            ModTarget::Probability => self.probability.value(),
            ModTarget::Swing => self.swing.value(),
            ModTarget::Gate => self.gate.value(),
            ModTarget::Velocity => self.velocity.value(),
            ModTarget::RegenerateEvery => self.regenerate_every.value() as f32,
            ModTarget::Symmetry => self.symmetry.value() as f32,
            ModTarget::RandomizeAfter => self.randomize_after_lengths.value() as f32,
            ModTarget::MaxSteps => self.max_steps.value() as f32,
        }
    }

    pub(crate) fn target_range(&self, target: ModTarget) -> (f32, f32) {
        match target {
            ModTarget::Steps => (1.0, self.step_ceiling() as f32),
            ModTarget::Pulses => (0.0, self.step_ceiling() as f32),
            ModTarget::Rotate => (0.0, (self.cell_count() - 1).max(0) as f32),
            ModTarget::Density
            | ModTarget::Seed
            | ModTarget::OscPhase
            | ModTarget::Probability => (0.0, 1.0),
            ModTarget::OscRate => (0.125, 8.0),
            ModTarget::CaRule => (0.0, 255.0),
            ModTarget::Swing => (0.0, 0.5),
            ModTarget::Gate => (0.02, 1.0),
            ModTarget::Velocity => (0.01, 1.0),
            ModTarget::RegenerateEvery | ModTarget::RandomizeAfter => (0.0, 256.0),
            ModTarget::Symmetry => (1.0, 8.0),
            ModTarget::MaxSteps => (MIN_MAX_STEPS as f32, MAX_STEPS as f32),
        }
    }

    /// Base value plus every linked LFO's signal, clamped to the target's range. Integer targets
    /// are rounded after summing so multiple LFOs combine predictably.
    pub(crate) fn target_value(&self, target: ModTarget, signals: &[f32; LFO_COUNT]) -> f32 {
        let base = self.target_base(target);
        let (min, max) = self.target_range(target);
        let mut offset = 0.0;
        for (slot, signal) in signals.iter().enumerate() {
            if target_in_mask(self.lfo_link_mask(slot), target) {
                offset += *signal;
            }
        }
        let value = fold(base, offset, min, max);
        if target.is_int() { value.round().clamp(min, max) } else { value }
    }

    /// Effective generation snapshot at a clock position. Only pattern-shaping links change the
    /// generated line; feel targets are still filled in so previews see one snapshot.
    pub(crate) fn gen_params_at(&self, clock: LfoClock) -> GenParams {
        let signals = self.lfo_signals(clock);
        let cap = self
            .target_value(ModTarget::MaxSteps, &signals)
            .round()
            .clamp(MIN_MAX_STEPS as f32, MAX_STEPS as f32) as usize;
        let raw = self.target_value(ModTarget::Steps, &signals).clamp(1.0, cap as f32);
        let snap = self.round_steps.value();
        let cells = cell_count_for(raw, snap, cap);
        let max_rotate = (cells - 1).max(0) as f32;
        let mut out = self.gen_params();
        out.steps = cells as usize;
        out.length = if snap { cells as f32 } else { raw };
        out.pulses = self
            .target_value(ModTarget::Pulses, &signals)
            .round()
            .clamp(0.0, cap as f32) as usize;
        out.rotate = self
            .target_value(ModTarget::Rotate, &signals)
            .round()
            .clamp(0.0, max_rotate) as usize;
        out.density = self.target_value(ModTarget::Density, &signals);
        out.probability = self.target_value(ModTarget::Probability, &signals);
        out.velocity = self.target_value(ModTarget::Velocity, &signals);
        out.seed = self.target_value(ModTarget::Seed, &signals);
        out.phase = self.target_value(ModTarget::OscPhase, &signals);
        out.osc_rate = self.target_value(ModTarget::OscRate, &signals);
        out.ca_rule = self.target_value(ModTarget::CaRule, &signals).round().clamp(0.0, 255.0) as u8;
        out.symmetry = self
            .target_value(ModTarget::Symmetry, &signals)
            .round()
            .clamp(1.0, 8.0) as usize;
        out
    }

    pub(crate) fn schedule_at(&self, clock: LfoClock) -> ScheduleValues {
        let signals = self.lfo_signals(clock);
        ScheduleValues {
            swing: self.target_value(ModTarget::Swing, &signals),
            gate: self.target_value(ModTarget::Gate, &signals),
            velocity: self.target_value(ModTarget::Velocity, &signals),
            regenerate_every: self.target_value(ModTarget::RegenerateEvery, &signals).round() as i32,
            randomize_after: self.target_value(ModTarget::RandomizeAfter, &signals).round() as i32,
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
/// tests exercise the real rule instead of comparing an inlined copy against itself.
fn cell_count_for(raw: f32, snap: bool, ceiling: usize) -> i32 {
    let cap = ceiling.clamp(MIN_MAX_STEPS, MAX_STEPS) as f32;
    let raw = raw.clamp(1.0, cap);
    (if snap { raw.round() } else { raw.ceil() }).clamp(1.0, cap) as i32
}

/// Lock-free signals shared between the audio scheduler and the editor. Auto-randomization is
/// requested on the audio thread and applied through `ParamSetter` on the editor thread.
pub struct Shared {
    pub current_step: AtomicI32,
    pub randomize_request: AtomicU32,
    pub regen_pulse: AtomicBool,
    /// Last host tempo as `f32` bits, for the editor-side MIDI export. Zero means "not seen yet".
    pub tempo_bits: AtomicU32,
    /// Current transport/free-run beat as `f64` bits, for LFO values and animation.
    pub last_beat: AtomicU64,
    /// Beat at which the current pattern was generated, for preview parity.
    pub pattern_beat: AtomicU64,
    /// Normalized phase of each LFO at the start of the last processed block.
    pub lfo_phase: [AtomicU32; LFO_COUNT],
    /// True while the host transport or free-run clock is advancing the LFOs.
    pub lfo_running: AtomicBool,
    /// Name of the DAW track, empty when the host does not say. Written on the host main thread
    /// (`initialize` / `track_context_changed`). The editor must not call
    /// `GuiContext::track_name()` itself: the CLAP wrapper answers from the calling thread, and
    /// the editor tick is not the CLAP main thread.
    pub track_name: Mutex<String>,
    /// Bumped whenever `track_name` changes, so the editor only re-reads it when needed.
    pub track_rev: AtomicU32,
    /// True while a text field in the editor has keyboard focus. Published by the editor tick,
    /// read by `Editor::on_virtual_key_from_host` to decide whether to claim a host-stolen key.
    pub text_focus: AtomicBool,
    /// Host-forwarded editing keys (`true` = pressed); filled on the host thread, drained by the editor.
    pub key_queue: Mutex<Vec<(keys::EditKey, bool)>>,
}
impl Default for Shared {
    fn default() -> Self {
        Self {
            current_step: AtomicI32::new(-1),
            randomize_request: AtomicU32::new(0),
            regen_pulse: AtomicBool::new(false),
            tempo_bits: AtomicU32::new(0),
            last_beat: AtomicU64::new(0),
            pattern_beat: AtomicU64::new(0),
            lfo_phase: std::array::from_fn(|_| AtomicU32::new(0)),
            lfo_running: AtomicBool::new(false),
            track_name: Mutex::new(String::new()),
            track_rev: AtomicU32::new(0),
            text_focus: AtomicBool::new(false),
            key_queue: Mutex::new(Vec::new()),
        }
    }
}
impl Shared {
    /// Record the host's track name. Returns without a bump when nothing changed.
    pub fn set_track_name(&self, name: Option<String>) {
        let name = name.map(|n| n.trim().to_string()).unwrap_or_default();
        if let Ok(mut current) = self.track_name.lock() {
            if *current != name {
                *current = name;
                self.track_rev.fetch_add(1, Relaxed);
            }
        }
    }

    pub(crate) fn sample_tempo(&self) -> f32 {
        f32::from_bits(self.tempo_bits.load(Relaxed))
    }

    /// Clock for live matrix/animation values.
    pub(crate) fn lfo_clock(&self) -> LfoClock {
        LfoClock::from_beat(
            f64::from_bits(self.last_beat.load(Relaxed)),
            self.sample_tempo() as f64,
        )
    }

    /// Clock at which the current pattern was generated. Preview generation uses this so it
    /// matches the loop-boundary snapshot the audio thread actually plays.
    pub(crate) fn pattern_clock(&self) -> LfoClock {
        LfoClock::from_beat(
            f64::from_bits(self.pattern_beat.load(Relaxed)),
            self.sample_tempo() as f64,
        )
    }
}

fn regeneration_cycle_at(step_index: i64, loop_length: f64, every: i64, last: Option<i64>) -> Option<i64> {
    if !loop_length.is_finite() || loop_length <= 0.0 || every <= 0 { return None; }
    // Count complete loop lengths, not grid cells: a 16.5-step loop wraps between cells 16 and
    // 17, so its first complete cycle starts at step 17, not at step 16.
    let cycle = (step_index as f64 / loop_length).floor() as i64;
    if cycle > 0 && cycle.rem_euclid(every) == 0 && last != Some(cycle) { Some(cycle) } else { None }
}

/// Grid cell for step `k` in a loop of `length` steps. Fractional lengths wrap mid-cell, so each
/// cycle starts later and the pattern drifts against the grid.
fn step_index(k: i64, length: f64) -> i64 {
    if length >= 1.0 && length.fract() == 0.0 { k.rem_euclid(length as i64) } else { (k as f64).rem_euclid(length).floor() as i64 }
}

/// Where the step cursor resumes in this block. A carried cursor is trusted only when it is
/// plausibly next: one the clock has passed is dropped, as is one far ahead after a slower rate
/// change (step 400 at 1/16 is beat 100, but at 1/4 it is beat 400).
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

pub struct NrumSeq {
    params: Arc<DrumParams>,
    shared: Arc<Shared>,
    /// Actual snapshot the current pattern was generated from (including modulation).
    last_key: Option<GenParams>,
    /// Unmodulated snapshot used to detect manual parameter edits.
    last_base_key: Option<GenParams>,
    pattern: Pattern,
    /// Plain mirror of the manual edit store, refreshed when its revision changes.
    edit_view: EditView,
    last_edit_rev: u32,
    /// Loop length in steps, from the same snapshot as `pattern`.
    loop_length: f64,
    last_generate: Option<bool>,
    active: [Option<Active>; TRACKS],
    expected_beat: Option<f64>,
    /// Next step index to schedule, carried across blocks so swing cannot drop a step.
    next_step: Option<i64>,
    /// Substep within the carried step for ratchets (0 when ratchets are off).
    next_sub: u8,
    /// Number of ratchet subdivisions used by the current step (1 or 12).
    subdivisions: u8,
    /// Ratchet count per lane for the current step, read from the edit overlay.
    ratchets: [u8; TRACKS],
    /// Step-wide microtiming delay in beats for the carried step.
    step_delay: f64,
    /// Set by `reset`, which has no `ProcessContext` to send NoteOffs with. The next
    /// `process` flushes them, so a host transport reset cannot leave notes hanging.
    pending_reset: bool,
    free_beat: f64,
    last_regen_cycle: Option<i64>,
    last_randomize_cycle: Option<i64>,
    /// Last loop cycle for which pattern-shaping LFO links were sampled.
    last_mod_cycle: Option<i64>,
    sample_rate: f64,
}
impl Default for NrumSeq {
    fn default() -> Self {
        Self {
            params: Arc::new(DrumParams::default()), shared: Arc::new(Shared::default()),
            last_key: None, last_base_key: None, pattern: Pattern::default(),
            edit_view: EditView::default(), last_edit_rev: 0, loop_length: 16.0, last_generate: None,
            active: [None; TRACKS], expected_beat: None, next_step: None, next_sub: 0,
            subdivisions: 1, ratchets: [1; TRACKS], step_delay: 0.0, pending_reset: false, free_beat: 0.0,
            last_regen_cycle: None, last_randomize_cycle: None, last_mod_cycle: None, sample_rate: 44100.0,
        }
    }
}

impl NrumSeq {
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
    pub const AUTHOR: &str = "ccne4mocc / feN";
    pub const GITHUB: &str = "https://github.com/ccne4mocc";
    pub const SUPPORT: &str = "";
    pub const EMAIL: &str = "";
}

impl Plugin for NrumSeq {
    const NAME: &'static str = "nrum seq";
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
    fn initialize(&mut self, _layout: &AudioIOLayout, config: &BufferConfig, context: &mut impl InitContext<Self>) -> bool {
        self.sample_rate = config.sample_rate as f64;
        // Some hosts report the track here, others only afterwards through `track_context_changed`.
        self.shared.set_track_name(context.track_name());
        true
    }
    fn track_context_changed(&mut self, context: &mut impl InitContext<Self>) {
        // Called on the host main thread, e.g. when the user renames the track.
        self.shared.set_track_name(context.track_name());
    }
    fn reset(&mut self) {
        // nice-plug hands `reset` no ProcessContext, so NoteOffs cannot be sent from here.
        // `active` is deliberately left intact and flushed at the top of the next `process`;
        // clearing it here would drop the note numbers and leave every sounding note stuck.
        self.pending_reset = self.active.iter().any(|a| a.is_some());
        self.expected_beat = None;
        self.next_step = None;
        self.next_sub = 0;
        self.subdivisions = 1;
        self.ratchets = [1; TRACKS];
        self.step_delay = 0.0;
        self.free_beat = 0.0;
        self.last_regen_cycle = None;
        self.last_randomize_cycle = None;
        self.last_mod_cycle = None;
        self.last_key = None;
        self.last_base_key = None;
        self.edit_view = EditView::default();
        self.last_edit_rev = 0;
        self.loop_length = 16.0;
        self.last_generate = None;
        self.shared.current_step.store(-1, Relaxed);
        self.shared.lfo_running.store(false, Relaxed);
    }

    fn process(&mut self, buffer: &mut Buffer, _aux: &mut AuxiliaryBuffers, context: &mut impl ProcessContext<Self>) -> ProcessStatus {
        let p = self.params.clone();
        let edit_rev = p.edit_overlay.revision();
        if edit_rev != self.last_edit_rev {
            p.edit_overlay.fill(&mut self.edit_view);
            self.last_edit_rev = edit_rev;
        }
        if self.pending_reset {
            self.pending_reset = false;
            self.stop_all(context);
        }
        while context.next_event().is_some() {} // This is a MIDI generator, not an input passthrough.

        let generated = p.generate.value();
        if matches!(self.last_generate, Some(prev) if prev != generated) { p.variation.fetch_add(1, Relaxed); }
        self.last_generate = Some(generated);

        let n_samples = buffer.samples();
        let transport = context.transport();
        let playing = transport.playing;
        // Track the host tempo exactly; clamping it would desynchronise `expected_beat` from
        // `pos_beats` and make the jump detector fire on our own drift. Only reject zero/non-finite.
        let bpm = transport.tempo.filter(|t| t.is_finite() && *t > 0.0).unwrap_or(120.0);
        self.shared.tempo_bits.store((bpm as f32).to_bits(), Relaxed);
        let beats_per_sample = (bpm / 60.0) / self.sample_rate.max(1.0);
        let block_beats = n_samples as f64 * beats_per_sample;
        let start = if playing {
            self.free_beat = 0.0;
            // Hosts that omit the position would pin every block to beat 0; keep our own clock.
            Some(transport.pos_beats().or(self.expected_beat).unwrap_or(0.0))
        } else if p.free_run.value() { Some(self.free_beat) } else { None };
        let Some(start) = start else {
            self.stop_all(context);
            self.expected_beat = None;
            self.next_step = None;
            self.shared.current_step.store(-1, Relaxed);
            self.shared.lfo_running.store(false, Relaxed);
            return ProcessStatus::Normal;
        };

        let clock_start = LfoClock::from_beat(start, bpm);
        self.shared.last_beat.store(start.to_bits(), Relaxed);
        self.shared.lfo_running.store(true, Relaxed);
        for slot in 0..LFO_COUNT {
            self.shared.lfo_phase[slot]
                .store(modulation::lfo_phase(p.lfo_spec(slot), clock_start).to_bits(), Relaxed);
        }

        // Base parameters drive change detection; LFO-only movement is sampled at loop boundaries.
        let base_key = p.gen_params();
        if self.last_base_key != Some(base_key) {
            let initial_key = p.gen_params_at(clock_start);
            self.pattern = generate_with(&initial_key, &self.edit_view);
            self.loop_length = initial_key.length as f64;
            self.last_key = Some(initial_key);
            self.last_base_key = Some(base_key);
            self.last_regen_cycle = None;
            self.last_randomize_cycle = None;
            self.last_mod_cycle = None;
            self.shared.pattern_beat.store(start.to_bits(), Relaxed);
        }

        let end = start + block_beats;
        // Block-relative tolerance: sample-grid quantisation and inter-block resync are not jumps.
        let jump_tolerance = 0.02_f64.max(block_beats * 2.0);
        let transport_jump = self.expected_beat.map(|e| (start - e).abs() > jump_tolerance).unwrap_or(false);
        if transport_jump {
            self.stop_all(context);
            self.last_regen_cycle = None;
            self.last_randomize_cycle = None;
            self.last_mod_cycle = None;
        }
        self.expected_beat = Some(end);
        if !playing { self.free_beat = end; }

        let timing = |beat: f64| -> u32 {
            (((beat - start) / beats_per_sample).round().max(0.0) as usize).min(n_samples.saturating_sub(1)) as u32
        };
        let spb = p.rate.value().steps_per_beat();
        let step_len = 1.0 / spb;
        let s1 = end * spb;
        // A swung step can fall beyond this block; carry the cursor instead of re-deriving it.
        let schedule_start = p.schedule_at(clock_start);
        let every = schedule_start.regenerate_every.max(0) as i64;
        let randomize_every = schedule_start.randomize_after.max(0) as i64;
        let carried_step = self.next_step;
        let mut k = resume_cursor(carried_step, transport_jump, start, end, spb);
        let mut sub = if carried_step == Some(k) { self.next_sub } else { 0 };
        if self.subdivisions == 0 { self.subdivisions = 1; }
        if self.last_mod_cycle.is_none() {
            self.last_mod_cycle = Some((k as f64 / self.loop_length).floor() as i64);
        }

        loop {
            let nominal_beat = k as f64 * step_len;
            let nominal_clock = LfoClock::from_beat(nominal_beat, bpm);
            let swing = p.schedule_at(nominal_clock).swing.clamp(0.0, 0.5) as f64 * step_len;
            let sub_len = step_len / self.subdivisions.max(1) as f64;
            if sub == 0 {
                // Microtiming is step-wide: the editor writes the same offset to every lane so
                // the scheduler can keep one onset per step. Negative values are accepted by the
                // store but ignored until a lookahead scheduler exists.
                let idx = step_index(k, self.loop_length) as usize;
                let idx = idx.min(self.pattern.steps.saturating_sub(1));
                let offset = self.edit_view.offset[0][idx].max(0) as f64 / 127.0;
                self.step_delay = (offset * step_len).min(step_len * 0.9);
            }
            let next_step = if (k as f64) < s1 + 1.0 {
                nominal_beat
                    + if k.rem_euclid(2) != 0 { swing } else { 0.0 }
                    + self.step_delay
                    + sub as f64 * sub_len
            } else {
                f64::INFINITY
            };
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
                if sub == 0 {
                    // Complete pattern lengths, not host beats, like regeneration. The request is
                    // lock-free here; the editor applies it through ParamSetter on its next tick.
                    if let Some(cycle) = regeneration_cycle_at(k, self.loop_length, randomize_every, self.last_randomize_cycle) {
                        self.shared.randomize_request.fetch_add(1, Relaxed);
                        self.last_randomize_cycle = Some(cycle);
                    }

                    // Sample pattern-shaping LFO links once per loop, using the current loop length.
                    let mod_cycle = (k as f64 / self.loop_length).floor() as i64;
                    let mod_due = p.has_generation_links() && self.last_mod_cycle != Some(mod_cycle);
                    let regen_cycle = regeneration_cycle_at(k, self.loop_length, every, self.last_regen_cycle);
                    if mod_due || regen_cycle.is_some() {
                        if let Some(cycle) = regen_cycle {
                            p.variation.fetch_add(1, Relaxed);
                            self.last_regen_cycle = Some(cycle);
                            self.shared.regen_pulse.store(true, Relaxed);
                        }
                        let next_key = p.gen_params_at(nominal_clock);
                        self.pattern = generate_with(&next_key, &self.edit_view);
                        self.loop_length = next_key.length as f64;
                        self.last_key = Some(next_key);
                        // Regenerate-after-N bumps `variation`; refresh the base snapshot to avoid a false manual change.
                        self.last_base_key = Some(p.gen_params());
                        self.last_mod_cycle = Some((k as f64 / self.loop_length).floor() as i64);
                        self.shared.pattern_beat.store(nominal_beat.to_bits(), Relaxed);
                    }
                }

                // Regeneration can change the step count; clamp the active index before reading hits.
                let idx = step_index(k, self.loop_length) as usize;
                let idx = idx.min(self.pattern.steps.saturating_sub(1));
                if sub == 0 {
                    self.shared.current_step.store(idx as i32, Relaxed);
                    // Read this step's ratchet counts once; the scheduler uses 12 subdivisions
                    // only when at least one lane actually ratchets.
                    self.subdivisions = 1;
                    self.ratchets = [1; TRACKS];
                    for tr in 0..TRACKS {
                        let count = self.edit_view.ratchet_at(tr, idx);
                        self.ratchets[tr] = count;
                        if count > 1 { self.subdivisions = 12; }
                    }
                }
                let note_clock = LfoClock::from_beat(next_step, bpm);
                let schedule_note = p.schedule_at(note_clock);
                for tr in 0..TRACKS {
                    if !p.track_enabled(tr) || !self.pattern.hit[tr][idx] { continue; }
                    let ratchet = self.ratchets[tr].clamp(1, 4);
                    // With 12 subdivisions every count 1..4 divides evenly: 12/3 = 4, 12/4 = 3.
                    if sub as u32 % (12 / ratchet as u32) != 0 { continue; }
                    let t = timing(next_step);
                    if let Some(a) = self.active[tr].take() {
                        context.send_event(NoteEvent::NoteOff { timing: t, voice_id: None, channel: 9, note: a.note, velocity: 0.0 });
                    }
                    let note = p.track_note(tr);
                    let velocity = (p.track_velocity(tr) * schedule_note.velocity * self.pattern.accent[tr][idx]).clamp(0.01, 1.0);
                    context.send_event(NoteEvent::NoteOn { timing: t, voice_id: None, channel: 9, note, velocity });
                    // A ratchet must not overlap the next substep; otherwise one lane would need
                    // polyphony. Gate is therefore capped to its own substep spacing.
                    let lane_step = step_len / ratchet as f64;
                    let gate = (schedule_note.gate as f64 * step_len).min(lane_step * 0.9);
                    self.active[tr] = Some(Active { note, off_beat: next_step + gate });
                }
                if sub + 1 < self.subdivisions {
                    sub += 1;
                } else {
                    sub = 0;
                    k += 1;
                }
            }
        }
        // Carry the unplayed step and substep into the next block; a swung or ratcheted event may
        // not have been reached.
        self.next_step = Some(k);
        self.next_sub = sub;
        ProcessStatus::Normal
    }
}

impl ClapPlugin for NrumSeq {
    const CLAP_ID: &'static str = "com.nrumseq.nrumseq";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Algorithmic percussion MIDI sequencer with eight drum lanes");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(contact::GITHUB);
    const CLAP_SUPPORT_URL: Option<&'static str> = Some(contact::SUPPORT);
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::NoteEffect, ClapFeature::Utility];
}
impl Vst3Plugin for NrumSeq {
    const VST3_CLASS_ID: [u8; 16] = *b"nrum-seq\0\0\0\0\0\0\0\0";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Instrument, Vst3SubCategory::Tools];
}
nice_export_clap!(NrumSeq);
nice_export_vst3!(NrumSeq);

/// Test-only allocation counter shared by every module's tests. There can only be one
/// `#[global_allocator]` per test binary, so it lives here instead of inside one test module.
/// The counter is thread-local so parallel tests cannot leak allocations into each other.
#[cfg(test)]
pub(crate) mod test_alloc {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;

    thread_local! {
        static ALLOCS: Cell<usize> = const { Cell::new(0) };
    }

    pub(crate) struct Counting;
    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let _ = ALLOCS.try_with(|count| count.set(count.get() + 1));
            System.alloc(layout)
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) { System.dealloc(ptr, layout) }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            let _ = ALLOCS.try_with(|count| count.set(count.get() + 1));
            System.realloc(ptr, layout, new_size)
        }
    }

    #[global_allocator]
    static ALLOCATOR: Counting = Counting;

    pub(crate) fn reset() { ALLOCS.with(|count| count.set(0)); }
    pub(crate) fn count() -> usize { ALLOCS.with(|count| count.get()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nice_plug::prelude::{Param, ProcessContext};
    use nice_plug::params::InternalParamMut;

    /// A transport-aware host stub for scheduler tests. It records every event with its absolute
    /// sample position so block-boundary and timing assertions can compare against the host grid.
    struct MockContext {
        transport: Transport,
        events: Vec<(u64, PluginNoteEvent<NrumSeq>)>,
        total_samples: u64,
        /// Off during the allocation test: recording an event would allocate and hide a real
        /// allocation in the plug-in.
        record: bool,
        /// Counts every event even when `record` is off, so stress tests stay allocation-free.
        sent: u64,
    }

    impl MockContext {
        fn new(sample_rate: f32) -> Self {
            let mut transport = Transport::new(sample_rate);
            transport.playing = true;
            transport.tempo = Some(120.0);
            transport.pos_beats = Some(0.0);
            Self { transport, events: Vec::new(), total_samples: 0, record: true, sent: 0 }
        }

        fn set_recording(&mut self, record: bool) { self.record = record; }

        fn process(&mut self, plugin: &mut NrumSeq, samples: usize) {
            let mut buffer = Buffer::default();
            // MIDI-only plug-in: no output channels, but the sample count still drives the grid.
            unsafe { buffer.set_slices(samples, |_| {}) };
            let mut inputs: [Buffer<'_>; 0] = [];
            let mut outputs: [Buffer<'_>; 0] = [];
            let mut aux = AuxiliaryBuffers { inputs: &mut inputs, outputs: &mut outputs };
            let _ = plugin.process(&mut buffer, &mut aux, self);
            self.total_samples += samples as u64;
        }

        fn note_ons(&self) -> Vec<(u64, u8, f32)> {
            self.events
                .iter()
                .filter_map(|(at, event)| match event {
                    NoteEvent::NoteOn { note, velocity, .. } => Some((*at, *note, *velocity)),
                    _ => None,
                })
                .collect()
        }

        fn note_offs(&self) -> Vec<(u64, u8)> {
            self.events
                .iter()
                .filter_map(|(at, event)| match event {
                    NoteEvent::NoteOff { note, .. } => Some((*at, *note)),
                    _ => None,
                })
                .collect()
        }
    }

    impl ProcessContext<NrumSeq> for MockContext {
        fn plugin_api(&self) -> PluginApi { PluginApi::Clap }
        fn execute_background(&self, _task: ()) {}
        fn execute_gui(&self, _task: ()) {}
        fn transport(&self) -> &Transport { &self.transport }
        fn next_event(&mut self) -> Option<PluginNoteEvent<NrumSeq>> { None }
        fn send_event(&mut self, event: PluginNoteEvent<NrumSeq>) {
            self.sent += 1;
            if !self.record { return; }
            let at = self.total_samples + event.timing() as u64;
            self.events.push((at, event));
        }
        fn set_latency_samples(&self, _samples: u32) {}
        fn set_current_voice_capacity(&self, _capacity: u32) {}
    }

    fn set_param<P: Param + InternalParamMut>(param: &P, value: P::Plain) {
        unsafe { param._internal_set_plain_value(value); }
    }

    /// A pattern with a hit on every step: oscillator top-k at full density. Used where a test
    /// needs consecutive retriggers rather than a sparse Euclidean line.
    fn every_step_plugin() -> (NrumSeq, MockContext) {
        let mut plugin = NrumSeq::default();
        plugin.sample_rate = 44_100.0;
        set_param(&plugin.params.mode, Mode::Oscillator);
        set_param(&plugin.params.steps, 16.0);
        set_param(&plugin.params.pulses, 16);
        set_param(&plugin.params.density, 1.0);
        set_param(&plugin.params.probability, 1.0);
        set_param(&plugin.params.gate, 1.0);
        set_param(&plugin.params.swing, 0.0);
        (plugin, MockContext::new(44_100.0))
    }

    fn run_playing(plugin: &mut NrumSeq, ctx: &mut MockContext, blocks: usize, block_samples: usize) {
        let bpm = ctx.transport.tempo.unwrap();
        let sr = ctx.transport.sample_rate as f64;
        let beats_per_sample = (bpm / 60.0) / sr;
        for _ in 0..blocks {
            ctx.process(plugin, block_samples);
            if let Some(pos) = ctx.transport.pos_beats.as_mut() {
                *pos += block_samples as f64 * beats_per_sample;
            }
        }
    }

    #[test]
    fn scheduler_places_hits_on_the_host_step_grid() {
        let (mut plugin, mut ctx) = every_step_plugin();
        // 1/16 at 120 BPM is 5512.5 samples per step. Run just over four steps.
        run_playing(&mut plugin, &mut ctx, 48, 512);
        let ons = ctx.note_ons();
        assert!(!ons.is_empty(), "the scheduler emitted no notes");
        let samples_per_step = 5512.5;
        for (at, note, velocity) in &ons {
            let step = (*at as f64 / samples_per_step).round();
            assert!((*at as f64 - step * samples_per_step).abs() <= 1.0,
                "note {note} at sample {at} is not on a step boundary");
            assert!((0.0..=1.0).contains(velocity), "velocity {velocity} is out of range");
        }
    }

    #[test]
    fn one_sample_blocks_do_not_drop_or_duplicate_steps() {
        let (mut plugin, mut ctx) = every_step_plugin();
        // One step is 5512.5 samples; run just under two full steps in one-sample blocks.
        run_playing(&mut plugin, &mut ctx, 11_000, 1);
        let kicks: Vec<u64> = ctx
            .note_ons()
            .into_iter()
            .filter(|(_, note, _)| *note == 36)
            .map(|(at, _, _)| at)
            .collect();
        assert_eq!(kicks.len(), 2, "expected the kick on the first two steps, got {kicks:?}");
        assert!(kicks[0] <= 1, "kick is not on the downbeat: {kicks:?}");
        assert!((kicks[1] as f64 - 5512.5).abs() <= 1.0,
            "second step is not on the next grid cell: {kicks:?}");
    }

    #[test]
    fn swing_delays_only_the_odd_steps() {
        let (mut plugin, mut ctx) = every_step_plugin();
        set_param(&plugin.params.swing, 0.5);
        run_playing(&mut plugin, &mut ctx, 48, 512);
        let ons = ctx.note_ons();
        assert!(!ons.is_empty());
        // Every onset must sit on an even step or half a step after an odd one.
        for (at, _, note) in &ons {
            let nearest = (0..8)
                .flat_map(|k| [k as f64 * 5512.5, k as f64 * 5512.5 + 2756.25])
                .map(|expect| (expect - *at as f64).abs())
                .fold(f64::INFINITY, f64::min);
            assert!(nearest <= 1.5, "note {note} at {at} is off the swung grid ({nearest} samples)");
        }
        assert!(ons.iter().any(|(at, _, _)| (*at as f64 - 8268.75).abs() <= 1.5),
            "the odd step was not delayed by half a step");
        assert!(!ons.iter().any(|(at, _, _)| (*at as f64 - 5512.5).abs() <= 1.5),
            "an odd step leaked onto the straight grid");
    }

    #[test]
    fn retrigger_sends_note_off_before_the_next_note_on() {
        let (mut plugin, mut ctx) = every_step_plugin();
        run_playing(&mut plugin, &mut ctx, 48, 512);
        // Walk the kick events in order. A note-on while the previous kick is still active would
        // either overlap or steal the voice, so the generator must close it first.
        let mut active = false;
        let mut ons = 0usize;
        for (at, event) in &ctx.events {
            match event {
                NoteEvent::NoteOn { note: 36, .. } => {
                    assert!(!active, "kick retriggered at {at} while the previous note was active");
                    active = true;
                    ons += 1;
                }
                NoteEvent::NoteOff { note: 36, .. } => active = false,
                _ => {}
            }
        }
        assert!(ons >= 2, "the test needs at least two kick notes, got {ons}");
    }

    #[test]
    fn stop_flushes_every_active_note() {
        let (mut plugin, mut ctx) = every_step_plugin();
        run_playing(&mut plugin, &mut ctx, 2, 512);
        assert!(!ctx.note_ons().is_empty());
        let sounding: Vec<u8> = ctx.note_ons().iter().map(|(_, note, _)| *note).collect();
        assert!(!sounding.is_empty());
        ctx.transport.playing = false;
        ctx.process(&mut plugin, 512);
        let offs = ctx.note_offs();
        for note in sounding {
            assert!(offs.iter().any(|(_, n)| *n == note), "stop did not flush note {note}");
        }
    }

    #[test]
    fn reset_flushes_notes_on_the_next_process_call() {
        let (mut plugin, mut ctx) = every_step_plugin();
        run_playing(&mut plugin, &mut ctx, 2, 512);
        plugin.reset();
        ctx.transport.playing = true;
        ctx.process(&mut plugin, 512);
        assert!(!ctx.note_offs().is_empty(), "reset left notes hanging");
    }

    #[test]
    fn a_transport_jump_flushes_notes_before_resuming() {
        let (mut plugin, mut ctx) = every_step_plugin();
        run_playing(&mut plugin, &mut ctx, 2, 512);
        let before = ctx.events.len();
        ctx.transport.pos_beats = Some(64.0);
        ctx.process(&mut plugin, 512);
        let after = &ctx.events[before..];
        assert!(after.iter().any(|(_, e)| matches!(e, NoteEvent::NoteOff { .. })),
            "a jump did not flush active notes");
        let first_off = after.iter().position(|(_, e)| matches!(e, NoteEvent::NoteOff { .. })).unwrap();
        let first_on = after.iter().position(|(_, e)| matches!(e, NoteEvent::NoteOn { .. }));
        if let Some(first_on) = first_on {
            assert!(first_off < first_on, "the jump resumed before flushing");
        }
    }

    #[test]
    fn lane_disable_suppresses_only_that_lane() {
        let (mut plugin, mut ctx) = every_step_plugin();
        set_param(&plugin.params.kick_on, false);
        run_playing(&mut plugin, &mut ctx, 12, 512);
        let notes: Vec<u8> = ctx.note_ons().iter().map(|(_, n, _)| *n).collect();
        assert!(!notes.contains(&36), "disabled kick still emitted note 36");
        assert!(notes.contains(&38), "snare should still be enabled");
    }

    #[test]
    fn one_step_patterns_still_emit_a_note() {
        let (mut plugin, mut ctx) = every_step_plugin();
        set_param(&plugin.params.steps, 1.0);
        set_param(&plugin.params.pulses, 1);
        run_playing(&mut plugin, &mut ctx, 8, 512);
        assert!(!ctx.note_ons().is_empty(), "a one-step pattern was silent");
    }

    #[test]
    fn process_never_allocates() {
        // Invariant #1 covers the whole audio callback, not just `sequence::generate()`. The mock
        // keeps recording off so its own event log cannot allocate during the measurement.
        for kind in Mode::all() {
            let (mut plugin, mut ctx) = every_step_plugin();
            set_param(&plugin.params.mode, kind);
            set_param(&plugin.params.steps, 16.0);
            set_param(&plugin.params.pulses, 5);
            set_param(&plugin.params.density, 0.65);
            set_param(&plugin.params.probability, 1.0);
            ctx.set_recording(false);
            // First block generates the initial pattern and primes the LFO/regeneration state.
            run_playing(&mut plugin, &mut ctx, 1, 64);
            crate::test_alloc::reset();
            run_playing(&mut plugin, &mut ctx, 16, 64);
            let used = crate::test_alloc::count();
            assert_eq!(used, 0, "process() allocated {used} times with {kind:?} on the audio thread");
        }
    }

    #[test]
    fn ratchets_distribute_substeps_within_the_step() {
        for (count, spacing) in [(2u8, 2756.25f64), (3, 1837.5), (4, 1378.125)] {
            let (mut plugin, mut ctx) = every_step_plugin();
            plugin.params.edit_overlay.set_ratchet(0, 0, count);
            run_playing(&mut plugin, &mut ctx, 48, 512);
            let kicks: Vec<u64> = ctx
                .note_ons()
                .into_iter()
                .filter(|(_, note, _)| *note == 36)
                .take(count as usize)
                .map(|(at, _, _)| at)
                .collect();
            assert_eq!(kicks.len(), count as usize, "expected {count} kick onsets, got {kicks:?}");
            for (j, at) in kicks.iter().enumerate() {
                let expected = j as f64 * spacing;
                assert!((*at as f64 - expected).abs() <= 1.5,
                    "ratchet {count}, substep {j}: {at} != {expected}");
            }
            // Every ratchet note must be closed before the next one starts.
            let mut active = false;
            for (at, event) in &ctx.events {
                match event {
                    NoteEvent::NoteOn { note: 36, .. } => {
                        assert!(!active, "ratchet retriggered at {at} while the previous note was active");
                        active = true;
                    }
                    NoteEvent::NoteOff { note: 36, .. } => active = false,
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn microtiming_delays_the_step_without_moving_the_grid() {
        let (mut plugin, mut ctx) = every_step_plugin();
        plugin.params.edit_overlay.set_offset(0, 0, 64);
        run_playing(&mut plugin, &mut ctx, 48, 512);
        let kicks: Vec<u64> = ctx
            .note_ons()
            .into_iter()
            .filter(|(_, note, _)| *note == 36)
            .map(|(at, _, _)| at)
            .collect();
        assert!(!kicks.iter().any(|at| *at <= 1), "the delayed step still sounded on the grid: {kicks:?}");
        let expected = 64.0 / 127.0 * 5512.5;
        assert!(kicks.iter().any(|at| (*at as f64 - expected).abs() <= 1.5),
            "the delayed kick is missing: {kicks:?}, expected ≈{expected}");
        // The next step is not delayed, so the loop still lands on its nominal grid.
        assert!(kicks.iter().any(|at| (*at as f64 - 5512.5).abs() <= 1.5),
            "the following step moved with the delayed one: {kicks:?}");
    }

    #[test]
    fn manual_step_edits_reach_the_scheduler() {
        let (mut plugin, mut ctx) = every_step_plugin();
        // Force the kick off on the downbeat and back on at step 2, where the same pattern also
        // has a kick. The revision must force a regeneration before the scheduler reads hits.
        plugin.params.edit_overlay.set_hit(0, 0, crate::edits::HIT_OFF);
        plugin.params.edit_overlay.set_hit(0, 2, crate::edits::HIT_ON);
        run_playing(&mut plugin, &mut ctx, 48, 512);
        let kicks: Vec<u64> = ctx
            .note_ons()
            .into_iter()
            .filter(|(_, note, _)| *note == 36)
            .map(|(at, _, _)| at)
            .collect();
        assert!(!kicks.iter().any(|at| *at <= 1), "the forced rest still sounded: {kicks:?}");
        assert!(kicks.iter().any(|at| (*at as f64 - 11025.0).abs() <= 1.5),
            "the forced hit at step 2 did not sound: {kicks:?}");
    }

    #[test]
    fn a_long_scheduler_run_stays_bounded() {
        let (mut plugin, mut ctx) = every_step_plugin();
        set_param(&plugin.params.steps, MAX_STEPS as f32);
        set_param(&plugin.params.max_steps, MAX_STEPS as i32);
        set_param(&plugin.params.pulses, MAX_STEPS as i32);
        set_param(&plugin.params.density, 1.0);
        ctx.set_recording(false);
        let blocks = 2_000usize;
        run_playing(&mut plugin, &mut ctx, blocks, 32);
        assert!(ctx.sent > 0, "the long run produced no events");
        // Eight lanes can at most produce a note on and an off per step, and a 32-sample block
        // cannot contain a whole step at these tempos.
        assert!(ctx.sent <= (blocks * 16 + 32) as u64, "event count {} looks unbounded", ctx.sent);
    }

    #[test]
    #[ignore = "timing run; use cargo test --release bench_process_at_max_steps -- --ignored --nocapture"]
    fn bench_process_at_max_steps() {
        use std::time::Instant;
        let (mut plugin, mut ctx) = every_step_plugin();
        set_param(&plugin.params.steps, MAX_STEPS as f32);
        set_param(&plugin.params.max_steps, MAX_STEPS as i32);
        set_param(&plugin.params.pulses, 32);
        set_param(&plugin.params.density, 0.8);
        ctx.set_recording(false);
        // Warm up: the first block generates the pattern and primes the clock.
        run_playing(&mut plugin, &mut ctx, 4, 32);
        let mut times = [0u64; 200];
        for time in times.iter_mut() {
            let start = Instant::now();
            ctx.process(&mut plugin, 32);
            *time = start.elapsed().as_nanos() as u64;
        }
        times.sort_unstable();
        println!("process() at {MAX_STEPS} steps: median {:.1} us, max {:.1} us per 32-sample block",
            times[100] as f64 / 1000.0, times[199] as f64 / 1000.0);
    }

    #[test]
    fn every_parameter_is_host_automatable_and_visible() {
        let params = DrumParams::default();
        let map = params.param_map();
        let ids: Vec<&str> = map.iter().map(|(id, _, _)| id.as_str()).collect();
        for expected in ["mode", "steps", "round_steps", "pulses", "rotate", "density", "probability", "velocity", "regenerate_every", "randomize_after_lengths", "randomize_scope", "theme", "seed", "phase", "osc_rate", "osc_wave", "ca_rule", "symmetry", "rate", "gate", "swing", "markov_bias", "lane1_mode", "lane2_mode", "lane3_mode", "lane4_mode", "lane5_mode", "lane6_mode", "lane7_mode", "lane8_mode", "lfo1_enabled", "lfo1_wave", "lfo1_rate_mode", "lfo1_freq", "lfo1_rate", "lfo1_depth", "lfo1_phase", "lfo1_polarity", "lfo2_enabled", "lfo2_wave", "lfo2_rate_mode", "lfo2_freq", "lfo2_rate", "lfo2_depth", "lfo2_phase", "lfo2_polarity", "lfo3_enabled", "lfo3_wave", "lfo3_rate_mode", "lfo3_freq", "lfo3_rate", "lfo3_depth", "lfo3_phase", "lfo3_polarity", "lfo4_enabled", "lfo4_wave", "lfo4_rate_mode", "lfo4_freq", "lfo4_rate", "lfo4_depth", "lfo4_phase", "lfo4_polarity", "generate", "invert", "mirror", "logging", "freerun", "kick_on", "snare_on", "closed_hat_on", "open_hat_on", "clap_on", "rim_on", "low_tom_on", "perc_on", "kick_note", "snare_note", "closed_hat_note", "open_hat_note", "clap_note", "rim_note", "low_tom_note", "perc_note", "kick_velocity", "snare_velocity", "closed_hat_velocity", "open_hat_velocity", "clap_velocity", "rim_velocity", "low_tom_velocity", "perc_velocity"] {
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
            (33, Mode::ThueMorse, "word strip"),
            (34, Mode::RudinShapiro, "word strip"),
            (35, Mode::Brownian, "walk trace"),
            (36, Mode::LevyFlight, "walk trace"),
            (37, Mode::TentMap, "return map"),
            (38, Mode::HenonMap, "attractor path"),
        ] {
            assert_eq!(Mode::from_index(index), mode, "mode {index} is {ui_meaning} in the UI");
        }
        // The UI reads names from the enum rather than carrying a second list that can drift.
        let ui = include_str!("../ui/app.slint");
        assert!(ui.contains("in property <[string]> mode-names;"), "the UI must receive its mode names from Rust");
        assert_eq!(crate::modes::GROUPS.iter().map(|(_, m)| m.len()).sum::<usize>(), Mode::all().count(),
            "the grouped display list must cover every mode");
        // The help text is still a Slint literal, so keep its entry count aligned with the enum.
        let help = ui.split("mode-help: [").nth(1).expect("no mode-help list in the UI");
        let help_count = help.lines()
            .take_while(|line| !line.trim_start().starts_with("];"))
            .filter(|line| line.trim_start().starts_with('"'))
            .count();
        assert_eq!(help_count, Mode::all().count(), "the UI describes {help_count} modes");
    }


    /// Waveform names and ids are host state; the picker, help line and generator must agree.
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
        assert_eq!(contact::AUTHOR, "ccne4mocc / feN");
        assert_eq!(contact::GITHUB, "https://github.com/ccne4mocc");
        assert_eq!(contact::SUPPORT, "");
        assert_eq!(contact::EMAIL, "");
        assert_eq!(<NrumSeq as Plugin>::VENDOR, contact::AUTHOR);
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
    fn plugin_identity_and_host_ids_are_stable() {
        // These strings are project state and host scan identity. Changing one breaks sessions.
        assert_eq!(<NrumSeq as Plugin>::NAME, "nrum seq");
        assert_eq!(<NrumSeq as Plugin>::VENDOR, contact::AUTHOR);
        assert_eq!(<NrumSeq as Plugin>::VERSION, "1.2.0");
        assert_eq!(<NrumSeq as ClapPlugin>::CLAP_ID, "com.nrumseq.nrumseq");
        assert_eq!(<NrumSeq as Vst3Plugin>::VST3_CLASS_ID, *b"nrum-seq\0\0\0\0\0\0\0\0");
    }

    #[test]
    fn parameter_ids_are_unique() {
        let params = DrumParams::default();
        let map = params.param_map();
        let mut ids: Vec<&str> = map.iter().map(|(id, _, _)| id.as_str()).collect();
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), count, "two parameters share a host id");
    }

    #[test]
    fn every_mode_id_is_unique_and_non_empty() {
        let ids = Mode::ids().expect("mode ids are host state");
        for id in ids {
            assert!(!id.is_empty(), "a mode has an empty host id");
        }
        let mut unique = ids.to_vec();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), ids.len(), "two modes share a host id");
        assert_eq!(ids.len(), <Mode as Enum>::variants().len());
    }

    #[test]
    fn lane_defaults_are_the_documented_gm_map() {
        let p = DrumParams::default();
        let notes: Vec<u8> = (0..TRACKS).map(|lane| p.track_note(lane)).collect();
        assert_eq!(notes, [36, 38, 42, 46, 39, 37, 45, 50]);
        let enabled: Vec<bool> = (0..TRACKS).map(|lane| p.track_enabled(lane)).collect();
        assert_eq!(enabled, [true, true, true, true, false, false, false, true]);
    }

    #[test]
    fn the_effective_step_ceiling_defaults_to_1024() {
        assert_eq!(DEFAULT_MAX_STEPS, 1024);
        assert_eq!(MIN_MAX_STEPS, 64);
        let p = DrumParams::default();
        assert_eq!(p.max_steps.value(), 1024);
        assert_eq!(p.step_ceiling(), 1024);
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
    fn every_theme_has_eight_distinct_lane_colours() {
        for theme in [Theme::Sketchbook, Theme::Neon, Theme::Lava, Theme::Forest] {
            let lanes = theme.lanes();
            for (i, colour) in lanes.iter().enumerate() {
                assert!(lanes[i + 1..].iter().all(|other| other != colour), "{theme:?} repeats lane colour {i}");
            }
        }
    }

    #[test]
    fn neon_is_the_fresh_default_theme() {
        assert_eq!(DrumParams::default().theme.value(), Theme::Neon);
        assert_eq!(<NrumSeq as Plugin>::VERSION, "1.2.0");
    }

    #[test]
    fn track_name_changes_bump_the_revision_only_when_the_name_changes() {
        let shared = Shared::default();
        assert_eq!(shared.track_rev.load(Relaxed), 0);
        shared.set_track_name(Some("  Drums 1 ".to_string()));
        assert_eq!(*shared.track_name.lock().unwrap(), "Drums 1");
        assert_eq!(shared.track_rev.load(Relaxed), 1);
        shared.set_track_name(Some("Drums 1".to_string()));
        assert_eq!(shared.track_rev.load(Relaxed), 1, "same name must not bump the revision");
        // A host that stops reporting a name clears it rather than leaving a stale one.
        shared.set_track_name(None);
        assert_eq!(*shared.track_name.lock().unwrap(), "");
        assert_eq!(shared.track_rev.load(Relaxed), 2);
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

    #[test]
    fn hz_formatter_and_parser_are_hz_only() {
        assert_eq!(format_hz(0.5), "0.50 Hz");
        assert_eq!(format_hz(7.5), "7.50 Hz");
        assert_eq!(format_hz(10.0), "10.0 Hz");
        assert_eq!(format_hz(1_500.0), "10.0 Hz");
        assert_eq!(parse_hz("0.5"), Some(0.5));
        assert_eq!(parse_hz(" 10 hz "), Some(10.0));
        assert_eq!(parse_hz("20000"), Some(10.0));
        assert_eq!(parse_hz("nope"), None);
    }

    #[test]
    fn lfo_links_drive_the_generation_snapshot() {
        let p = DrumParams {
            lfo1_links: AtomicU64::new(target_bit(ModTarget::Steps)),
            lfo1_wave: EnumParam::new("LFO 1 Wave", OscWave::Sine),
            lfo1_rate_mode: EnumParam::new("LFO 1 Rate Mode", LfoRateMode::Hz),
            lfo1_freq: hz_param("LFO 1 Frequency", 1.0),
            lfo1_depth: FloatParam::new("LFO 1 Depth", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            steps: FloatParam::new("Steps", 16.0, FloatRange::Linear { min: 1.0, max: MAX_STEPS as f32 }),
            ..DrumParams::default()
        };
        assert!(p.has_generation_links());
        assert!(target_in_mask(p.lfo_link_mask(0), ModTarget::Steps));
        let base = p.gen_params();
        let peak = p.gen_params_at(LfoClock { beat: 0.25, seconds: 0.25 });
        assert!(peak.steps > base.steps, "{} -> {}", base.steps, peak.steps);
        p.set_lfo_link(0, ModTarget::Steps, false);
        assert!(!p.has_generation_links());
    }

    #[test]
    fn full_depth_sweeps_the_whole_parameter_and_polarity_sets_the_direction() {
        let base = |polarity| DrumParams {
            lfo1_links: AtomicU64::new(target_bit(ModTarget::Swing)),
            lfo1_wave: EnumParam::new("LFO 1 Wave", OscWave::Sine),
            lfo1_rate_mode: EnumParam::new("LFO 1 Rate Mode", LfoRateMode::Hz),
            lfo1_freq: hz_param("LFO 1 Frequency", 1.0),
            lfo1_depth: FloatParam::new("LFO 1 Depth", 1.0, FloatRange::Linear { min: 0.0, max: LFO_MAX_DEPTH }),
            lfo1_polarity: EnumParam::new("LFO 1 Polarity", polarity),
            swing: FloatParam::new("Swing", 0.25, FloatRange::Linear { min: 0.0, max: 0.5 }),
            ..DrumParams::default()
        };
        let sweep = |p: &DrumParams| {
            let (mut lo, mut hi) = (f32::MAX, f32::MIN);
            // Two beats at 120 BPM is one second, i.e. one full cycle of a 1 Hz LFO.
            for k in 0..=128 {
                let v = p.schedule_at(LfoClock::from_beat(k as f64 / 64.0, 120.0)).swing;
                lo = lo.min(v);
                hi = hi.max(v);
            }
            (lo, hi)
        };
        let (lo, hi) = sweep(&base(LfoPolarity::Bipolar));
        assert!(lo <= 1e-4 && hi >= 0.5 - 1e-4, "bipolar swept {lo}..{hi}");
        let (lo, hi) = sweep(&base(LfoPolarity::Unipolar));
        assert!((lo - 0.25).abs() <= 0.06 && hi >= 0.5 - 1e-4, "unipolar swept {lo}..{hi}");
        let (lo, hi) = sweep(&base(LfoPolarity::UnipolarNegative));
        assert!(lo <= 1e-4 && (hi - 0.25).abs() <= 0.06, "unipolar negative swept {lo}..{hi}");
    }

    #[test]
    fn every_target_modulation_stays_inside_its_range() {
        for target in ModTarget::ALL {
            let p = DrumParams {
                lfo1_links: AtomicU64::new(target_bit(target)),
                lfo1_wave: EnumParam::new("LFO 1 Wave", OscWave::Sine),
                lfo1_rate_mode: EnumParam::new("LFO 1 Rate Mode", LfoRateMode::Hz),
                lfo1_freq: hz_param("LFO 1 Frequency", 1.0),
                lfo1_depth: FloatParam::new("LFO 1 Depth", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
                ..DrumParams::default()
            };
            let (min, max) = p.target_range(target);
            for step in 0..16 {
                let clock = LfoClock { beat: step as f64 * 0.1, seconds: step as f64 * 0.1 };
                let value = p.target_value(target, &p.lfo_signals(clock));
                assert!(value >= min && value <= max, "{target:?}: {value} outside {min}..{max}");
            }
        }
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
        // Regression: snapping must store the rounded length, not the raw slider float.
        let p = DrumParams::default();
        let g = p.gen_params();
        assert!(p.round_steps.value());
        assert_eq!(g.length, g.steps as f32);
        assert_eq!(g.length.fract(), 0.0);
    }
}
