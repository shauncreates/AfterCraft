# Source provenance

AfterCraft is an independent modified distribution, under MIT OR Apache-2.0.

The core starts at `storytold/effectcraft` tag `v0.16.0`, commit
`9581b53c3f8945683252af0eb5fac79e0e43aed4`. Original licenses, notices and openly licensed
assets are retained. ArtCraft's restricted marks in `docs/brand/` are excluded and replaced
by original AfterCraft artwork in `assets/aftercraft/`.

`native/ofx-helper/` derives from `glumbis/ec-extension-ofx` commit
`63059c6eb231a251ab01c4d62ff7e89050ca3a17`. Individual licenses and locked helper dependencies
are retained there. OpenFX declarations carry their BSD license.

The project translator includes code adapted from `glumbis/effectcraft` commit
`f5a35251689ab8978b6b0af2be7f542b74d995d0` and uses the unmodified MIT `py-aep` 0.18.0
package. See `tools/aep-import/PROVENANCE.md` for mapping and preservation details.

The Adobe helper uses `after-effects-sys` 0.4.0's public ABI declarations. Its gain fixture
is original. No Adobe application code, commercial plugins, activation files, user projects
or user media are distributed.

Changes include isolated native-plugin hosting/installation, bounded temporal callbacks,
edition preference/filtering, transactional AE project import, parameter/keyframe translation,
resource admission limits and independent product packaging. Full AE/vendor conformance is unverified.
