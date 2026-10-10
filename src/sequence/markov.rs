use super::{set_hit, GenParams, Pattern, Rng, TRACKS};

/// A first-order Markov walk over REST, kick, snare, hats, and auxiliary percussion states.
/// Transition weights favor instrument-role-compatible movement (kick -> snare/hat, hat -> hat).
pub(super) fn generate(p: &GenParams) -> Pattern {
    let mut out = Pattern { steps: p.steps, ..Pattern::default() };
    let mut rng = Rng::new(p.seed, p.variation, 0x4D4B_0A01);
    let mut state = 0usize;
    for i in 0..p.steps {
        let mut weights = [1u32; TRACKS + 1];
        weights[0] = 5;
        match state {
            0 => { weights[1] = 6; weights[2] = 3; weights[3] = 9; weights[7] = 2; weights[8] = 3; }
            1 => { weights[2] = 8; weights[3] = 8; weights[1] = 4; weights[0] = 3; }
            2 => { weights[3] = 9; weights[5] = 4; weights[1] = 3; weights[0] = 4; }
            3 => { weights[3] = 10; weights[4] = 4; weights[1] = 2; weights[0] = 4; }
            _ => { weights[3] = 5; weights[2] = 3; weights[0] = 4; }
        }
        let total: u32 = weights.iter().sum();
        let pulse_ratio = p.pulses as f32 / p.steps as f32;
        let event_chance = (p.density.clamp(0.0, 1.0) * (0.25 + pulse_ratio * 1.5)).clamp(0.0, 1.0);
        // A one-step pattern must make a sound, so the walk cannot be allowed to rest.
        if p.steps > 1 && rng.unit() > event_chance { state = 0; }
        else {
            let mut pick = (rng.next_u64() % total as u64) as u32;
            let mut next = 0usize;
            for (j, w) in weights.iter().enumerate() {
                if pick < *w { next = j; break; }
                pick -= *w;
            }
            state = next;
        }
        out.state[i] = state as u8;
        if state > 0 {
            let tr = state - 1;
            if tr < TRACKS {
                let mut local = Rng::new(p.seed, p.variation, 0x4D4B_1000 + (i * TRACKS + tr) as u64);
                set_hit(&mut out, tr, i, local.unit() < (0.25 + p.density * 0.5 + pulse_ratio * 0.25).clamp(0.0, 1.0), 0.68 + local.unit() * 0.32);
            }
        }
    }
    out
}
