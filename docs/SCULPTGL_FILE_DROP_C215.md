# C215: Source File Add by drop

The SculptGL-profile browser now accepts OBJ/SGL/PLY/STL files dropped on the
window. It takes the browser `DataTransfer.files` list, ignores unknown
extensions, and routes recognized files through the same revision-bound,
bounded shared-Rust importer as the C214 picker. One batch guard prevents a
second drop/picker read from overlapping the first. Each accepted file remains
an independent Undo step; invalid files are reported while later files are
attempted. The native egui window now accepts supported dropped paths through
the same Source Add file/request path as its picker. No new file parser or
document replacement path was added.

An isolated normal Chrome check dispatched a real DOM `DragEvent` with a
`DataTransfer` containing an independent four-vertex/four-face tetra OBJ and
an ignored `.txt` file. Both `dragover` and `drop` were prevented. OBrush
appended exactly one part; its saved ten-part project had SHA-256
`10b1afaeda6d65b9b41c8af5ecb436c6f726743a412ef47fb25eeb154094fcdc`.
One visible keyboard Undo restored the exact prior nine-part project hash
`304a0f787882ff453692b183f81252866634df40622db16205d1aecabc5032e0`.
An ordinary-profile drop of the same file was prevented from navigating away
and left its one-part scene unchanged, with a visible instruction to use Open
or Add.
The same DOM drop in the pinned hosted SculptGL browser accepted the OBJ as
four vertices/four faces and ignored the text file. Neither browser recorded
an exception. This checks the event-to-import route with synthetic DOM drop
events, not physical file dragging, native GUI input, arbitrary mixed file
semantics, matched pixels or large-batch responsiveness.

The final passing receipt is
`.artifacts/source-file-add-browser/025d477a-1b16-4f25-ba42-3aa6cb35cc35/result.json`
(SHA-256 `92cdf531792aaa4885b8562787baad8860f80c1f22653d1398f1df0b397c7e24`).
The runner SHA-256 is
`bdcc0f73352a5b7ab59b6fea23e7ec24d56c61de6e30337aafe56a1853e9d5af`;
portable WASM SHA-256 is
`aaf7bdcaf90e6f548336c3796eea711c51b83c9b195199c2665e93e05ee7be7e`;
the pinned hosted reference bundle SHA-256 is
`79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600`.
The first expanded run failed before its drop case: its reference multi-file
assertion read meshes while SculptGL's asynchronous file readers were still
completing. The failed receipt remains at
`ec3f09d8-3dda-49e2-b1ff-9953ecdc9b0f`; the final
runner waits for expected geometry and passed.
Earlier passing drop receipts at `70e3332c-a243-4d16-8f0e-71c9101b64f8`
and `628e66c2-726a-43ed-a09f-ce8b1231ed08` preceded the native
drag/drop route or the ordinary-profile navigation guard check.

This closes the browser drop entry point and adds a native path, but does not
complete the frozen `SGL-IO-ADD` acceptance gate or SculptGL/ZBrush parity.

Final-source checks pass `cargo fmt --all -- --check`, `git diff --check`,
strict native/WASM Clippy, `scripts/build-web.sh`, `scripts/package-macos.sh`
and `codesign --verify`. The complete workspace run passed 2,519 tests with
zero failures and one intentionally ignored test across 83 result groups;
`.artifacts/c215-tests.log` SHA-256 is
`359fb72aea2c5b4e9516006e1aa09f35b77706d92098b9560c8c91aedcaad657`.
The packaged native executable SHA-256 is
`3bd37fd76b81f1ac7533482435c048eac36d47e5914827959196e84e7da13181`.
