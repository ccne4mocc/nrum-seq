---
name: nrum seq
description: Algorithmic MIDI percussion with the mathematics visible on screen
colors:
  signal-yellow: "#fde047"
  ember-orange: "#f97316"
  alarm-red: "#dc2626"
  focus-cyan: "#38bdf8"
  chamber-deep: "#1b1010"
  chamber-warm: "#201113"
  chamber-cool: "#180e13"
  surface: "#221416"
  control: "#332024"
  control-hover: "#4a2c2f"
  text: "#f3e8e0"
  text-muted: "#b09a94"
  border: "#4a3134"
  cell-off: "#2a1a1c"
  on-accent: "#1a0d0d"
  lane-1: "#fde047"
  lane-2: "#f97316"
  lane-3: "#dc2626"
  lane-4: "#38bdf8"
  lane-5: "#f472b6"
  lane-6: "#f3e8e0"
  lane-7: "#ffa87a"
  lane-8: "#e0a020"
typography:
  display:
    fontFamily: "system-ui, sans-serif"
    fontSize: "24px"
    fontWeight: 700
    letterSpacing: "0.5px"
  title:
    fontFamily: "system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 700
  body:
    fontFamily: "system-ui, sans-serif"
    fontSize: "12px"
  value:
    fontFamily: "system-ui, sans-serif"
    fontSize: "11px"
  caption:
    fontFamily: "system-ui, sans-serif"
    fontSize: "10px"
  dense:
    fontFamily: "system-ui, sans-serif"
    fontSize: "9px"
rounded:
  tick: "1px"
  chip: "3px"
  control: "4px"
  action: "5px"
  handle: "11px"
spacing:
  hairline: "3px"
  tight: "5px"
  row: "6px"
  panel: "8px"
  section: "10px"
  gutter: "12px"
components:
  action-primary:
    backgroundColor: "{colors.focus-cyan}"
    textColor: "{colors.on-accent}"
    rounded: "{rounded.action}"
    typography: "{typography.value}"
    width: "112px"
    height: "30px"
  action-secondary:
    backgroundColor: "{colors.control}"
    textColor: "{colors.text}"
    rounded: "{rounded.action}"
    typography: "{typography.value}"
    width: "112px"
    height: "30px"
  chip-toggle:
    backgroundColor: "{colors.control}"
    textColor: "{colors.text}"
    rounded: "{rounded.chip}"
    typography: "{typography.caption}"
    height: "24px"
  slider-row:
    backgroundColor: "{colors.control}"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    typography: "{typography.value}"
    height: "25px"
  card-panel:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
  field-input:
    backgroundColor: "{colors.control}"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    typography: "{typography.value}"
    height: "25px"
---

# Design System: nrum seq

## Overview

**Creative North Star: "The Signal Observatory"**

nrum seq is a monitoring surface for an algorithm that is already running. The rosette, the
cobweb, the phase circle and the attractor path are the product; the chrome around them is the
instrument housing that lets you read and steer those drawings without competing with them. The
surface is dark not to look technical but because a dark chamber is what makes a plotted signal
legible: every bright mark on screen is data, and every dull mark is structure that must stay
quiet enough to ignore.

Density is the house style. The editor is a fixed 860x768 logical window holding a drawing, a
mode description, sixteen parameter rows, an eight-lane matrix and a status strip. Nothing
reflows; everything is placed. That constraint is why the type scale runs 9-14px and why weight
and tone carry hierarchy rather than size. Precision is the ambition here, not expression.

**Key Characteristics:**
- Dark chamber with one plotted signal and three accent hues doing distinct jobs.
- Fixed 860x768 coordinate space; no responsive breakpoints, only deliberate placement.
- Type named by job, not size, and dominated by 10-11px functional text.
- Flat surfaces; depth from tonal layering, with shadows reserved for overlays.
- Controls machined to the same spec as the drawings: slots, caps, ticks, 3-4px radii.

## Colors

One warm dark chamber, three signal accents, and a cold focus hue that never decorates.

### Primary
- **Signal Yellow** (#fde047): the first generator accent. Lane 1, the phyllotaxis rosette's inner
  band, active parameter fills, the mode header. It marks "this is the primary reading".
- **Ember Orange** (#f97316): the second accent. Lane 2, secondary data series, the in-progress
  state of a control or a warning that is not yet an error.
- **Alarm Red** (#dc2626): the third accent and the only saturated colour allowed to mean "stop".
  Lane 3, capped/overflow states, destructive intent. Used sparingly, never as decoration.

### Neutral
- **Chamber Deep** (#1b1010): the window behind everything. Never a panel fill.
- **Chamber Warm / Cool** (#201113, #180e13): alternate chamber tones that separate the drawing
  field from the control field without a border.
- **Surface** (#221416): cards, menus, drawers. One step above the chamber.
- **Control** (#332024): the resting fill of every interactive row and chip.
- **Control Hover** (#4a2c2f): the only hover fill. There is no second hover tone.
- **Border** (#4a3134): 1px separators and card edges. Deliberately low-contrast; it describes
  structure, not emphasis.
- **Text** (#f3e8e0) and **Text Muted** (#b09a94): the two text tones. Muted carries labels,
  headings and support copy; full text carries values and anything the user must act on.
- **Cell Off** (#2a1a1c): the resting matrix cell. It must be darker than Control so that hits
  read as bodies, not as fills.
- **On Accent** (#1a0d0d): the only text colour permitted on a saturated fill.
- **Focus Cyan** (#38bdf8): reserved for the focus ring, the playhead and the primary action. It is
  a state colour, not a brand colour.

### Lane Palette
Eight distinguishable hues, assigned per track index and never by user choice: yellow, orange,
red, cyan, pink, bone, apricot, ochre. They exist so an eight-lane drawing can be read without a
legend.

### Named Rules
**The One Reading Rule.** At rest, a screen carries one drawing in the accent hues and one primary
action in focus cyan. Everything else is neutral. Two saturated focal points means the hierarchy
is wrong.

**The Signal Not Decor Rule.** Saturated colour is data. If a saturated element does not encode a
value, a lane, or a state the user can act on, it is a bug.

## Typography

**Display Font:** system UI sans (no font asset ships; the family deliberately falls back)
**Body Font:** system UI sans
**Label Font:** the same face, differentiated by weight, case and 0.5px tracking

**Character:** A single system face at eight sizes, differentiated by weight and tone rather than
family. Headings are uppercase, bold and tracked; values are sentence case and full-tone; support
copy is muted. The wordmark is the same face at 24px/700 with 0.5px tracking — weight and spacing
carry it where a display face would normally sit.

### Hierarchy
- **Display** (700, 24px, 0.5px tracking): the wordmark only.
- **Title** (700, 14px): panel headings and the active generator name.
- **Body** (400, 12px): generator descriptions, empty states, settings explanations.
- **Value** (400, 11px): parameter values, option labels, matrix rows, primary action labels.
- **Caption** (400-700, 10px, uppercase when a heading): chips, controls, status text, column
  headers. The dominant functional size.
- **Dense** (400, 9px): metadata forced into tighter geometry — axis labels, four-digit badges.

### Named Rules
**The Two-Tone Rule.** A label and its value share a row and differ in tone, not in size. If a
value needs a larger type size to be found, the row is too wide.

**The 9px Floor Rule.** 9px exists for labels inside geometry that cannot grow (axis ticks, step
badges). A control label at 9px is a layout failure, not a type choice.

## Layout

A fixed 860x768 canvas. The top band holds the wordmark and transport on the left, the generator
panel on the right; a 300px drawing panel sits beside a parameter column; the matrix fills the
middle; the status strip closes the bottom. There are no breakpoints and no reflow: the host
supplies the window at exactly this size and the editor does not adapt below it.

Spacing follows a 3/5/6/8/10/12 rhythm. Three is the default gap inside a control, five to six
between controls in a row, eight to ten between panel groups, twelve at panel padding. Panels are
insets, never floating cards; the matrix is a card because it scrolls.

The generator panel is the only place that spends vertical space on prose: a mode header, one
description paragraph, and a one-line live formula. Everything else earns its height with data.

## Elevation & Depth

Depth is tonal, not shadowed. The stack is chamber → surface → control → control-hover, and that
is the entire depth vocabulary at rest. Shadows appear only on overlays that float above the
window, and they are always the same colour at two blur sizes.

### Shadow Vocabulary
- **Popover** (`drop-shadow-blur: 12px; drop-shadow-color: #00000055; drop-shadow-offset-y: 3px`):
  anchored menus, waveform pickers, the edit grid.
- **Drawer** (`drop-shadow-blur: 16px; drop-shadow-color: #00000055; drop-shadow-offset-y: 4px`):
  the settings panel and any edge-anchored surface.
- **Scrim** (`#00000044` popover, `#00000066` drawer): the dimming layer under a modal surface.
  A popover that does not block the panel uses the lighter scrim or none.

### Named Rules
**The Flat-At-Rest Rule.** No surface inside the panel carries a shadow. If a control appears to
float, it has been over-styled.

## Shapes

Corners are cut to the size of the thing they wrap: 1px for ticks and hairline marks, 3px for
chips and matrix cells, 4px for parameter rows, inputs and cards, 5px for the two primary actions,
11px for the round LFO drag handle. Nothing is fully rounded except a circular drag point.

Borders are 1px everywhere and always low-contrast. A control is identified by its fill, never by
an outline; the border only closes the edge against the chamber. Clipping is used on parameter
rows so the fill bar cannot escape the row's radius.

The recurring silhouette is the horizontal slot: a wide, short rectangle with a fill that grows
from the left. It appears as a parameter row, a matrix row, a history bar and a chip. That
repetition is the family's shape signature.

## Components

### Buttons
- **Shape:** 5px radius, 30px high, 112px wide for the two actions.
- **Primary:** focus cyan fill, on-accent text, 11px/700. There is exactly one on screen:
  GENERATE.
- **Secondary:** control fill, full text, 11px/400 at the same size. RANDOMIZE sits beside the
  primary and is deliberately quieter.
- **Hover / Focus:** hover swaps to control-hover and, for the primary, draws a 2px accent border.
  Focus is a 1px ring in focus cyan, inset by 3px.

### Chips
- **Style:** 24px high, 3px radius, control fill, caption text. Toggle chips fill with the state
  colour at 35-80% alpha and switch text to on-accent.
- **State:** off is neutral fill with full text; on is a tinted fill with on-accent text. A chip
  never uses a border to show state.

### Cards / Containers
- **Corner Style:** 4px.
- **Background:** surface, one step above the chamber.
- **Border:** 1px border at low contrast.
- **Internal Padding:** 5-10px, never more than 10 inside the panel.
- **Shadow Strategy:** none at rest; see Elevation.

### Inputs / Fields
- **Style:** control fill, 1px border, 4px radius, 25px high, value text right-aligned.
- **Focus:** the row itself receives a 1px focus-cyan ring; the text entry replaces the row
  interior rather than sitting beside it.
- **Error / Disabled:** muted text at 55% alpha; no dedicated error style.

### Parameter Row (signature)
The family's defining control: a 25px slot with a left-anchored fill, the label at the left inset
and the value at the right, both on top of the fill. It is operated by press, drag and wheel, and
its value can be typed by double-click or Enter. The current generation renders the fill at 30%
opacity with a three-stop gradient; that is the component's known weak point and the first target
of the redesign.

### Matrix Cell
A 3px-radius cell whose height encodes velocity, width encodes gate, opacity encodes probability
and internal segments encode ratchet. It is the one component where the drawing and the control are
the same object.

## Do's and Don'ts

### Do:
- **Do** keep saturated colour for data and state only; the neutral stack carries the structure.
- **Do** place controls at their final position in the fixed 860x768 space rather than adding
  breakpoints or scroll regions to the main panel.
- **Do** use 3-4px radii and 1px low-contrast borders; the surface should read as a panel, not a
  set of cards.
- **Do** reserve 12px shadows for popovers and 16px for drawers, with the single shadow colour.
- **Do** let one drawing and one action own the accent hues at rest.

### Don't:
- **Don't** use gradients as fills, glows, or glass; the only gradient permitted is the three-stop
  accent fill inside a parameter row, and it is being retired.
- **Don't** build a dark web dashboard: no purple, no translucent cards, no pill-shaped
  everything, no emoji as iconography.
- **Don't** imitate a generic DAW: no brushed metal, no knobs, no tabbed preferences, no stock
  widget styling.
- **Don't** build a sci-fi HUD: no scanlines, no neon wireframes, no hexagonal grids, no telemetry
  animation for its own sake.
- **Don't** raise functional labels above 11px to create hierarchy; use weight and tone instead,
  and if that fails the layout is wrong.
