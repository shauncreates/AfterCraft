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

The native edition preference selects the first eligible effect mapping. Installed
Adobe descriptors can retain Adobe effect identities; corresponding Sapphire OFX IDs
are selected from the registered catalog. Other vendors require a known match-name
mapping or a usable Adobe registration. Installing a similarly named effect is not
enough to establish equivalent behavior.

Parameters map through explicit IDs/paths and unique semantic names. Cross-edition
scalar units remain unsupported unless their correspondence is known. For example,
Sapphire AE Wipe Percent (0–100) maps to OFX Wipe Amount (0–1); both values and temporal
ease speeds scale by 0.01. Popup options must correspond in order. Point, angle,
boolean and color controls use their documented scripting types. Unsupported controls
are reported and retained in the source record.

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

Source/catalog/report sizes, tree depths, property/key counts, diagnostic output and
external conversion time are bounded. Projects over a bound fail explicitly. The
external conversion deadline is 120 seconds.

Native desktop builds need Python with py-aep 0.18.0. `EFFECTCRAFT_AEP_PYTHON` selects
the interpreter, `EFFECTCRAFT_AEP_SITE_PACKAGES` selects a private dependency directory,
and `EFFECTCRAFT_AEP_SCRIPT` can select a converter for development. Packaged converter
files live in `aep-import/` beside the executable. The test-copy launcher supplies its
private runtime paths. See [converter provenance](../tools/aep-import/PROVENANCE.md).
