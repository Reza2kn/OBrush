# Combined C302/C305 local checkpoint

The main checkout at `6a2652c` combines the scoped Source Brush same-frame pen
replay from [C302](SCULPTGL_SOURCE_PEN_BURST_C302_RESULT.md) and Source Drag
mirror/mask behavior from [C305](SCULPTGL_SOURCE_DRAG_C305_RESULT.md). Each
candidate passed its separately frozen pinned-reference browser pair before
integration. The C305 candidate also passed the unchanged C295 regression.

After merging, `cargo fmt --all -- --check`, `cargo test --workspace
--all-targets`, strict native workspace Clippy, strict WASM-target workspace
Clippy, `bash scripts/build-web.sh`, and `bash scripts/package-macos.sh` all
exited zero. The macOS package signature was verified by the package script.
The merged portable WASM SHA-256 is
`67051db667ad51adba1fc65f9b28bdb05bd23a1610058285836082c191a7ada6`,
SIMD WASM is
`649e50fc7f3725d45b56e7d435320382293b104ca3c49fadfaad6d37e54ba750`,
and the packaged macOS executable is
`44ba09a26e9bda315ff98ac636b6db3b2dd2b5a1ef0d600fd84ce28e76df57a9`.

A separate hidden normal-browser load at `http://127.0.0.1:8792/` displayed
the default 40,962-vertex sphere and sculpting interface from the rebuilt
bundle. The screenshot is retained locally in ignored
`.artifacts/combined-browser-smoke.png`. This is a startup smoke check, not a
repeat of the C302/C305 paired gestures on the combined build. Their raw
ignored receipts remain in the two isolated worktrees named in those reports.

The 317-ID pinned inventory still has 309 active IDs and every complete
acceptance gate open. Physical pen hardware, other Source pressure paths,
other Drag settings and long-session million-vertex responsiveness were not
validated by this checkpoint.
