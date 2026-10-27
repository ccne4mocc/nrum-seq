//! Pure GUI geometry. Keep this free of Slint and plug-in types so every view can be tested.
use crate::sequence::{GenParams, Pattern, MAX_STEPS, TRACKS, CA_ROWS};
use crate::modes::Mode;
use std::f32::consts::TAU;
use std::fmt::Write;

const SIZE: f32 = 210.0;
const C: f32 = SIZE * 0.5;

#[derive(Clone, Debug, Default)]
pub struct Node { pub x: f32, pub y: f32, pub r: f32, pub on: bool, pub cur: bool, pub label: String }
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

/// Generation the cellular generator reads for one lane. Must match `cellular::generate`;
/// it also collapses to row 0 at one step, where the ring wraps onto itself.
fn selected_row(params: &GenParams, tr: usize) -> usize {
    if params.steps <= 1 {
        0
    } else {
        (params.pulses + (params.phase * CA_ROWS as f32).round() as usize + tr * 2) % CA_ROWS
    }
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
                    v.nodes.push(Node { x, y, r: if on { 3.6 } else { 1.8 }, on, cur: play == Some(i), label: String::new() });
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
                v.nodes.push(Node { x, y, r: if i == 0 { 9.0 } else { 11.0 }, on: i != 0, cur: state_cur, label: labels[i].into() });
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
            circle(&mut v.edges, C, C, max_r);
            for tr in 0..TRACKS {
                let threshold = crate::sequence::phyllotaxis_threshold(params, tr) as f32;
                arc(&mut v.lines, max_r - 2.0 - tr as f32 * 2.2, 0.0, threshold);
                arc(&mut v.lines, (sec_r - 2.0 - tr as f32 * 1.2).max(18.0), 0.0, threshold * 0.18);
            }

            let stride = n.div_ceil(128);
            for g0 in (0..n).step_by(stride) {
                let g1 = (g0 + stride).min(n);
                let mut lanes = 0usize;
                for g in g0..g1 {
                    let slot = (g + shift) % n;
                    for tr in 0..TRACKS {
                        if p.hit[tr][slot]
                            && crate::sequence::phyllotaxis_rules(g, tr, params) & 1 != 0
                        {
                            lanes += 1;
                        }
                    }
                }
                let mid = g0 + (g1 - g0) / 2;
                let r = max_r * ((mid as f32 + 1.0) / n as f32).sqrt();
                let (x, y) = polar(r, crate::sequence::phyllotaxis_primary(mid, 0, params) as f32);
                let cur = (g0..g1).any(|g| play == Some((g + shift) % n));
                let on = lanes > 0;
                v.nodes.push(Node {
                    x, y,
                    r: if on { 3.0 + lanes.min(TRACKS) as f32 * 0.35 } else { 1.1 },
                    on, cur, label: String::new(),
                });

                // The secondary rule is sparse by design, so draw only the groups where it
                // fired. That keeps the inner band readable and its length bounded too.
                let mut secondary = 0usize;
                for g in g0..g1 {
                    let slot = (g + shift) % n;
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
                        on: true, cur: false, label: String::new(),
                    });
                }
            }
            // The playhead: the radius line to the step currently sounding, on lane 0.
            if let Some(i) = play {
                let g = (i + n - shift) % n;
                let r = max_r * ((g as f32 + 1.0) / n as f32).sqrt();
                let (x, y) = polar(r, crate::sequence::phyllotaxis_primary(g, 0, params) as f32);
                mv(&mut v.hand, C, C); ln(&mut v.hand, x, y);
            }
        }
        Mode::Cellular => {
            let cw = 184.0 / n as f32;
            let ch = 172.0 / CA_ROWS as f32;
            // Each lane reads its own generation, so mark every lane's row rather than the one
            // the old `pulses % CA_ROWS` guess pointed at.
            let lane_rows: [usize; TRACKS] = std::array::from_fn(|tr| selected_row(params, tr));
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
                        label: String::new(),
                    });
                }
            }
            for row in 0..=CA_ROWS {
                let y = 18.0 + row as f32 * ch; mv(&mut v.edges, 12.0, y); ln(&mut v.edges, 198.0, y);
            }
        }
        Mode::Oscillator => {
            mv(&mut v.edges, 12.0, 105.0); ln(&mut v.edges, 198.0, 105.0);
            let points = (n * 3).max(24);
            for j in 0..points {
                let phase = (j as f32 / points as f32 * params.osc_rate + params.phase).fract();
                let value = crate::sequence::oscillator_value(params.osc_wave, phase);
                let x = 12.0 + j as f32 / (points - 1) as f32 * 186.0;
                let y = 176.0 - value * 130.0;
                if j == 0 { mv(&mut v.lines, x, y); } else { ln(&mut v.lines, x, y); }
            }
            let threshold = crate::sequence::oscillator_threshold(params);
            let threshold_y = 176.0 - (1.0 - threshold) * 130.0;
            mv(&mut v.edges, 12.0, threshold_y); ln(&mut v.edges, 198.0, threshold_y);
            for i in 0..n {
                let x = 12.0 + (i as f32 + 0.5) / n as f32 * 186.0;
                let phase = (i as f32 / n as f32 * params.osc_rate + params.phase).fract();
                let value = crate::sequence::oscillator_value(params.osc_wave, phase);
                v.nodes.push(Node { x, y: 176.0 - value * 130.0, r: 3.0, on: aggregate_hit(p, i), cur: play == Some(i), label: String::new() });
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
                    v.nodes.push(Node { x, y, r: if on { 4.1 } else { 2.0 }, on, cur: play == Some(i), label: String::new() });
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
            // The generator's own envelope and cutoff, so the drawn curve is the curve that
            // produced the hits. Orbit lane 0's envelope because that is the one it publishes.
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
                v.nodes.push(Node { x, y: 180.0 - value * 148.0, r: 3.0, on: aggregate_hit(p, i), cur: play == Some(i), label: String::new() });
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
                    v.nodes.push(Node { x, y, r: if on { 4.3 } else { 1.8 }, on, cur: play == Some(i), label: String::new() });
                }
            }
            if let Some(i) = play { let x = 13.0 + (i as f32 + 0.5) * cw; mv(&mut v.hand, x, 16.0); ln(&mut v.hand, x, 196.0); }
        }
        Mode::ToussaintDistance => {
            // Distance circle: one ring per lane. Every onset sits at its *actual* step angle on
            // the lane's ring, with a radial spoke to the rim and a tick for each grid step, so
            // the two inter-onset gaps read directly against the step grid.
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
                        label: String::new(),
                    });
                }
            }
            if let Some(i) = play { let (x, y) = polar(100.0, i as f32 / n as f32); mv(&mut v.hand, C, C); ln(&mut v.hand, x, y); }
        }
        Mode::WellFormed => {
            // One band per lane: the Bresenham remainder climbs from the lane's baseline and
            // resets at each onset, so the two inter-onset lengths are visible as the slope
            // changes. Stacked bands keep eight lanes from collapsing into one sawtooth pile.
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
                        v.nodes.push(Node { x, y: baseline, r: 2.6, on: true, cur: play == Some(i), label: String::new() });
                    }
                }
            }
            if let Some(i) = play { let x = 13.0 + (i as f32 + 0.5) / n as f32 * 184.0; mv(&mut v.hand, x, 12.0); ln(&mut v.hand, x, 196.0); }
        }
        Mode::LSystem => {
            // The grammar's own output: the A/B word it rewrote to, drawn as a two-line strip
            // (A above, B below), and under it, per lane, the gate the generator actually
            // rolled against — threshold x grammar weight x metrical weight.
            //
            // The old panel inferred a "nesting depth" by scanning runs of hits, which is a
            // shape the generator never computes.
            let cw = 184.0 / n as f32;
            let ch = 152.0 / TRACKS as f32;
            let shift = p.rotation % n;
            // The word the grammar produced, one symbol per step.
            for i in 0..n {
                let x = 13.0 + (i as f32 + 0.5) * cw;
                let a = p.lsym[i] == 0;
                v.nodes.push(Node { x, y: if a { 12.0 } else { 21.0 }, r: if a { 2.2 } else { 1.5 }, on: a, cur: false, label: String::new() });
            }
            for tr in 0..TRACKS {
                let baseline = 34.0 + (tr as f32 + 1.0) * ch;
                let threshold = crate::sequence::lsystem_threshold(params, tr);
                for i in 0..n {
                    // The generator decided this step before the pattern was rotated, so the
                    // metrical weight belongs to the step the slot came from.
                    let g = (i + n - shift) % n;
                    let chance = crate::sequence::lsystem_step_chance(threshold, p.lsym[i], tr, g);
                    let x = 13.0 + (i as f32 + 0.5) * cw;
                    let top = baseline - chance * (ch - 4.0);
                    mv(&mut v.lines, x, baseline); ln(&mut v.lines, x, top);
                    if p.hit[tr][i] {
                        v.nodes.push(Node { x, y: top, r: 3.4, on: true, cur: play == Some(i), label: String::new() });
                    }
                }
            }
            if let Some(i) = play { let x = 13.0 + (i as f32 + 0.5) * cw; mv(&mut v.hand, x, 8.0); ln(&mut v.hand, x, 192.0); }
        }
        Mode::PhaseShift => {
            // Every lane is the same motif at a different circular offset, so draw the motif
            // itself on one ring per lane: eight congruent outlines rotated against each other.
            // The old first-onset spokes showed only the offsets and hid the shared rhythm.
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
                        label: if first { labels[tr].into() } else { String::new() },
                    });
                }
                // Close the polygon so each lane reads as one rotated copy of the motif.
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
                    v.nodes.push(Node { x, y, r: 2.6 + accent * 2.4, on: true, cur: play == Some(i), label: String::new() });
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
                    v.nodes.push(Node { x, y: 176.0, r: if p.hit[tr][i] { 4.2 } else { 2.4 }, on: p.hit[tr][i], cur: play == Some(i), label: String::new() });
                }
            }
            if let Some(i) = play { let x = 13.0 + (i as f32 + 0.5) / n as f32 * 184.0; mv(&mut v.hand, x, 20.0); ln(&mut v.hand, x, 192.0); }
        }
        Mode::Logistic => {
            // Return map: the parabola is y = r·x·(1−x) for the density's r, and the samples are
            // the orbit's (x[n], x[n+1]) pairs connected in order. The old centre cross was not
            // an axis at all; the actual axes sit at the plot origin.
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
            // Dots on the parabola: each pair (x[n], x[n+1]) is one iteration, and the wrap from
            // the last step back to the first is not an iteration, so it is skipped. Connecting
            // them drew chords that do not exist in the map.
            for i in 0..n.saturating_sub(1) {
                let x = bx + p.orbit[i] * w;
                let y = by - p.orbit[i + 1] * hgt;
                let hit = aggregate_hit(p, i);
                v.nodes.push(Node { x, y, r: if hit { 4.8 } else { 2.4 }, on: true, cur: play == Some(i), label: String::new() });
            }
            // Slim orbit trace in the lower half.
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
                v.nodes.push(Node { x, y, r: if hit { 6.2 } else { 4.6 }, on: true, cur: hit, label: String::new() });
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
            // The sieve's own lattice: one row per residue class. The two union terms are lit
            // cyan; the subtracted class is amber, because those cells are removed. Under them,
            // the full sieve the lane reads, rotated into display slots like the hits. Evaluated
            // per cell rather than through the 64-bit mask, so 1024-step patterns stay exact.
            let cls = crate::sequence::sieve_classes(params);
            let shift = p.rotation % n;
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
                        label: String::new(),
                    });
                }
            }
            let y = 150.0;
            mv(&mut v.edges, 12.0, y); ln(&mut v.edges, 198.0, y);
            let cw = 184.0 / n as f32;
            for i in 0..n {
                let member = crate::sequence::sieve_member(params, (i + n - shift) % n);
                let hit = aggregate_hit(p, i);
                v.nodes.push(Node {
                    x: 13.0 + (i as f32 + 0.5) * cw,
                    y,
                    r: if hit { 4.4 } else if member { 2.6 } else { 1.4 },
                    on: member,
                    cur: play == Some(i),
                    label: String::new(),
                });
            }
            if let Some(i) = play {
                let x = 13.0 + (i as f32 + 0.5) * cw;
                mv(&mut v.hand, x, 20.0); ln(&mut v.hand, x, 190.0);
            }
        }
        Mode::Kuramoto => {
            // The bank itself: eight phase pointers on one circle, each on the phase the
            // generator published for this step. Low coupling spreads them; high coupling
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
                v.nodes.push(Node { x, y, r: if hit { 5.2 } else { 4.0 }, on: true, cur: hit, label: String::new() });
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
                    v.nodes.push(Node { x: px, y: py, r: 2.6, on: true, cur: play == Some(i), label: String::new() });
                }
            }
            if let Some((px, py)) = prev {
                v.nodes.push(Node { x: px, y: py, r: 5.0, on: true, cur: true, label: String::new() });
            }
        }
        Mode::EuclideanSwing | Mode::Bernoulli | Mode::AfricanBell | Mode::PhaseCanon | Mode::ReactionDiffusion | Mode::Cantor | Mode::Fibonacci | Mode::Genetic => {
            circle(&mut v.edges, C, C, 90.0);
            // At long pattern lengths, sample the ring instead of drawing up to 8k dots.
            let stride = n.div_ceil(96).max(1);
            for tr in 0..TRACKS {
                for i in (0..n).step_by(stride) {
                    if p.hit[tr][i] {
                        let a = (i as f32 / n as f32) * std::f32::consts::TAU;
                        let r = 30.0 + tr as f32 * 7.0;
                        let (x, y) = (C + r * a.cos(), C + r * a.sin());
                        v.nodes.push(Node { x, y, r: 3.0, on: true, cur: play == Some(i), label: String::new() });
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
        // The old panel drew one primary and one secondary node for every step of every lane:
        // 16k rectangles at 1024 steps. The sampled rosette must stay small and still light.
        let g = GenParams {
            kind: Mode::Phyllotaxis,
            steps: MAX_STEPS,
            pulses: MAX_STEPS,
            density: 1.0,
            probability: 1.0,
            ..GenParams::default()
        };
        let view = build(&g, &generate(&g), 0);
        assert!(view.nodes.len() <= 256, "phyllotaxis drew {} nodes", view.nodes.len());
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
        assert_eq!(view.nodes.len(), steps - 1, "the wrap pair is not an iteration");
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
        assert!(marked.iter().any(|y| (*y - row_y(selected_row(&g, 0))).abs() < 0.6), "lane 0 row missing: {marked:?}");
        // The lane offset puts later lanes on different rows, so one marker cannot serve all and
        // the panel has to mark each lane's own generation.
        assert_ne!(selected_row(&g, 0), selected_row(&g, 1));
        assert!(marked.iter().any(|y| (*y - row_y(selected_row(&g, 1))).abs() < 0.6), "lane 1 row missing: {marked:?}");
    }
}
