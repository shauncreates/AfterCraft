# Experimental After Effects project import

File > Import > After Effects Project accepts `.aep` and `.aepx`. It reads the project
through an external MIT-licensed py-aep parser, generates a script against EffectCraft's
own scripting API, and builds a staging document. A successful staging document merges
into the open project in one undo step, with IDs and references remapped. A fatal parser
or script error leaves the open project intact.

The translator attempts folders, compositions, used footage, layer order and timing,
parenting, transforms, masks, supported text/shape properties, 3D layer switches,
cameras/lights, expressions, and keyframe interpolation/easing/spatial tangents. These
are structural translations; they do not establish matching rendered output. Missing
or unsupported media becomes an explicit placeholder, preserving its layer and timing.

## Effects and animation

Known original built-ins are selected first. For example, `Keylight 906` maps to
AfterCraft's included **Key Light**, `ec.keying.keylight`, without requiring an Adobe
plugin or its activation. For other native effects the edition preference selects
the first eligible mapping. Installed Adobe descriptors can retain Adobe effect
identities; corresponding Sapphire OFX IDs are selected from the registered catalog.
Other vendors require a known match-name mapping or a usable Adobe registration.
Installing a similarly named effect is not enough to establish equivalent behavior.

Parameters map through explicit IDs/paths and unique semantic names. Adobe native
registrations retain SDK disk parameter IDs, so a parameter's source identity can
select its current engine path without relying on its display label or scan order.
Built-in match-ID tables distinguish Shatter's two forces, Key Light's duplicate
controls and other first-party controls. See [built-in mappings](aep-builtin-mapping.md).

Cross-edition units require declared semantics. Sapphire AE Wipe Percent (0–100)
maps to OFX Wipe Amount (0–1); values and temporal ease speeds scale by 0.01.
Native metadata identifies normalized OFX distances, which use the source layer's
width or height to translate AE pixels. Legacy scalar X/Y point controls receive
normalized coordinates with the appropriate Y origin. Animated spatial point curves
remain unsupported where preserving their spatial interpolation needs further work.
Point, angle, boolean and color controls otherwise use their documented scripting
types. Scalar normalization also checks known built-in storage ranges: Fill opacity
can be 0–1 and Drop Shadow opacity 0–255 while the built-ins use percent.
Key Light's Right/Bottom source crop boundaries translate to removed amounts with
`100 - source`; values receive the affine offset and ease speeds receive only the
signed scale, so an unchanged full-frame boundary does not crop the entire image.

Popup choices are matched by unique label and reindexed individually. Menus may
contain separators, different orders or a supported subset; the full menus need
not be identical. Every selected animated key must translate before any of that
property is applied. Unknown choices, unverified units and unsupported controls are
reported and retained in the source record instead of guessed.

Easing is applied to supported properties after values and key times. Stored spatial
tangents and resolved times are preferred over recomputing automatic/roving curves.
Scripted layer timing and key insertion retain subframe times; ordinary interactive
engine calls still snap by default. Generated time-remap endpoints are removed before
the source keyframe curve is installed.
In/out interpolation types are restored after handle setters, preserving keys that mix
a linear side with a Bezier side. Layer trim windows expand before source trims are
applied, avoiding clamps against the newly created layer's old out point.
Expressions are copied as text; the converter does not execute them. Their execution
and effect references still depend on EffectCraft supporting the relevant APIs.

## Preservation and reports

Each import retains a private `AepImports/import-*/` folder containing the input copy,
generated script, catalog and complete JSON report. The report includes source hashes,
decoded original properties/keyframes, mapping decisions, actual created counts,
runtime warnings and unsupported data. Smaller source documents also have an exact
base64 archive in the report. The input copy always retains the original bytes.

Each mapped effect records `coverage.mapped`, `coverage.unmapped` and
`coverage.unmappedChanged` parameter counts. Its status is `partial` when any
parameter could not translate, rather than claiming complete support from an effect
identity alone. Unmapped controls can include unchanged or host-only settings;
`unmappedChanged` identifies modified or animated source properties that remain
unsupported. An unavailable effect has no target implementation. None of these
metadata counts establish pixel-identical rendering; verify the imported frames.

Command replies summarize this report and omit the large source archive/snapshot.
Use the reported artifact directory to inspect the full record. Original AE project
files are never overwritten. A preservation record does not make an unsupported effect
or property editable or renderable.

## Current limitations

Full AE project/render parity is not implemented. Proprietary custom controls, cached
Roto Brush strokes/mattes, some gradients/curves and layer styles, Essential Properties,
tracker data, complex expression dependencies, plugin custom UI, advanced 3D renderers
and model plugins such as Element 3D require further support. Review every import's
warnings before relying on its render. Plugin activation remains the vendor's normal
responsibility; translating a project does not convert plugin binaries or activation.

Importing Adobe `.ffx` animation presets is deferred until AEP effect translation is
reliable. `.ecpreset` remains the supported native preset format; the project importer
does not currently treat an `.ffx` file as a preset or a project.

Source/catalog/report sizes, tree depths, property/key counts, diagnostic output and
external conversion time are bounded. Projects over a bound fail explicitly. The
external conversion deadline is 120 seconds.

Native desktop builds need Python with py-aep 0.18.0. `EFFECTCRAFT_AEP_PYTHON` selects
the interpreter, `EFFECTCRAFT_AEP_SITE_PACKAGES` selects a private dependency directory,
and `EFFECTCRAFT_AEP_SCRIPT` can select a converter for development. Packaged converter
files live in `aep-import/` beside the executable. The test-copy launcher supplies its
private runtime paths. See [converter provenance](../tools/aep-import/PROVENANCE.md).
