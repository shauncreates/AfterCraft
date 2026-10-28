# Current portable extension

**Superseded for After Effects:** use the [Current CEP installer](https://github.com/shauncreates/Current/releases/tag/v1.0.0-preview.3).
Close the old Current.jsx panel in AE. This legacy ScriptUI preview raised a blocking
graph.notify error there. EffectCraft/AfterCraft currently do not host CEP panels;
AfterCraft has its own native Current panel. See the standalone repository for current
host support and installation instructions.

An independent ScriptUI curve editor for normal EffectCraft and Adobe After Effects. This package
contains Bezier, Spring, Elastic and Bounce modes, a draggable graph, original visual presets,
named user curves, compact layout, optional Live apply and an independent motion preview.
AfterCraft also keeps its built-in native Current panel; this extension does not replace it.

## Docked or floating

For a docked panel, use **File > Scripts > Install ScriptUI Panel** and choose `Current.jsx`.
Then open **Window > Current.jsx**. Current uses the panel supplied by the host.

For a floating palette, use **File > Scripts > Run Script File** and choose the same `Current.jsx`.
You can keep docked and floating instances open. Each has its own working curve and preview.
They share the Current library in that host's settings.

Recent After Effects versions provide the ScriptUI panel installer in the File > Scripts menu.
Restart After Effects if its Window menu has not refreshed. For older versions, follow Adobe's
[documented ScriptUI installation procedure](https://helpx.adobe.com/ca/after-effects/desktop/automate-in-after-effects/automate-animation/scripts.html).
No CEP runtime, debugger registry flags, network bridge, or AfterCraft installation is required.

### Optional Windows user-folder installer

Preview destinations before copying:

```powershell
.\Install-Current.ps1 -HostApp EffectCraft -Mode Both -WhatIf
```

Remove `-WhatIf` to copy the extension. `-Mode Docked` installs the Window-menu panel;
`-Mode Floating` adds **File > Scripts > Current Floating.jsx**. `Both` installs both entries.
EffectCraft uses `%APPDATA%\EffectCraft` by default. Supply `-SettingsFolder` if your host uses
a portable/custom settings folder. Existing Current script files are backed up before replacement.

For After Effects, prefer its built-in installer. The optional helper accepts
`-HostApp AfterEffects -SettingsFolder <existing versioned per-user preferences folder>`.
Choose the actual preferences folder used by your AE version; the helper does not guess a version
or write into Adobe's application installation. It changes no scripting security preferences.

## Use

Select two adjacent keys on an animatable numeric property. In **Bezier**, drag the two handles
or edit X1/Y1/X2/Y2, then click **Apply**. Shift-drag snaps to 0.05; Alt-drag mirrors the other handle.
Reverse flips the easing direction. Endpoint times and values remain intact.

In **Spring**, **Elastic** or **Bounce**, change Frequency, Damping and Amplitude, then Apply.
Current inserts linear-interpolated sample keys between the selected boundaries. Spring and
Elastic can overshoot; Bounce stays between the endpoints. Amplitude zero produces linear motion.
Each application is one undo group. Reapplying an uninterrupted physics recipe replaces its
generated samples. An external key or selection edit blocks replacement. **Use selection** starts
a new recipe after changing the selection or undoing; it does not change any keyframes itself.

Physics supports scalar and nonspatial numeric vector properties. Separate Position dimensions
before baking motion. Disable expressions and unlock layers first. Shapes, colors, discrete types
and spatial physics are refused. Bezier spatial easing uses a bounded approximation of motion-path
length; separate dimensions for overshoot. Roving spatial keys require roving to be turned off.

EffectCraft scripting editions use one sample per composition frame because older versions snap
scripted writes to whole frames. Higher densities are explicitly refused there. Adobe and AfterCraft
can request one to eight samples per frame. Every bake is limited to 8,192 new keys, 16,384 existing
source keys and 256 selected intervals. Overshoot outside a property's documented range is refused.
When applying Bezier time handles outside 0.001–0.999, Current makes a small temporary
approximation and displays a notice. Saved and exported curves retain their exact coordinates.

Click a preset to preview; double-click to apply. Defaults include easing families and variants.
Create multiple named custom lists to organize your curves. Export the selected list as `.current`
to retain Bezier, standard, and physics graphs. Importing a `.current` or `.flow` file creates a
separate list named after the file; duplicate names get a numbered suffix. Flow export includes
Bezier graphs and reports any skipped standard or physics graphs. An all-incompatible list cannot
be exported to Flow. Current's `.flow` reader and writer have been checked against a real library;
an import into the Flow extension itself has not yet been verified.

Save and Delete affect only Current's own library. It uses short, validated settings records with a committed active slot;
user strings are never executed. The portable library is separate from the native AfterCraft library.

Play preview runs a small 20 Hz drawing task; pausing or closing cancels it. It does not render the
project or move its playhead. Live apply is off initially and runs after a numeric control or graph
gesture finishes, not on every mouse move. A host error turns Live apply off. If a host unexpectedly
fails midway through a write, the message directs you to Undo Current to restore the prior keys.

## Verification and limits

The original math, settings fault protection and public API model are covered by `test-current.cjs`.
In the source checkout run `node extensions/current/test-current.cjs`.
An isolated normal EffectCraft 0.17 CLI integration test has exercised the real ScriptUI callbacks,
Bezier influence, all three physics modes, refinement, undo, library saving and compact mode.
EffectCraft 0.16 support is based on its public scripting source/API audit.

Actual After Effects execution has not yet been verified. The extension targets its documented
ScriptUI/keyframe API, but synthetic API tests do not establish AE-version compatibility.
MFlow preset files, CEP panels and proprietary expressions are not imported. No third-party binaries
or presets are included.

Standalone source and issues: [Current](https://github.com/shauncreates/Current). Built-in panel: [AfterCraft](https://github.com/shauncreates/AfterCraft).
Public API references: [Property](https://ae-scripting.docsforadobe.dev/property/property/),
[ScriptUI drawing](https://extendscript.docsforadobe.dev/user-interface-tools/drawing-objects/),
[host panel contract](https://ae-scripting.docsforadobe.dev/introduction/overview/).

MIT OR Apache-2.0. See `NOTICE.md`, `Current.jsx.attribution`, and both license files.

