# AfterCraft OpenFX helper

This separate native process hosts CPU OpenFX effects for AfterCraft. It derives from
[ec-extension-ofx](https://github.com/glumbis/ec-extension-ofx); see PROVENANCE.md and
ATTRIBUTION.md. Original licenses and the locked helper-protocol dependencies are retained.

Build the application and both helpers with the root `build.ps1`. Installed AfterCraft
includes `effectcraft-helper-ofx.exe`; the filename is retained for protocol compatibility.
Use the app's plugin scanning commands to discover your separately installed OFX bundles.
Commercial plugins and their licenses are not included.

The helper supports metadata, standard parameters, float CPU frames and bounded temporal
input callbacks. Idle instances and host-provided threads are limited. Additional layer
inputs, adjustment-layer temporal sampling, custom vendor interfaces, expanded output bounds
and native GPU suites remain incomplete. A discovered effect can still require unsupported
host services. Registration alone is not proof of render compatibility.

For synthetic checks, run `./native/ofx-helper/build.ps1 -Action test` from the repository root.
This builds the original test-plugin fixture and runs the unit and fixture integration tests.
Commercial-effect tests require an official trial or licensed installation and should record
the exact effect, host build, preview resolution and parameter response.

MIT OR Apache-2.0; OpenFX API declarations are BSD-3-Clause.
