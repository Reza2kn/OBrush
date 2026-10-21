# Public alpha launch verification (2026-10-20)

The [repository](https://github.com/Reza2kn/OBrush),
[public alpha release](https://github.com/Reza2kn/OBrush/releases/tag/v0.1.20-alpha.1),
[browser preview](https://reza2kn.github.io/OBrush/) and
[community feedback discussion](https://github.com/Reza2kn/OBrush/discussions/4)
are publicly accessible. Known issues are recorded as
[Draw saturation](https://github.com/Reza2kn/OBrush/issues/1),
[worked-surface artifacts](https://github.com/Reza2kn/OBrush/issues/2), and
[dense-mesh responsiveness](https://github.com/Reza2kn/OBrush/issues/3).

## Fixed release

Tag `v0.1.20-alpha.1` resolves to commit
`b603a3e7b6ba82e69d2065e3c2438e746a2e3552`. Independent unauthenticated
downloads of all five release assets passed GitHub digest/size checks.
The three ZIPs match their published `SHA256SUMS.txt`, pass CRC checks,
and match the frozen local files byte for byte:

| Archive | SHA-256 |
| --- | --- |
| macOS arm64 | `91c9d94cd07302cd4828f6d1ca6bb22f0a8e0bc26c366295f181cce65ea7a983` |
| Browser bundle | `159ad0c075cb1dd76e51eb50e3ea3bcc0ee258226d5f45b461704ce86b370a85` |
| Source | `f4ea9e5a65c01983dd01b56a214399fa4d9cae6f1cde7838e9a1a9c540366f94` |

The macOS manifest's signed executable SHA-256 is
`4a1962bde99b1657d6a1a8859c9372d837e6dd2fe1ac61ce8064755387dbbeeb`.
The app is ad-hoc signed, not Apple notarized. Clean-machine installation and
the new native close dialog remain unverified; the local GUI attempt was
blocked by the locked Mac. The previous packaged Draw/Save/Undo/Redo witness
does not establish those new checks.

## Hosted browser

[The successful Pages run](https://github.com/Reza2kn/OBrush/actions/runs/39518404525)
deployed clean commit `cb7e896054e542fe6f352edba000e46b0a9ef8ac`.
Unauthenticated downloads of its three WASM files match the public
`build-info.json`; each is served as `application/wasm`:

- Portable: `02eaa5802697951ea74e69cdb6024b579dcb08f7dc15eddce083df42aac0cf72`.
- SIMD: `2248d789ce9b1d322a7ac6452cac58b7f94840ae3bfedfd0e15b3cf48e7627cf`.
- GPU preview: `18bfb72eb00ca7f2d1e2a28ae92980d9127e4efc6ff00879239d1c364c3c6945`.

An owned Chrome 155 process and fresh profile loaded the actual public URL,
selected the SIMD route, performed ordinary Draw input and Command-S, and
downloaded a 5,317,475-byte, 40,962-vertex project. Its SHA-256
`4e2d439e88c41e227b15a70b07067d9db44327fe55e70374cb61fb15096c81ac`
matches the earlier local scalar/SIMD saves for this input. The leave warning
remained armed after both accepted and blocked downloads; canceling navigation
preserved the document revision and URL. All seven scoped checks passed,
with no recorded runtime exception and Chrome exit zero.

Receipts are retained in ignored local directories:
`.artifacts/public-release-audit-render-resolution/` and
`.artifacts/alpha-unsaved/d233cbc0-03a1-4eaf-936e-fb83dd447525/`.
This is a mouse/browser save-safety witness, not pen, long-session, geometry
quality, or parity proof. The hosted preview can advance independently of
the fixed release; consult its build metadata for the actual revision.

## Toolchain and retained CI failure

The local release checks used Rust 1.96.0. The first hosted CI core job used
Rust 1.99.1: formatting, inventory audit and all core tests passed, while strict
Clippy failed on newer style/deprecation diagnostics. That
[failed run](https://github.com/Reza2kn/OBrush/actions/runs/39518404448)
is retained. The supported toolchain and workflows are now pinned to Rust
1.96.0; the CI configuration adds manual dispatch so that pin can be verified
without rebuilding the already checked Pages deployment. This documentation
does not assert an uncompleted CI run passed.
