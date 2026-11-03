# Research: Fractal, Geometric / Sacred-Geometry and Cellular Generation

Status: **research only — nothing in this document is implemented.** This is a shared reference
for proofreading before the next mode wave. The companion document in nelody seq
(`_Nelody/_Nelody/docs/RESEARCH_FRACTAL_GEOMETRIC_CELLULAR.md`) carries the same algorithm
review with melodic mappings.

## 1. Scope and constraints

The existing mode list already covers four relevant families:

- **Fractal / self-similar:** Cantor set, Hilbert curve, plus chaotic maps that are often
  discussed alongside fractals (logistic, tent, Hénon, Lorenz).
- **Geometric:** Euclidean ring, phyllotaxis rosette, crystallographic symmetry, Hilbert grid,
  Ulam spiral.
- **Cellular:** one elementary 1-D Wolfram automaton with 16 generations and a ring boundary.
- **Stochastic / physics:** Markov, Bernoulli, genetic mutation, Brownian, Lévy, pebble box,
  Kuramoto, reaction-diffusion.

The gap is specific: there is no **Mandelbrot/Julia escape-time mode**, no **Newton basin mode**,
no **2-D Life-like automaton**, no **Langton's Ant**, and no **parametric / polyhedral geometry**
mode. Those are the highest-value additions.

Any new mode must respect the project invariants:

1. `generate(&GenParams) -> Pattern` is pure, deterministic and allocation-free.
2. Every loop has a fixed upper bound. Iteration caps, grid sizes and tile counts are constants
   or derived from validated parameters; no `while` loop can run on an unbounded attractor.
3. All output arrays are indexed only through `0..steps`.
4. Views read generator-published data or a helper exported by the generator. A panel must not
   re-derive the algorithm.
5. New modes append to the enum; existing host indices never move.
6. `steps = 1`, density 0 and density 1 must be defined and tested.

The current parameter surface is enough for most candidates. `pulses`, `density`, `phase`,
`seed`, `ca_rule`, `symmetry` and `osc_rate` are already overloaded per mode, as documented in
`REFERENCE_MAPPING.md`. nrum seq has no `mode_data` bridge; a candidate that needs more than two
independent knobs should either reuse `symmetry`/`ca_rule` or be deferred until the shared
contract is extended.

## 2. Shared grouping taxonomy

The presentation groups are now shared with nelody seq:

`Rhythm`, `Stochastic`, `Chaos / Physics`, `Fractal`, `Cellular`, `Geometric`,
`Number Theory`, `Music Theory`, `Acid`.

nrum seq has no acid modes, so `Acid` is omitted. `Cantor` moves to `Fractal`; `Cellular` moves to
`Cellular`; `Eulerian`, `Phyllotaxis`, `Crystallographic`, `Hilbert` and `Ulam` move to
`Geometric`. Adding the candidates below would extend the existing groups rather than create new
ones, except where a candidate is intentionally rejected.

## 3. Fractal generation

### 3.1 What "fractal" means here

A fractal generator should use **self-similarity or a scale-dependent boundary** as the actual
mechanism, not merely a chaotic map with a pretty name. Three families are practical in a
1024-step, bounded-work snapshot:

1. **Escape-time fractals** — Mandelbrot, Julia, Burning Ship, Multibrot.
2. **Root-finding / basin fractals** — Newton, Nova.
3. **Iterated function systems and automatic sequences** — chaos game, paperfolding.

### 3.2 Candidates

| Candidate | Mechanic | nrum seq mapping | Cost | Recommendation |
|---|---|---|---|---|
| **Mandelbrot / Julia** | `z ← z² + c`, escape at `|z| > 2` | Step samples `c` (Mandelbrot) or `z₀` (Julia); escape band → lane/hit; smooth escape → accent | `steps × 64` complex iterations | **Wave 1** |
| **Newton basins** | `z ← z − p(z)/p'(z)`, `p(z)=z^d−1` | Root index → lane/degree; convergence time → accent; basin boundary → onset | `steps × 32` | **Wave 2** |
| **Feigenbaum / Lyapunov scan** | Scan `r`; λ from logistic orbit | λ sign changes / periodic windows → onsets; λ → accent | `steps × ~128` | **Wave 3** |
| **IFS / chaos game** | `p ← A p + b` from a map set | Occupied bin → lane; jump size/novelty → accent | `steps` iterations | **Wave 3** |
| **Paperfolding word** | `w → w 1 reverse(complement(w))` | `1` → hit; run length → accent | `steps` bit operations | **Wave 3** |
| **Mandelbulb / 3-D escape** | 3-D complex extension | Poor 8-lane fit, high cost | High | Reject |
| **Diffusion-limited aggregation** | Random walkers stick to a cluster | Visually strong but needs a grid and many walkers | High / noisy | Defer |

### 3.3 Mandelbrot / Julia — detailed design

**Algorithm**

```
z = z0
repeat up to N:
    z = z*z + c
    if |z| > 2: escaped at iteration n
```

For Mandelbrot, `z0 = 0` and `c` is the sampled point. For Julia, `c` is fixed and `z0` is the
sampled point.

- `N` is bounded: `N = 32 + 4 * pulses`, capped at 96.
- `|z|² > 4` is the escape test (avoids `sqrt` in the inner loop).
- Keep an escape guard for NaN/infinity and clamp the final value.

**Parameter mapping**

| Parameter | Mandelbrot | Julia |
|---|---|---|
| `pulses` | iteration cap / zoom depth | iteration cap |
| `density` | escape-band width / threshold | escape-band width |
| `phase` | imaginary coordinate of the scan row | angle of `c` on the unit circle |
| `seed` | horizontal offset / zoom centre | `z0` offset |
| `symmetry` | power `d = 2..5` (Multibrot) | power `d` |
| `ca_rule` | reserved for a named preset/zoom | reserved |

**Hit / lane mapping**

- Compute smooth escape time `μ = n + 1 − log₂(log₂|z|)` when escaped; use `N` for interior
  points.
- Normalise `μ / N` to `0..1`.
- Onset when the escape band changes between adjacent steps, or when the sampled point crosses
  the set boundary. A density threshold thins the selected bands.
- `accent = 0.55 + 0.45 * μ_norm`; interior points get the strongest accent or are treated as
  rests depending on `density`.
- Lane: `band mod TRACKS` for nrum seq. This gives a deterministic distribution across the eight
  lanes without a second random source.

**View options**

- **Cheap:** draw the sampled `c` / `z0` path and colour nodes by `μ`. Reuse `orbit`, `theta`,
  `u`.
- **Faithful:** store a 32×32 escape map in `Pattern` and draw it as a grid. This needs a new
  bounded field and a view-contract change; defer until the cheap view has been evaluated.
- Do not compute the full Mandelbrot image inside `view.rs`; that duplicates the generator.

**Risks**

- Interior points never escape; the cap must be honoured, not raised dynamically.
- The most interesting part of the set is a thin boundary. A line scan can miss it; `phase` and
  `seed` must offer a usable zoom/offset range.
- Julia `c` values outside the connected region produce dust; seed-derived presets should be
  tested for non-empty output at `steps = 1`.

### 3.4 Newton basins — detailed design

**Algorithm**

```
p(z)  = z^d - 1
p'(z) = d * z^(d-1)
z ← z - p(z)/p'(z)
```

Stop when `|p(z)| < ε` or `|Δz| < ε`, with a hard cap of 32 iterations. The `d` roots are the
`d`-th roots of unity, so the root index is a natural lane/degree selector.

**Parameter mapping**

- `symmetry` or `pulses`: degree `d = 3..8` (3 gives a triad-like set, 5 pentatonic, 7 diatonic).
- `phase`: starting angle / real offset.
- `density`: convergence threshold and onset thinning.
- `seed`: horizontal offset and starting radius.
- `ca_rule`: reserved for polynomial presets (`z^d−1`, `z^d−a`, Nova).

**Hit / lane mapping**

- Root index `k` → lane `k mod TRACKS`.
- Onset at basin changes or when the iteration count crosses a density-scaled threshold.
- Accent from iteration count and the final residual `|p(z)|`.
- View: basin colour strip or a 32×32 basin map; export the polynomial constants from the
  generator so the panel cannot drift.

**Why it is musically useful**

The roots are exact symmetry points. A phrase that walks between basins has a clear harmonic
gravity, and the boundary is genuinely fractal rather than decorative.

### 3.5 Feigenbaum / Lyapunov scan — detailed design

This is distinct from the existing Logistic mode, which fixes one `r` and plots one orbit. Here
`r` is scanned across the pattern:

```
r_i = r_min + (r_max - r_min) * i / (n - 1)
warm up x for 64 steps at r_i
λ_i = mean(ln |r_i * (1 - 2x)|) over 64 steps
```

- Onset at a sign change of λ, at a local minimum, or where the period estimate changes.
- Pitch/accent from `λ` or from the detected period.
- `density` sets the scanned `r` window; `pulses` sets the sample count; `phase` offsets the
  window.
- View: λ curve over `r`, with the sampled window marked. Reuse `orbit` for the curve.
- Cost is bounded and cheap. This is a strong "fractal" candidate because the bifurcation diagram
  is visually familiar without requiring complex arithmetic.

### 3.6 IFS / chaos game — detailed design

Use a fixed map set (Sierpinski triangle, Barnsley fern, dragon) selected by `ca_rule` or
`symmetry`. Start from a seeded point, apply one affine map per step with the deterministic
`Rng`, and record the point.

- Lane: quantised x/y bin.
- Onset: new bin, large jump, or a density threshold on bin occupancy.
- Accent: inverse local density (sparse regions are accented) or jump length.
- View: the point cloud and the active bin/point.
- Risk: the musical mapping is less direct than Mandelbrot/Newton; prototype the view first.

### 3.7 Paperfolding word — detailed design

The regular paperfolding sequence is generated by repeated folding:

```
w(0) = "1"
w(k+1) = w(k) + "1" + reverse(complement(w(k)))
```

Or, per index, `t(n) = 1` at powers of two and `t(n) = 1 − t(2^k − n)` for `2^(k−1) < n < 2^k`.

- `1` → hit; `0` → rest.
- Accent from run length or `v2(n+1)`.
- Pitch/contour from the index or a second delayed word.
- Very cheap, deterministic and self-similar. It also complements Thue–Morse and
  Rudin–Shapiro without adding a new parameter surface.

## 4. Geometric / sacred-geometry generation

### 4.1 Framing

"Sacred geometry" is treated here as a family of exact constructions: circles, tilings,
polyhedra, φ proportions and symmetric projections. The implementation should describe the
actual math; the help text can mention the traditional name, but the generator must not depend
on an arbitrary drawing.

Candidates already present or adjacent:

- Phyllotaxis (golden-angle rosette) — present.
- Crystallographic symmetry — present.
- Hilbert curve and Ulam spiral — present.
- Euclidean necklace — present but rhythmic rather than pictorial.

### 4.2 Candidates

| Candidate | Mechanic | nrum seq mapping | Cost | Recommendation |
|---|---|---|---|---|
| **Spirograph / Lissajous** | `x=cos(a t+δ)`, `y=sin(b t)` | y bucket → lane; extrema/crossings → onset; speed → accent | `steps` trig evaluations | **Wave 1** |
| **Platonic-solid walk** | Traverse vertices/edges/faces of a Platonic solid | Projected vertex → lane; edge/face traversal → onset | `steps` table lookups | **Wave 2** |
| **Penrose tiling** | P3 kite/dart substitution, walk tile edges | Orientation → lane; tile type → accent | Precomputed bounded tiling | **Wave 3** |
| **Truchet path** | Follow arc tiles on a grid | Turn/straight → onset; direction → lane | `steps` grid moves | **Wave 2/3** |
| **Flower of Life / hex rosette** | Hex lattice circle centres/intersections | Ring → lane; intersection → onset | Low | Optional view variant |
| **Metatron's Cube / Sri Yantra** | Literal diagram traversal | Weak, arbitrary mapping | Low | Defer as a mode; possible view-only art |
| **Islamic girih / star patterns** | Tessellation + strapwork | Strong visual, complex topology | Medium–high | Defer |
| **Voronoi / Delaunay** | Cell adjacency from seed points | Cell index → lane; boundary → onset | Medium | Defer |

### 4.3 Spirograph / Lissajous — detailed design

```
t_i = 2π i / n
x_i = cos(a t_i + δ)
y_i = sin(b t_i)
```

- `a` from `pulses`, `b` from `symmetry`, `δ` from `phase`.
- Onset at extrema (`dy/dt = 0`), zero crossings, or crossings of a radial grid.
- Lane: `floor((y + 1) / 2 * TRACKS)` clamped to `0..TRACKS-1`.
- Accent: local speed `sqrt((x_i−x_{i−1})² + (y_i−y_{i−1})²)` normalised.
- View: the curve plus active points; export the sampled path from the generator.
- Edge cases: `a = b` gives a line/circle; `a/b` rational gives a closed curve; irrational ratios
  fill the square. Both are musically useful, but the help text should explain which is which.

### 4.4 Platonic-solid walk — detailed design

Platonic solids and their `(V, E, F)` counts:

| Solid | V | E | F | Face type |
|---|---:|---:|---:|---|
| Tetrahedron | 4 | 6 | 4 | triangle |
| Cube | 8 | 12 | 6 | square |
| Octahedron | 6 | 12 | 8 | triangle |
| Dodecahedron | 20 | 30 | 12 | pentagon |
| Icosahedron | 12 | 30 | 20 | triangle |

All satisfy `V − E + F = 2`.

Implementation options:

1. **Vertex tour:** precompute a Hamiltonian cycle for each solid (all five have one) and step
   through it.
2. **Face tour:** walk adjacent faces; each face is a stable “chord” or lane group.
3. **Edge walk:** visit edges in a deterministic order; edge length/angle selects the accent.

Mapping:

- Solid selected by `symmetry` (or `pulses`).
- Projection: rotate the 3-D vertex set by `phase`, project to 2-D, scale into the 210-unit
  panel.
- Lane/degree: projected vertex index or face index.
- Onset: every vertex/edge traversal, thinned by `density`.
- View: projected edges, active vertex, optional face fill. Export the projection helper so the
  panel and generator share one formula.

Risks: the precomputed tours are fixed data and must be tested for every vertex; the mapping
from a 3-D symmetry to eight lanes is a design choice, not a mathematical necessity. Keep the
solid choice and tour in the generator, not in Slint.

### 4.5 Penrose tiling — detailed design

Use the P3 kite/dart substitution or the pentagrid method. For a bounded implementation:

- Generate a fixed number of substitution generations (e.g. 5–7) into a bounded tile array.
- Walk tile adjacency or sort tiles by a deterministic key (distance, angle, orientation).
- Lane: tile orientation (there are finitely many) or kite/dart type.
- Onset: tile boundary crossing; accent: tile area/edge length, which follows φ ratios.
- View: draw the rhombi/kites and the active tile.

This is the strongest “aperiodic geometry” candidate but also the most complex. Defer until the
simpler Spirograph/Platonic modes are proven.

### 4.6 Truchet path — detailed design

Two square tiles:

- tile A: two quarter-circle arcs joining opposite corners;
- tile B: the same arcs rotated 90°.

Pick a start tile and follow the arc to the next tile. Each step is one tile move.

- Onset on a turn or on every second tile; accent from run length.
- Lane from movement direction (`N/E/S/W`) or tile type.
- View: draw the arcs and the visited path.
- Cheap, visual and nrum seq-friendly. A strong alternative to Platonic Walk if the latter feels too
  abstract.

### 4.7 Flower of Life / hex rosette — detailed design

Place circle centres on a hexagonal lattice. Draw circles of radius equal to the lattice spacing;
intersections occur at 60° and 120°.

- Ring index → lane or octave.
- Intersection point → onset.
- Angle/radius → pitch or accent.
- View: circles plus highlighted intersection nodes.

This overlaps phyllotaxis visually. It is probably better as an alternate view or a palette
variant than as a separate mode, unless the traversal rule is clearly different.

## 5. Cellular generation

### 5.1 Current state

`src/sequence/cellular.rs` is a 1-D elementary CA:

- ring of `steps` cells;
- 8 neighbourhoods, rule `0..255`;
- 16 generations in `Pattern.ca`;
- lane `tr` reads generation `selected_row(p, tr)`;
- initial row seeded at `density`.

This is a solid base. The missing families are 2-D, ant-like and multi-state automata.

### 5.2 Candidates

| Candidate | Mechanic | nrum seq mapping | Cost | Recommendation |
|---|---|---|---|---|
| **Life-like 2-D CA** | `B/S` rules on a torus | Row/live count → lane; live cells → hits; neighbours → accent | `steps × grid²` | **Wave 1** |
| **Langton's Ant** | Turn/flip/move on a torus | Direction → lane; cell flip → onset; run length → accent | `steps` moves | **Wave 2** |
| **Cyclic CA** | `s → s+1` when a neighbour is `s+1` | State → lane/degree; wave front → onset | `steps × grid²` | **Wave 3** |
| **Radius-2 / totalistic 1-D** | 5-cell neighbourhood, 32-bit rule | Reuse Cellular with presets | Low | Extend existing UI first |
| **Brian's Brain / multi-state** | 3-state excitable medium | State → lane; excitation → onset | Medium | Defer |
| **Lenia / continuous CA** | Continuous state + kernel | Not a fixed-cell hit model | High | Reject |
| **3-D CA** | 3-D lattice | Poor fit for 8 lanes / 1024 steps | High | Reject |

### 5.3 Life-like 2-D CA — detailed design

Use a small torus, not a full-screen simulation:

- `W × H` with `W = 16`, `H = 8` (nrum seq lanes) or `16 × 16` (more spatial detail).
- One generation per pattern step, or a snapshot scan if the temporal mapping is preferred.
- Presets selected by `ca_rule`:
  - 0: Life `B3/S23`
  - 1: HighLife `B36/S23`
  - 2: Seeds `B2/S`
  - 3: Day & Night `B3678/S34678`
  - 4: Brian's Brain-style 3-state (if the state array is extended)
- `density` seeds the initial live fraction; `pulses` sets the warm-up generations; `phase`
  picks a start pattern.

**Temporal mapping (recommended)**

- Each generation is one pattern step.
- For each generation, collect live cells.
- nrum seq lane `tr` fires if row `tr` has at least one live cell; velocity/accent scales with the
  live count. This gives all eight lanes a chance and makes the rhythm follow population waves.
- Store a 16-column projection of the generation in `Pattern.ca[row][step]` for the view.

**Snapshot mapping (alternative)**

- Generate a `W × H` snapshot after a fixed warm-up and scan it row-major into the step list.
- Lane = `(x + y) mod 8`; hit = alive.
- This is cheaper to reason about but less temporal.

**Cost**

`steps × W × H` cell updates. At 1024 × 16 × 8 = 131k updates; benchmark before shipping. If it
is too expensive, reduce to 8×8 or update every second step. Do not allocate a grid; use fixed
arrays or a stack array.

**View**

Reuse `Pattern.ca` as a 16 × steps space-time strip. Draw live cells as nodes or blocks. For a
true 2-D snapshot, a new bounded bitmap field would be needed; defer that.

### 5.4 Langton's Ant — detailed design

Classic rule `RL`:

- on a white cell, turn right, flip to black, move forward;
- on a black cell, turn left, flip to white, move forward.

Generalise to a rule string such as `RLR`, `RLLR`, or `LRRL` selected by `ca_rule`.

- Torus grid, fixed size (e.g. 32×32; only the visited trail matters).
- One ant, or multiple ants for a denser texture.
- One pattern step = one ant move.
- Onset when the cell flips or when the ant turns; accent from straight-run length.
- Lane from direction (`N/E/S/W`) plus cell colour.
- Store the visited trail in `Pattern.ca` for the view.
- Cost is `O(steps)`; very cheap.

Risks: the classic ant has a long “highway” phase after ~10k steps, which is longer than a
1024-step pattern. Use a seeded start or a generalised rule to avoid a long featureless prefix;
test the first 1024 moves.

### 5.5 Cyclic CA — detailed design

States `0..k−1`; a cell in state `s` advances to `s+1` when at least one neighbour is in state
`s+1` (mod `k`). This produces spiral waves and rotating fronts.

- State → lane/degree.
- Excited state or state change → onset.
- `k` from `symmetry` or `pulses`.
- Needs a `u8` grid; `Pattern.ca` is boolean, so either store only “excited” cells or add a
  bounded state field. This is the main reason to defer it.

### 5.6 Radius-2 / totalistic 1-D CA

The current elementary CA uses a 3-cell neighbourhood and an 8-bit rule. A radius-2 CA uses five
cells per neighbourhood and a 32-bit rule. This would need a new rule parameter (or a preset
table) because `ca_rule` is `u8`.

Recommendation: add named preset chips to the existing Cellular mode first:

- 30 (chaos)
- 90 (Sierpinski)
- 110 (edge of chaos / computation)
- 150 (checkerboard)
- 184 (traffic)

This is a UI addition, not a new mode. It also gives the 2-D Life mode a natural place to share
preset UI.

## 6. Cross-project mapping summary

| Candidate | nrum seq (8 lanes) | nelody seq (mono voice) |
|---|---|---|
| Mandelbrot / Julia | Escape band → lane; smooth escape → accent | Escape time → `u`; basin/quadrant → `slots`; boundary → onset |
| Newton | Root index → lane; iterations → accent | Root index → degree; convergence → accent |
| Feigenbaum | λ/period → onset/accent | λ/period → interval/accent |
| IFS | Bin → lane; jump → accent | Point → pitch; bin novelty → onset |
| Paperfolding | Word → hit; run → accent | Word → degree; run → contour |
| Spirograph | y bucket → lane; extrema → onset | y → pitch; crossing → onset |
| Platonic walk | Projected vertex/face → lane | Projected vertex → degree; edge → interval |
| Penrose | Orientation → lane; tile type → accent | Orientation/type → pitch class |
| Truchet | Direction/turn → lane/onset | Direction/turn → contour |
| Flower of Life | Ring → lane; intersection → onset | Radius → pitch; intersection → onset |
| Life-like 2-D | Row/live count → lane; live → hit | Live cell → pitch; neighbour count → accent |
| Langton Ant | Direction → lane; flip → onset | x/y → pitch; turn → contour |
| Cyclic CA | State → lane; front → onset | State → degree; front → onset |

nelody seq has `mode_data: [f32; 4]` for extra knobs; nrum seq does not. Prefer reusing existing
parameters in both so the two implementations stay comparable.

## 7. Recommended waves

### Wave 1 — cheap, distinct, low contract risk

1. **Mandelbrot / Julia** (Fractal)
2. **Life-like 2-D CA** (Cellular)
3. **Spirograph / Lissajous** (Geometric)

Why: each has a clear visual identity, uses existing parameters, and can start with a cheap
view. None requires a new `Pattern` field if the first view is a contour/strip.

### Wave 2 — stronger musical structure

4. **Newton basins** (Fractal; especially strong for nelody seq)
5. **Langton's Ant** (Cellular)
6. **Platonic-solid walk** (Geometric)
7. **Truchet path** (Geometric; nrum seq first)

### Wave 3 — richer but more expensive

8. **Feigenbaum / Lyapunov scan** (Fractal)
9. **Penrose tiling** (Geometric)
10. **Paperfolding word** (Fractal sequence)
11. **Cyclic CA** (Cellular)
12. **Flower of Life** (Geometric; view variant first)

## 8. Rejected / deferred

| Idea | Reason |
|---|---|
| Mandelbulb / 3-D escape-time | High cost, hard to map to lanes/pitch, weak rhythmic identity |
| 3-D CA | Same; memory and work grow cubically |
| Lenia / continuous CA | Continuous state does not fit the fixed-cell hit model; expensive |
| DLA | Many walkers and a grid; stochastic output hard to make deterministic and cheap |
| Literal Metatron's Cube / Sri Yantra traversal | Attractive art, but no natural onset/traversal rule |
| Islamic girih | Beautiful but high implementation cost for the first wave |
| Voronoi / Delaunay | Useful, but the mapping is less distinct than the shortlisted modes |

## 9. Proofreading checklist

Before any candidate is promoted to a mode:

- [ ] The algorithm is stated with bounded loops and an explicit iteration/grid cap.
- [ ] `steps = 1`, density 0 and density 1 are defined.
- [ ] The generator is deterministic for `(seed, variation, params)`.
- [ ] No allocation, file I/O, locks or GUI calls in `generate`.
- [ ] Every output index is inside `0..steps`.
- [ ] The parameter mapping is documented in `REFERENCE_MAPPING.md`.
- [ ] The view reads published `Pattern` data or an exported generator helper.
- [ ] The view stays inside the 210-unit panel and does not produce 1024 heavy nodes.
- [ ] The mode is appended to `Mode` and to exactly one shared group.
- [ ] The help text explains the actual math, not only the traditional name.
- [ ] Worst-case cost is measured at `steps = 1024`.
- [ ] `generation_never_allocates` and the full mode sweep still pass.

## 10. Open questions for review

1. Do we want a **faithful 2-D fractal bitmap** (new `Pattern` field) or a contour/path view
   first?
2. Should **Mandelbrot and Julia** be one mode with a `symmetry`/`ca_rule` selector, or two
   modes?
3. For **Life**, is the temporal mapping (one generation per step) preferred over a snapshot
   scan?
4. For **Platonic Walk**, should the solid be selected by `symmetry`, `pulses`, or a new
   `ca_rule` preset?
5. Is **Fractal** allowed to remain an empty group until Wave 1 lands? The UI already omits
   empty groups.
6. Do we want `mode_data` in nrum seq before Wave 1, or is the current parameter reuse sufficient?

## 11. References

- Mandelbrot plotting / escape-time algorithms:
  <https://en.wikipedia.org/wiki/Plotting_algorithms_for_the_Mandelbrot_set>
- Fractal image and music generator (escape-time, distance, angle mappings):
  <https://www.wseas.us/e-library/conferences/2007athensel/papers/564-098.pdf>
- Newton fractal:
  <https://en.wikipedia.org/wiki/Newton_fractal>
- Penrose tiling / deflation:
  <https://en.wikipedia.org/wiki/Penrose_tiling>
- Regular paperfolding sequence:
  <https://en.wikipedia.org/wiki/Regular_paperfolding_sequence>
- Cellular automata in generative music (review):
  <https://web2.qatar.cmu.edu/~gdicaro/15382/additional/generative-music-ca-review.pdf>
- Langton's Ant:
  <https://en.wikipedia.org/wiki/Langton%27s_ant>
- MIT OCW, algorithmic and generative music lecture index:
  <https://ocw.mit.edu/courses/21m-380-music-and-technology-algorithmic-and-generative-music-spring-2010/pages/lecture-notes/>
