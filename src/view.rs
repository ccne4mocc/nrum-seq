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

#[inline]
fn oscillator_value(wave: usize, phase: f32) -> f32 {
    let phase = phase.rem_euclid(1.0);
    match wave % 4 {
        0 => 0.5 + 0.5 * (TAU * phase).sin(),
        1 => 1.0 - (2.0 * phase - 1.0).abs(),
        2 => phase,
        _ => if phase < 0.5 { 1.0 } else { 0.0 },
    }.clamp(0.0, 1.0)
}

pub fn build(params: &GenParams, p: &Pattern, cur: i32) -> View {
    let n = p.steps.clamp(1, MAX_STEPS);
    let play = (cur >= 0 && (cur as usize) < n).then_some(cur as usize);
    let mut v = View::default();
    match params.kind {
        GenKind::Euclidean => {
            circle(&mut v.edges, C, C, 82.0);
            for i in 0..n {
                let (x, y) = polar(82.0, i as f32 / n as f32);
                v.nodes.push(Node { x, y, r: if aggregate_hit(p, i) { 6.5 } else { 3.2 }, on: aggregate_hit(p, i), cur: play == Some(i), label: String::new() });
            }
            if let Some(i) = play { let (x, y) = polar(82.0, i as f32 / n as f32); mv(&mut v.hand, C, C); ln(&mut v.hand, x, y); }
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
            for row in 0..CA_ROWS {
                for i in 0..n {
                    let on = p.ca[row][i];
                    v.nodes.push(Node {
                        x: 13.0 + (i as f32 + 0.5) * cw,
                        y: 18.0 + (row as f32 + 0.5) * ch,
                        r: (cw.min(ch) * 0.40).max(1.1),
                        on,
                        cur: play == Some(i) && row == params.pulses % CA_ROWS,
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
                let value = oscillator_value(params.osc_wave, phase);
                let x = 12.0 + j as f32 / (points - 1) as f32 * 186.0;
                let y = 176.0 - value * 130.0;
                if j == 0 { mv(&mut v.lines, x, y); } else { ln(&mut v.lines, x, y); }
            }
            let pulse_ratio = params.pulses as f32 / n as f32;
            let threshold = (pulse_ratio * (0.25 + 0.75 * params.density.clamp(0.0, 1.0))
                * (0.5 + crate::sequence::DENSITY_PROFILE[0])).clamp(0.01, 0.99);
            let threshold_y = 176.0 - (1.0 - threshold) * 130.0;
            mv(&mut v.edges, 12.0, threshold_y); ln(&mut v.edges, 198.0, threshold_y);
            for i in 0..n {
                let x = 12.0 + (i as f32 + 0.5) / n as f32 * 186.0;
                let phase = (i as f32 / n as f32 * params.osc_rate + params.phase).fract();
                let value = oscillator_value(params.osc_wave, phase);
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
        GenKind::RaisedCosine | GenKind::Slowcat | GenKind::ToussaintDistance | GenKind::WellFormed
        | GenKind::LSystem | GenKind::PhaseShift | GenKind::Polyrhythm | GenKind::Syncopation => {
            // Shared step-lattice visualization; each new mode retains a distinct kind/index.
            let cw = 184.0 / n as f32;
            let ch = 168.0 / TRACKS as f32;
            for tr in 0..TRACKS {
                for i in 0..n {
                    let on = p.hit[tr][i];
                    let x = 13.0 + (i as f32 + 0.5) * cw;
                    let y = 21.0 + (tr as f32 + 0.5) * ch;
                    v.nodes.push(Node { x, y, r: if on { 4.3 } else { 2.0 }, on, cur: play == Some(i), label: String::new() });
                }
            }
            for i in 0..=n { let x = 13.0 + i as f32 * cw; mv(&mut v.edges, x, 18.0); ln(&mut v.edges, x, 192.0); }
            for tr in 0..=TRACKS { let y = 21.0 + tr as f32 * ch; mv(&mut v.edges, 11.0, y); ln(&mut v.edges, 199.0, y); }
            if let Some(i) = play { let x = 13.0 + (i as f32 + 0.5) * cw; mv(&mut v.hand, x, 18.0); ln(&mut v.hand, x, 192.0); }
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
        assert!((oscillator_value(0, 0.25) - 1.0).abs() < 1e-5);
        assert!((oscillator_value(1, 0.25) - 0.5).abs() < 1e-5);
        assert!((oscillator_value(2, 0.25) - 0.25).abs() < 1e-5);
        assert_eq!(oscillator_value(3, 0.25), 1.0);
        assert_eq!(oscillator_value(3, 0.75), 0.0);
        for wave in 0..4 { for i in 0..=100 { assert!((0.0..=1.0).contains(&oscillator_value(wave, i as f32 / 100.0))); } }
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
}
