#!/usr/bin/env python3
"""Small independent oracle tests for the sustained GPU receipt auditor."""

import hashlib
import json
import struct
import tempfile
import unittest
from importlib.machinery import SourceFileLoader
from pathlib import Path

HERE = Path(__file__).resolve().parent
audit_module = SourceFileLoader("source_gpu_audit", str(HERE / "audit-source-gpu-words.py")).load_module()


class GpuWordAuditTest(unittest.TestCase):
    def test_normal_word_change_is_detected_independently(self):
        positions = [[0, 0, 0], [1, 0, 0], [0, 1, 0]]
        normals = [[0, 0, 1]] * 3
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            project = folder / "tiny.obrush"
            project.write_text(json.dumps({"mesh": {"positions": positions, "normals": normals}}))
            project_hash = hashlib.sha256(project.read_bytes()).hexdigest()
            pos_words = b"".join(struct.pack("<fff", *row) for row in positions)
            normal_words = b"".join(struct.pack("<fff", *row) for row in normals)
            stages = ("before", "after", "stable", "undo", "redo")
            result = {"fixture": {"vertices": 3}, **{stage: {"path": str(project),
                      "sha256": project_hash} for stage in stages}}
            result_path = folder / "result.json"
            result_path.write_text(json.dumps(result))
            browser = {"stages": {stage: {"actual_readback_sha256": "0" * 64,
                       "position_words_sha256": hashlib.sha256(pos_words).hexdigest(),
                       "normal_words_sha256": hashlib.sha256(normal_words).hexdigest(),
                       "readback_bytes": 120, "active_vbo_candidates": 1} for stage in stages}}
            browser_path = folder / "browser.json"
            browser_path.write_text(json.dumps(browser))
            good = audit_module.audit(result_path, browser_path)
            self.assertEqual(good["failed_checks"], [])
            browser["stages"]["redo"]["normal_words_sha256"] = "f" * 64
            browser_path.write_text(json.dumps(browser))
            bad = audit_module.audit(result_path, browser_path)
            self.assertEqual(bad["failed_checks"], ["redo"])


if __name__ == "__main__":
    unittest.main()
