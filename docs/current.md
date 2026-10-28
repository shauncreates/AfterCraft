# Current easing editor

Open **Window > Current** and select adjacent keyframes in the Timeline or Graph Editor.
Choose **Bezier**, **Spring**, **Elastic**, **Bounce**, or **Standard**. The graph and motion preview show
normalized time and value; changing a control previews the curve until you apply it.

## Bezier easing

Drag the two handles or edit X1, Y1, X2 and Y2. Shift-drag snaps to increments of 0.05;
Alt-drag mirrors the opposite handle. The graph expands to show overshoot.
**Apply to selected keys** changes easing without moving the keys, their values, or spatial
motion paths. Each selected adjacent pair gets the same curve; unselected intervals stay intact.
**Reverse** reverses the easing direction. **Read selected keys** reads the first selected
interval's first changing dimension. Different per-dimension curves cannot collapse to one curve.

**Copy** writes `cubic-bezier(x1, y1, x2, y2)`. Paste those values, or four comma-separated
numbers, into the values field and click **Use values**.

## Spring, Elastic and Bounce

These modes create intermediate native keyframes rather than approximating an oscillation
with one cubic curve. **Frequency** controls oscillations per selected interval, **Damping**
controls decay, and **Amplitude** blends from a straight ramp to the motion response.
An amplitude of zero is linear. Spring and Elastic can overshoot; Bounce stays between the
endpoint values. The functions are original normalized mathematical models, not copied presets.

**Samples/frame** chooses one to eight samples per composition frame. Click **Bake motion
between selected keys**. The original endpoint times and values remain intact. Reapplying
controls during the same uninterrupted refinement session replaces the generated samples and
keeps one undo step. Undo restores the original keys. Changing the project, key selection,
active composition, or history invalidates that refinement session; it cannot overwrite
unrelated edits. Counts and allocations are bounded, and overly large operations report an error.

Physics baking currently supports finite scalar and nonspatial numeric vector properties.
Separate Position dimensions before applying motion. Spatial paths, colors, shapes, gradients,
hold-only controls, and properties with enabled expressions report unsupported use rather than
silently changing their meaning.

## Preview and library

**Play preview** moves a marker along the response independently of the project playhead.
Pause, scrub the preview time, or Reset it. Playback requests up to 30 repaints per second only
while the panel is visible and playing; it never renders project footage or edits keyframes.
**Compact** reduces the graph and preset thumbnails for a narrow dock.

Preset thumbnails show the actual response. Click to load a graph; double-click to apply it.
The **Defaults** list is read-only. It includes the original Current curves, Spring/Elastic/Bounce
starting points, and the standard families described below. **Find a graph** filters the current
list without changing project data.

Use **List name > New list** to create your own collection. Select it in **Graph lists**, name the
current graph and click **Save graph**. Graph names are local to each list, so different lists may
contain graphs with the same name. Saving an existing name updates that graph. **Rename** changes
the selected custom list's name. **Delete list...** asks for confirmation because it deletes that
list and its saved graphs; **Delete graph** removes only the selected saved graph. To copy a default,
select its thumbnail, switch to a custom list and save it there.

Lists live in AfterCraft's `current-graph-lists.json`. The library supports 32 custom lists,
128 graphs per list, 4,096 graphs in total and 2 MiB of serialized JSON. Names contain 1–80 printable
characters and are unique without case differences within their scope. On first access, old
`current-curves.json` and `current-physics-curves.json` files migrate into separate lists. Original
files remain untouched; conflicting old graph labels receive numeric suffixes. Library operations
never change project undo history.

## Standard easing families

**Standard** offers Linear, Ease, Quad, Cubic, Quart, Quint, Sine, Expo, Circ and Back. Choose **In**,
**Out** or **In / Out**. In Defaults, the family name alone means In / Out; the other graphs have
In or Out in their name. Quad through Quint evaluate the exact powers of normalized time before
sampling. Sine, exponential, circular and Back curves use their mathematical responses rather than
an approximation with one cubic curve. Back can overshoot.

Current's Ease convention starts and/or ends at rest: Ease uses `[1/3,0,2/3,1]`, Ease In uses
`[1/3,0,2/3,2/3]`, and Ease Out uses `[1/3,1/3,2/3,1]`. These are original normalized mathematical
conventions. **CSS Ease** is separately named and uses `[0.25,0.1,0.25,1]`.

Standard modes bake intermediate keys through the same protected refinement and undo workflow
as physics modes. Samples match the selected formula; playback between samples is piecewise linear.
Use a higher Samples/frame setting when a sharper curve needs more temporal detail.

## Import and export

**Export...** writes the selected list as a `.current` file (Current JSON). Exporting Defaults produces an editable **Defaults Copy** list. **Import...**
accepts a single-list `.current`, legacy Current JSON or `.flow` library and creates a list named after the file without replacing existing collections. Conflicting list
names receive numeric suffixes. Invalid, oversized or unsupported data leaves the stored library
unchanged. Current JSON preserves Bezier, standard and physics settings without executing code.

The versioned interchange envelope is:

```json
{"format":"current.graph-lists","version":1,"lists":[{"name":"Custom","graphs":[{"name":"Drift","config":{"mode":"bezier","curve":[0.3,0,0.7,1],"frequency":3,"damping":6,"amplitude":1,"density":1,"family":"quad","direction":"inout"}}]}]}
```

Enable **Flow (.flow)** for Bezier-only Flow data export from one list. Non-Bezier graphs are skipped with their names and count clearly reported. If no Bezier graphs
are available, export reports an error and writes no empty file. Use `.current` to preserve mixed
graph types without omissions. The Flow adapter accepts UTF-8 JSON arrays of records with
`name` and `value`, older `curve` records, and legacy name-to-curve objects. Coordinates may be
four numeric array entries or a comma-separated numeric string. Optional `_id` values are ignored;
exports omit them so the receiving panel can create its own IDs. Each Flow import creates one list
of at most 128 graphs, renaming repeated graph labels to preserve every entry.

Flow tests use original synthetic data. A round trip through an actual Flow host remains unverified.
No vendor preset pack, expressions, plugin executables or licensing data are bundled. This interchange
is separate from After Effects `.ffx` animation preset support.

**Live apply** is optional and starts off in each new session. Once enabled, control changes
apply after the gesture ends and has settled for 250 milliseconds. Playback, library loading,
and enabling the checkbox do not themselves edit the project. An apply error turns Live apply
off and shows its reason. Physics refinements reuse the protected refinement session described
above; they do not add keys on every repaint.

Current is an independent native implementation informed by the general curve workflow
[described by Flow's developers](https://aescripts.com/flow/) and the public feature descriptions
of [MFlow](https://codeberg.org/MaaruAx/MFlow). No Flow or MFlow source, artwork, preset packs,
licensing components or CEP runtime are included. Their implementation code and expressions are not imported. The bounded Flow JSON adapter above
translates user-selected curve data only. MFlow's additional Catenary, Pulse, Noise, Resonance and multisegment modes
are not implemented here.

Bezier time handles stay within 0–1; finite value handles can range from -9 to 10. A vertical
endpoint handle with a nonzero value offset needs infinite velocity. Applying it uses a small
0.001 / 0.999 time-handle adjustment and reports that approximation. Library storage, export and
preview preserve the original coordinates exactly; one Undo restores the previous project easing.
Spatial Position curves have bounded endpoints: separate dimensions before applying overshoot.
Locked layers and nonadjacent selections report an actionable error or have no applicable interval.
