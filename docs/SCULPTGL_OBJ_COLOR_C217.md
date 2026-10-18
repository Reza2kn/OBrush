# C217: SculptGL-style OBJ color and material switches

The SculptGL-profile File panel now has independent **OBJ color zbrush** and
**OBJ color append** switches with the pinned defaults on/off. The shared Rust
scene exporter writes one OBJ object per scene part, bakes world poses,
preserves quad/triangle faces and indexed corner UVs, and emits `#MRGB` in
64-vertex chunks and `#MAT` in 46-vertex chunks when the first switch is on.
`#MRGB` uses the pinned constant `ff` leading byte, followed by RGB bytes;
`#MAT` contains roughness, metalness and unmasked influence bytes. The second
switch independently appends unquantized RGB to each `v` record. Both off
intentionally omit color; the export report and normal File panel say so.
The ordinary OBrush profile retains its earlier inline-RGB writer.

The writer converts canonical protected or source unprotected mask values to
the reference's unmasked third material channel without changing the source
mesh. Float32 channel values are multiplied at JavaScript's binary64 precision
before rounding to 8 bits. A regression check covers a half-step value where
an early Float32 multiply would produce the wrong byte. The report exposes
quantized row counts and maximum RGB/material/mask error. Core checks cover
all four switch combinations with nondefault material/mask values, OBJ
roundtrip within one 8-bit step, cross-object UV offsets and both chunk
boundaries. A protected-default mesh with an actual mask value is also reported
as lossy when `#MAT` is off, even though its mask convention is the default.
Source project JSON is unchanged by the exports.

The normal browser check imports the same independently authored two-object
OBJ with six explicit nondefault `#MAT` records into OBrush and the pinned
hosted SculptGL runtime. The OBrush side selects those two parts through the
visible scene controls and downloads all four OBJ switch combinations through
the visible File controls. The reference side invokes its unmodified GuiFiles
export method with each option pair after the normal file-input import; its
all-scene output also includes the startup sphere. For the matching two
imported object blocks, every `#MRGB` and `#MAT` payload matches exactly in
each option case. Nearest world-position component error is at most
`1.929e-6` across the six imported vertices. The fixture is 163 bytes (SHA-256
`2292fce8e1624a429316949f7c6199dcbfdb6082ff57745db089bc3a3a920aad`).
The option combinations produce
the expected `v` record widths and the browser shows zero errors. OBrush's
saved project SHA-256 is identical before and after all downloads. The final
receipt is
`.artifacts/sculptgl-obj-color-browser/48193ee2-8dbf-4094-a594-2e177cefcdc5/result.json`
(SHA-256 `e87b61f654a41dd21bb8556d47309d9361e3ffccd479bcc0280f572797efc350`).
The browser runner SHA-256 is
`7a0c79f0f416be74f9a8f1497653d94566ca78dece1d49939de111e10c17bb06`;
portable/SIMD WASM SHA-256 values are
`cbebff998531d7bb6cf0572204bf2414a98d8c2905d821fb7da4d14227413232`
and `e27231293a8a5af939f5f18f10ca7953b4c45ae18821114f896f14b4e3873c22`.
OBRush's 8-bit report on this exact-byte source fixture has maximum material
and mask errors `2.969e-8` and `2.992e-8` from Float32 storage; RGB is exact.

This is a strong small-fixture OBJ color/material interchange comparison,
not complete SculptGL OBJ export parity. The reference was driven through its
runtime method rather than physical GUI clicks for the four option switches;
OBRush used its visible controls. The reference all-scene output includes its
startup sphere, so complete downloaded file bytes intentionally differ.
Large materials, arbitrary UV seams/quads, source opacity behavior, native
physical GUI and long-session export responsiveness remain open. The third
SculptGL material channel is encoded from OBrush's canonical mask, not stored
as an independent third material field.

The first strict native/WASM Clippy runs failed on two test-only range loops
under `needless_range_loop`; the loops were rewritten as enumerations and
the failed logs remain in `.artifacts/c217-clippy-native.log` and
`.artifacts/c217-clippy-wasm.log`.

Final-source checks pass `cargo fmt --all -- --check`, `git diff --check`,
strict native/WASM Clippy, `scripts/build-web.sh`, `scripts/package-macos.sh`
and its signature verification, and the paired browser run above. The complete
workspace test run passed 2,522 tests with zero failures and one intentional
ignore across 103 result groups. Its log SHA-256 is
`063367202c63d30cbc9849c836f77782fc5c71420baa86d39e514cce81ea9fd2`.
The packaged native executable SHA-256 is
`00eafb0dc4ee76ff43bf56b0f96934577ccfd801ffbf28830afa60719f82061f`.
