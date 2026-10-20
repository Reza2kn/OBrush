# Public-alpha save safety (2026-10-20)

Product commit `258c7379bc41338af206c538a307a01c68132984` adds a browser
leave-page warning for edited documents and active strokes, plus a native
close dialog offering Keep sculpting, Save and close, or Close without saving.
Canceling the native save picker leaves the dialog open. Native writes use a
same-directory temporary file, flush it, and rename only after successful
completion; the containing directory is not synced, so this is not a claim of
crash durability. Four helper tests cover exact create/replace, an injected
pre-rename failure that preserves old bytes and removes the temporary file,
non-file rejection, and Unix permission preservation.

The official browser build's portable WASM SHA-256 was
`f4c005e29f7963ac2df6e29771a3141f1dc427c5b8966ec2506f27b042ef04cf`.
`node scripts/check-unsaved-browser.mjs` ran an owned Chrome 155 process and
fresh profile with ordinary Draw input, Command-S and an actual downloaded
`.obrush` file. All seven frozen checks passed:

- The fresh document is clean and reloads without a warning.
- A Draw stroke makes the document dirty.
- Both a requested download and a browser-blocked download keep the warning armed.
- Navigating away invokes the actual beforeunload dialog.
- Canceling that dialog preserves the document revision and URL.

The downloaded project was 5,317,475 bytes with 40,962 vertices, SHA-256
`4e2d439e88c41e227b15a70b07067d9db44327fe55e70374cb61fb15096c81ac`.
The browser recorded no runtime exceptions and exited zero. Receipt:
`.artifacts/alpha-unsaved/e0c50dd7-9294-434e-b6e2-b519f12e80b1/result.json`,
SHA-256 `4548fdba06c289bdc6bda67e25f72ab28637f3d0cae4403940d78a9ff901ad4e`.
The screenshot and downloaded project are retained alongside it.

Full workspace tests, formatting and strict native/WASM Clippy passed for this
product change. The ignored stationary Draw regression remains a known failure;
these checks do not establish brush quality, recovery, parity, or a long session.
The browser run used the scalar WASM route, not physical pen input.

A separately identified copy of the packaged macOS app was launched for the
close-dialog GUI smoke. Computer control reported that the Mac was locked;
no native close/save gesture was performed. The owned test app was terminated.
That GUI check remains unverified. No autosave or crash recovery was added.
