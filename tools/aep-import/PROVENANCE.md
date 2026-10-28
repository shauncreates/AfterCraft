# AE project import provenance

`_base.py`, `aepx.py`, and the built-in effect match-name table derive from the
MIT/Apache-2.0 EffectCraft fork at https://github.com/glumbis/effectcraft,
commit `f5a35251689ab8978b6b0af2be7f542b74d995d0`. The wrapper, bounds,
semantic mappings, preservation report, and tests were added for this experimental
AfterCraft build. The original license texts are included in this directory.

Parsing uses the unmodified MIT `py-aep` 0.18.0 package, bundled in the app-private
`runtime/python/Lib/site-packages`. Its license and dependency
licenses remain in the installed `.dist-info` directories. This converter does
not contain Adobe implementation code, vendor plugin binaries, or user projects.

The generated import script uses EffectCraft's own scripting interface. AE/OFX
effect mappings translate project data; they do not convert plugin executables,
transfer activation, or establish pixel-identical behavior. Unsupported data and
the source project are retained in the per-import report directory.

The Sapphire transition mapping follows the vendor's documented AE Wipe Percent
and OFX Wipe Amount ranges, with corresponding scaling of temporal ease speeds:
https://borisfx.com/documentation/sapphire/ae/wipeline/ and
https://borisfx.com/documentation/sapphire/ofx/wipeline/ . Runtime range metadata
is checked before applying the percentage conversion.

`builtin_maps.py` describes source project match IDs and original engine parameter
paths. It does not contain code from commercial effects. The selected popup choice
is matched by label, and stored Fill/Drop Shadow opacity ranges are checked before
normalizing to the built-in's percent units. See `docs/aep-builtin-mapping.md` for
scope and unsupported controls. Channel Combiner's channel replacement behavior
follows Adobe's public documentation:
https://helpx.adobe.com/ca/after-effects/desktop/apply-effects-and-animation-presets/list-of-effects/channel-effects.html .
