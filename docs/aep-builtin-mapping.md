# Built-in effects during AEP import

AfterCraft includes its own screen-colour keyer, **Key Light**, by default. Search
Effects & Presets for `Keylight` or `Key Light`; its effect ID is
`ec.keying.keylight`. The original implementation accepts the `Keylight (1.2)`
name for convenience. Importing `Keylight 906` selects this built-in instead of
requiring the commercial Adobe plugin. It is an original screen-difference keyer,
not the vendor's Keylight binary, and matching controls do not establish identical
pixels for every shot.

## Stable identities and units

An effect's display name, or a parameter's display name, is not its identity.
Shatter has two Position, Depth, Radius and Strength controls; Key Light has
several Replace Method, Invert, Saturation and Contrast controls. The translator
uses explicit source match IDs to select the appropriate engine parameter path.
These paths also avoid failures caused by localized or renamed labels.

The explicit tables cover Key Light, Shatter, Bass & Treble, Fill, Drop Shadow,
Motion Tile, Turbulent Displace, Transform, Solid Composite, Channel Combiner,
Tint and Linear Wipe. Other effects may use their existing verified identity or
unique semantic name mapping. An identity match alone does not prove unit or
renderer equivalence.

Stored project units may differ from the units displayed in Effect Controls:

| Source control | Stored bounds | Built-in bounds | Conversion |
| --- | --- | --- | --- |
| Fill > Opacity | 0..1 | 0..100 | multiply by 100 |
| Drop Shadow > Opacity | 0..255 | 0..100 | multiply by 100/255 |
| Key Light > Source Crops > Right / Bottom | boundary position 0..100 | removed amount 0..100 | 100 minus the source value |

These conversions require the expected source bounds. Already normalized
0..100 values retain their scale; unknown bounds are reported as unsupported.
Key values and temporal ease speeds use the same scale, preserving the animated
curve's units instead of fixing only its static value.
For Right/Bottom crop boundaries, the offset applies only to values; ease speeds
are multiplied by -1. This preserves a full-frame boundary at 100 as zero removed
pixels instead of cropping the whole image. The distinction follows the crop
position controls in Foundry's public
[Keylight on After Effects user guide](https://thefoundry.s3.amazonaws.com/products/keylight/ae/documentation/Keylight_1.2v12_AE.pdf).

## Popup menus

Menus can contain separators, different option orders, or choices the built-in
does not implement. Each selected source option is translated by its unique label
to a target scripting index. Every animated key must have a supported choice.
Unknown, ambiguous, separator and invalid numeric selections are preserved in
the report instead of changing into a convenient default.

For example, Shatter's Glass selection maps to the built-in Glass pattern even
though the source menu has many more patterns. The built-in implements Bricks,
Glass, Hexagons, Squares, Triangles and Custom; other source patterns remain
unsupported when there is no exact corresponding option. Its two force radii
map independently, including their keyframes. Camera Position and Corner Pins
also retain separate Focal Length controls.

Channel Combiner distinguishes **Alpha** from **Alpha Only**: Alpha replaces the
alpha channel while keeping the source colour; Alpha Only produces a white matte.
The built-in's appended RGB/HLS channel replacements similarly retain the other
channels. Existing option indices remain compatible with saved native projects.
These additional channel replacements use the CPU until the GPU kernel supports
the same behavior. The distinction follows Adobe's public
[Channel effects documentation](https://helpx.adobe.com/ca/after-effects/desktop/apply-effects-and-animation-presets/list-of-effects/channel-effects.html).

## Limits

Unsupported Shatter patterns, cached Roto Brush stroke/matte data, and effects
without a corresponding implementation retain their source data and warnings.
Wiggle Position imports as two original expression controls with its source instance
name, labels and control order. An adjacent position expression must consume those
controls to produce motion; the controls themselves pass pixels through. Import reports describe missing and unsupported
properties; a successful file import does not mean every original effect renders.

See [AEP import and preservation](aep-import.md) for the full import workflow.
