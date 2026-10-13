# AlgoDrumNemShine – review

**Scope and honesty note.** The full plug-in (nih-plug + Slint) could not be built here: the only
toolchain available was Rust 1.75 and the project needs a far newer one. What I did instead:

* compiled and ran the **pure generator module** (`src/sequence/`) on 1.75 in a scratch crate
  (with the unstable `GOLDEN_RATIO` constant shimmed) – the original's own tests plus my probes;
* re-implemented the scheduler's step-cursor logic as a replica to reproduce scheduler bugs;
* compile-checked both `.slint` files with the Slint 1.18 interpreter (project pins 1.16.1);
* read `lib.rs`, `editor.rs`, `view.rs`, `globals.rs` line by line.

Status of each finding is marked **[ran]** (reproduced by execution), **[replica]** (reproduced on a
copy of the scheduler logic) or **[read]** (found by reading, not executed).
`lib.rs` / `editor.rs` edits are **not compiled**.

Baseline: the original's 16 tests – 15 pass, `generation_never_allocates` **fails** on 1.75.

## Bugs found (all fixed in `AlgoDrumNemShine_fixed`)

| # | Severity | Finding | Status |
|---|---|---|---|
| 1 | High | **Oscillator mode ignores `pulses`.** Ties at the cutoff were all kept: a square wave sounded 8/16 steps on *every* lane whatever `pulses` was; sine/triangle gave 6 instead of 3. Code comment claims "exactly `wanted`, ties to earlier step". | [ran] fixed (rank selection) |
| 2 | High (RT-safety) | `sort_by` in the oscillator **allocates** on older stdlib (32 allocations in the AGENTS-mandated test). Only stays allocation-free on new toolchains by accident. | [ran] fixed (same change) |
| 3 | High (musical) | **Euclidean and Well-Formed are not anchored on step 0.** E(4,16) played 3,7,11,15 (kick behind the beat); default lanes started on 3/5/7. | [ran] fixed: E(4,16)=0,4,8,12 |
| 4 | High | **Changing Rate to a slower value silences the plug-in** (~121 beats ≈ 60 s at 120 bpm in the replica): the carried step cursor counts steps of the old rate and was only lower-bounded. | [replica] fixed (`resume_cursor` upper bound) |
| 5 | High | **"Whole steps" on, but loop length stays fractional.** `gen_params` passed the raw float as `length`; `set_opt` and the spin box store raw slider values (e.g. 16.37, displayed "16"), so the pattern drifts 0.37 step per cycle (cell index 0,16,15,15,14… in the replica). | [replica]+[read] fixed in `gen_params` and `set_opt` |
| 6 | Medium | Logistic map silent at density 100 % for short patterns (r=4 burn-in ends near 0; steps=2 → no hit). Violates the "hits at full density" rule in AGENTS.md. | [ran] fixed |
| 7 | Medium | Eulerian: closed circuit repeated whole, so every seam played **two rests in a row**. | [ran] fixed |
| 8 | Medium | The AGENTS.md-required test "every mode produces hits at steps=1 and full density" **did not exist.** | added |
| 9 | Medium | Metric accent (downbeat/backbeat) was baked in *before* rotation, so Rotate moved the loud step off the beat. | [ran] fixed – **behaviour change**, accents now stay on the grid |
| 10 | Medium | `set_opt` key 0 reads `cell_count()` before the host echoes the new Steps value → Pulses/Rotate are not clamped when steps shrink (UI shows pulses > steps). | [read] fixed |
| 11 | Medium | Mode list: `mode-scroll-y` is **positive**, but a Slint viewport moves with *negative* values, so it clamps to 0 and modes 8–15 can be selected (wheel/click) but never scrolled into view. Not run visually. | [read] fixed (negative, travel 264 px) |
| 12 | Low | Host that gives no `pos_beats` → every block pinned to beat 0 (step 0 replayed, jump detector fires). Now keeps its own clock. | [read] fixed |
| 13 | Low | `std::f64::consts::GOLDEN_RATIO` is rejected by older compilers (confirmed on 1.75) although the README promises "Rust 1.88 or newer". Replaced by a literal. | [ran] fixed |

## Found, not changed (decisions for you)

* **Variation rotates the whole pattern** (`generate()`: `shift = rotate + variation % steps`). Every
  Generate press / "Regenerate after N" moves everything one step, so a kick leaves the downbeat. It is
  what makes closed-form modes change at all, but it is probably not what you want for drums.
* **Auto-Randomize only works while the editor is open** (it is applied by the GUI thread) – documented, but it
  makes the feature useless during normal playback with the window closed.
* View mismatches: Cellular highlights row `pulses % 16` (ignores phase and per-lane offset); the
  oscillator threshold line uses a different formula than the generator; the Euclidean clock shows disabled lanes.
* Two lanes mapped to the same MIDI note: one lane's NoteOff cuts the other.
* Raised-Cos count was off in 3 of 6400 lane/seed/phase samples (fixed in the Mini by construction).
* Fractional-length mode counts regeneration cycles in `ceil(length)` cells.
* `globals::save` is leading-edge: a change inside 750 ms of the last write is only saved on editor close.
* VST3 with no audio buses is poorly supported by many hosts; test CLAP first.
* **The tarball's `.clap`/`.so` (built 16:32) predate the last source edits (17:33–17:35)**, so the shipped
  binary is not the current source. Rebuild before testing.

## Tests

* Fixed original: 23 generator tests pass on 1.75, including `generation_never_allocates`. Added: oscillator
  exact counts (4 waves), Euclidean exact/even/anchored, Well-Formed anchor, AGENTS required "steps=1 / full
  density", Eulerian seam, accent-on-grid, Logistic short patterns; plus `resume_cursor` and snapped-length
  tests in `lib.rs` (the `resume_cursor` one was also executed in the scratch crate; the rest of `lib.rs` is uncompiled).
* Mini: 32 tests pass (generators 17, scheduler 11, view 4), including a counting-allocator test, exact onset
  counts over 5,000+ parameter combinations, NoteOn/NoteOff pairing at six block sizes, swing, slower-rate
  switch, transport jump, channel change and regeneration.
