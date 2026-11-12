use super::{set_hit, GenParams, Pattern, Rng, TRACKS};

// A 12-step bell pattern (standard African bell). Rotated per lane/variation.
const BELL: [bool; 12] = [true,false,false,true,false,false,true,false,false,true,false,false];

pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let n = p.steps.max(1);
    // The 12-step bell is the mode's own period; longer pattern lengths preview extra cells
    // but playback loops on the bell.
    let period = if n >= BELL.len() { BELL.len() as u16 } else { n as u16 };
    for tr in 0..TRACKS {
        out.lane_periods[tr] = period;
        let mut rng = Rng::new(p.seed, p.variation, 0xAF31_0000 + tr as u64);
        let rot = rng.below(n);
        for i in 0..n {
            let src = BELL[i % BELL.len()];
            let idx = (i + rot) % n;
            let mut hit = src;
            if hit && tr > 0 && (0..tr).all(|k| out.hit[k][idx]) { hit = false; }
            let accent = if hit { 0.75 + 0.25 * rng.unit() } else { 0.0 };
            set_hit(&mut out, tr, idx, hit, accent);
        }
    }
    out
}