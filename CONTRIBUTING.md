# Contributing to OBrush

OBRush is an independently written Rust sculpting studio. Its current
public alpha is a place to test real workflows and make the next release
better; [the feature ledger](docs/SCULPTGL_CURRENT_LEDGER.md) keeps
SculptGL parity claims open until their full acceptance gates pass.

## Report a bug

Use the GitHub **Bug report** issue form. Include the OBrush build or commit,
macOS/browser version, input device, mesh origin, exact tool radius and
strength, symmetry/pressure settings, steps, and the expected versus actual
result. Say whether the problem persists in a saved `.obrush` file and
whether Undo still works. A screenshot, short recording, or minimal project
helps, but issue attachments are public. Share only files you are comfortable
publishing; do not include license, account, or private project data.

Before filing, search for a matching issue. Add new reproduction evidence to
that issue when possible. For feature priorities, use the **Feature request
and vote** form, or react 👍 to an existing request. The number of reactions
is useful input, not a promise or a parity score.

## Build and change the code

Install a current Rust toolchain, then run:

```sh
cargo fmt --all --check
cargo test --locked -p obrush-core
cargo clippy --locked --workspace --all-targets -- -D warnings
```

For browser changes, install the `wasm32-unknown-unknown` target and the
lockfile-matched `wasm-bindgen` CLI, run `scripts/build-web.sh`, and test the
result through the ordinary browser UI. For a local macOS app, use
`scripts/package-macos.sh`; the result is ad-hoc signed, not notarized.
The existing GitHub Actions CI runs broader checks. Include a clear before
and after example and the checks you ran in your pull request.

Keep geometry, file IO, and history in `obrush-core` so native and browser
behavior stay shared. Validate imported files before replacing a document,
preserve Undo, and bound allocations. Use independently created test assets.
Do not submit commercial binaries, copied proprietary source, license data,
or private work. New code is offered under the repository's MIT OR
Apache-2.0 license; bundled public assets retain their own notices.

## Release channels

The browser preview workflow builds `web/dist` for GitHub Pages from `main`.
The macOS alpha workflow accepts a tag matching the package version, creates
an arm64 ZIP and checksum, and opens a **draft** GitHub Release for review.
Maintainers enable Pages for GitHub Actions and publish a draft only after
checking its assets and release notes. There is no Linux/Windows package or
physical-pen validation claim yet.
