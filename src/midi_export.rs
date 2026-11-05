//! Standard MIDI File export for the current pattern.
//!
//! Editor thread only; `process()` never builds or writes a file. One format-0 track with note
//! events and a tempo meta event is small enough to encode directly, with no MIDI crate.

use crate::clock::Meter;
use crate::sequence::{Pattern, MAX_STEPS, TRACKS};

const PPQ: u16 = 480;
const CHANNEL: u8 = 9;

#[derive(Clone, Copy)]
pub(crate) struct Lane {
    pub enabled: bool,
    pub note: u8,
    pub velocity: f32,
}

pub(crate) struct ExportSettings {
    /// Host tempo. Invalid or missing values fall back to 120 BPM.
    pub bpm: f64,
    /// Clock rate in steps per beat (4 = 1/16).
    pub steps_per_beat: f64,
    /// Meter implied by the mode and pattern; `None` for polymetric loops.
    pub meter: Option<Meter>,
    /// Exact loop length in steps. Polymetric modes tile their lane periods over this.
    pub loop_steps: u32,
    pub gate: f64,
    pub swing: f64,
    pub velocity: f32,
    /// Step-wide microtiming in fractions of one step (0..0.9), applied to every lane.
    pub offsets: [f32; MAX_STEPS],
    /// Ratchet count per lane and step (1..=4).
    pub ratchets: [[u8; MAX_STEPS]; TRACKS],
    /// Per-cell gate/length override in fractions of one step; 0 follows `gate`.
    pub lengths: [[f32; MAX_STEPS]; TRACKS],
    pub lanes: [Lane; TRACKS],
}

struct Event {
    tick: u32,
    /// 0 = note off, 1 = note on. Offs sort first so a retrigger cannot cut the new note.
    order: u8,
    note: u8,
    velocity: u8,
}

/// Encode one generated loop as a format-0 Standard MIDI File.
pub(crate) fn smf(pattern: &Pattern, settings: &ExportSettings) -> Vec<u8> {
    let steps = pattern.steps.clamp(1, MAX_STEPS);
    let spb = if settings.steps_per_beat.is_finite() && settings.steps_per_beat > 0.0 {
        settings.steps_per_beat
    } else {
        4.0
    };
    let step_len = 1.0 / spb;
    let loop_steps = if settings.loop_steps > 0 { settings.loop_steps } else { steps as u32 };
    let gate = settings.gate.clamp(0.02, 1.0) * step_len;
    let swing = settings.swing.clamp(0.0, 0.5) * step_len;
    let to_tick = |beat: f64| (beat * PPQ as f64).round().clamp(0.0, u32::MAX as f64) as u32;

    // Tile the polymetric loop: a lane with its own period reads its source step modulo that
    // period, while step-wide offsets/ratchets follow the generated pattern row.
    let loop_cells = (loop_steps as usize).clamp(1, MAX_STEPS);
    let mut events = Vec::new();
    for i in 0..loop_cells {
        let source = i % steps;
        let offset = settings.offsets[source].clamp(0.0, 0.9) as f64 * step_len;
        let onset = i as f64 * step_len + if i % 2 == 1 { swing } else { 0.0 } + offset;
        for tr in 0..TRACKS {
            let lane = settings.lanes[tr];
            let raw_period = pattern.lane_periods[tr] as usize;
            let period = if raw_period == 0 { steps } else { raw_period.min(steps) }.max(1);
            let idx = i % period;
            if !lane.enabled || !pattern.hit[tr][idx] {
                continue;
            }
            let ratchet = settings.ratchets[tr][idx].clamp(1, 4) as usize;
            let lane_step = step_len / ratchet as f64;
            // A per-cell length replaces the global gate; a ratchet still caps it to its own
            // substep spacing so one lane never overlaps itself.
            let cell_gate = if settings.lengths[tr][idx] > 0.0 {
                settings.lengths[tr][idx].clamp(0.02, 1.0) as f64 * step_len
            } else {
                gate
            };
            let sub_gate = if ratchet > 1 { cell_gate.min(lane_step * 0.9) } else { cell_gate };
            let velocity = (settings.velocity * lane.velocity * pattern.accent[tr][idx])
                .clamp(0.0, 1.0);
            let velocity = (velocity * 127.0).round().clamp(1.0, 127.0) as u8;
            let note = lane.note.min(127);
            for r in 0..ratchet {
                let at = onset + r as f64 * lane_step;
                events.push(Event { tick: to_tick(at), order: 1, note, velocity });
                events.push(Event { tick: to_tick(at + sub_gate), order: 0, note, velocity: 0 });
            }
        }
    }
    events.sort_unstable_by(|a, b| {
        a.tick.cmp(&b.tick).then(a.order.cmp(&b.order)).then(a.note.cmp(&b.note))
    });

    let loop_end = to_tick(loop_steps as f64 * step_len);
    let mut track = Vec::new();
    // Tempo meta event. 0x51 carries three bytes of microseconds per quarter note.
    let bpm = if settings.bpm.is_finite() && settings.bpm > 0.0 { settings.bpm } else { 120.0 };
    let us_per_beat = (60_000_000.0 / bpm).round().clamp(1.0, 16_777_215.0) as u32;
    track.extend_from_slice(&[0, 0xFF, 0x51, 0x03]);
    track.extend_from_slice(&us_per_beat.to_be_bytes()[1..]);
    // Implied meter, written only when the derived pattern has an exact one. Polymetric or
    // triplet-based loops leave the host signature alone instead of claiming 4/4.
    if let Some(meter) = settings.meter {
        let den = meter.den.max(1);
        if den.is_power_of_two() {
            let exp = den.trailing_zeros() as u8;
            let num = meter.num.clamp(1, 255) as u8;
            track.extend_from_slice(&[0, 0xFF, 0x58, 0x04, num, exp, 24, 8]);
        }
    }

    let mut last_tick = 0;
    for event in &events {
        push_vlq(&mut track, event.tick - last_tick);
        let status = if event.order == 0 { 0x80 | CHANNEL } else { 0x90 | CHANNEL };
        track.extend_from_slice(&[status, event.note, event.velocity]);
        last_tick = event.tick;
    }
    let end = loop_end.max(last_tick);
    push_vlq(&mut track, end - last_tick);
    track.extend_from_slice(&[0xFF, 0x2F, 0x00]);

    let mut out = Vec::with_capacity(14 + 8 + track.len());
    out.extend_from_slice(b"MThd");
    out.extend_from_slice(&6u32.to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes()); // format 0
    out.extend_from_slice(&1u16.to_be_bytes()); // one track
    out.extend_from_slice(&PPQ.to_be_bytes());
    out.extend_from_slice(b"MTrk");
    out.extend_from_slice(&(track.len() as u32).to_be_bytes());
    out.extend_from_slice(&track);
    out
}

fn push_vlq(out: &mut Vec<u8>, mut value: u32) {
    let mut bytes = [0u8; 5];
    let mut at = 4;
    bytes[at] = (value & 0x7F) as u8;
    value >>= 7;
    while value > 0 {
        at -= 1;
        bytes[at] = ((value & 0x7F) as u8) | 0x80;
        value >>= 7;
    }
    out.extend_from_slice(&bytes[at..]);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn for_one_lane(note: u8, steps: usize, hits: &[(usize, bool)]) -> (Pattern, ExportSettings) {
        let mut pattern = Pattern { steps, ..Pattern::default() };
        for &(step, hit) in hits {
            pattern.hit[0][step] = hit;
            pattern.accent[0][step] = 1.0;
        }
        let lanes = std::array::from_fn(|tr| Lane {
            enabled: tr == 0,
            note,
            velocity: 1.0,
        });
        let settings = ExportSettings {
            bpm: 120.0,
            steps_per_beat: 4.0,
            meter: crate::clock::meter_from_steps(steps as u32, crate::clock::StepUnit::Sixteenth),
            loop_steps: steps as u32,
            gate: 0.5,
            swing: 0.0,
            velocity: 1.0,
            offsets: [0.0; MAX_STEPS],
            ratchets: [[1; MAX_STEPS]; TRACKS],
            lengths: [[0.0; MAX_STEPS]; TRACKS],
            lanes,
        };
        (pattern, settings)
    }

    #[test]
    fn variable_length_quantities_match_the_midi_encoding() {
        for (value, expected) in [
            (0u32, &[0x00][..]),
            (0x7F, &[0x7F][..]),
            (0x80, &[0x81, 0x00][..]),
            (0x2000, &[0xC0, 0x00][..]),
            (0x0FFF, &[0x9F, 0x7F][..]),
        ] {
            let mut out = Vec::new();
            push_vlq(&mut out, value);
            assert_eq!(out, expected, "vlq({value})");
        }
    }

    #[test]
    fn one_loop_writes_note_pairs_and_an_end_of_track() {
        let (pattern, settings) = for_one_lane(36, 4, &[(0, true), (1, true)]);
        let bytes = smf(&pattern, &settings);
        assert_eq!(&bytes[..4], b"MThd");
        assert_eq!(&bytes[14..18], b"MTrk");
        let track_len = u32::from_be_bytes(bytes[18..22].try_into().unwrap()) as usize;
        let track = &bytes[22..];
        assert_eq!(track.len(), track_len);

        let events = channel_events(track);
        assert_eq!(events.len(), 4, "two hits must produce two on/off pairs");
        assert_eq!((events[0].0, events[0].1, events[0].2), (0, 0x99, 36));
        // gate 0.5 of a 1/4-beat step = 60 ticks at 480 PPQ.
        assert_eq!((events[1].0, events[1].1, events[1].2), (60, 0x89, 36));
        // Step 1 starts one step later, at 120 ticks.
        assert_eq!((events[2].0, events[2].1, events[2].2), (120, 0x99, 36));
        assert_eq!((events[3].0, events[3].1, events[3].2), (180, 0x89, 36));
        assert!(track.ends_with(&[0xFF, 0x2F, 0x00]), "missing end of track");
    }

    #[test]
    fn note_off_at_a_retrigger_comes_before_the_new_note_on() {
        let (mut pattern, mut settings) = for_one_lane(36, 4, &[(0, true), (1, false)]);
        pattern.hit[0][1] = true;
        settings.gate = 1.0;
        let events = channel_events(&smf(&pattern, &settings)[22..]);
        let at_120: Vec<_> = events.iter().filter(|e| e.0 == 120).collect();
        assert_eq!(at_120.len(), 2);
        assert_eq!(at_120[0].1, 0x89, "note off must sort first");
        assert_eq!(at_120[1].1, 0x99);
    }

    #[test]
    fn per_cell_length_replaces_the_global_gate() {
        let (pattern, mut settings) = for_one_lane(36, 4, &[(0, true), (1, true)]);
        // 0.25 of a step is 30 ticks at 480 PPQ and 1/16.
        settings.lengths[0][0] = 0.25;
        let events = channel_events(&smf(&pattern, &settings)[22..]);
        assert_eq!((events[0].0, events[0].1), (0, 0x99));
        assert_eq!((events[1].0, events[1].1), (30, 0x89));
        // The next cell still follows the global gate (60 ticks).
        assert_eq!((events[3].0, events[3].1), (180, 0x89));
    }

    #[test]
    fn microtiming_shifts_the_exported_tick() {
        let (pattern, mut settings) = for_one_lane(36, 4, &[(0, true), (1, true)]);
        settings.offsets[0] = 0.5;
        let events = channel_events(&smf(&pattern, &settings)[22..]);
        assert_eq!(events[0].0, 60, "half a step is 60 ticks at 480 PPQ and 1/16");
        assert_eq!(events[2].0, 120, "the next step must not inherit the delay");
    }

    #[test]
    fn ratchets_export_even_substeps() {
        let (pattern, mut settings) = for_one_lane(36, 4, &[(0, true)]);
        settings.ratchets[0][0] = 2;
        let events = channel_events(&smf(&pattern, &settings)[22..]);
        let ons: Vec<u32> = events.iter().filter(|e| e.1 == 0x99).map(|e| e.0).collect();
        // One 1/16 step is 120 ticks, so a two-note ratchet fires at 0 and 60.
        assert_eq!(ons, vec![0, 60]);
    }

    #[test]
    fn disabled_lanes_do_not_export() {
        let (pattern, settings) = for_one_lane(36, 4, &[(0, true)]);
        let mut settings = settings;
        settings.lanes[0].enabled = false;
        let events = channel_events(&smf(&pattern, &settings)[22..]);
        assert!(events.is_empty());
    }

    #[test]
    fn loop_length_is_always_whole_steps() {
        let (pattern, mut settings) = for_one_lane(36, 5, &[]);
        settings.loop_steps = 4;
        let bytes = smf(&pattern, &settings);
        // 4 steps * 1/4 beat * 480 PPQ = 480 ticks, encoded as VLQ 0x83 0x60.
        assert!(bytes[22..].ends_with(&[0x83, 0x60, 0xFF, 0x2F, 0x00]));
    }

    /// Minimal reader for the events this writer emits: skips meta events and returns
    /// `(absolute_tick, status, note)` for channel messages.
    fn channel_events(track: &[u8]) -> Vec<(u32, u8, u8)> {
        let mut out = Vec::new();
        let mut at = 0;
        let mut tick = 0u32;
        while at < track.len() {
            let (delta, used) = read_vlq(&track[at..]);
            at += used;
            tick += delta;
            match track[at] {
                0xFF => {
                    at += 2; // FF, type
                    let (len, used) = read_vlq(&track[at..]);
                    at += used + len as usize;
                }
                0x80..=0x9F => {
                    out.push((tick, track[at], track[at + 1]));
                    at += 3;
                }
                other => panic!("unexpected event byte {other:#04x} at {at}"),
            }
        }
        out
    }

    fn track_data(bytes: &[u8]) -> Vec<u8> {
        let at = bytes.windows(4).position(|w| w == b"MTrk").unwrap() + 8;
        bytes[at..].to_vec()
    }

    fn time_signature(bytes: &[u8]) -> Option<(u8, u8)> {
        bytes.windows(6).find_map(|w| {
            (w[0] == 0xFF && w[1] == 0x58 && w[2] == 0x04).then_some((w[3], w[4]))
        })
    }

    #[test]
    fn export_writes_the_derived_time_signature() {
        let (pattern, mut settings) = for_one_lane(60, 16, &[(0, true), (8, true)]);
        settings.meter = crate::clock::meter_from_steps(16, crate::clock::StepUnit::Sixteenth);
        let bytes = smf(&pattern, &settings);
        assert_eq!(time_signature(&bytes), Some((4, 2)), "16 steps at 1/16 should export 4/4");

        settings.meter = crate::clock::meter_from_steps(13, crate::clock::StepUnit::Sixteenth);
        settings.loop_steps = 13;
        let bytes = smf(&pattern, &settings);
        assert_eq!(time_signature(&bytes), Some((13, 4)), "13 steps at 1/16 should export 13/16");

        settings.meter = None;
        let bytes = smf(&pattern, &settings);
        assert_eq!(time_signature(&bytes), None, "polymetric patterns must not claim a signature");
    }

    #[test]
    fn export_tiles_mode_owned_lane_periods() {
        let (mut pattern, settings) = for_one_lane(60, 16, &[]);
        // Lane 0 owns a 3-step period with hits on the first three cells.
        pattern.lane_periods = [3, 16, 16, 16, 16, 16, 16, 16];
        for i in 0..3 { pattern.hit[0][i] = true; pattern.accent[0][i] = 1.0; }
        let bytes = smf(&pattern, &settings);
        let track = track_data(&bytes);
        let ons: Vec<u32> = channel_events(&track)
            .into_iter()
            .filter(|(_, st, _)| st & 0xF0 == 0x90)
            .map(|(t, _, _)| t)
            .collect();
        // 0.25-beat steps at 480 PPQ = 120 ticks. The 16-step loop plays the 3-step lane six times.
        assert_eq!(ons, vec![0, 120, 240, 360, 480, 600, 720, 840, 960, 1080, 1200, 1320, 1440, 1560, 1680, 1800]);
    }

    fn read_vlq(data: &[u8]) -> (u32, usize) {
        let mut value = 0u32;
        for (i, &byte) in data.iter().enumerate() {
            value = (value << 7) | (byte & 0x7F) as u32;
            if byte & 0x80 == 0 {
                return (value, i + 1);
            }
        }
        panic!("unterminated vlq");
    }
}
