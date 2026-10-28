# AfterCraft OpenFX helper

This separate native process hosts OpenFX effects for AfterCraft on the CPU and, on Windows,
through native hardware OpenGL when the plug-in supports it. It derives from
[ec-extension-ofx](https://github.com/glumbis/ec-extension-ofx); see PROVENANCE.md and
ATTRIBUTION.md. Original licenses and the locked helper-protocol dependencies are retained.

Build the application and both helpers with the root `build.ps1`. Installed AfterCraft
includes `effectcraft-helper-ofx.exe`; the filename is retained for protocol compatibility.
Use the app's plugin scanning commands to discover your separately installed OFX bundles.
Commercial plugins and their licenses are not included.

The helper supports metadata, standard parameters, float CPU frames and bounded temporal
input callbacks. Idle instances and host-provided threads are limited. Additional layer
inputs, adjustment-layer temporal sampling, custom vendor interfaces, expanded output bounds
and native CUDA/Metal host APIs remain incomplete. A discovered effect can still require unsupported
host services. Registration alone is not proof of render compatibility.

For synthetic checks, run `./native/ofx-helper/build.ps1 -Action test` from the repository root.
This builds the original test-plugin fixture and runs the unit and fixture integration tests.
Commercial-effect tests require an official trial or licensed installation and should record
the exact effect, host build, preview resolution and parameter response.

MIT OR Apache-2.0; OpenFX API declarations are BSD-3-Clause.

## Native GPU rendering

Compatible effects automatically receive OpenGL textures in a private hardware context. The
host pairs context attachment/detachment, bounds input textures to a 512 MiB per-render budget,
and releases textures after every render. Output is read back into the existing float-frame
protocol. CPU-capable effects retry on the CPU after a GPU allocation/render error; license
errors are reported without retrying around them. GPU-only effects report missing hardware.

`effectcraft-helper-ofx --gpu-info` reports the actual vendor, renderer and GL version.
`AFTERCRAFT_OFX_GPU=0` disables the native GPU path for troubleshooting. A software GL renderer
is not advertised as GPU acceleration. This implements the OpenFX OpenGL API, not native CUDA,
OpenCL, Metal or Adobe GPU suites; a plug-in may use its own CUDA/HIP implementation internally.
Accelerating an effect does not correct imported parameter units or animation mappings.

For a bounded per-frame diagnostic, set `AFTERCRAFT_OFX_GPU_TRACE` to an absolute JSONL path.
Each attempted render records `opengl`, `cpu`, or `cpu-fallback`, with the OFX status. The trace
stops growing at 16 MiB. Leave this unset for ordinary use.

`--describe <bundle> [effect-id]` reports declared GPU capabilities and parameter units.
During ordinary scans, those unit declarations are retained in the helper state and copied into
the `.ecofx.json` profile's `parameterSemantics` field for project translation.

Public contract: [OpenFX GPU rendering](https://openfx.readthedocs.io/en/main/Reference/ofxOpenGLRender.html)
and [ofxGPURender.h](https://github.com/AcademySoftwareFoundation/openfx/blob/main/include/ofxGPURender.h).
