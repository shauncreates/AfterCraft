# AfterCraft

An independent, experimental motion-graphics compositor based on **EffectCraft 0.16.0**.
AfterCraft adds Windows native-plugin hosting, edition-aware project import and bounded
plugin preview rendering. It installs alongside normal EffectCraft.

## Install and open

Download the Windows x64 installer from [Releases](https://github.com/shauncreates/AfterCraft/releases).
Install it, then open **AfterCraft** from the Start menu or its optional desktop shortcut.

- Default program folder: `%LOCALAPPDATA%\Programs\AfterCraft`
- Settings, plugin registrations, logs and recovery: `%APPDATA%\AfterCraft`
- Normal EffectCraft's installation and settings are separate.
- Uninstall removes AfterCraft's program files and shortcuts; your settings and projects remain.

Preview releases are experimental and unsigned. They do not include commercial plugins.
Install and license third-party plugins separately through their vendors.

## Plugins and projects

Use **Effect > Scan Installed Plug-ins**, **Scan Plug-in Folder**, or **Install Plug-in Download**.
The native scanner reads plugin metadata and has no four-effect or vendor whitelist.
OFX and Windows Adobe `.aex` effects run in separate helper processes. Initialization or
registration does not establish rendering compatibility with every commercial effect.

When Adobe plugins are found in standard installed locations, the edition chooser can prefer
Adobe or OFX. Effects & Presets displays the selected edition; existing projects keep their IDs.

**File > Import > After Effects Project** accepts `.aep` and `.aepx`. Supported effect identities,
parameters and animation are rebuilt against installed plugins. Sapphire has explicit Adobe-to-OFX
mappings; other vendors need known mappings. The saved edition preference is a priority, with
fallback when a matching preferred edition is unavailable. Unknown effects and unsupported data
are recorded in the import report and preserved source copy. Import is one undo step.

This translates project data, not plugin executables or licenses. Full AE rendering parity,
advanced 3D/Element 3D, CEP extensions, all custom plugin controls and all parameter mappings
are not implemented. See [import support](docs/aep-import.md) and [plugin hosting](docs/plugins.md).

## Resource use

Plugin preview frames queue rather than multiplying native instances across every frame worker.
Default cache allowances are 256 MiB for layers, 512 MiB for media and 1024 MiB for previews.
Native allocations and temporary render buffers are additional. Change the caches in
**Settings > Memory & CPU**. Smaller caches retain fewer preview frames in RAM.

The OFX host thread suite defaults to four threads; `AFTERCRAFT_OFX_THREADS=1..16` overrides it.
Plugins can create their own threads. Native-plugin GPU suites are not implemented; the built-in
GPU compositor remains available. Full-resolution plugin previews may run below real time.

## Build

Windows builds need Rust 1.95 or later, Python with pip for preparing the import runtime,
and Inno Setup 7 for the installer. Installed builds need no system Python.

```powershell
.\build.ps1 -Configuration Debug
.\packaging\windows\build-installer.ps1 -Payload .\artifacts\AfterCraft
```

Source crates retain their `effectcraft-*` names and `.ecproj` compatibility to limit upstream
merge churn. The installed executable, app identity, settings and installer are AfterCraft's.
See [building and packaging](docs/aftercraft-build.md).

## Contribute and report issues

Use [Issues](https://github.com/shauncreates/AfterCraft/issues) for reproducible bugs and
[Discussions](https://github.com/shauncreates/AfterCraft/discussions) for questions.
Commercial-plugin fixes should be verified with official trial or licensed installations.
Synthetic fixtures are preferred for public tests; user projects and vendor binaries are not
part of this repository. Run `cargo xtask ci` before committing core changes.

## Credits and license

MIT OR Apache-2.0. Original copyright notices and licenses are retained.

- [EffectCraft](https://github.com/storytold/effectcraft), the upstream compositor and its contributors.
- [glumbis/effectcraft](https://github.com/glumbis/effectcraft) and
  [ec-extension-ofx](https://github.com/glumbis/ec-extension-ofx), sources of the permissively licensed
  importer/reference and OFX helper work.
- [py-aep](https://pypi.org/project/py-aep/), the MIT-licensed project parser.

AfterCraft is an independent fork, not an official ArtCraft or Adobe product. Upstream's restricted
brand artwork is replaced with original AfterCraft artwork. See [provenance](PROVENANCE.md),
[NOTICE](NOTICE), and [asset attribution](ATTRIBUTION.md).
