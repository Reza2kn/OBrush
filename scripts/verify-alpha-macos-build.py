#!/usr/bin/env python3
"""Reject an arm64 alpha app whose signed build identity is absent or stale."""

import hashlib
import json
import plistlib
import re
import sys
from pathlib import Path


def fail(message: str) -> None:
    raise SystemExit(f"Invalid macOS alpha build: {message}")


if len(sys.argv) != 5:
    fail("usage: verify-alpha-macos-build.py APP VERSION COMMIT LOCK_SHA256")
app = Path(sys.argv[1])
version, commit, lock_sha = sys.argv[2:]
binary = app / "Contents/MacOS/OBrush"
metadata = app / "Contents/Resources/build-info.json"
plist = app / "Contents/Info.plist"
if not binary.is_file() or not metadata.is_file() or not plist.is_file():
    fail("missing app binary, Info.plist, or signed build-info.json")
try:
    info = json.loads(metadata.read_text())
    with plist.open("rb") as stream:
        app_info = plistlib.load(stream)
except (OSError, ValueError, json.JSONDecodeError) as error:
    fail(f"malformed app metadata: {error}")
if not isinstance(info, dict) or info.get("schema_version") != 1:
    fail("unsupported build identity schema")
if not re.fullmatch(r"[0-9a-f]{40}", commit):
    fail("current checkout has no complete source commit")
for key, expected in (
    ("version", version),
    ("source_commit", commit),
    ("source_tree_state", "clean"),
    ("cargo_lock_sha256", lock_sha),
    ("architecture", "arm64"),
):
    if info.get(key) != expected:
        fail(f"{key} differs from the clean current checkout")
if app_info.get("CFBundleShortVersionString") != version:
    fail("Info.plist version differs from the release tag")
actual_binary_hash = hashlib.sha256(binary.read_bytes()).hexdigest()
if info.get("app_binary_sha256") != actual_binary_hash:
    fail("app binary differs from signed build identity")
print(f"Verified recorded app build {commit} ({version}, arm64, {actual_binary_hash})")
