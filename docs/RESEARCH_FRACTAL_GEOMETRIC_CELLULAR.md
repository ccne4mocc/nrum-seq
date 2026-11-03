# Future mode research: fractal, geometric and cellular

Status: **backlog only — not a roadmap.** Life-like 2-D CA is implemented as mode 39. Nothing else
in this document is scheduled or implemented. The purpose of this file is to preserve the design
space, constraints and source links for a later mode wave.

The companion document in nelody seq carries the same candidate review with melodic mappings.

## Scope and constraints

The current mode set already covers the obvious cases:

- **Fractal / self-similar:** Cantor set, Hilbert curve, logistic/tent/Hénon/Lorenz maps and
  automatic sequences such as Thue–Morse and Rudin–Shapiro.
- **Geometric:** Euclidean ring, phyllotaxis, crystallographic symmetry, Hilbert grid, Ulam spiral.
- **Cellular:** a 1-D elementary Wolfram automaton; Life-like 2-D CA is now implemented.
- **Stochastic / physics:** Markov, Bernoulli, genetic mutation, Brownian, Lévy, Pebble Box,
  Kuramoto and Reaction–Diffusion.

The remaining gap is specific: there is no escape-time fractal mode, no Newton basin mode, no
Langton's Ant, no cyclic/multi-state CA, and no parametric / polyhedral geometry mode.

Constraints for any future candidate:

1. Output stays deterministic for `(seed, variation, params)`.
2. The generator is allocation-free in `generate()` and `process()`.
3. Fixed-size state; no unbounded grids, recursion or iteration counts.
4. `steps = 1`, density 0 and density 1 must be defined and tested.
5. New modes append to the enum; existing host indices never move.
6. A mode-specific visualisation reads `Pattern` or a generator helper, not a second copy of the
   formula.

The shared control surface (`pulses`, `density`, `phase`, `seed`, `ca_rule`, `symmetry`,
`osc_rate`) is usually enough. A candidate that needs more than two independent controls should
wait for a typed mode-parameter contract.

## Candidate shortlist

| Candidate | Mechanic | nrum seq mapping | Cost | Recommendation |
|---|---|---|---|---|
| **Mandelbrot / Julia** | `z ← z² + c`, escape at `|z| > 2` | Step samples `c` or `z₀`; escape band → lane/hit; smooth escape → accent | `steps × 64` complex iterations | Wave 1 |
| **Newton basins** | `z ← z − p(z)/p'(z)`, `p(z)=z^d−1` | Root index → lane; convergence time → accent; basin boundary → onset | `steps × 32` | Wave 2 |
| **Feigenbaum / Lyapunov scan** | Scan `r`; λ from the logistic orbit | λ sign changes / periodic windows → onsets; λ → accent | `steps × ~128` | Wave 3 |
| **IFS / chaos game** | `p ← A p + b` from a map set | Occupied bin → lane; jump size/novelty → accent | `steps` iterations | Wave 3 |
| **Paperfolding word** | `w → w 1 reverse(complement(w))` | `1` → hit; run length → accent | `steps` bit operations | Wave 3 |
| **Spirograph / Lissajous** | Two rotating phasors | y bucket → lane; extrema → onset | Low | Wave 1 |
| **Platonic-solid walk** | Walk vertices/edges of a solid | Projected vertex/face → lane | Medium | Wave 2 |
| **Penrose tiling** | Deflation/substitution | Orientation → lane; tile type → accent | High | Wave 3 |
| **Truchet path** | Random tile orientation, trace a path | Direction/turn → lane/onset | Low | Wave 2 |
| **Flower of Life / hex rosette** | Concentric circle intersections | Ring → lane; intersection → onset | Low | Wave 3 |
| **Langton's Ant** | Turn/flip/move on a torus | Direction → lane; cell flip → onset; run length → accent | `steps` moves | Wave 2 |
| **Cyclic CA** | `s → s+1` when a neighbour is `s+1` | State → lane; wave front → onset | `steps × grid²` | Wave 3 |
| **Radius-2 / totalistic 1-D CA** | 5-cell neighbourhood, 32-bit rule | Reuse Cellular with presets | Low | Extend the existing UI first |

## Already implemented

Life-like 2-D CA shipped as mode 39: a 16×8 torus, one generation per step, row population mapped
to lane hits. The detailed design that was here is preserved in git history and in
[`REFERENCE_MAPPING.md`](REFERENCE_MAPPING.md).

## Recommended waves

**Wave 1 — cheap, distinct, low contract risk**

1. Mandelbrot / Julia (Fractal)
2. Spirograph / Lissajous (Geometric)

**Wave 2 — stronger musical structure**

3. Newton basins (Fractal)
4. Langton's Ant (Cellular)
5. Platonic-solid walk (Geometric)
6. Truchet path (Geometric)

**Wave 3 — richer but more expensive**

7. Feigenbaum / Lyapunov scan (Fractal)
8. Penrose tiling (Geometric)
9. Paperfolding word (Fractal sequence)
10. Cyclic CA (Cellular)
11. Flower of Life (Geometric; view variant first)

## Rejected or deferred

| Idea | Reason |
|---|---|
| Mandelbulb / 3-D escape-time | High cost, awkward mapping to eight lanes, weak rhythmic identity |
| 3-D CA | Memory and work grow cubically |
| Lenia / continuous CA | Continuous state does not fit the fixed-cell hit model; expensive |
| DLA | Many walkers and a grid; hard to keep deterministic and cheap |
| Literal Metatron's Cube / Sri Yantra traversal | Attractive art, but no natural onset/traversal rule |
| Islamic girih | Beautiful but high implementation cost for the first wave |
| Voronoi / Delaunay | Useful, but less distinct than the shortlisted modes |

## Proofreading checklist

Before a candidate is promoted to a mode:

- [ ] The algorithm has bounded loops and an explicit iteration/grid cap.
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

## Open questions

1. Should Mandelbrot and Julia be one mode with a selector, or two modes?
2. For geometric modes, is the projected 2-D path enough, or does a panel need a new bitmap field?
3. Should Fractal remain an empty group until the first fractal mode lands? The UI already omits
   empty groups.
4. Do future modes reuse the shared knobs, or wait for a typed mode-parameter contract?

## References

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
