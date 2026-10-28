# OFX helper provenance

Based on https://github.com/glumbis/ec-extension-ofx at
63059c6eb231a251ab01c4d62ff7e89050ca3a17 (MIT OR Apache-2.0).
Its locked EffectCraft helper-protocol/effects dependencies are from glumbis/effectcraft,
commit f3ab0a6e (full revision is recorded in Cargo.lock), also MIT OR Apache-2.0.
The original reference checkout remains separate and unmodified.

The inspected in-process fork is https://github.com/glumbis/effectcraft at
f5a35251689ab8978b6b0af2be7f542b74d995d0. Its permissively licensed importer was adapted separately in tools/aep-import.

AfterCraft changes include Windows job containment, suppressed operating-system crash
dialogs, temporal-frame callbacks, bounded instance reuse, host thread budgeting and
foreground interactive host reporting. Build tooling uses the selected system Rust toolchain. The EffectCraft
core uses an original safe Rust protocol-v1 client, with explicit limits for unsupported
host callbacks. Native OFX ABI code remains in this separate helper workspace.

OpenFX API declarations are BSD-3-Clause; see ATTRIBUTION.md and LICENSE-OPENFX-BSD.
The Windows hardware OpenGL host and synthetic GPU fixtures are original AfterCraft code,
implemented from the public `ofxGPURender.h` contract and Win32/OpenGL interfaces.
No commercial plugins, presets, fonts, icons or Adobe assets are redistributed here.
