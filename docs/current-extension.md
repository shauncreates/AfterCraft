# Portable Current extension

Current is also distributed as an independent ScriptUI extension for normal EffectCraft and
After Effects. Its single original `Current.jsx` file supports a docked host Panel or floating
Window, with Bezier, Spring, Elastic and Bounce editing. AfterCraft retains its native panel too.

See [installation, use and limits](../extensions/current/README.md). The portable edition uses
the public keyframe API and its own bounded settings format. It does not depend on the native
`keys.currentApply` or `keys.currentPhysics` commands, a control server, CEP or a private runtime.

Build the reviewable ZIP with `extensions/current/build-package.ps1`. The output contains only
original extension source, documentation, installer helper and license texts. The package script
does not install or run it in any user's host.

The normal EffectCraft 0.17 CLI integration test has exercised actual ScriptUI callbacks, Bezier
influence, the three physics modes, refinement, undo and private settings persistence. The
original installed application and user preferences were left unchanged. Actual After Effects
execution remains unverified; a public API model is not a substitute for that runtime check.
