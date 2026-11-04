# `_nDrums` / nrum seq — Sequencer Mode Parameter & Calculation Overview

Source read: `src/modes.rs`, `src/lib.rs`, `src/sequence/mod.rs`, all `src/sequence/*.rs`, `src/modulation.rs`.
This is a working reference for changing mode calculations. It separates **what the host parameter means**, **how it becomes `GenParams`**, and **how the generated pattern is transformed before MIDI scheduling**.

- `n = steps` = number of integer grid cells the generator fills.
- `d = density`, `tr` = lane index 0..7, `P = DENSITY_PROFILE[tr] = [0.50, 0.30, 1.00, 0.12, 0.18, 0.15, 0.12, 0.28]`.
- `φ = 1.618033988749894`, `φ⁻¹ = φ − 1`, golden angle `= 2.399963229728653`.

---

## 1. Shared parameter transfer: host → GenParams → Pattern → MIDI

| Host parameter | `GenParams` field | Transfer / calculation | Consumed by |
|---|---|---|---|
| `mode` | `kind` | Selected enum variant; also drives `Mode::min_steps()` and `Mode::steps_for_mode()`. | Dispatch in `run_mode()` |
| `steps` (1..1024) | `steps`, `length` | `raw = clamp(steps, 1, ceiling)`; `cells = round(raw)` if `round_steps` else `ceil(raw)`, clamped to 1..ceiling. `GenParams.steps = cells`; `GenParams.length = cells` when snapped, otherwise `raw`. | Generators read `steps`; scheduler reads `length` |
| `max_steps` (64..1024) | ceiling | `step_ceiling() = clamp(max_steps, 64, 1024)`. The UI-side `steps_for_mode()` may raise `steps` and `max_steps` to the mode floor. | `cell_count_for()`; LFO `MaxSteps` target |
| `round_steps` | — | Chooses `round` vs `ceil` for `cells`; only affects fractional `steps`. | `gen_params()` |
| `pulses` (0..1024) | `pulses` | `q.pulses = min(pulses, steps)`, except **Cellular** uses `min(pulses, CA_ROWS−1) = 15`. Meaning is mode-specific. `pulses == 0` hard-mutes every mode except Cellular and Life. | Generators |
| `rotate` (0..1023) | `rotate` | `q.rotate %= steps`. Actual display shift is `(rotate + variation) % steps`, not rotate alone. | Post-transform |
| `density` (0.01..1) | `density` | Passed through. Often not a hit-density: it can be a map coefficient, coupling, gravity or bias. `density <= 0` clears every generated hit after the generator runs. | Generators; global mute |
| `probability` (0..1) | `probability` | Applied **after** generation: each existing hit is retained with `rng.unit() < probability`. Some nDrums generators also read it internally, so those modes apply it twice (see §5). | `generate_impl()` |
| `velocity` (.01..1) | `velocity` | Not read by generators. At note emission: `lane_velocity × schedule_velocity × pattern.accent`. | Scheduler |
| `seed` (0..1) | `seed` | Normalized deterministic seed. Some modes use it directly; some only use it through RNG; some ignore it in the generator while the shared accent humanizer still reads it. | Generators/accent RNG |
| `phase` (0..1) | `phase` | Mode-specific second phase, start position, row offset, tilt, etc. | Generators |
| `osc_rate` (.125..8) | `osc_rate` | Mode-specific overload: oscillator cycles, Pebble Box substeps, Kuramoto coupling. | Generators |
| `osc_wave` (8 enum values) | `osc_wave` | `% 8`; selects oscillator/LFO waveform. | Oscillator, LFO signal |
| `ca_rule` (0..255) | `ca_rule` | Mode-specific overload: Wolfram rule, Pebble gravity, Sieve moduli, Vuza modulus, Life rule. | Generators |
| `symmetry` (1..8) | `symmetry` | Mode-specific overload: CA seed shape, L-system family, crystallographic order, Sieve residue, Life seed. | Generators |
| `markov_bias` (0..255) | `markov_bias` | Four 2-bit fields: rest, repeat, role, novelty. | Markov only |
| `variation` (persisted `u32`) | `variation` | Incremented when the `generate` bool changes either direction and by regenerate-after-N. It does two things: (1) always contributes `variation % steps` to rotation, (2) seeds most generators/RNGs. | Rotation + generators |
| `invert` | `invert` | After rotation: flips every hit/rest on every lane. | Post-transform |
| `mirror` | `mirror` | After invert: reverses every per-step array. | Post-transform |
| `edit_overlay` | `edit_rev` | Revision is part of the cache key. A plain `EditView` is passed to `generate_with()`; hit/accent/probability are display-space and win over the algorithm. Length/ratchet/offset are scheduler-only. | Generation + scheduler |
| `rate` | — | `steps_per_beat`: 1/4=1, 1/8=2, 1/16=4, 1/32=8, 1/64=16, dotted = `/1.5`, triplet = ×1.5. `step_len = 1 / steps_per_beat` beats. | Scheduler |
| `gate` (.02..1) | — | Note length as a fraction of one step. Per-step edit length overrides it. Ratchet gate is capped to the substep spacing. | Scheduler |
| `swing` (0..0.5) | — | Only odd steps are delayed, by `swing × step_len` beats. | Scheduler |
| `regenerate_every` (0..256) | — | Counted in complete loop lengths, not grid cells. `cycle = floor(step_index / length)`; fires when `cycle > 0`, `cycle % every == 0`, and not already fired. | Scheduler |
| `randomize_after_lengths` (0..256) | — | Same complete-loop counting; requests editor-side randomize. | Scheduler |
| LFO links | `GenParams` subset | Pattern-shaping targets are sampled once per loop, then `generate_with()` runs again. Feel targets are sampled per note/block. | Scheduler |

### Generation pipeline

1. `gen_params()` builds a plain snapshot from host parameters.
2. `gen_params_at(clock)` overlays linked LFO values. `MaxSteps` is resolved first because it changes the allowed `Steps` range; `Steps` is then quantized to cells; `Pulses` and `Rotate` are clamped against that result.
3. `generate_with()` clamps `steps`, `pulses`, `rotate`, `symmetry`, `osc_wave`; runs `run_mode()`.
4. Probability is applied per lane/step, then the hard density/pulses mute.
5. Rotation `shift = (rotate % steps + variation % steps) % steps` reorders every per-step view array.
6. Invert, mirror, metric accents, then manual edits are applied.
7. Scheduler maps `step_index(k, length)` to a `Pattern` cell, applies swing, reads ratchets/microtiming/gate from the edit overlay, and emits MIDI on zero-based channel 9.

---

## 2. Per-mode parameters, calculations and transfers

`P` is the lane density profile. “Seed/RNG” means the value only enters through `Rng::new(seed, variation, salt)`, not as a direct formula input. All rows also pass through `generate_impl()` at `_nDrums/src/sequence/mod.rs:387`; the `Code reference` column lists the mode-specific implementation.

| # | Mode (group) | Controls actually read | Calculation / mapping | Transfer / output | Code reference |
|---:|---|---|---|---|---|
| 0 | Euclidean (Rhythm) | `pulses`, `density`, `seed` (accents only) | `k = floor(pulses × d × (0.35 + P) + 0.5)`, limited to `n`. Lane phase `= tr×n/8`. `hit = anchored_onset(pos, k, n)`, i.e. `(n − (pos×k mod n)) mod n < k`. Accent `1.0` on `i%4==0`, else `0.72 + 0.15×rng`. | `hit`, `accent`; seed does not change the rhythm, only accent humanization. | _nDrums/src/sequence/euclidean.rs:12 generate; :5 lane_pulses; _nDrums/src/sequence/mod.rs:243 anchored_onset; :249 rotated_pos |
| 1 | Eulerian (Geometric) | `pulses` (degree), `density` (gate), `seed`/`variation` (shuffle) | `degree = 1 + pulses % 4`. Builds a balanced 9-vertex digraph (rest + 8 lanes) from a seeded permutation, then a Hierholzer circuit. Step reads `circuit[i % circuit_len]`; at `steps=1` starts at the first non-rest. `threshold = d × (0.4 + P)`; hit if `steps==1` or local RNG `< threshold`. | `state` = vertex; hit on `state > 0`; accent `0.7 + 0.3×rng`. | _nDrums/src/sequence/eulerian.rs:5 generate |
| 2 | Phyllotaxis (Geometric) | `pulses`, `density`, `seed`, `phase` | `pulses_of = max(1, min(pulses, n))`; `thr = (pulses/n clamped .02..95) × (0.35 + P) × (0.5 + d)`, clamped `.02..98`. Primary phase `= frac(i×φ⁻¹ + seed + phase + tr×0.137)`. Secondary phase `= frac((i + tr×φ⁻¹ + phase×n) × golden_angle / (2πφ))`. Hit if primary `< thr` or secondary `< thr×0.18`. | `hit`, `accent = .65 + (1−primary)×.35`. No per-step view array beyond accent. | _nDrums/src/sequence/phyllotaxis.rs:57 generate; :47 rules_fired; :39 lane_threshold; :23 primary_phase; :34 secondary_phase |
| 3 | Cellular (Cellular) | `ca_rule`, `symmetry` (seed shape), `density`, `seed`/`variation`, `phase`, `pulses` | Initial row: `symmetry=1` centre seed, `2` alternating stripes, `3` solid with one gap, else random with probability `d`. 16 rows of Wolfram `ca_rule` on a ring. `selected_row = 0` if `n<=1`, else `(pulses + round(phase×16) + tr×2) % 16`. Hit if cell is live and (`steps==1` or local RNG `< 0.45 + (d×P)×0.55`). | Publishes `ca[16][n]`; accent `1.0` if cell else `0.6`. | _nDrums/src/sequence/cellular.rs:14 generate; :4 selected_row; _nDrums/src/sequence/mod.rs:222 enabled_threshold |
| 4 | Oscillator (Rhythm) | `pulses`, `density`, `phase`, `osc_rate`, `osc_wave` | `wanted = round(pulses × d × (0.5 + P))`, limited to `n`. Wave value at `frac(i/n × osc_rate + phase + tr×0.071)` using one of 8 shapes. `top_k(wave, n, wanted)` selects hits; ties go to the earlier step. | `orbit` = lane 0 wave curve; accent `0.55 + 0.45×wave`. No seed/phase formula beyond `phase`. | _nDrums/src/sequence/oscillator.rs:42 generate; :8 value; _nDrums/src/sequence/mod.rs:266 top_k |
| 5 | Crystallographic (Geometric) | `pulses`, `density`, `seed`/`variation`, `symmetry` | `order = clamp(symmetry,1,8)`; `unit = n/order` if `>=2`, else `n`. `thr = (pulses/n clamped .02..95) × (0.55 + .45d) × (0.4 + .6P)`. Random motif cell of length `unit` with probability `thr`; if empty, force one hit. Symmetry cases 1..8 map `(cell, offset)` by translation, mirror, glide, inversion, screw, centering, alternating mirror, staggered lattice. `symmetry=8` odd cells invert when `d>0.35`. | `hit`, `accent` (`.95` even cell, `.72` odd). | _nDrums/src/sequence/crystallographic.rs:5 generate |
| 6 | Logistic (Chaos / Physics) | `pulses`, `density`, `seed`, `phase`, `variation` | `r = 3.5 + .5d`; `x0 = clamp(.05 + .9×frac(seed + phase + variation×φ⁻¹), .001, .999)`. 24 burn-in iterations plus up to 32 until `x` is in `.1..9`. Orbit `x ← r×x×(1−x)`. `threshold = (pulses/n) × (.3 + .7d) × (.5 + P)`, clamped `.03..97`; `cutoff = 1−threshold`. Lane offset `= tr × max(1,n/8)`. Hit if `x >= cutoff` and `x` is a local peak (`x >= prev && x > next`), with `n<=2` bypass and `n==1` always hit. | `orbit` = logistic orbit; accent `.55 + .45x`. | _nDrums/src/sequence/logistic.rs:18 generate; :4 r_of; :9 x0 |
| 7 | Markov (Stochastic) | `pulses`, `density`, `seed`/`variation`, `markov_bias` | Hard-coded 9-state weights (rest + 8 lanes), biased by four 2-bit fields: rest `+2`, self-repeat `+3`, role-compatible lane weights, novelty `+1` to all other lanes. `event_chance = clamp(d × (.25 + pulse_ratio×1.5), 0,1)`. If `n>1` and RNG exceeds it, state becomes rest; otherwise weighted pick. Local hit gate `≈ .25 + .5d + .25×pulse_ratio`. | `state`; hit/accent `.68 + .32×rng`. `markov_bias` only affects this mode. | _nDrums/src/sequence/markov.rs:5 generate |
| 8 | Raised-Cosine (Rhythm) | `pulses`, `density`, `seed`, `phase` | `hits = round((pulses + tr%3) × d)`, limited to `n`. `phase0 = phase + seed×0.4`. Envelope `= .5 − .5cos(2π(i/n × hits + phase0))`. `duty = hits/n`; `cutoff = .5(1 + cos(π×duty))`. Each lane rotates its output by `tr % n`; accent `.55 + .45×envelope`. | `orbit` = lane 0 envelope; hit/accent. | _nDrums/src/sequence/raised_cosine.rs:26 generate; :6 envelope; :12 phase_offset; :18 cutoff; _nDrums/src/sequence/mod.rs:228 pulse_hits |
| 9 | Slowcat (Rhythm) | `pulses`, `density`, `seed`, `variation` | `count = max(1,pulses)`; `motif = clamp(2 + (tr+count)%7, 1, n)`. `shift = (floor(seed×997) + tr×3 + variation) % motif`; `slot = (i+shift)%motif`; `phrase = (i/motif + tr) % (tr%3 + 2)`. Hit requires `(slot×count + tr) % motif < min(count,motif)`, `(phrase != 0 or n <= motif)`, and (`(i+tr)%4==0` or `d>.35`), then additionally `((i+tr×2)%7 != 6 or d>.75)`. | `hit`, accent `1.0` on downbeat else `.66`. | _nDrums/src/sequence/slowcat.rs:10 generate; :5 motif_len |
| 10 | Toussaint Distance (Rhythm) | `pulses`, `density`, `seed` | `k = pulse_hits = round((pulses + tr%3) × d)`, limited to `n`. `seed_offset = floor(seed×n) % n`; lane offset `= (seed_offset + tr×n/8) % n`. Boundary-crossing hit: `floor((pos+1)k/n) != floor(pos×k/n)`. Accent `= .62 + .38(1 − modular_distance(pos,k,n)/n)`. | `hit`, accent; no extra view array. | _nDrums/src/sequence/toussaint.rs:6 generate; _nDrums/src/sequence/mod.rs:228 pulse_hits; :236 modular_distance; :249 rotated_pos |
| 11 | Well-Formed (Rhythm) | `pulses`, `density` | `k = pulse_hits`; lane phase `= tr×n/8`; same anchored Bresenham onset as Euclidean. | `hit`, accent `1.0` hit else `.72`. Seed ignored in generator. | _nDrums/src/sequence/well_formed.rs:6 generate; _nDrums/src/sequence/mod.rs:228 pulse_hits; :243 anchored_onset |
| 12 | L-System (Rhythm) | `pulses`, `density`, `seed`, `variation`, `symmetry` | `symmetry` 1/2/3/5 selects grammar family; otherwise `(floor(seed×4) + variation) % 4`. 16 parallel rewrites into a 1024-symbol buffer, then repeated/read to `n`. `thr = (pulses/n) × (.35 + .65d) × (.45 + P)`. Per-symbol chance `= thr × grammar_weight(symbol,lane) × metrical_weight(step)`; `grammar_weight` is `1.0` for the preferred A/B symbol and `.55` otherwise; metrical `i%4==0 → 1.0`, `i%2==0 → .84`, else `.72`. | Publishes `lsym`; accent `.96` for A, `.72` for B. | _nDrums/src/sequence/lsystem.rs:9 generate; :66 lane_threshold; :81 grammar_weight; :86 metrical_weight; :91 step_chance |
| 13 | Phase Shift (Rhythm) | `pulses`, `density`, `seed`, `phase` | `pulses = round(pulses×d)`, limited to `n`. Base train: `bucket += pulses`; when `bucket >= n`, mark base hit and subtract `n`. Shift `= tr×max(1,n/8) + round(phase×n) + round(seed×n)`. Hit reads `base[(i+n−shift)%n]`. | `hit`, accent by source `src%4` (`1.0` downbeat, `.82` backbeat, `.66` else). | _nDrums/src/sequence/phase_shift.rs:5 generate; _nDrums/src/sequence/mod.rs:249 rotated_pos |
| 14 | Polyrhythm Stack (Rhythm) | `pulses`, `density`, `seed` | Fixed ratios `[(1,1),(3,2),(2,1),(5,2),(4,3),(2,3),(5,4),(3,4)]`. `scaled = (pulses×num + den/2)/den`; `k = round(scaled×d)`, limited to `n`. Phase `= round(seed×n) + tr×n/8`. Boundary-crossing onset on `pos`. | `hit`, metric accent by `pos`. | _nDrums/src/sequence/polyrhythm.rs:5 generate; _nDrums/src/sequence/mod.rs:249 rotated_pos |
| 15 | Weighted Syncopation (Rhythm) | `pulses`, `density`, `seed`/`variation` | `target = round(pulses×d×(.45+P))`, limited to `n`. Score `= beat_weight(i%4: 0→.58, 2→.86, else 1.28) × shifted_weight((i + (tr×3)%4)%4: 0→.60, 2→.88, else 1.24) × jitter(.85..1.15)`. `top_k` selects exactly `target`. | `orbit` = lane 0 normalized scores; accent `1.0` downbeat, `.84` backbeat, `.62` other. | _nDrums/src/sequence/syncopation.rs:5 generate; _nDrums/src/sequence/mod.rs:266 top_k |
| 16 | Pebble Box (Chaos / Physics) | `pulses`, `density`, `phase`, `osc_rate`, `ca_rule`, `seed`/`variation` | `gravity = .01 + ca_rule/255×2.15`; `tilt = phase×2π`; `substeps = clamp(round(osc_rate),1,16)`; `dt = 1/substeps`; impact threshold `= (1−d)×4.2`. `shaker = clamp(pulses,1,n)`; impulse every `max(1,n/shaker)` steps to lanes with `tr%3 == impulse%3`. Kick speed `2.4 + 1.8×rng`; bounce `.40..55`. Hit when impact `>= threshold` and `>= .6`. | `pebbles` = per-step 8×(x,y) bytes; `impact` = wall 1..4; accent `= clamp(impact/4.2,.15,1)`. | _nDrums/src/sequence/pebble.rs:19 generate; :15 gravity |
| 17 | Sieve (Number Theory) | `ca_rule` (moduli), `symmetry` (residues), `phase`, `variation`, `seed`, `pulses` (terms), `density` | `m1 = MODULI[ca_rule%6]`, `m2 = MODULI[(ca_rule/6)%6]`, `m3 = MODULI[(ca_rule/36 + symmetry)%6]`, `MODULI=[2,3,5,7,11,13]`. `shift = round(phase×6) + variation`; `r1=(symmetry+shift)%m1`, `r2=(2×symmetry+shift)%m2`, `r3=(3×symmetry+shift+floor(seed×97))%m3`. `terms = 2 if pulses>=2 else 1`. Member `= (class1 OR (terms==2 AND class2)) AND NOT class3`. Lane shift `= tr×max(1,n/8)`. `wanted = round(members×d×(.35+P))`; `select_spread` chooses evenly. | `hit`, accent `.55 + .15×(1 + .25×depth)`; no separate view array. | _nDrums/src/sequence/sieve.rs:47 generate; :11 classes; :34 is_member; :43 member; _nDrums/src/sequence/mod.rs:285 select_spread |
| 18 | Kuramoto (Chaos / Physics) | `pulses`, `density`, `phase`, `osc_rate`, `seed`, `variation` | `coupling = (osc_rate−.125)/(8−.125) × 2` rad/step. `rate = max(pulses/n,.02)` turns/step. `detune = 1 + (.05 + .45phase) × 2 × (tr/7 − .5)`. Init `θ = 2π×frac(seed + variation×.137 + tr/8)`. Each step: `θ += 2π×rate×detune + coupling/8×(sin(θ_left−θ)+sin(θ_right−θ))`. Score `1.0` on a whole-turn wrap, else `.5 + .45(1−phase)`. `wanted = round(min(pulses,n)×d×(.5+P))`; `top_k`. | `phases` = per-step per-lane normalized phase bytes; accent `1.0` on wrap else score+.05. | _nDrums/src/sequence/kuramoto.rs:44 generate; :16 coupling; :22 rate; :27 detune; _nDrums/src/sequence/mod.rs:266 top_k |
| 19 | Euclidean-Swing (Rhythm) | Same as Euclidean + `swing` | Runs Euclidean, then for every odd-step hit sets accent `= min(.55 + .25×rng, 1.0)`. The actual timing swing is applied by the scheduler, not the generator. | `hit`, `accent`; scheduler delays odd steps by `swing×step_len`. | _nDrums/src/sequence/euclidean_swing.rs:3 generate; _nDrums/src/sequence/euclidean.rs:12 generate |
| 20 | Bernoulli (Stochastic) | `pulses` (gate only), `density`, `probability`, `seed`/`variation` | `prob = d × probability`. Per step, draw and hit if draw `< prob`. If lane `tr>0` and all earlier lanes hit the same step, suppress it. Lane 0 also publishes its draw to `u`. | `hit`, accent `.6 + .4×rng`; `u` lane 0. **Probability is applied again globally** → effective `d × probability²`. | _nDrums/src/sequence/bernoulli.rs:3 generate |
| 21 | African Bell (Music Theory) | `pulses` (gate only), `probability`, `seed`/`variation` | 12-step figure `[T,F,F,T,F,F,T,F,F,T,F,F]`. Per lane `rot = rng.below(n)`; `hit = BELL[i%12] && rng < probability`; column suppression as Bernoulli. | `hit`, accent `.75+.25×rng`. **Double probability** → effective `probability²`. | _nDrums/src/sequence/african_bell.rs:6 generate |
| 22 | Phase Canon (Music Theory) | `pulses`, `density`, `probability`, `seed`/`variation` | `pulses = max(1,min(n,pulses))`. Base `= floor((i+1)pulses/n) != floor(i×pulses/n)`. Shift `= tr×n/8`. `hit = base && rng < probability×d`; column suppression. | `hit`, accent `.6+.4×rng`. **Double probability** → effective `p×d×p`. | _nDrums/src/sequence/phase_canon.rs:3 generate |
| 23 | Reaction-Diffusion (Chaos / Physics) | `pulses` (gate only), `density`, `probability`, `seed`/`variation` | `a = sin(i/n×2π×(1 + tr×.1) + tr×.7)`. `hit = a >= (1−d) && rng < probability`; column suppression. Lane 0 publishes `(a+1)/2` to `u`. | `hit`, `u` lane 0, accent `.6+.4×rng`. **Double probability** → effective `p²`. | _nDrums/src/sequence/reaction_diffusion.rs:3 generate |
| 24 | Lorenz (Chaos / Physics) | `pulses` (gate only), `density`, `probability`, `seed`/`variation` | Euler integration: `sigma=10`, `rho=28`, `beta=8/3`, `dt=.01`, 10 inner iterations per step, lane start `x=.1+tr×.05`. Onset on local `z` peak (`z>prev_z && z>1+tr×.5`) or `i==0`. `hit = onset && rng < probability×d`; column suppression. | `hit`, accent `= min(abs(z)/30,1)`. **Double probability** → effective `p×d×p`. | _nDrums/src/sequence/lorenz.rs:3 generate |
| 25 | Cantor (Fractal) | `pulses` (gate only), `density`, `probability`, `seed`/`variation` | `depth = 2 + min(tr,3)`; `x = i/n`. `in_cantor(x,depth)` rejects any step whose ternary expansion enters the middle third. `hit = in_cantor && rng < probability×d`; column suppression. | `hit`, accent `.7+.3×rng`. **Double probability** → effective `p×d×p`. | _nDrums/src/sequence/cantor.rs:13 generate; :3 in_cantor |
| 26 | Fibonacci/Sturmian (Number Theory) | `pulses` (gate only), `density`, `seed`, `phase`, `variation` | `alpha = clamp(d,.05,1)×φ⁻¹`; base `= frac(seed + phase + variation×φ⁻¹)`. Lane slope `a = clamp(alpha×(1−.03tr),.03,.95)`; lane phase `= base + tr×n/8`. Onset when `floor((i+1+phase)a) != floor((prev+phase)a)`. If no onset, force step 0. | `hit`, accent `.9` hit else `.65`. | _nDrums/src/sequence/fibonacci.rs:11 generate; :4 alpha |
| 27 | Genetic Mutation (Stochastic) | `pulses` (gate only), `density`, `probability`, `seed`/`variation` | Base `= (i×(tr+1)) % n < n/2`; mutate with probability `1−d` (mutation flips the bit). `n==1` forces true. `hit = base/mutated && rng < probability`; column suppression. | `hit`, accent `.6+.4×rng`. **Double probability** → effective `p²`. | _nDrums/src/sequence/genetic.rs:3 generate |
| 28 | Arnold Circle Map (Chaos / Physics) | `pulses` (Ω), `density` (K), `seed`, `phase`, `variation` | `Ω = max(1,min(pulses,n))/n`; `K = (1−d)×1.4` (high density = weak coupling). Start `= frac(seed + phase + variation×φ⁻¹ + tr/8)`. Iterate `θ' = θ + Ω − K/(2π)sin(2πθ)`; fire on a whole-turn wrap (`floor(next+1e-9)>=1`). If a lane never wraps, force the highest-phase step. | `orbit`, `theta`, `u` = lane 0 θ; accent `.55+.45θ`. `phase`/seed/variation move the orbit. | _nDrums/src/sequence/arnold.rs:13 generate; :6 coupling |
| 29 | Sturmian Word (Number Theory) | `pulses` (gate only), `density`, `probability`, `seed`/`variation` | `alpha = clamp(.3 + .4×phase, .05,.95)` (phase is the slope, not an offset). Lane phase offset `= tr×.125`. Onset when the floor of `(i+1+offset)α` changes, or `i==0`. `hit = onset && rng < d×probability`; `n==1` draws directly. Column suppression; fallback step 0 if `probability>0`. | `hit`, accent `.6+.4×rng`. **Double probability** → effective `d×p²`. | _nDrums/src/sequence/sturmian.rs:3 generate |
| 30 | Hilbert Curve (Geometric) | `pulses` (order), `density`, `phase` (turn), `seed` | `order = clamp(pulses,1,5)`; grid `2^order`; `d = i×total/n`; convert Hilbert index to `(x,y)`, rotate by `floor(phase×4)` quarter turns. Lane `= (x + 2y + i + round(seed×8)%8) % 8`. Each sampled cell becomes a primary hit on that lane. `wanted = round(m×d).max(1)`, limited to `m`; `select_spread`. | `orbit` = x normalized, `theta` = y normalized, `u` = x; accent `.55+.45×theta`. | _nDrums/src/sequence/hilbert.rs:51 generate; :5 point; :31 turns; :36 rotate; _nDrums/src/sequence/mod.rs:285 select_spread |
| 31 | Ulam Spiral Primes (Geometric) | `pulses` (target onsets), `density` (keep fraction), `phase`, `variation`; `seed` ignored | Spiral start `offset = round(phase×64) + 5×(variation % 1024)`; `cell = 1 + offset + i`. Walk legs `1,1,2,2,3,3…`; prime cells (including 1) assigned to lane `(x−y) mod 8`. `wanted = lane_wanted = round(min(pulses,n)×d×(.5+P))`; `select_spread` over that lane's primes. If a lane has no prime, force a hit at its phase position. | `state` = prime flag; `orbit`/`theta` = normalized spiral; `u` = average; accent `.95` on selected hits. | _nDrums/src/sequence/ulam.rs:19 generate; :3 is_prime; _nDrums/src/sequence/mod.rs:307 lane_wanted; :285 select_spread |
| 32 | Vuza Canon (Number Theory) | `density`, `seed`/`variation`, `ca_rule` (modulus), `probability`; `pulses` unused internally | `modulus = clamp(ca_rule,8,32)`. `a_size = max(1,round(modulus×d))`; build a seeded subset `A` of `Z_modulus` by Bernoulli `.5` until size. Lane shift `= tr×max(1,modulus/8)`; `residue = (i+shift)%modulus`; `hit = A[residue] && rng < probability`. | `hit`, accent `.6+.4×rng`. `pulses` still hard-mutes at 0. **Double probability** → effective `p²`. | _nDrums/src/sequence/vuza.rs:3 generate |
| 33 | Thue-Morse (Number Theory) | `pulses` (target), `density`, `phase`, `variation`; `seed` ignored | `offset = round(phase×16) + variation % 1024`. Word bit `= parity(popcount(i+offset))`. Lane window adds `tr×max(1,n/8)`. `wanted = lane_wanted(...).max(1)`, limited to the number of 1s; `select_spread` over 1s. Contour uses Nørgård's infinity series `a(0)=0, a(2n)=−a(n), a(2n+1)=a(n)+1`. | `lsym`/`orbit` = bit; `u` = contour; accent `.55+.45×contour`; fallback if window has no 1s. | _nDrums/src/sequence/thue_morse.rs:27 generate; :4 bit; :11 infinity; _nDrums/src/sequence/mod.rs:307 lane_wanted; :285 select_spread |
| 34 | Rudin-Shapiro (Number Theory) | `pulses` (target), `density`, `phase`, `variation`; `seed` ignored | `offset` as Thue-Morse. Bit `= parity(popcount(n & (n>>1)))` (overlapping `11` pairs). Lane window `tr×max(1,n/8)`. `wanted = lane_wanted(...).max(1)`, limited to the number of 1s; `select_spread`. | `lsym`/`orbit` = bit; `u = ((i+b)%7 + .5)/7`; accent `.55+.45×((i+tr)%7)/6`; fallback if no 1s. | _nDrums/src/sequence/rudin_shapiro.rs:12 generate; :5 bit; _nDrums/src/sequence/mod.rs:307 lane_wanted; :285 select_spread |
| 35 | Brownian Walk (Stochastic) | `pulses` (target), `density` (σ), `phase` (start), `seed`/`variation` | `σ = .01 + .2d`. Start `x = frac(.5 + phase + tr/8)`. Gaussian increment via Irwin-Hall sum of 4 uniforms, zero mean unit variance, multiplied by σ; position folds back into `0..1` with `fold01`. Score `= abs(step)` (step 0 is 0); `wanted = lane_wanted(...)`, `top_k`. | `orbit`/`u` = lane 0 walk; `theta` = step size; accent `.62+.38×walk`. | _nDrums/src/sequence/brownian.rs:17 generate; :5 gaussian; :11 sigma; _nDrums/src/sequence/mod.rs:277 fold01; :307 lane_wanted; :266 top_k |
| 36 | Levy Flight (Stochastic) | `pulses` (target), `density` (tail/scale), `phase` (start), `seed`/`variation` | Pareto tail `α = 2 − 1.5d`; minimum step `s_min = .02 + .06d`; step `= min(s_min×U^(−1/α), 8)`. Sign random; start `frac(.5 + phase + tr/8)`; fold into `0..1`. Score `= length`; `wanted = lane_wanted(...)`, `top_k`. | `orbit`/`u` = lane 0 walk; `theta` = step size; accent `.58+.42×walk`. | _nDrums/src/sequence/levy_flight.rs:22 generate; :4 tail_alpha; :9 step_min; :15 step_len; _nDrums/src/sequence/mod.rs:277 fold01; :307 lane_wanted; :266 top_k |
| 37 | Tent Map (Chaos / Physics) | `pulses` (target), `density` (μ), `seed`, `phase`, `variation` | `μ = 1 + .99d`; `f(x) = μ×min(x,1−x)`. Start `x = .05 + .9×frac(seed + phase + variation×φ⁻¹ + tr/8)`; 32 burn-in steps. Scores = orbit `x`; `wanted = lane_wanted(...)`, `top_k`. Normalize against `lo = μ(1−μ/2)`, `hi = μ/2`. | `orbit`; `u`/`theta` = normalized orbit; accent `.62+.38×norm`. | _nDrums/src/sequence/tent_map.rs:11 generate; :5 mu_of; _nDrums/src/sequence/mod.rs:307 lane_wanted; :266 top_k |
| 38 | Henon Map (Chaos / Physics) | `pulses` (target), `density` (a), `seed`, `phase`, `variation` | `a = 1 + .4d`, `b = .3`. Start `x = (frac(seed×.7 + phase + variation×φ⁻¹ + tr/8)−.5)×.4`, `y = (frac(seed + variation×(1−φ⁻¹) + phase×.7 + tr/8)−.5)×.2`; 64 burn-in steps. Update `x' = 1 − a x² + y`, `y' = b x`; restart at `(.1,.05)` if escaped. Score `= abs(x−prev_x)`; `wanted = lane_wanted(...)`, `top_k`. | `orbit` = normalized x, `theta` = normalized y; accent `.58+.42×ux`; column de-duplication with a fallback so no lane is silent. | _nDrums/src/sequence/henon_map.rs:11 generate; :5 a_of; _nDrums/src/sequence/mod.rs:307 lane_wanted; :266 top_k |
| 39 | Life-like CA (Cellular) | `ca_rule`, `symmetry`, `density`, `pulses` (warm-up), `seed`/`variation` | Fixed 16×8 torus. `ca_rule % 5` selects Life, HighLife, Seeds, Day & Night, or dense/high-survival. `symmetry` selects centre block, stripes, sparse random, or denser random seed. `pulses = min(64, pulses)` warm-up generations (after the global `min(pulses, steps)` clamp). Each step reads one row's live count; hit if `count>0` and (`i==0` or RNG `< .55 + d×.45`) and not all previous lanes already hit. | Publishes `ca` grid; accent `.35 + count/16×.65`; fallback hits the lane with the largest live count. | _nDrums/src/sequence/life.rs:85 generate; :11 rule_masks; :27 seed; :63 step |

---

## 3. Shared post-generation pipeline

1. **Input clamps:** `steps` 1..1024; `pulses` to `steps` (Cellular to 15); `rotate %= steps`; `symmetry` 1..8; `osc_wave %= 8`.
2. **Generator:** `run_mode(&q)` produces hits, accents and view data.
3. **Probability:** For each lane/step with a hit, draw `Rng::new(seed, variation, 0xBABA_0000 + tr×1024 + i)`. Retain if `unit() < probability`; an edit-overlay probability for the display slot replaces the global value.
4. **Hard mute:** If `density <= 0` or (`kind` is not Cellular/Life and `pulses == 0`), clear all hits and impacts.
5. **Rotation:** `shift = ((rotate % steps) + (variation as usize % steps)) % steps`. Every per-step array is reordered: `hit`, `accent`, `state`, `ca`, `orbit`, `theta`, `u`, `lsym`, `pebbles`, `impact`, `phases`. `Pattern.rotation = shift`.
6. **Invert:** Flips every hit/rest after rotation; cleared hits also clear `impact`.
7. **Mirror:** Reverses every per-step array after invert.
8. **Metric accents:** Saves generator accents, writes metric accents (`i%4==0 → 1.0`, `i%2==0 → .82`, else `.68`) with a seeded human factor `.90..1.0`, then multiplies by the generator accent; clamps `.05..1`.
9. **Manual edits:** Display-space hit override, accent override, probability override. Edits win over generator and metric accents.
10. **Scheduler:** `step_index(k, length)` maps the continuous step cursor to a pattern cell; integer lengths use `k mod length`, fractional lengths use `floor((k mod length))`. Swing, microtiming, ratchets and gate are then applied.

### nDrums double-probability list

These generators already multiply by `p.probability`, and `generate_impl()` applies the global probability again:

| Mode | Internal use | Effective retention |
|---|---|---|
| Bernoulli | `d × probability` | `d × probability²` |
| African Bell | `probability` | `probability²` |
| Phase Canon | `probability × d` | `d × probability²` |
| Reaction-Diffusion | `probability` | `probability²` |
| Lorenz | `probability × d` | `d × probability²` |
| Cantor | `probability × d` | `d × probability²` |
| Genetic | `probability` | `probability²` |
| Sturmian | `d × probability` | `d × probability²` |
| Vuza | `probability` | `probability²` |

All other modes rely on the single global probability pass.

---

## 4. LFO target transfer (nDrums)

`ModTarget::ALL` has 16 targets. Links are persisted as one bit per target in `lfo*_links`.

| Target | Base / range | Generation-time or scheduling | Notes |
|---|---|---|---|
| `Steps` | 1..`step_ceiling` | Generation | Quantized to cells after summation |
| `Pulses` | 0..`step_ceiling` | Generation | Integer after summation |
| `Rotate` | 0..`cell_count−1` | Generation | Integer after summation |
| `Density` | 0..1 | Generation | Also global mute at 0 |
| `Seed` | 0..1 | Generation | Full re-generation |
| `OscPhase` (`phase`) | 0..1 | Generation | Oscillator/phyllotaxis/etc. |
| `OscRate` | .125..8 | Generation | Oscillator, Pebble, Kuramoto |
| `CaRule` | 0..255 | Generation | Integer |
| `Probability` | 0..1 | Generation | Applied in the global pass |
| `Swing` | 0..0.5 | Scheduling | Per note |
| `Gate` | .02..1 | Scheduling | Per note |
| `Velocity` | .01..1 | Scheduling | Per note |
| `RegenerateEvery` | 0..256 | Scheduling | Can be raised by an LFO from 0 |
| `Symmetry` | 1..8 | Generation | Integer |
| `RandomizeAfter` | 0..256 | Scheduling | Can be raised by an LFO from 0 |
| `MaxSteps` | 64..1024 | Generation | Resolved before `Steps` |

Signal: `LfoSpec` has wave (8 shapes), rate mode (Hz or beats), frequency/rate, depth 0..2, phase, polarity (bipolar, unipolar, unipolar negative). `lfo_signal = depth × shaped_wave`, clamp ±2. `fold(base, offset, min, max)` adds `offset × (max−min)` to the base and clamps; integer targets round after summing all linked LFOs.

Pattern-shaping links are sampled once per loop (`last_mod_cycle`); feel targets are sampled at each note. `regenerate_every` and `randomize_after_lengths` count complete loop lengths, not grid cells.

---

## 5. Mode step floors

`Mode::min_steps()` is applied when selecting a mode by `Mode::steps_for_mode()`. It raises `steps` and, if necessary, `max_steps`; it never lowers a longer loop or higher ceiling.

| Floor | Modes |
|---:|---|
| 24 | African Bell |
| 32 | Phyllotaxis, Logistic, Slowcat, Phase Shift, Pebble Box, Sieve, Bernoulli, Phase Canon, Vuza |
| 64 | L-System, Kuramoto, Reaction-Diffusion, Lorenz, Fibonacci, Sturmian, Ulam, Thue-Morse, Rudin-Shapiro, Brownian, Levy Flight, Tent Map, Henon Map, Life |
| 81 | Cantor |
| none | Euclidean, Eulerian, Cellular, Oscillator, Crystallographic, Markov, Raised-Cosine, Toussaint, Well-Formed, Polyrhythm, Syncopation, Euclidean-Swing, Genetic, Arnold, Hilbert |

---

## 6. Calculation gotchas worth knowing

- **`length` never reaches a generator.** Only `steps` does. Fractional loops are a scheduler wrapping feature, not a generator resolution change.
- **`pulses == 0` is a global mute** for every mode except Cellular and Life, even for modes that do not use `pulses` internally (Vuza is the clearest example).
- **`density <= 0` is also a global mute.** In chaos/physics modes, density is primarily a coefficient, so zero is both “map parameter 0” and “silence”.
- **`rotate` is composed with `variation`.** A Generate press can move the downbeat even when the algorithm is deterministic in `seed`.
- **`seed` is hidden in some UIs but still shapes accents.** Oscillator, Well-Formed, Ulam, Thue-Morse and Rudin-Shapiro do not use seed for the line, but the shared accent RNG does.
- **Mode-scoped reuse is intentional but easy to misread:** `ca_rule` is a rule, gravity, sieve modulus or Life rule; `osc_rate` is cycles, substeps or coupling; `symmetry` is a CA seed, grammar family, crystallographic order, sieve residue or Life seed.
- **The Cellular special case matters:** `pulses` is capped to `CA_ROWS−1` (15), not to `steps`, because it selects a generation row.
- **Manual edits are display-space.** They are applied after rotate/mirror, so an edit stays on the step the user clicked even if Generate rotates the underlying line.
