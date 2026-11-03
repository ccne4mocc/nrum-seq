//! Pure GUI geometry. Keep this free of Slint and plug-in types so every view can be tested.
use crate::sequence::{
    cellular_row, oscillator_value, oscillator_wave_count, GenParams, Pattern, MAX_STEPS, TRACKS,
    CA_ROWS,
};
use crate::modes::Mode;
use std::f32::consts::TAU;
use std::fmt::Write;

const SIZE: f32 = 210.0;
const C: f32 = SIZE * 0.5;
const P: f32 = SIZE;
fn x_of(i: usize, n: usize) -> f32 { 8.0 + i as f32 / n.max(1) as f32 * (P - 16.0) }

#[derive(Clone, Debug, Default)]
pub struct Node { pub x: f32, pub y: f32, pub r: f32, pub on: bool, pub cur: bool, pub label: String, pub lane: u8 }
#[derive(Clone, Debug, Default)]
pub struct View {
    pub nodes: Vec<Node>,
    pub edges: String,
    pub lines: String,
    pub hand: String,
}

fn polar(r: f32, turns: f32) -> (f32, f32) {
    let a = TAU * turns;
    (C + r * a.sin(), C - r * a.cos())
}
fn mv(s: &mut String, x: f32, y: f32) { let _ = write!(s, "M{:.1} {:.1} ", x, y); }
fn ln(s: &mut String, x: f32, y: f32) { let _ = write!(s, "L{:.1} {:.1} ", x, y); }
fn line(s: &mut String, a: (f32, f32), b: (f32, f32)) { mv(s, a.0, a.1); ln(s, b.0, b.1); }

/// Thumbnail paths for the oscillator/LFO picker, one per waveform in `OscWave` order. Each path
/// samples the generator's own `oscillator_value`, so the picker cannot advertise a shape it does not play.
pub fn wave_icon_paths() -> Vec<String> {
    (0..oscillator_wave_count)
        .map(|wave| {
            let mut s = String::new();
            for k in 0..=64 {
                let phase = k as f32 / 64.0;
                let x = 2.0 + phase * 96.0;
                let y = 4.0 + (1.0 - oscillator_value(wave, phase)) * 32.0;
                if k == 0 { mv(&mut s, x, y) } else { ln(&mut s, x, y) }
            }
            s
        })
        .collect()
}
fn circle(s: &mut String, cx: f32, cy: f32, r: f32) {
    let _ = write!(s, "M{:.1} {:.1} A{r:.1} {r:.1} 0 1 1 {:.1} {:.1} A{r:.1} {r:.1} 0 1 1 {:.1} {:.1} ", cx + r, cy, cx - r, cy, cx + r, cy);
}
/// Open arc around the panel centre, `from`/`to` in turns clockwise from 12 o'clock.
fn arc(s: &mut String, r: f32, from: f32, to: f32) {
    let (x0, y0) = polar(r, from);
    let (x1, y1) = polar(r, to);
    let large = if (to - from).abs() > 0.5 { 1 } else { 0 };
    let sweep = if to >= from { 1 } else { 0 };
    let _ = write!(s, "M{x0:.1} {y0:.1} A{r:.1} {r:.1} 0 {large} {sweep} {x1:.1} {y1:.1} ");
}
fn aggregate_hit(p: &Pattern, i: usize) -> bool { (0..TRACKS).any(|t| p.hit[t][i]) }

/// Generator step that produced display slot `i`: the inverse of the rotation and mirror applied
/// by `sequence::generate`. Views that redraw generator geometry must index through this or they
/// drift from the notes after Generate, Rotate or Mirror.
fn src_index(params: &GenParams, pat: &Pattern, i: usize, n: usize) -> usize {
    let j = if params.mirror { n - 1 - i } else { i };
    (j + n - pat.rotation % n) % n
}

/// Rank cutoff of lane 0 for the oscillator panel, read from the generator's published `hit` and
/// `orbit`. Hits are the top-ranked samples; with `invert` the cutoff is the highest hit.
fn oscillator_cutoff(p: &Pattern, n: usize, invert: bool) -> f32 {
    let hits = (0..n).filter(|&i| p.hit[0][i]).map(|i| p.orbit[i]);
    if invert { hits.fold(0.0, f32::max) } else { hits.fold(1.0, f32::min) }
}

pub fn build(params: &GenParams, p: &Pattern, cur: i32) -> View {
    let n = p.steps.clamp(1, MAX_STEPS);
    let play = (cur >= 0 && (cur as usize) < n).then_some(cur as usize);
    let mut v = View::default();
    match params.kind {
        Mode::Euclidean => {
            // Per-lane rings rather than one aggregate ring
            for tr in 0..TRACKS {
                let radius = 24.0 + tr as f32 * 9.2;
                circle(&mut v.edges, C, C, radius);
                for i in 0..n {
                    let (x, y) = polar(radius, i as f32 / n as f32);
                    let on = p.hit[tr][i];
                    v.nodes.push(Node { x, y, r: if on { 3.6 } else { 1.8 }, on, cur: play == Some(i), label: String::new(), lane: u8::MAX });
                }
            }
            if let Some(i) = play {
                mv(&mut v.hand, C, C);
                for tr in 0..TRACKS {
                    let (x, y) = polar(24.0 + tr as f32 * 9.2, i as f32 / n as f32);
                    ln(&mut v.hand, x, y);
                }
            }
        }
        Mode::Eulerian | Mode::Markov => {
            let vertices = TRACKS + 1;
            let pos = |state: usize| polar(78.0, state as f32 / vertices as f32);
            let labels = ["R", "K", "S", "CH", "OH", "CL", "RM", "T", "P"];
            let mut prev = p.state[0] as usize;
            for i in 1..n {
                let next = p.state[i] as usize;
                if next < vertices && prev < vertices {
                    let (x1, y1) = pos(prev); let (x2, y2) = pos(next);
                    mv(&mut v.edges, x1, y1); ln(&mut v.edges, x2, y2);
                }
                prev = next;
            }
            for i in 0..vertices {
                let (x, y) = pos(i);
                let state_cur = play.map(|step| p.state[step] as usize == i).unwrap_or(false);
                v.nodes.push(Node { x, y, r: if i == 0 { 9.0 } else { 11.0 }, on: i != 0, cur: state_cur, label: labels[i].into(), lane: u8::MAX });
            }
            if let Some(i) = play {
                let state = p.state[i] as usize;
                let (x, y) = pos(state.min(vertices - 1));
                mv(&mut v.hand, C, C); ln(&mut v.hand, x, y);
            }
        }
        Mode::Phyllotaxis => {
            // One shared √i rosette instead of eight overlaid lane spirals.
            let max_r = 92.0;
            let sec_r = 40.0;
            let shift = p.rotation % n;
            // Generator step -> display slot. Mirror runs after rotation, so it reverses the slot.
            let slot_of = |g: usize| {
                let rotated = (g + shift) % n;
                if params.mirror { (2 * n - 1 - rotated) % n } else { rotated }
            };
            circle(&mut v.edges, C, C, max_r);
            for tr in 0..TRACKS {
                let threshold = crate::sequence::phyllotaxis_threshold(params, tr) as f32;
                arc(&mut v.lines, max_r - 2.0 - tr as f32 * 2.2, 0.0, threshold);
                arc(&mut v.lines, (sec_r - 2.0 - tr as f32 * 1.2).max(18.0), 0.0, threshold * 0.18);
            }

            // Eight rosettes, one per lane. Each lane uses its own golden-angle phase, so the
            // rosettes are visibly distinct instead of eight dots sharing one aggregate angle.
            let stride = n.div_ceil(128);
            for g0 in (0..n).step_by(stride) {
                let g1 = (g0 + stride).min(n);
                let mid = g0 + (g1 - g0) / 2;
                let r = max_r * ((mid as f32 + 1.0) / n as f32).sqrt();
                let cur = (g0..g1).any(|g| play == Some(slot_of(g)));
                for tr in 0..TRACKS {
                    let on = (g0..g1).any(|g| {
                        p.hit[tr][slot_of(g)]
                            && crate::sequence::phyllotaxis_rules(g, tr, params) & 1 != 0
                    });
                    let (x, y) = polar(r, crate::sequence::phyllotaxis_primary(mid, tr, params) as f32);
                    v.nodes.push(Node {
                        x, y,
                        r: if on { 2.8 } else { 1.0 },
                        on, cur, label: String::new(), lane: tr as u8,
                    });
                }

                // The secondary rule is sparse by design, so draw only the groups where it
                // fired. It stays a single inner aggregate band.
                let mut secondary = 0usize;
                for g in g0..g1 {
                    let slot = slot_of(g);
                    for tr in 0..TRACKS {
                        if p.hit[tr][slot]
                            && crate::sequence::phyllotaxis_rules(g, tr, params) & 2 != 0
                        {
                            secondary += 1;
                        }
                    }
                }
                if secondary > 0 {
                    let r2 = sec_r * ((mid as f32 + 1.0) / n as f32).sqrt();
                    let (x2, y2) = polar(r2, crate::sequence::phyllotaxis_secondary(mid, 0, params) as f32);
                    v.nodes.push(Node {
                        x: x2, y: y2,
                        r: 2.4 + secondary.min(TRACKS) as f32 * 0.2,
                        on: true, cur: false, label: String::new(), lane: u8::MAX,
                    });
                }
            }
            // The playhead: the radius line to the step currently sounding, on lane 0.
            if let Some(i) = play {
                let g = src_index(params, p, i, n);
                let r = max_r * ((g as f32 + 1.0) / n as f32).sqrt();
                let (x, y) = polar(r, crate::sequence::phyllotaxis_primary(g, 0, params) as f32);
                mv(&mut v.hand, C, C); ln(&mut v.hand, x, y);
            }
        }
        Mode::Cellular => {
            let cw = 184.0 / n as f32;
            let ch = 172.0 / CA_ROWS as f32;
            // Each lane reads its own generation, so mark every lane's row.
            let lane_rows: [usize; TRACKS] = std::array::from_fn(|tr| cellular_row(params, tr));
            for row in 0..CA_ROWS {
                for i in 0..n {
                    let on = p.ca[row][i];
                    let cur = play == Some(i) && lane_rows.contains(&row);
                    v.nodes.push(Node {
                        x: 13.0 + (i as f32 + 0.5) * cw,
                        y: 18.0 + (row as f32 + 0.5) * ch,
                        r: (cw.min(ch) * 0.40).max(1.1),
                        on,
                        cur,
                        label: String::new(), lane: u8::MAX,
                    });
                }
            }
            for row in 0..=CA_ROWS {
                let y = 18.0 + row as f32 * ch; mv(&mut v.edges, 12.0, y); ln(&mut v.edges, 198.0, y);
            }
        }
        Mode::Oscillator => {
            // Draw the generator's published lane-0 curve (`orbit`), already rotated into display
            // slots, so the picture matches the onsets after Generate.
            let x_at = |i: usize| 12.0 + (i as f32 + 0.5) / n as f32 * 186.0;
            let y_at = |value: f32| 176.0 - value * 130.0;
            mv(&mut v.edges, 12.0, 105.0); ln(&mut v.edges, 198.0, 105.0);
            for i in 0..n {
                let (x, y) = (x_at(i), y_at(p.orbit[i]));
                if i == 0 { mv(&mut v.lines, x, y); } else { ln(&mut v.lines, x, y); }
            }
            // The rank cutoff lane 0 actually applied, read from `hit[0]` and `orbit`, not
            // re-derived. With Invert the cutoff is the highest hit; a dropped onset can move it slightly.
            let cutoff = oscillator_cutoff(p, n, params.invert);
            let threshold_y = y_at(cutoff);
            mv(&mut v.edges, 12.0, threshold_y); ln(&mut v.edges, 198.0, threshold_y);
            for i in 0..n {
                v.nodes.push(Node { x: x_at(i), y: y_at(p.orbit[i]), r: 3.0, on: aggregate_hit(p, i), cur: play == Some(i), label: String::new(), lane: u8::MAX });
            }
        }
        Mode::Crystallographic => {
            let cw = 184.0 / n as f32;
            let ch = 168.0 / TRACKS as f32;
            for tr in 0..TRACKS {
                for i in 0..n {
                    let on = p.hit[tr][i];
                    let x = 13.0 + (i as f32 + 0.5) * cw;
                    let y = 21.0 + (tr as f32 + 0.5) * ch;
                    v.nodes.push(Node { x, y, r: if on { 4.1 } else { 2.0 }, on, cur: play == Some(i), label: String::new(), lane: u8::MAX });
                }
            }
            for i in 0..=n {
                let x = 13.0 + i as f32 * cw; mv(&mut v.edges, x, 18.0); ln(&mut v.edges, x, 192.0);
            }
            for tr in 0..=TRACKS {
                let y = 21.0 + tr as f32 * ch; mv(&mut v.edges, 11.0, y); ln(&mut v.edges, 199.0, y);
            }
            let unit = (n / params.symmetry.clamp(1, 8)).max(1);
            for i in 0..=n / unit {
                let x = 13.0 + (i * unit) as f32 * cw;
                mv(&mut v.lines, x, 16.0); ln(&mut v.lines, x, 194.0);
            }
        }
        Mode::RaisedCosine => {
            // The generator's own envelope and cutoff, so the drawn curve produced the hits.
            mv(&mut v.edges, 12.0, 180.0); ln(&mut v.edges, 198.0, 180.0);
            let hits = crate::sequence::pulse_hits(params.pulses, params.density, 0, n);
            let phase = crate::sequence::raised_cosine_phase(params.seed, params.phase);
            let samples = (n * 6).max(48);
            for j in 0..samples {
                // Sample the envelope densely: it can complete several cycles between steps.
                let pos = j as f32 / (samples - 1) as f32;
                let hits_f = (pos * hits as f32).round() as usize;
                let value = crate::sequence::raised_cosine_envelope(0, 1000, hits_f, phase);
                let x = 12.0 + pos * 186.0;
                let y = 180.0 - value * 148.0;
                if j == 0 { mv(&mut v.lines, x, y); } else { ln(&mut v.lines, x, y); }
            }
            let cut = crate::sequence::raised_cosine_cutoff(hits, n);
            let cut_y = 180.0 - cut * 148.0;
            mv(&mut v.edges, 12.0, cut_y); ln(&mut v.edges, 198.0, cut_y);
            for i in 0..n {
                let x = 12.0 + (i as f32 + 0.5) / n as f32 * 186.0;
                let value = crate::sequence::raised_cosine_envelope(i, n, hits, phase);
                v.nodes.push(Node { x, y: 180.0 - value * 148.0, r: 3.0, on: aggregate_hit(p, i), cur: play == Some(i), label: String::new(), lane: u8::MAX });
            }
            if let Some(i) = play { let x = 12.0 + (i as f32 + 0.5) / n as f32 * 186.0; mv(&mut v.hand, x, 18.0); ln(&mut v.hand, x, 194.0); }
        }
        Mode::Slowcat => {
            // Phrase timeline: one band per lane, ticks at that lane's motif boundaries, onsets
            // as nodes on the band. Motif length comes from the generator.
            let cw = 184.0 / n as f32;
            let ch = 168.0 / TRACKS as f32;
            let count = params.pulses.max(1);
            for tr in 0..TRACKS {
                let y = 21.0 + (tr as f32 + 0.5) * ch;
                let motif = crate::sequence::slowcat_motif_len(tr, count, n).max(1);
                mv(&mut v.edges, 11.0, y); ln(&mut v.edges, 199.0, y);
                let mut cell = 0usize;
                while cell < n {
                    let x = 13.0 + cell as f32 * cw;
                    mv(&mut v.lines, x, y - ch * 0.42); ln(&mut v.lines, x, y + ch * 0.42);
                    cell += motif;
                }
                for i in 0..n {
                    let on = p.hit[tr][i];
                    let x = 13.0 + (i as f32 + 0.5) * cw;
                    v.nodes.push(Node { x, y, r: if on { 4.3 } else { 1.8 }, on, cur: play == Some(i), label: String::new(), lane: u8::MAX });
                }
            }
            if let Some(i) = play { let x = 13.0 + (i as f32 + 0.5) * cw; mv(&mut v.hand, x, 16.0); ln(&mut v.hand, x, 196.0); }
        }
        Mode::ToussaintDistance => {
            // Distance circle: one ring per lane, every onset at its actual step angle, with a
            // radial spoke and a tick per grid step so inter-onset gaps read against the grid.
            circle(&mut v.edges, C, C, 100.0);
            for i in 0..n {
                let (x0, y0) = polar(94.0, i as f32 / n as f32);
                let (x1, y1) = polar(100.0, i as f32 / n as f32);
                mv(&mut v.edges, x0, y0); ln(&mut v.edges, x1, y1);
            }
            for tr in 0..TRACKS {
                let radius = 30.0 + tr as f32 * 8.5;
                circle(&mut v.edges, C, C, radius);
                for i in 0..n {
                    if !p.hit[tr][i] { continue; }
                    let (x, y) = polar(radius, i as f32 / n as f32);
                    let (rx, ry) = polar(100.0, i as f32 / n as f32);
                    mv(&mut v.lines, rx, ry); ln(&mut v.lines, x, y);
                    v.nodes.push(Node {
                        x, y,
                        r: 3.2 + 1.8 * p.accent[tr][i],
                        on: true,
                        cur: play == Some(i),
                        label: String::new(), lane: u8::MAX,
                    });
                }
            }
            if let Some(i) = play { let (x, y) = polar(100.0, i as f32 / n as f32); mv(&mut v.hand, C, C); ln(&mut v.hand, x, y); }
        }
        Mode::WellFormed => {
            // One band per lane: the Bresenham remainder climbs from the baseline and resets at
            // each onset, so inter-onset lengths show as slope changes.
            let band = 182.0 / TRACKS as f32;
            for tr in 0..TRACKS {
                let baseline = 14.0 + (tr as f32 + 1.0) * band - 2.0;
                mv(&mut v.edges, 12.0, baseline); ln(&mut v.edges, 198.0, baseline);
                let k = crate::sequence::pulse_hits(params.pulses, params.density, tr, n);
                if k == 0 { continue; }
                let height = band * 0.72;
                let phase = (tr * n / TRACKS) % n;
                for i in 0..n {
                    let d = crate::sequence::modular_distance((i + n - phase) % n, k, n);
                    let x = 13.0 + (i as f32 + 0.5) / n as f32 * 184.0;
                    let y = baseline - d as f32 / n as f32 * height;
                    if i == 0 { mv(&mut v.lines, x, y); } else { ln(&mut v.lines, x, y); }
                    if p.hit[tr][i] {
                        mv(&mut v.edges, x, baseline); ln(&mut v.edges, x, y);
                        v.nodes.push(Node { x, y: baseline, r: 2.6, on: true, cur: play == Some(i), label: String::new(), lane: u8::MAX });
                    }
                }
            }
            if let Some(i) = play { let x = 13.0 + (i as f32 + 0.5) / n as f32 * 184.0; mv(&mut v.hand, x, 12.0); ln(&mut v.hand, x, 196.0); }
        }
        Mode::LSystem => {
            // The grammar's own A/B word as a two-line strip, then per lane the gate the
            // generator rolled against: threshold x grammar weight x metrical weight.
            let cw = 184.0 / n as f32;
            let ch = 152.0 / TRACKS as f32;
            // The word the grammar produced, one symbol per step.
            for i in 0..n {
                let x = 13.0 + (i as f32 + 0.5) * cw;
                let a = p.lsym[i] == 0;
                v.nodes.push(Node { x, y: if a { 12.0 } else { 21.0 }, r: if a { 2.2 } else { 1.5 }, on: a, cur: false, label: String::new(), lane: u8::MAX });
            }
            for tr in 0..TRACKS {
                let baseline = 34.0 + (tr as f32 + 1.0) * ch;
                let threshold = crate::sequence::lsystem_threshold(params, tr);
                for i in 0..n {
                    // The generator decided this step before the pattern was rotated or
                    // mirrored, so the metrical weight belongs to the step the slot came from.
                    let g = src_index(params, p, i, n);
                    let chance = crate::sequence::lsystem_step_chance(threshold, p.lsym[i], tr, g);
                    let x = 13.0 + (i as f32 + 0.5) * cw;
                    let top = baseline - chance * (ch - 4.0);
                    mv(&mut v.lines, x, baseline); ln(&mut v.lines, x, top);
                    if p.hit[tr][i] {
                        v.nodes.push(Node { x, y: top, r: 3.4, on: true, cur: play == Some(i), label: String::new(), lane: u8::MAX });
                    }
                }
            }
            if let Some(i) = play { let x = 13.0 + (i as f32 + 0.5) * cw; mv(&mut v.hand, x, 8.0); ln(&mut v.hand, x, 192.0); }
        }
        Mode::PhaseShift => {
            // Every lane is the same motif at a different circular offset: draw eight congruent
            // outlines rotated against each other.
            let labels = ["K", "S", "CH", "OH", "CL", "RM", "T", "P"];
            let mut outer = 0.0;
            for tr in 0..TRACKS {
                let radius = 26.0 + tr as f32 * 9.0;
                outer = radius;
                circle(&mut v.edges, C, C, radius);
                let mut first_point = None;
                for i in 0..n {
                    if !p.hit[tr][i] { continue; }
                    let (x, y) = polar(radius, i as f32 / n as f32);
                    let first = first_point.is_none();
                    if first { mv(&mut v.lines, x, y); first_point = Some((x, y)); } else { ln(&mut v.lines, x, y); }
                    v.nodes.push(Node {
                        x, y,
                        r: if first { 6.4 } else { 3.4 },
                        on: true,
                        cur: play == Some(i),
                        label: if first { labels[tr].into() } else { String::new() }, lane: u8::MAX,
                    });
                }
                if let Some((x, y)) = first_point { ln(&mut v.lines, x, y); }
            }
            if let Some(i) = play {
                let (x, y) = polar(outer + 6.0, i as f32 / n as f32);
                mv(&mut v.hand, C, C); ln(&mut v.hand, x, y);
            }
        }
        Mode::Polyrhythm => {
            // Ratio stack: a dot ring per lane shows the pulse count, but sized by accent so the
            // long/short relationship of the ratios is visible rather than just counted.
            circle(&mut v.edges, C, C, 8.0);
            for tr in 0..TRACKS {
                let radius = 24.0 + tr as f32 * 9.0;
                circle(&mut v.edges, C, C, radius);
                for i in 0..n {
                    if !p.hit[tr][i] { continue; }
                    let (x, y) = polar(radius, i as f32 / n as f32);
                    let accent = p.accent[tr][i];
                    v.nodes.push(Node { x, y, r: 2.6 + accent * 2.4, on: true, cur: play == Some(i), label: String::new(), lane: u8::MAX });
                }
            }
            if let Some(i) = play { let (x, y) = polar(24.0 + 7.0 * 9.0, i as f32 / n as f32); mv(&mut v.hand, C, C); ln(&mut v.hand, x, y); }
        }
        Mode::Syncopation => {
            // The generator publishes lane 0's metrical score, so the bars are the weights that
            // drove the selection rather than a re-derivation of them.
            mv(&mut v.edges, 12.0, 176.0); ln(&mut v.edges, 198.0, 176.0);
            for tr in 0..TRACKS {
                let weight = if tr == 0 { 1.0 } else { 0.0 };
                for i in 0..n {
                    let value = (p.orbit[i] * weight).max(if p.hit[tr][i] { 1.0 } else { 0.0 });
                    if value <= 0.0 { continue; }
                    let x = 13.0 + (i as f32 + 0.5) / n as f32 * 184.0;
                    let y = 176.0 - value * 140.0;
                    mv(&mut v.lines, x, 176.0); ln(&mut v.lines, x, y);
                    v.nodes.push(Node { x, y: 176.0, r: if p.hit[tr][i] { 4.2 } else { 2.4 }, on: p.hit[tr][i], cur: play == Some(i), label: String::new(), lane: u8::MAX });
                }
            }
            if let Some(i) = play { let x = 13.0 + (i as f32 + 0.5) / n as f32 * 184.0; mv(&mut v.hand, x, 20.0); ln(&mut v.hand, x, 192.0); }
        }
        Mode::Logistic => {
            // Return map: y = r·x·(1−x) for the density's r, with orbit samples (x[n], x[n+1])
            // connected in order. The axes sit at the plot origin.
            let (bx, by, w, hgt) = (18.0, 147.0, 174.0, 125.0);
            mv(&mut v.edges, bx, by); ln(&mut v.edges, bx + w, by);
            mv(&mut v.edges, bx, by); ln(&mut v.edges, bx, 22.0);
            let r = crate::sequence::logistic_r(params.density);
            for s in 0..=48 {
                let x = s as f32 / 48.0;
                let y = (r * x as f64 * (1.0 - x as f64)) as f32;
                let px = bx + x * w;
                let py = by - y * hgt;
                if s == 0 { mv(&mut v.edges, px, py); } else { ln(&mut v.edges, px, py); }
            }
            // Dots on the parabola: every orbit sample is an (x, r·x·(1−x)) iteration. The
            // successor comes from generator order, not display order.
            for i in 0..n {
                let xv = p.orbit[i];
                let x = bx + xv * w;
                let y = by - (r * xv as f64 * (1.0 - xv as f64)) as f32 * hgt;
                let hit = aggregate_hit(p, i);
                v.nodes.push(Node { x, y, r: if hit { 4.8 } else { 2.4 }, on: true, cur: play == Some(i), label: String::new(), lane: u8::MAX });
            }
            for i in 0..n {
                let ox = 12.0 + (i as f32 + 0.5) / n as f32 * 186.0;
                let oy = 185.0 - p.orbit[i] * 32.0;
                if i == 0 { mv(&mut v.lines, ox, oy); } else { ln(&mut v.lines, ox, oy); }
            }
        }
        Mode::PebbleBox => {
            // The box, with each pebble at its recorded position for this step. Slint tweens
            // x/y between steps, so this is the same simulation the scheduler is playing.
            mv(&mut v.edges, 14.0, 14.0); ln(&mut v.edges, 196.0, 14.0);
            ln(&mut v.edges, 196.0, 196.0); ln(&mut v.edges, 14.0, 196.0); ln(&mut v.edges, 14.0, 14.0);
            let frame = play.unwrap_or(0).min(n - 1);
            for tr in 0..TRACKS {
                let mut x = 14.0 + p.pebbles[frame][tr * 2] as f32 / 255.0 * 182.0;
                let mut y = 14.0 + p.pebbles[frame][tr * 2 + 1] as f32 / 255.0 * 182.0;
                let hit = p.hit[tr][frame];
                let wall = p.impact[frame][tr];
                if hit {
                    // The frame records the end of the step, after the bounce: put the pebble
                    // back on the wall it actually struck and flash that wall.
                    match wall {
                        1 => x = 14.0,
                        2 => x = 196.0,
                        3 => y = 14.0,
                        4 => y = 196.0,
                        _ => {}
                    }
                    let len = (8.0 + 16.0 * p.accent[tr][frame]).min(30.0);
                    match wall {
                        1 => { mv(&mut v.hand, 14.0, (y - len).clamp(14.0, 196.0)); ln(&mut v.hand, 14.0, (y + len).clamp(14.0, 196.0)); }
                        2 => { mv(&mut v.hand, 196.0, (y - len).clamp(14.0, 196.0)); ln(&mut v.hand, 196.0, (y + len).clamp(14.0, 196.0)); }
                        3 => { mv(&mut v.hand, (x - len).clamp(14.0, 196.0), 14.0); ln(&mut v.hand, (x + len).clamp(14.0, 196.0), 14.0); }
                        4 => { mv(&mut v.hand, (x - len).clamp(14.0, 196.0), 196.0); ln(&mut v.hand, (x + len).clamp(14.0, 196.0), 196.0); }
                        _ => {}
                    }
                }
                v.nodes.push(Node { x, y, r: if hit { 6.2 } else { 4.6 }, on: true, cur: hit, label: String::new(), lane: u8::MAX });
            }
            // Gravity is the mode-scoped `ca_rule` knob; the arrow length shows its strength and
            // the direction follows the tilt.
            let tilt = params.phase.clamp(0.0, 1.0) * std::f32::consts::TAU;
            let gravity = crate::sequence::pebble_gravity(params.ca_rule);
            let len = 12.0 + gravity * 16.0;
            let (dx, dy) = (tilt.sin(), tilt.cos());
            let (px, py) = (-dy * 5.0, dx * 5.0);
            let tip = (C + dx * len, C + dy * len);
            mv(&mut v.lines, C, C);
            ln(&mut v.lines, tip.0, tip.1);
            mv(&mut v.lines, tip.0 - px, tip.1 - py);
            ln(&mut v.lines, tip.0 + px, tip.1 + py);
        }
        Mode::Sieve => {
            // One row per residue class: union terms cyan, the subtracted class amber. Below,
            // the full sieve in display slots, evaluated per cell so 1024-step patterns stay exact.
            let cls = crate::sequence::sieve_classes(params);
            for (row, &(m, r)) in cls.iter().enumerate() {
                let y = 34.0 + row as f32 * 26.0;
                mv(&mut v.edges, 12.0, y); ln(&mut v.edges, 198.0, y);
                let cw = 184.0 / m as f32;
                for cell in 0..m {
                    let selected = cell == r;
                    v.nodes.push(Node {
                        x: 13.0 + (cell as f32 + 0.5) * cw,
                        y,
                        r: if selected { 4.2 } else { 2.0 },
                        on: selected && row < 2,
                        cur: selected && row == 2,
                        label: String::new(), lane: u8::MAX,
                    });
                }
            }
            let y = 150.0;
            mv(&mut v.edges, 12.0, y); ln(&mut v.edges, 198.0, y);
            let cw = 184.0 / n as f32;
            for i in 0..n {
                // Undo both rotation and mirror: the mask is drawn in generator coordinates while
                // the hits are in display coordinates.
                let member = crate::sequence::sieve_member(params, src_index(params, p, i, n));
                let hit = aggregate_hit(p, i);
                v.nodes.push(Node {
                    x: 13.0 + (i as f32 + 0.5) * cw,
                    y,
                    r: if hit { 4.4 } else if member { 2.6 } else { 1.4 },
                    on: member,
                    cur: play == Some(i),
                    label: String::new(), lane: u8::MAX,
                });
            }
            if let Some(i) = play {
                let x = 13.0 + (i as f32 + 0.5) * cw;
                mv(&mut v.hand, x, 20.0); ln(&mut v.hand, x, 190.0);
            }
        }
        Mode::Kuramoto => {
            // Eight phase pointers on one circle. Low coupling spreads them; high coupling
            // bunches them into a rotating column. Trails show where each lane has been.
            let frame = play.unwrap_or(0).min(n - 1);
            let radius = 88.0;
            circle(&mut v.edges, C, C, radius);
            circle(&mut v.edges, C, C, 24.0);
            for tr in 0..TRACKS {
                let mut prev: Option<f32> = None;
                for i in 0..n {
                    let phase = p.phases[i][tr] as f32 / 255.0;
                    let (x, y) = polar(radius, phase);
                    match prev {
                        // A jump of half a turn or more is a wrap; start a new subpath so the
                        // trail never draws a chord across the circle.
                        Some(last) if (phase - last).rem_euclid(1.0) < 0.5 => ln(&mut v.lines, x, y),
                        _ => mv(&mut v.lines, x, y),
                    }
                    prev = Some(phase);
                }
            }
            for tr in 0..TRACKS {
                let phase = p.phases[frame][tr] as f32 / 255.0;
                let (x, y) = polar(radius, phase);
                let hit = p.hit[tr][frame];
                mv(&mut v.lines, C, C); ln(&mut v.lines, x, y);
                v.nodes.push(Node { x, y, r: if hit { 5.2 } else { 4.0 }, on: true, cur: hit, label: String::new(), lane: u8::MAX });
            }
            // The order parameter: the bank's mean phase. Long when the lanes lock, short when
            // they drift, so the picture shows synchrony without a second computation.
            let (mx, my) = crate::sequence::kuramoto_order(&p.phases[frame]);
            let len = (mx * mx + my * my).sqrt();
            if len > 0.001 {
                let (x, y) = (C + my / len * radius, C - mx / len * radius);
                mv(&mut v.hand, C, C); ln(&mut v.hand, x, y);
            }
        }
        Mode::Lorenz => {
            // Butterfly body: draw the attractor orbit for lane 0, colour the current point.
            let mut x = 0.1 + (params.seed * 0.7).clamp(0.0, 0.6);
            let mut y = 0.0;
            let mut z = 25.0;
            let mut prev: Option<(f32, f32)> = None;
            for i in 0..n {
                for _ in 0..10 {
                    let dx = 10.0 * (y - x);
                    let dy = x * (28.0 - z) - y;
                    let dz = x * y - (8.0 / 3.0) * z;
                    x += dx * 0.01; y += dy * 0.01; z += dz * 0.01;
                }
                let px = C + x * 5.0;
                let py = C - (z - 25.0) * 3.0;
                let (px, py) = (px.clamp(10.0, 200.0), py.clamp(10.0, 200.0));
                if let Some((lx, ly)) = prev { mv(&mut v.lines, lx, ly); ln(&mut v.lines, px, py); }
                prev = Some((px, py));
                if p.hit[0][i] {
                    v.nodes.push(Node { x: px, y: py, r: 2.6, on: true, cur: play == Some(i), label: String::new(), lane: u8::MAX });
                }
            }
            if let Some((px, py)) = prev {
                v.nodes.push(Node { x: px, y: py, r: 5.0, on: true, cur: true, label: String::new(), lane: u8::MAX });
            }
        }
        Mode::Arnold => {
            // Polar phase trajectory: circle + rotating points connected in order.
            circle(&mut v.edges, C, C, 96.0);
            circle(&mut v.edges, C, C, 30.0);
            line(&mut v.edges, (C, C), (C, C - 96.0));
            let mut prev: Option<(f32, f32)> = None;
            for i in 0..n {
                let radius = 30.0 + 66.0 * (i as f32 + 1.0) / n as f32;
                let angle = p.orbit[i];
                let (x, y) = polar(radius, angle);
                if let Some(q) = prev { mv(&mut v.lines, q.0, q.1); ln(&mut v.lines, x, y); }
                prev = Some((x, y));
                v.nodes.push(Node { x, y, r: if p.hit[0][i] { 4.5 } else { 2.0 }, on: p.hit[0][i], cur: play == Some(i), label: String::new(), lane: u8::MAX });
            }
            if let Some(i) = play {
                let radius = 30.0 + 66.0 * (i as f32 + 1.0) / n as f32;
                let (x, y) = polar(radius, p.orbit[i]);
                mv(&mut v.hand, C, C); ln(&mut v.hand, x, y);
            }
        }
        Mode::Hilbert => {
            // The generator's own curve: order from Pulses, quarter turns from Phase, same
            // sampling and lane mapping. Rotate the path with the points.
            let order = params.pulses.clamp(1, 5) as u32;
            let grid = 1usize << order;
            let turns = crate::sequence::hilbert_turns(params.phase);
            let cell = (P - 20.0) / grid as f32;
            for k in 0..=grid {
                let q = 10.0 + k as f32 * cell;
                line(&mut v.edges, (q, 10.0), (q, P - 10.0));
                line(&mut v.edges, (10.0, q), (P - 10.0, q));
            }
            let mut prev: Option<(f32, f32)> = None;
            for d in 0..grid * grid {
                let (gx, gy) = crate::sequence::hilbert_rotate(order, d, turns);
                let px = 10.0 + (gx as f32 + 0.5) * cell;
                let py = P - 10.0 - (gy as f32 + 0.5) * cell;
                if let Some(q) = prev { mv(&mut v.lines, q.0, q.1); ln(&mut v.lines, px, py); }
                prev = Some((px, py));
            }
            for i in 0..n {
                let x = 10.0 + (p.orbit[i] * (grid.max(1) - 1) as f32 + 0.5) * cell;
                let y = P - 10.0 - (p.theta[i] * (grid.max(1) - 1) as f32 + 0.5) * cell;
                let lanes = (0..TRACKS).filter(|&tr| p.hit[tr][i]).count();
                if lanes > 0 {
                    v.nodes.push(Node {
                        x, y, r: 2.4 + lanes.min(TRACKS) as f32 * 0.3,
                        on: true, cur: play == Some(i), label: String::new(), lane: u8::MAX,
                    });
                }
            }
            if let Some(i) = play {
                let x = 10.0 + (p.orbit[i] * (grid.max(1) - 1) as f32 + 0.5) * cell;
                let y = P - 10.0 - (p.theta[i] * (grid.max(1) - 1) as f32 + 0.5) * cell;
                mv(&mut v.hand, C, C); ln(&mut v.hand, x, y);
            }
        }
        Mode::Ulam => {
            // The generator publishes the spiral coordinates it actually used.
            mv(&mut v.edges, 2.0, 2.0); ln(&mut v.edges, P - 2.0, 2.0); ln(&mut v.edges, P - 2.0, P - 2.0); ln(&mut v.edges, 2.0, P - 2.0); ln(&mut v.edges, 2.0, 2.0);
            let at = |i: usize| (3.0 + p.orbit[i] * (P - 6.0), P - 3.0 - p.theta[i] * (P - 6.0));
            let mut prev: Option<(f32, f32)> = None;
            for i in 0..n {
                let q = at(i);
                if let Some(last) = prev { mv(&mut v.lines, last.0, last.1); ln(&mut v.lines, q.0, q.1); }
                prev = Some(q);
                let lanes = (0..TRACKS).filter(|&tr| p.hit[tr][i]).count();
                if lanes > 0 {
                    v.nodes.push(Node {
                        x: q.0, y: q.1,
                        r: 2.6 + lanes.min(TRACKS) as f32 * 0.35,
                        on: true, cur: play == Some(i), label: String::new(), lane: u8::MAX,
                    });
                }
            }
            if let Some(i) = play {
                let q = at(i);
                mv(&mut v.hand, C, C); ln(&mut v.hand, q.0, q.1);
            }
        }
        Mode::ThueMorse | Mode::RudinShapiro => {
            // The word the generator read, plus one band per lane with its selected onsets.
            let cw = 184.0 / n as f32;
            for i in 0..n {
                let x = 13.0 + (i as f32 + 0.5) * cw;
                let one = p.lsym[i] == 1;
                v.nodes.push(Node { x, y: if one { 12.0 } else { 22.0 }, r: if one { 2.4 } else { 1.6 }, on: one, cur: false, label: String::new(), lane: u8::MAX });
            }
            let ch = 152.0 / TRACKS as f32;
            for tr in 0..TRACKS {
                let y = 36.0 + (tr as f32 + 1.0) * ch;
                mv(&mut v.edges, 11.0, y); ln(&mut v.edges, 199.0, y);
                for i in 0..n {
                    if !p.hit[tr][i] { continue; }
                    v.nodes.push(Node {
                        x: 13.0 + (i as f32 + 0.5) * cw,
                        y,
                        r: 3.2,
                        on: true,
                        cur: play == Some(i),
                        label: String::new(), lane: u8::MAX,
                    });
                }
            }
            if let Some(i) = play {
                let x = 13.0 + (i as f32 + 0.5) * cw;
                mv(&mut v.hand, x, 8.0); ln(&mut v.hand, x, 194.0);
            }
        }
        Mode::Brownian | Mode::LevyFlight => {
            // Lane 0's published walk and the step sizes that ranked its onsets.
            mv(&mut v.edges, 12.0, 120.0); ln(&mut v.edges, 198.0, 120.0);
            for i in 0..n {
                let x = 12.0 + (i as f32 + 0.5) / n as f32 * 186.0;
                let y = 14.0 + (1.0 - p.orbit[i]) * 100.0;
                if i == 0 { mv(&mut v.lines, x, y); } else { ln(&mut v.lines, x, y); }
            }
            let peak = (0..n).fold(0.0f32, |m, i| m.max(p.theta[i])).max(1e-6);
            for i in 0..n {
                let x = 12.0 + (i as f32 + 0.5) / n as f32 * 186.0;
                let h = p.theta[i] / peak * 54.0;
                if h > 0.5 { mv(&mut v.lines, x, 196.0); ln(&mut v.lines, x, 196.0 - h); }
                if aggregate_hit(p, i) {
                    v.nodes.push(Node {
                        x,
                        y: 14.0 + (1.0 - p.orbit[i]) * 100.0,
                        r: 3.4,
                        on: true,
                        cur: play == Some(i),
                        label: String::new(), lane: u8::MAX,
                    });
                }
            }
            if let Some(i) = play {
                let x = 12.0 + (i as f32 + 0.5) / n as f32 * 186.0;
                mv(&mut v.hand, x, 8.0); ln(&mut v.hand, x, 198.0);
            }
        }
        Mode::TentMap => {
            // Return map y = mu min(x, 1-x), with the orbit samples on the curve.
            let (bx, by, w, hgt) = (18.0, 147.0, 174.0, 125.0);
            mv(&mut v.edges, bx, by); ln(&mut v.edges, bx + w, by);
            mv(&mut v.edges, bx, by); ln(&mut v.edges, bx, 22.0);
            let mu = crate::sequence::tent_mu(params.density);
            for s in 0..=48 {
                let x = s as f32 / 48.0;
                let value = (mu * (x as f64).min(1.0 - x as f64)) as f32;
                let px = bx + x * w;
                let py = by - value * hgt;
                if s == 0 { mv(&mut v.edges, px, py); } else { ln(&mut v.edges, px, py); }
            }
            for i in 0..n {
                let x = bx + p.orbit[i] * w;
                let y = by - (mu * (p.orbit[i] as f64).min(1.0 - p.orbit[i] as f64)) as f32 * hgt;
                let hit = aggregate_hit(p, i);
                v.nodes.push(Node { x, y, r: if hit { 4.2 } else { 2.0 }, on: true, cur: play == Some(i), label: String::new(), lane: u8::MAX });
            }
            for i in 0..n {
                let ox = 12.0 + (i as f32 + 0.5) / n as f32 * 186.0;
                let oy = 185.0 - p.u[i] * 32.0;
                if i == 0 { mv(&mut v.lines, ox, oy); } else { ln(&mut v.lines, ox, oy); }
            }
        }
        Mode::HenonMap => {
            // The attractor as the generator published it: (x, y) from orbit/theta.
            let at = |i: usize| (8.0 + p.orbit[i] * (P - 16.0), P - 8.0 - p.theta[i] * (P - 16.0));
            let mut prev: Option<(f32, f32)> = None;
            for i in 0..n {
                let q = at(i);
                if let Some(last) = prev { mv(&mut v.lines, last.0, last.1); ln(&mut v.lines, q.0, q.1); }
                prev = Some(q);
                if aggregate_hit(p, i) {
                    v.nodes.push(Node { x: q.0, y: q.1, r: 3.4, on: true, cur: play == Some(i), label: String::new(), lane: u8::MAX });
                }
            }
            if let Some(i) = play {
                let q = at(i);
                mv(&mut v.hand, C, C); ln(&mut v.hand, q.0, q.1);
            }
        }
        Mode::Bernoulli => {
            // Scatter of per-step draws with a threshold line.
            line(&mut v.edges, (8.0, P - 12.0), (P - 8.0, P - 12.0));
            for i in 0..n {
                let x = x_of(i, n);
                let y = 12.0 + (1.0 - p.u[i]) * (P - 24.0);
                if p.hit[0][i] {
                    v.nodes.push(Node { x, y, r: 4.0, on: true, cur: play == Some(i), label: String::new(), lane: u8::MAX });
                } else {
                    v.nodes.push(Node { x, y, r: 1.8, on: false, cur: false, label: String::new(), lane: u8::MAX });
                }
            }
            let prob = (params.density.clamp(0.0, 1.0) * params.probability.clamp(0.0, 1.0)).clamp(0.0, 1.0);
            let ty = 12.0 + (1.0 - prob) * (P - 24.0);
            line(&mut v.lines, (8.0, ty), (P - 8.0, ty));
            if let Some(i) = play {
                let x = x_of(i, n);
                mv(&mut v.hand, x, 12.0); ln(&mut v.hand, x, P - 12.0);
            }
        }
        Mode::ReactionDiffusion => {
            // Activator threshold plot: draw the activator curve and the threshold line.
            let mut started = false;
            for i in 0..n {
                let x = x_of(i, n);
                let y = 12.0 + (1.0 - p.u[i]) * (P - 24.0);
                if !started { mv(&mut v.lines, x, y); started = true; } else { ln(&mut v.lines, x, y); }
                if p.hit[0][i] {
                    v.nodes.push(Node { x, y, r: 3.5, on: true, cur: play == Some(i), label: String::new(), lane: u8::MAX });
                }
            }
            let thr = 1.0 - params.density.clamp(0.0, 1.0);
            let ty = 12.0 + (1.0 - thr) * (P - 24.0);
            line(&mut v.edges, (8.0, ty), (P - 8.0, ty));
            if let Some(i) = play {
                let x = x_of(i, n);
                mv(&mut v.hand, x, 12.0); ln(&mut v.hand, x, P - 12.0);
            }
        }
        Mode::Cantor => {
            // Cantor set: draw surviving intervals at successive depths.
            let mut cur_intervals: Vec<(f32, f32)> = vec![(8.0, P - 8.0)];
            for depth in 0..4 {
                let mut next: Vec<(f32, f32)> = Vec::new();
                for (a, b) in &cur_intervals {
                    let (a, b) = (*a, *b);
                    let third = (b - a) / 3.0;
                    next.push((a, a + third));
                    next.push((b - third, b));
                }
                let y = 30.0 + depth as f32 * 36.0;
                for (a, b) in &cur_intervals {
                    let (a, b) = (*a, *b);
                    line(&mut v.edges, (a, y), (b, y));
                }
                cur_intervals = next;
            }
            for i in 0..n {
                let x = x_of(i, n);
                let mut t = i as f32 / n as f32;
                let mut member = true;
                for _ in 0..6 {
                    t *= 3.0;
                    let digit = (t as usize) % 3;
                    if digit == 1 { member = false; break; }
                    t -= (t as usize) as f32;
                }
                if member && p.hit[0][i] {
                    v.nodes.push(Node { x, y: 30.0, r: 3.5, on: true, cur: play == Some(i), label: String::new(), lane: u8::MAX });
                }
            }
            if let Some(i) = play {
                let x = x_of(i, n);
                mv(&mut v.hand, x, 12.0); ln(&mut v.hand, x, P - 12.0);
            }
        }
        Mode::EuclideanSwing | Mode::AfricanBell | Mode::PhaseCanon | Mode::Fibonacci | Mode::Genetic | Mode::Sturmian | Mode::Vuza => {
            circle(&mut v.edges, C, C, 90.0);
            // At long pattern lengths, sample the ring instead of drawing up to 8k dots.
            let stride = n.div_ceil(96).max(1);
            for tr in 0..TRACKS {
                for i in (0..n).step_by(stride) {
                    if p.hit[tr][i] {
                        let a = (i as f32 / n as f32) * std::f32::consts::TAU;
                        let r = 30.0 + tr as f32 * 7.0;
                        let (x, y) = (C + r * a.cos(), C + r * a.sin());
                        v.nodes.push(Node { x, y, r: 3.0, on: true, cur: play == Some(i), label: String::new(), lane: u8::MAX });
                    }
                }
            }
            if let Some(i) = play {
                let a = (i as f32 / n as f32) * std::f32::consts::TAU;
                let (x, y) = (C + 90.0 * a.cos(), C + 90.0 * a.sin());
                mv(&mut v.hand, C, C); ln(&mut v.hand, x, y);
            }
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequence::generate;
    #[test]
    fn oscillator_view_matches_each_selected_waveform() {
        assert!((crate::sequence::oscillator_value(0, 0.25) - 1.0).abs() < 1e-5);
        assert!((crate::sequence::oscillator_value(1, 0.25) - 0.5).abs() < 1e-5);
        assert!((crate::sequence::oscillator_value(2, 0.25) - 0.25).abs() < 1e-5);
        assert_eq!(crate::sequence::oscillator_value(3, 0.25), 1.0);
        assert_eq!(crate::sequence::oscillator_value(3, 0.75), 0.0);
        for wave in 0..crate::sequence::oscillator_wave_count {
            for i in 0..=100 {
                assert!((0.0..=1.0).contains(&crate::sequence::oscillator_value(wave, i as f32 / 100.0)),
                    "wave {wave} left 0..=1 at phase {}", i as f32 / 100.0);
            }
        }
    }

    /// The panel must draw the curve the generator ranked, after rotation: every lane-0 onset
    /// sits at or above the drawn cutoff and every other step at or below it.
    #[test]
    fn oscillator_panel_follows_the_rotated_ranked_curve() {
        for (invert, mirror) in [(false, false), (true, false), (false, true), (true, true)] {
            for rotate in [0usize, 3, 7, 11] {
                for wave in 0..crate::sequence::oscillator_wave_count {
                    let g = GenParams {
                        kind: Mode::Oscillator, steps: 16, pulses: 5, density: 0.8, probability: 1.0,
                        rotate, osc_wave: wave, invert, mirror, ..GenParams::default()
                    };
                    let pat = generate(&g);
                    let cutoff = oscillator_cutoff(&pat, 16, invert);
                    let ctx = format!("wave {wave} rotate {rotate} invert {invert} mirror {mirror}");
                    for i in 0..16 {
                        // Normal: hits rank at or above the cutoff. Inverted: at or below it.
                        let (hit, v) = (pat.hit[0][i], pat.orbit[i]);
                        if invert { if hit { assert!(v <= cutoff + 1e-6, "{ctx}: step {i}"); } else { assert!(v >= cutoff - 1e-6, "{ctx}: step {i} outranked by the cutoff"); } }
                        else if hit { assert!(v >= cutoff - 1e-6, "{ctx}: step {i}"); } else { assert!(v <= cutoff + 1e-6, "{ctx}: step {i} outranks the cutoff"); }
                    }
                    let view = build(&g, &pat, -1);
                    assert_eq!(view.nodes.len(), 16);
                    // Node heights are the published (rotated, mirrored) samples, not a fresh waveform.
                    for (i, node) in view.nodes.iter().enumerate() {
                        assert!((node.y - (176.0 - pat.orbit[i] * 130.0)).abs() < 1e-4, "{ctx}");
                    }
                }
            }
        }
    }

    #[test]
    fn every_mode_draws_a_populated_panel() {
        // Every mode must produce panel geometry; a mode that falls through the match would
        // otherwise render as an empty box with no compile error.
        let p = GenParams::default();
        for k in Mode::all() {
            let g = GenParams { kind: k, ..p };
            let view = build(&g, &generate(&g), -1);
            assert!(!view.nodes.is_empty(), "{k:?} produced no view nodes");
            // Path data is split across `edges` and `lines` depending on the drawing, so
            // only the aggregate is a real invariant.
            let paths = format!("{}{}{}", view.edges, view.lines, view.hand);
            assert!(paths.starts_with('M'), "{k:?} produced malformed path data");
            for node in &view.nodes {
                assert!(node.x.is_finite() && node.y.is_finite() && node.r.is_finite(),
                    "{k:?} produced a non-finite node");
            }
        }
    }

    #[test]
    fn phyllotaxis_panel_draws_each_lane_trigger_window() {
        let g = GenParams { kind: Mode::Phyllotaxis, steps: 16, pulses: 5, density: 0.65, ..GenParams::default() };
        let view = build(&g, &generate(&g), -1);
        let radii: Vec<f32> = view.lines.split('A').skip(1)
            .filter_map(|chunk| chunk.split_whitespace().next())
            .filter_map(|v| v.parse::<f32>().ok())
            .collect();
        assert_eq!(radii.len(), TRACKS * 2, "one primary and one secondary arc per lane");
        let mut unique = radii.clone();
        unique.sort_by(|a, b| a.partial_cmp(b).unwrap());
        unique.dedup_by(|a, b| (*a - *b).abs() < 0.01);
        assert_eq!(unique.len(), TRACKS * 2, "arcs must not collapse onto one radius");
    }

    #[test]
    fn phyllotaxis_panel_stays_bounded_at_long_patterns() {
        // The sampled rosette must stay small: a node per lane per step would reach 16k nodes.
        let g = GenParams {
            kind: Mode::Phyllotaxis,
            steps: MAX_STEPS,
            pulses: MAX_STEPS,
            density: 1.0,
            probability: 1.0,
            ..GenParams::default()
        };
        let view = build(&g, &generate(&g), 0);
        assert!(view.nodes.len() <= 1_200, "phyllotaxis drew {} nodes", view.nodes.len());
        assert!(view.nodes.iter().any(|n| n.lane < TRACKS as u8), "no per-lane rosette nodes");
        assert!(view.nodes.iter().any(|n| n.on), "sampled rosette lost every hit");
    }

    #[test]
    fn pebble_hits_are_drawn_on_the_wall_they_struck() {
        let g = GenParams { kind: Mode::PebbleBox, steps: 32, pulses: 8, density: 1.0, ca_rule: 200, ..GenParams::default() };
        let pat = generate(&g);
        let mut hits = 0;
        for step in 0..32 {
            let view = build(&g, &pat, step as i32);
            assert_eq!(view.nodes.len(), TRACKS);
            let mut step_hits = 0;
            for tr in 0..TRACKS {
                if !pat.hit[tr][step] { continue; }
                step_hits += 1;
                hits += 1;
                let node = &view.nodes[tr];
                match pat.impact[step][tr] {
                    1 => assert!((node.x - 14.0).abs() < 0.01, "left wall hit drawn at x={}", node.x),
                    2 => assert!((node.x - 196.0).abs() < 0.01, "right wall hit drawn at x={}", node.x),
                    3 => assert!((node.y - 14.0).abs() < 0.01, "top wall hit drawn at y={}", node.y),
                    4 => assert!((node.y - 196.0).abs() < 0.01, "bottom wall hit drawn at y={}", node.y),
                    wall => panic!("hit at step {step} lane {tr} recorded wall {wall}"),
                }
            }
            assert_eq!(!view.hand.is_empty(), step_hits > 0, "wall flash missing at step {step}");
        }
        assert!(hits > 0, "no pebble hits to check");
    }

    #[test]
    fn the_playhead_is_only_set_for_a_valid_step() {
        let g = GenParams::default();
        let pat = generate(&g);
        assert!(build(&g, &pat, -1).nodes.iter().all(|n| !n.cur));
        assert!(build(&g, &pat, 0).nodes.iter().any(|n| n.cur));
        assert!(build(&g, &pat, 9999).nodes.iter().all(|n| !n.cur));
    }

    #[test]
    fn word_panels_draw_the_published_word_and_lane_onsets() {
        for kind in [Mode::ThueMorse, Mode::RudinShapiro] {
            let g = GenParams { kind, steps: 32, pulses: 12, density: 0.8, ..GenParams::default() };
            let pat = generate(&g);
            let view = build(&g, &pat, -1);
            let word: Vec<&Node> = view.nodes.iter().filter(|n| n.y == 12.0 || n.y == 22.0).collect();
            assert_eq!(word.len(), 32, "{kind:?}: one word cell per step");
            for (i, node) in word.iter().enumerate() {
                assert_eq!(node.on, pat.lsym[i] == 1, "{kind:?} word step {i}");
            }
            let hit_nodes = view.nodes.iter().filter(|n| n.y > 30.0).count();
            let hits: usize = (0..TRACKS).map(|tr| pat.hit[tr][..32].iter().filter(|&&h| h).count()).sum();
            assert_eq!(hit_nodes, hits, "{kind:?}: one marker per selected onset");
        }
    }

    #[test]
    fn walk_panels_plot_the_published_orbit() {
        for kind in [Mode::Brownian, Mode::LevyFlight] {
            let g = GenParams { kind, steps: 32, pulses: 12, density: 0.8, ..GenParams::default() };
            let pat = generate(&g);
            let view = build(&g, &pat, -1);
            for node in view.nodes.iter().filter(|n| n.y < 120.0) {
                let covered = (0..32).any(|i| {
                    let x = 12.0 + (i as f32 + 0.5) / 32.0 * 186.0;
                    let y = 14.0 + (1.0 - pat.orbit[i]) * 100.0;
                    (node.x - x).abs() < 1e-4 && (node.y - y).abs() < 1e-4
                });
                assert!(covered, "{kind:?}: node ({}, {}) is not on the published orbit", node.x, node.y);
            }
        }
    }

    #[test]
    fn tent_panel_dots_sit_on_the_return_map() {
        let g = GenParams { kind: Mode::TentMap, steps: 32, pulses: 12, density: 0.65, ..GenParams::default() };
        let pat = generate(&g);
        let view = build(&g, &pat, -1);
        let (bx, by, w, hgt) = (18.0, 147.0, 174.0, 125.0);
        let mu = crate::sequence::tent_mu(0.65);
        assert!(!view.nodes.is_empty());
        for node in &view.nodes {
            let x = (node.x - bx) / w;
            let want = by - (mu * (x as f64).min(1.0 - x as f64)) as f32 * hgt;
            assert!((node.y - want).abs() < 0.5, "dot off the tent map: {} vs {want}", node.y);
        }
    }

    #[test]
    fn henon_panel_uses_the_published_pair() {
        let g = GenParams { kind: Mode::HenonMap, steps: 32, pulses: 12, density: 0.65, ..GenParams::default() };
        let pat = generate(&g);
        let view = build(&g, &pat, -1);
        for node in &view.nodes {
            let covered = (0..32).any(|i| {
                let x = 8.0 + pat.orbit[i] * (P - 16.0);
                let y = P - 8.0 - pat.theta[i] * (P - 16.0);
                (node.x - x).abs() < 1e-4 && (node.y - y).abs() < 1e-4
            });
            assert!(covered, "Henon node ({}, {}) is not a published attractor point", node.x, node.y);
        }
    }

    #[test]
    fn src_index_undoes_rotation_and_mirror() {
        let n = 17usize;
        for (rotate, mirror) in [(0usize, false), (5, false), (0, true), (5, true)] {
            for kind in Mode::all() {
                let p = GenParams { kind, steps: n, pulses: 7, density: 0.82, probability: 1.0, rotate, mirror, ..GenParams::default() };
                let pat = generate(&p);
                let base = generate(&GenParams { rotate: 0, mirror: false, ..p });
                for i in 0..n {
                    let src = src_index(&p, &pat, i, n);
                    assert_eq!(pat.hit[0][i], base.hit[0][src], "{kind:?} rotate={rotate} mirror={mirror} slot {i}");
                }
            }
        }
    }

    #[test]
    fn sieve_panel_follows_mirror_as_well_as_rotation() {
        for (rotate, mirror) in [(0usize, false), (3, false), (0, true), (5, true)] {
            let g = GenParams { kind: Mode::Sieve, steps: 16, pulses: 2, density: 1.0, rotate, mirror, ..GenParams::default() };
            let pat = generate(&g);
            let view = build(&g, &pat, -1);
            let strip: Vec<&Node> = view.nodes.iter().filter(|n| (n.y - 150.0).abs() < 0.01).collect();
            assert_eq!(strip.len(), 16);
            for (i, node) in strip.iter().enumerate() {
                let src = src_index(&g, &pat, i, 16);
                let bit = crate::sequence::sieve_member(&g, src);
                assert_eq!(node.on, bit, "rotate={rotate} mirror={mirror} step {i}");
            }
        }
    }

    #[test]
    fn sieve_panel_draws_the_generator_mask() {
        let g = GenParams { kind: Mode::Sieve, steps: 16, pulses: 2, density: 0.8, ..GenParams::default() };
        let pat = generate(&g);
        let view = build(&g, &pat, -1);
        let strip: Vec<&Node> = view.nodes.iter().filter(|n| (n.y - 150.0).abs() < 0.01).collect();
        assert_eq!(strip.len(), 16, "one mask cell per step");
        for (i, node) in strip.iter().enumerate() {
            let bit = crate::sequence::sieve_member(&g, (i + 16 - pat.rotation) % 16);
            assert_eq!(node.on, bit, "step {i}");
        }
        // Three residue rows, each with one selected cell.
        for (row, y) in [34.0f32, 60.0, 86.0].iter().enumerate() {
            let selected = view.nodes.iter().filter(|n| (n.y - y).abs() < 0.01 && (n.on || n.cur)).count();
            assert_eq!(selected, 1, "residue row {row} has {selected} selected cells");
        }
    }

    #[test]
    fn kuramoto_panel_draws_one_pointer_per_phase() {
        let g = GenParams { kind: Mode::Kuramoto, steps: 16, pulses: 6, density: 0.8, ..GenParams::default() };
        let pat = generate(&g);
        let frame = 5usize;
        let view = build(&g, &pat, frame as i32);
        assert_eq!(view.nodes.len(), TRACKS, "one pointer per lane, so Slint can tween them");
        for tr in 0..TRACKS {
            let phase = pat.phases[frame][tr] as f32 / 255.0;
            let (x, y) = polar(88.0, phase);
            let node = &view.nodes[tr];
            assert!((node.x - x).abs() < 0.01 && (node.y - y).abs() < 0.01,
                "lane {tr} pointer not on its published phase");
        }
    }

    #[test]
    fn every_panel_stays_inside_the_viewbox() {
        // A mode whose geometry escapes the 210x210 panel is clipped by Slint and reads as a
        // blank or half-drawn box. Every mode, at the sizes most likely to overflow.
        for k in Mode::all() {
            for steps in [1usize, 2, 7, 16, 32, MAX_STEPS] {
                for density in [0.0f32, 0.5, 1.0] {
                    let g = GenParams { kind: k, steps, pulses: steps, density, probability: 1.0, ..GenParams::default() };
                    for cur in [-1, 0, (steps / 2) as i32] {
                        for node in build(&g, &generate(&g), cur).nodes {
                            assert!(node.x >= -12.0 && node.x <= SIZE + 12.0 && node.y >= -12.0 && node.y <= SIZE + 12.0,
                                "{k:?} steps={steps} density={density} node=({:.1},{:.1})", node.x, node.y);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn polyrhythm_rings_grow_with_the_lane() {
        // Lane tr is drawn on radius 24 + tr*9; the test reads those radii back out of the path
        // data rather than restating them as constants, so the two cannot drift apart.
        let g = GenParams { kind: Mode::Polyrhythm, steps: 16, pulses: 8, density: 1.0, ..GenParams::default() };
        let view = build(&g, &generate(&g), -1);
        // `circle` writes two half-arcs, so each radius appears twice per lane ring.
        let mut radii: Vec<f32> = view.edges.split('A').skip(1)
            .filter_map(|chunk| chunk.split_whitespace().next())
            .filter_map(|v| v.parse::<f32>().ok())
            .filter(|r| *r > 8.0)
            .collect();
        radii.sort_by(|a, b| a.partial_cmp(b).unwrap());
        radii.dedup_by(|a, b| (*a - *b).abs() < 0.15);
        assert_eq!(radii.len(), TRACKS, "expected one ring per lane, found {radii:?}");
        for tr in 0..TRACKS {
            let want = 24.0 + tr as f32 * 9.0;
            assert!((radii[tr] - want).abs() < 0.15, "lane ring {tr} at radius {:.2}, expected {want:.2}: {radii:?}", radii[tr]);
        }
    }

    #[test]
    fn phase_shift_rings_trace_each_lane_motif() {
        // One ring per lane; every onset sits on the lane's ring and the labelled node marks
        // the lane's first onset. The outlines are the shared motif, rotated per lane.
        let g = GenParams { kind: Mode::PhaseShift, steps: 16, pulses: 4, density: 1.0, ..GenParams::default() };
        let pat = generate(&g);
        let view = build(&g, &pat, -1);
        let labels = ["K", "S", "CH", "OH", "CL", "RM", "T", "P"];
        for tr in 0..TRACKS {
            let radius = 26.0 + tr as f32 * 9.0;
            let first = (0..16).find(|&i| pat.hit[tr][i]).expect("lane has an onset");
            let (fx, fy) = polar(radius, first as f32 / 16.0);
            let label_node = view.nodes.iter().find(|nd| nd.label == labels[tr]).expect("labelled first onset");
            assert!((label_node.x - fx).abs() < 0.6 && (label_node.y - fy).abs() < 0.6,
                "lane {tr} label is not on its first onset");
            for i in 0..16 {
                if !pat.hit[tr][i] { continue; }
                let (x, y) = polar(radius, i as f32 / 16.0);
                assert!(view.nodes.iter().any(|nd| (nd.x - x).abs() < 0.6 && (nd.y - y).abs() < 0.6),
                    "lane {tr} onset {i} missing from its ring");
            }
        }
    }

    #[test]
    fn well_formed_traces_one_band_per_lane() {
        // Regression: all eight lanes shared one 55px strip at the bottom and became a pile.
        let g = GenParams { kind: Mode::WellFormed, steps: 32, pulses: 5, density: 0.7, ..GenParams::default() };
        let view = build(&g, &generate(&g), -1);
        let mut rows: Vec<f32> = view.nodes.iter().map(|n| n.y).collect();
        rows.sort_by(|a, b| a.partial_cmp(b).unwrap());
        rows.dedup_by(|a, b| (*a - *b).abs() < 0.5);
        assert!(rows.len() >= 6, "lanes collapsed into {} rows", rows.len());
        // Every baseline is one band apart and inside the panel.
        let band = 182.0 / TRACKS as f32;
        for row in &rows {
            let lane = (row - 14.0 + 2.0) / band - 1.0;
            assert!((lane - lane.round()).abs() < 0.02, "row {row} is not a lane baseline");
            assert!((14.0..=196.0).contains(row));
        }
    }

    #[test]
    fn logistic_dots_sit_on_the_drawn_return_map() {
        // The parabola is y = r*x*(1-x) for the density's r; every plotted pair must lie on it.
        let (steps, density) = (32usize, 0.65f32);
        let g = GenParams { kind: Mode::Logistic, steps, density, ..GenParams::default() };
        let pat = generate(&g);
        let view = build(&g, &pat, -1);
        let r = crate::sequence::logistic_r(density) as f32;
        let (bx, by, w, hgt) = (18.0, 147.0, 174.0, 125.0);
        assert_eq!(view.nodes.len(), steps, "every orbit sample is one iteration");
        for (i, n) in view.nodes.iter().enumerate() {
            let x = (n.x - bx) / w;
            assert!((0.0..=1.0).contains(&x), "dot {i} outside the domain");
            let want = by - r * x * (1.0 - x) * hgt;
            assert!((n.y - want).abs() < 0.5, "dot {i} off the parabola: {} vs {want}", n.y);
        }
    }

    #[test]
    fn pebble_panel_reads_the_recorded_frame() {
        // The Slint panel tweens node x/y between steps, so each step's eight nodes must be the
        // recorded simulation positions in stable lane order.
        let g = GenParams { kind: Mode::PebbleBox, steps: 16, pulses: 5, density: 0.8, ..GenParams::default() };
        let pat = generate(&g);
        let view = build(&g, &pat, 5);
        assert_eq!(view.nodes.len(), TRACKS);
        for tr in 0..TRACKS {
            let x = 14.0 + pat.pebbles[5][tr * 2] as f32 / 255.0 * 182.0;
            let y = 14.0 + pat.pebbles[5][tr * 2 + 1] as f32 / 255.0 * 182.0;
            assert!((view.nodes[tr].x - x).abs() < 0.01 && (view.nodes[tr].y - y).abs() < 0.01,
                "lane {tr} does not sit on its recorded pebble");
        }
        assert_ne!(pat.pebbles[5], pat.pebbles[6], "no motion between frames");
    }

    #[test]
    fn toussaint_places_onsets_at_their_real_step_angle() {
        // Regression: nodes sat at the ordinal angle j/k while the spokes used the true step,
        // so every spoke crossed the circle and the picture was a star.
        let n = 16;
        let g = GenParams { kind: Mode::ToussaintDistance, steps: n, pulses: 5, density: 0.8, ..GenParams::default() };
        let pat = generate(&g);
        let view = build(&g, &pat, -1);
        let mut nodes = 0;
        for tr in 0..TRACKS {
            let radius = 30.0 + tr as f32 * 8.5;
            for i in 0..n {
                if !pat.hit[tr][i] { continue; }
                let (x, y) = polar(radius, i as f32 / n as f32);
                assert!(view.nodes.iter().any(|nd| (nd.x - x).abs() < 0.6 && (nd.y - y).abs() < 0.6),
                    "lane {tr} onset {i} is not at its step angle");
                nodes += 1;
            }
        }
        assert!(nodes > 0);
    }

    #[test]
    fn cellular_highlights_the_generation_each_lane_reads() {
        // Regression: the panel marked `pulses % CA_ROWS`, ignoring the phase offset and the
        // per-lane step, so a marker could sit on a row the generator never plays.
        let g = GenParams { kind: Mode::Cellular, steps: 16, pulses: 5, phase: 0.5, ..GenParams::default() };
        let view = build(&g, &generate(&g), 0);
        let row_y = |row: usize| 18.0 + (row as f32 + 0.5) * (172.0 / CA_ROWS as f32);
        let marked: Vec<f32> = view.nodes.iter().filter(|n| n.cur).map(|n| n.y).collect();
        assert!(marked.iter().any(|y| (*y - row_y(cellular_row(&g, 0))).abs() < 0.6), "lane 0 row missing: {marked:?}");
        // The lane offset puts later lanes on different rows, so one marker cannot serve all and
        // the panel has to mark each lane's own generation.
        assert_ne!(cellular_row(&g, 0), cellular_row(&g, 1));
        assert!(marked.iter().any(|y| (*y - row_y(cellular_row(&g, 1))).abs() < 0.6), "lane 1 row missing: {marked:?}");
    }
}
