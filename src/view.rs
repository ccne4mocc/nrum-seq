//! Pure GUI geometry. Keep this free of Slint and plug-in types so every view can be tested.
use crate::sequence::{GenKind, GenParams, Pattern, MAX_STEPS, TRACKS, CA_ROWS};
use std::f32::consts::TAU;
use std::fmt::Write;

const SIZE: f32 = 210.0;
const C: f32 = SIZE * 0.5;
const GOLDEN_ANGLE: f32 = 2.399_963_1;

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

/// Panel half-width available for drawing, keeping 10px of margin inside the 210 viewbox.
const PLOT: f32 = 92.0;
/// `traj` is stored in simulation units and clamped to `R_MAX`; this maps it into the panel.
const ORBIT_SCALE: f32 = PLOT / 3.2;

#[inline]
fn osc_value(wave: usize, phase: f32) -> f32 {
    crate::sequence::oscillator_value(wave, phase)
}

pub fn build(params: &GenParams, p: &Pattern, cur: i32) -> View {
    let n = p.steps.clamp(1, MAX_STEPS);
    let play = (cur >= 0 && (cur as usize) < n).then_some(cur as usize);
    let mut v = View::default();
    match params.kind {
        GenKind::Euclidean => {
            // Per-lane rings rather than one aggregate ring: the lanes have different densities,
            // and collapsing them hid that (and showed hits for lanes the user disabled).
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
        GenKind::Eulerian | GenKind::Markov => {
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
        GenKind::Phyllotaxis => {
            let max_r = 88.0;
            for i in 0..n {
                let a = i as f32 * GOLDEN_ANGLE + params.phase * TAU;
                let r = max_r * ((i as f32 + 1.0) / n as f32).sqrt();
                let x = C + r * a.cos(); let y = C + r * a.sin();
                let on = aggregate_hit(p, i);
                v.nodes.push(Node { x, y, r: if on { 5.5 } else { 2.6 }, on, cur: play == Some(i), label: if on { (i + 1).to_string() } else { String::new() } });
                if i == 0 { mv(&mut v.lines, x, y); } else { ln(&mut v.lines, x, y); }
            }
        }
        GenKind::Cellular => {
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
        GenKind::Oscillator => {
            mv(&mut v.edges, 12.0, 105.0); ln(&mut v.edges, 198.0, 105.0);
            let points = (n * 3).max(24);
            for j in 0..points {
                let phase = (j as f32 / points as f32 * params.osc_rate + params.phase).fract();
                let value = osc_value(params.osc_wave, phase);
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
                let value = osc_value(params.osc_wave, phase);
                v.nodes.push(Node { x, y: 176.0 - value * 130.0, r: 3.0, on: aggregate_hit(p, i), cur: play == Some(i), label: String::new() });
            }
        }
        GenKind::Crystallographic => {
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
        GenKind::RaisedCosine => {
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
        GenKind::Slowcat => {
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
        GenKind::ToussaintDistance => {
            // Distance circle: each lane's k onsets sit at evenly spaced angles, radius by lane.
            // Tick marks on the rim give the step grid, so the circular origin each lane was
            // rotated by is readable against them.
            for tr in 0..TRACKS {
                let k = crate::sequence::pulse_hits(params.pulses, params.density, tr, n);
                if k == 0 { continue; }
                let radius = 30.0 + tr as f32 * 8.5;
                let mut j = 0usize;
                for i in 0..n {
                    if !p.hit[tr][i] { continue; }
                    let (x, y) = polar(radius, j as f32 / k as f32);
                    let (tx, ty) = polar(100.0, i as f32 / n as f32);
                    mv(&mut v.lines, tx, ty); ln(&mut v.lines, x, y);
                    v.nodes.push(Node { x, y, r: 4.2, on: true, cur: play == Some(i), label: String::new() });
                    j += 1;
                }
            }
            circle(&mut v.edges, C, C, 100.0);
            if let Some(i) = play { let (x, y) = polar(100.0, i as f32 / n as f32); mv(&mut v.hand, C, C); ln(&mut v.hand, x, y); }
        }
        GenKind::WellFormed => {
            // Christoffel slack: the Bresenham remainder rises linearly and resets at each onset,
            // so the two inter-onset interval lengths are visible as the slope changes.
            mv(&mut v.edges, 12.0, 186.0); ln(&mut v.edges, 198.0, 186.0);
            for tr in 0..TRACKS {
                let k = crate::sequence::pulse_hits(params.pulses, params.density, tr, n);
                if k == 0 { continue; }
                let phase = (tr * n / TRACKS) % n;
                let height = 24.0 + tr as f32 * 4.5;
                for i in 0..n {
                    let d = crate::sequence::modular_distance((i + n - phase) % n, k, n);
                    let x = 13.0 + (i as f32 + 0.5) / n as f32 * 184.0;
                    let y = 186.0 - d as f32 / n as f32 * height;
                    if i == 0 { mv(&mut v.lines, x, y); } else { ln(&mut v.lines, x, y); }
                    if p.hit[tr][i] {
                        mv(&mut v.edges, x, 186.0 - height); ln(&mut v.edges, x, y);
                    }
                }
            }
            for i in 0..n {
                let x = 13.0 + (i as f32 + 0.5) / n as f32 * 184.0;
                v.nodes.push(Node { x, y: 186.0, r: if aggregate_hit(p, i) { 4.0 } else { 1.9 }, on: aggregate_hit(p, i), cur: play == Some(i), label: String::new() });
            }
            if let Some(i) = play { let x = 13.0 + (i as f32 + 0.5) / n as f32 * 184.0; mv(&mut v.hand, x, 40.0); ln(&mut v.hand, x, 190.0); }
        }
        GenKind::LSystem => {
            // Nesting depth per step, recovered by walking the lane's onsets: a hit that starts
            // a new run raises the level, consecutive hits hold it. Bars make the grammar's
            // branching readable where a plain grid only showed on/off.
            let cw = 184.0 / n as f32;
            let ch = 168.0 / TRACKS as f32;
            for tr in 0..TRACKS {
                let y = 21.0 + (tr as f32 + 0.5) * ch;
                let mut depth = 0usize;
                for i in 0..n {
                    let on = p.hit[tr][i];
                    if on && (i == 0 || !p.hit[tr][i - 1]) { depth += 1; }
                    let height = if depth == 0 { 0.0 } else { (depth as f32 * ch * 0.34).min(ch * 0.9) };
                    if height > 0.0 {
                        let x = 13.0 + (i as f32 + 0.5) * cw;
                        mv(&mut v.lines, x, y + ch * 0.42); ln(&mut v.lines, x, y + ch * 0.42 - height);
                    }
                    if on {
                        v.nodes.push(Node { x: 13.0 + (i as f32 + 0.5) * cw, y, r: 4.0, on: true, cur: play == Some(i), label: String::new() });
                    }
                }
            }
            for tr in 0..=TRACKS { let y = 21.0 + tr as f32 * ch; mv(&mut v.edges, 11.0, y); ln(&mut v.edges, 199.0, y); }
            if let Some(i) = play { let x = 13.0 + (i as f32 + 0.5) * cw; mv(&mut v.hand, x, 18.0); ln(&mut v.hand, x, 192.0); }
        }
        GenKind::PhaseShift => {
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
        GenKind::Polyrhythm => {
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
        GenKind::Syncopation => {
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
        GenKind::ThreeBody => {
            // The orbit itself: bodies 1 and 2 traced relative to body 0, with a step marker on
            // the third body. This is the only panel whose picture *is* the generator's state.
            circle(&mut v.edges, C, C, 96.0);
            for body in 0..2 {
                for i in 0..n {
                    let x = C + p.traj[i][body * 2] * ORBIT_SCALE;
                    let y = C - p.traj[i][body * 2 + 1] * ORBIT_SCALE;
                    if i == 0 { mv(&mut v.lines, x, y); } else { ln(&mut v.lines, x, y); }
                }
            }
            for i in 0..n {
                let x = C + p.traj[i][2] * ORBIT_SCALE;
                let y = C - p.traj[i][3] * ORBIT_SCALE;
                v.nodes.push(Node { x, y, r: if aggregate_hit(p, i) { 4.4 } else { 2.4 }, on: aggregate_hit(p, i), cur: play == Some(i), label: String::new() });
            }
            v.nodes.push(Node { x: C, y: C, r: 3.0, on: true, cur: false, label: "0".into() });
            if let Some(i) = play {
                let x = C + p.traj[i][2] * ORBIT_SCALE;
                let y = C - p.traj[i][3] * ORBIT_SCALE;
                mv(&mut v.hand, C, C); ln(&mut v.hand, x, y);
            }
        }
        GenKind::Logistic => {
            // Orbit diagram x[n] -> x[n+1], plus a slim orbit trace in the lower half.
            for i in 0..n {
                let x = 18.0 + p.orbit[i] * 174.0;
                let y = 22.0 + (1.0 - p.orbit[(i + 1) % n]) * 125.0;
                v.nodes.push(Node { x, y, r: if aggregate_hit(p, i) { 4.8 } else { 2.6 }, on: aggregate_hit(p, i), cur: play == Some(i), label: String::new() });
                let ox = 12.0 + (i as f32 + 0.5) / n as f32 * 186.0;
                let oy = 185.0 - p.orbit[i] * 32.0;
                if i == 0 { mv(&mut v.lines, ox, oy); } else { ln(&mut v.lines, ox, oy); }
            }
            mv(&mut v.edges, 18.0, 84.0); ln(&mut v.edges, 192.0, 84.0);
            mv(&mut v.edges, 105.0, 20.0); ln(&mut v.edges, 105.0, 151.0);
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequence::{generate, GenKind};
    #[test]
    fn oscillator_view_matches_each_selected_waveform() {
        assert!((osc_value(0, 0.25) - 1.0).abs() < 1e-5);
        assert!((osc_value(1, 0.25) - 0.5).abs() < 1e-5);
        assert!((osc_value(2, 0.25) - 0.25).abs() < 1e-5);
        assert_eq!(osc_value(3, 0.25), 1.0);
        assert_eq!(osc_value(3, 0.75), 0.0);
        for wave in 0..4 { for i in 0..=100 { assert!((0.0..=1.0).contains(&osc_value(wave, i as f32 / 100.0))); } }
    }

    #[test]
    fn every_mode_draws_a_populated_panel() {
        // Every mode must produce panel geometry; a mode that falls through the match would
        // otherwise render as an empty box with no compile error.
        let p = GenParams::default();
        for k in GenKind::ALL {
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
    fn the_playhead_is_only_set_for_a_valid_step() {
        let g = GenParams::default();
        let pat = generate(&g);
        assert!(build(&g, &pat, -1).nodes.iter().all(|n| !n.cur));
        assert!(build(&g, &pat, 0).nodes.iter().any(|n| n.cur));
        assert!(build(&g, &pat, 9999).nodes.iter().all(|n| !n.cur));
    }

    #[test]
    fn every_panel_stays_inside_the_viewbox() {
        // A mode whose geometry escapes the 210x210 panel is clipped by Slint and reads as a
        // blank or half-drawn box. Every mode, at the sizes most likely to overflow.
        for k in GenKind::ALL {
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
        let g = GenParams { kind: GenKind::Polyrhythm, steps: 16, pulses: 8, density: 1.0, ..GenParams::default() };
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
        let g = GenParams { kind: GenKind::PhaseShift, steps: 16, pulses: 4, density: 1.0, ..GenParams::default() };
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
    fn chaos_panel_traces_the_recorded_orbit() {
        // The panel must draw the generator's own trajectory, not a default-constructed one.
        let g = GenParams { kind: GenKind::ThreeBody, steps: 32, pulses: 8, density: 1.0, ..GenParams::default() };
        let pat = generate(&g);
        let view = build(&g, &pat, 3);
        let expected = C + pat.traj[3][2] * ORBIT_SCALE;
        assert!(view.nodes.iter().any(|n| n.cur && (n.x - expected).abs() < 0.6), "playhead node not on the orbit");
        // A default Pattern would collapse every trail point onto the centre.
        assert!(view.nodes.iter().filter(|n| (n.x - C).abs() > 1.0).count() > 5, "orbit collapsed to the centre");
    }

    #[test]
    fn cellular_highlights_the_generation_each_lane_reads() {
        // Regression: the panel marked `pulses % CA_ROWS`, ignoring the phase offset and the
        // per-lane step, so a marker could sit on a row the generator never plays.
        let g = GenParams { kind: GenKind::Cellular, steps: 16, pulses: 5, phase: 0.5, ..GenParams::default() };
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
