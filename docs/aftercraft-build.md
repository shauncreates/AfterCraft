# Building and packaging AfterCraft

The source root is the compositor workspace; `native/native-host` and `native/ofx-helper`
are separate FFI workspaces. Internal crate/protocol identifiers are retained for compatibility.

Windows x64 is the first packaged target. Install Rust 1.95+, the Windows SDK/MSVC build tools
(recommended), Python with pip for preparing the private parser runtime, and Inno Setup 7.
The GNU build also needs a working MinGW import-library setup; the build helper selects LLVM
linking to avoid the observed GNU-linked test startup failure.

Run `build.ps1 -Configuration Release`, then
`packaging/windows/build-installer.ps1 -Payload artifacts/AfterCraft -IsccPath <path-to-ISCC.exe>`.
`-RceditPath` can apply AfterCraft's icon and version resources when the native resource
compiler is unavailable. Use the official electron/rcedit release. `-Python` selects the build
interpreter; it is not required on installed machines. `-PythonArchive` supplies the pinned,
checksum-verified CPython embeddable archive. Parser requirements are pinned under
`packaging/python/`. Python and package licenses remain in the bundled runtime.

Development builds omit debug symbols to reduce build-disk requirements; optimized pixel
loops remain enabled. Set `CARGO_PROFILE_DEV_DEBUG=1` when symbols are needed for debugging.
The build collects third-party notices and versioned source links into `licenses/`.

On a clean Windows account/VM without AfterCraft already installed, run
`packaging/windows/test-installer.ps1 -Installer <setup.exe>` to check file placement,
product identity, executable/parser startup, shortcuts, registration, uninstall and data
preservation. It also hashes any normal EffectCraft executables before and after. The test
keeps its temporary logs and refuses to replace an existing AfterCraft installation.

The default install is per-user at `%LOCALAPPDATA%\Programs\AfterCraft`, with independent
settings at `%APPDATA%\AfterCraft`. `AFTERCRAFT_CONFIG_DIR` overrides only AfterCraft's data
folder. Portable builds can add `AfterCraft.portable` beside the executable, using
`AfterCraftData` there. Installer builds do not add a portable marker.

Before a core commit, run `cargo xtask ci` (formatting, workspace Clippy/tests, layering,
asset attribution, PDF dependency policy and wasm checks). Native helper tests are additional.
Real commercial-plugin testing requires separate official installations and licenses.

Upstream base: EffectCraft tag `v0.16.0`, commit
`9581b53c3f8945683252af0eb5fac79e0e43aed4`. The `upstream` Git remote tracks
`https://github.com/storytold/effectcraft`. Future upstream merges must preserve AfterCraft's
product identity, separate data folders and helper/plugin behavior; test and document conflicts.
