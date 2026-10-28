# Experimental Windows Adobe effect host

A separate Rust process loads Adobe `.aex` effects through public ABI declarations from the
pinned MIT `after-effects-sys` dependency. Native FFI and plugin crashes are isolated here.

The supported subset includes typed parameters, sequence lifecycle, CPU legacy and SmartFX
rendering, rational preview downsampling and bounded image/callback transport. Temporal/additional
inputs, HDR, advanced AEGP services, native GPU worlds and vendor custom interfaces remain incomplete.
Initialization is not proof of compatibility.

The read-only AEGP context represents the current untransformed raster, with logical layer
bounds, a local one-layer timeline and no ICC working profile. Partial versioned SDK suites
provide typed errors for unsupported services. Expression-control disk IDs and unbounded
floating slider ranges are retained during discovery. Preferences belong only to AfterCraft;
Adobe application preferences and vendor activation data are never imported.

Expanded SmartFX outputs are rejected before pixel rendering. Cropping a vendor's declared
expanded result is not treated as equivalent to supporting it. Effects requiring an expanded
output, additional layers, temporal checkouts or a different color-management context need
further host work even when registration succeeds.

Build with the root AfterCraft `build.ps1`. This workspace's tests use an original gain fixture
under `fixtures/gain/`; no vendor code or media is included.

Commercial plugins must be installed and licensed separately. Project translation does not
transfer licenses. No commercial binaries or activation modifications belong in this repository.

MIT OR Apache-2.0. See the license files in this directory.
