//! Live one-line math for the generator panel.
//!
//! The description next to each mode says what it does; this line says *how*, with the current
//! parameter snapshot substituted in. Derived quantities come from the generators' own helpers
//! (`lane_pulses`, `pulse_hits`, `raised_cosine_cutoff`, `motif_len`, …), so a
//! displayed value cannot drift from the pattern that is actually playing.

use crate::modes::Mode;
use nih_plug::prelude::Enum as _;
use crate::sequence::{
    euclidean_lane_pulses, pulse_hits, raised_cosine_cutoff,
    raised_cosine_phase, slowcat_motif_len, GenParams, CA_ROWS, DENSITY_PROFILE,
    MAX_STEPS, TRACKS,
};

/// Math line for the current snapshot. Deterministic, allocation-only, and called when the
/// generation snapshot changes rather than every frame.
pub(crate) fn math(p: &GenParams) -> String {
    let n = p.steps.clamp(1, MAX_STEPS);
    let d = p.density.clamp(0.0, 1.0);
    let w0 = DENSITY_PROFILE[0];
    let pulses = if p.kind == Mode::Cellular { p.pulses.min(CA_ROWS - 1) } else { p.pulses.min(n) };
    match p.kind {
        Mode::Euclidean => {
            let k = euclidean_lane_pulses(pulses, d, 0, n);
            format!("E(k,n) with n={n}: k=round({pulses}×{d:.2}×(0.35+{w0:.2}))={k} (lane 0); onset j at floor(j·n/k) from each lane's phase origin")
        }
        Mode::Eulerian => {
            let degree = 1 + pulses % 4;
            let circuit = (TRACKS + 1) * degree;
            format!("n={n}: out-degree d=1+({pulses} mod 4)={degree}; Hierholzer circuit over 9 vertices has 9·d={circuit} steps; step i reads circuit[i mod {circuit}]")
        }
        Mode::Phyllotaxis => {
            let threshold = ((p.pulses.max(1) as f32 / n as f32) * (0.35 + w0) * (0.5 + d))
                .clamp(0.02, 0.98);
            format!("n={n}: 1/φ rotation (222.49° = mirrored golden angle 137.51°), threshold T={threshold:.3}; beat(i)=frac(i·0.618034+{:.3}+{:.3}+lane·0.137), hit when beat<T; second rule 1/φ³ (84.99°) adds the inner band below 0.18·T", p.seed, p.phase)
        }
        Mode::Cellular => {
            let row = if n == 1 {
                0
            } else {
                (pulses + (p.phase * CA_ROWS as f32).round() as usize) % CA_ROWS
            };
            let gate = 0.45 + d * w0 * 0.55;
            format!("n={n}: row=(pulses+round(phase·16)+2·lane) mod 16 → lane 0 reads row {row}; cell=rule {} bit; gate={gate:.2}", p.ca_rule)
        }
        Mode::Oscillator => {
            let wanted = ((p.pulses as f32 * d * (0.5 + w0)).round() as usize).min(n);
            let wave = crate::OscWave::ids()
                .map_or("sine", |ids| ids[p.osc_wave % crate::sequence::oscillator_wave_count]);
            format!("n={n}: wave(i)={wave}({:.2}·i/n+{:.2}); keep the {wanted} highest samples (lane 0, rank<{wanted})", p.osc_rate, p.phase)
        }
        Mode::Crystallographic => {
            let order = p.symmetry.clamp(1, 8);
            let unit = if n / order >= 2 { n / order } else { n };
            let threshold = ((p.pulses.max(1) as f32 / n as f32) * (0.55 + 0.45 * d)
                * (0.4 + 0.6 * w0))
                .clamp(0.0, 1.0);
            format!("n={n}: order={order}, unit={unit} (whole pattern if <2 slots); motif cell filled at T={threshold:.3}; op=({order}−1) mod 8 applied per cell")
        }
        Mode::Logistic => {
            let r = crate::sequence::logistic_r(d);
            let x0 = (0.13 + p.seed as f64 * 0.73 + p.phase as f64 * 0.11).clamp(0.001, 0.999);
            let threshold = ((p.pulses as f32 / n as f32) * (0.3 + 0.7 * d) * (0.5 + w0))
                .clamp(0.03, 0.97);
            format!("n={n}: x[n+1]=r·x[n]·(1−x[n]), r=3.5+0.5·{d:.2}={r:.3}, x0={x0:.3}; lane 0 cutoff=1−{threshold:.3}={:.3}, strict local peaks", 1.0 - threshold)
        }
        Mode::Markov => {
            let chance = (d * (0.25 + p.pulses as f32 / n as f32 * 1.5)).clamp(0.0, 1.0);
            format!("n={n}: p(rest)=5, role weights per state; event chance={d:.2}·(0.25+1.5·{}/{n})={chance:.2}", p.pulses)
        }
        Mode::RaisedCosine => {
            let hits = pulse_hits(pulses, d, 0, n);
            let phase = raised_cosine_phase(p.seed, p.phase);
            let cutoff = raised_cosine_cutoff(hits, n);
            format!("n={n}: env(i)=0.5−0.5·cos(2π({hits}·i/n+φ)), hits={hits}, φ={phase:.3}, cutoff={cutoff:.3}; hit when env≥cutoff")
        }
        Mode::Slowcat => {
            let count = p.pulses.max(1);
            let len = slowcat_motif_len(0, count, n);
            let shift = ((p.seed * 997.0) as usize + p.variation as usize) % len;
            format!("n={n}: motif={len}=2+({count} mod 7); shift={shift}; slot=(i+shift) mod {len}, phrase=(i/{len}+lane) mod (lane mod 3+2)")
        }
        Mode::ToussaintDistance => {
            let k = pulse_hits(pulses, d, 0, n);
            let origin = (p.seed.clamp(0.0, 1.0) * n as f32).floor() as usize % n;
            format!("n={n}: k={k}, lane 0 origin={origin}; onset when floor((pos+1)·k/n)≠floor(pos·k/n); accent=0.62+0.38·(1−d/n)")
        }
        Mode::WellFormed => {
            let k = pulse_hits(pulses, d, 0, n);
            format!("n={n}: k={k}; d=(pos·k) mod n; hit when (n−d) mod n<k; lane phase=lane·n/8, so words interlock")
        }
        Mode::LSystem => {
            let family = ((p.seed.clamp(0.0, 0.999_999) * 4.0) as usize + p.variation as usize) % 4;
            let threshold = (pulses as f32 / n as f32 * (0.35 + 0.65 * d) * (0.45 + w0))
                .clamp(0.0, 1.0);
            format!("n={n}: family=(floor({:.3}·4)+{}) mod 4={family}; ≤12 parallel rewrites publish an A/B word (rotation-aware); lane 0 chance={threshold:.2}·grammar·metrical", p.seed, p.variation)
        }
        Mode::PhaseShift => {
            let k = ((p.pulses as f32 * d).round() as usize).min(n);
            let span = (n / TRACKS).max(1);
            let user = (p.phase.clamp(0.0, 1.0) * n as f32).round() as usize;
            let seed = (p.seed.clamp(0.0, 1.0) * n as f32).round() as usize;
            let shift0 = (user + seed) % n;
            format!("n={n}: k=round({}×{d:.2})={k}; lane shift=lane·{span}+round(phase·n)+round(seed·n); lane 0 shift={shift0}", p.pulses)
        }
        Mode::Polyrhythm => {
            let k_for = |num: usize, den: usize| {
                let scaled = (pulses.saturating_mul(num) + den / 2) / den;
                ((scaled as f32 * d).round() as usize).min(n)
            };
            format!("n={n}: lane 0 is 1:1 → k=round({}×{d:.2})={}; lane 1 is 3:2 → k={}; onset when floor((pos+1)k/n)≠floor(pos·k/n)", pulses, k_for(1, 1), k_for(3, 2))
        }
        Mode::Syncopation => {
            let target = ((p.pulses as f32 * d * (0.45 + w0)).round() as usize).min(n);
            format!("n={n}: target=round({}×{d:.2}×(0.45+{w0:.2}))={target}; score(i)=w(i mod 4)·w((i+3·lane) mod 4)·jitter; keep top {target}", p.pulses)
        }
        Mode::PebbleBox => {
            let substeps = (p.osc_rate.round() as i32).clamp(1, 16);
            let gravity = crate::sequence::pebble_gravity(p.ca_rule);
            let shaker = p.pulses.clamp(1, n);
            let stride = (n / shaker).max(1);
            format!("n={n}: {substeps} substep(s)/step; gravity {gravity:.2} tilted {:.0}°; shaker impulse every {stride} step(s) kicks a rotating third of 8 pebbles; a lane fires when its wall impact exceeds {:.2}", p.phase * 360.0, (1.0 - d) * 4.2)
        }
        Mode::Sieve => {
            let cls = crate::sequence::sieve_classes(p);
            let terms = if p.pulses >= 2 { 2 } else { 1 };
            let members = (0..n).filter(|&i| crate::sequence::sieve_member(p, i)).count();
            let kept = (members as f32 * d * (0.35 + w0)).round() as usize;
            format!("n={n}: M=(({},{})∪({},{}))\\{};{} with {terms} union term(s); {members} member step(s), lane 0 keeps round({members}×{d:.2}×(0.35+{w0:.2}))={kept}",
                cls[0].0, cls[0].1, cls[1].0, cls[1].1, cls[2].0, cls[2].1)
        }
        Mode::Kuramoto => {
            let k = crate::sequence::kuramoto_coupling(p);
            let rate = crate::sequence::kuramoto_rate(p, n);
            let target = ((pulses as f32 * d * (0.5 + w0)).round() as usize).min(n);
            format!("n={n}: dθᵢ=2π·{rate:.3}·detune+({k:.2}/8)·Σsin(θⱼ−θᵢ), spread={:.2}; lane fires on a phase wrap; keep top {target} (lane 0)", 0.05 + 0.45 * p.phase.clamp(0.0, 1.0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequence::generate;

    #[test]
    fn every_mode_has_deterministic_math_with_the_current_step_count() {
        for kind in Mode::all() {
            let p = GenParams { kind, steps: 13, ..GenParams::default() };
            let line = math(&p);
            assert_eq!(line, math(&p), "{kind:?} is not deterministic");
            assert!(line.contains("13"), "{kind:?} math omits the step count: {line}");
        }
    }

    #[test]
    fn euclidean_k_matches_the_generated_lane_zero_hits() {
        for (steps, pulses, density) in [(16usize, 5usize, 0.65f32), (7, 3, 1.0), (13, 2, 0.5)] {
            let p = GenParams { steps, pulses, density, ..GenParams::default() };
            let k = euclidean_lane_pulses(pulses, density, 0, steps);
            let hits = generate(&p).hit[0][..steps].iter().filter(|&&h| h).count();
            assert_eq!(hits, k);
            assert!(math(&p).contains(&format!("={k} (lane 0)")), "math={}", math(&p));
        }
    }
}
