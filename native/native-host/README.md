# Experimental Windows Adobe effect host

A separate Rust process loads Adobe `.aex` effects through public ABI declarations from the
pinned MIT `after-effects-sys` dependency. Native FFI and plugin crashes are isolated here.

The supported subset includes typed parameters, sequence lifecycle, CPU legacy and SmartFX
rendering, rational preview downsampling and bounded image/callback transport. Temporal/additional
inputs, HDR, advanced AEGP services, native GPU worlds and vendor custom interfaces remain incomplete.
Initialization is not proof of compatibility.

Build with the root AfterCraft `build.ps1`. This workspace's tests use an original gain fixture
under `fixtures/gain/`; no vendor code or media is included.

Commercial plugins must be installed and licensed separately. Project translation does not
transfer licenses. No commercial binaries or activation modifications belong in this repository.

MIT OR Apache-2.0. See the license files in this directory.
