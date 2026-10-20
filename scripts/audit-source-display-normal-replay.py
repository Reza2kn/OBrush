#!/usr/bin/env python3
"""Independent offline Source display-normal replay; fail closed on absent trace.

The public project records positions and mesh normals, not retained Source
display normals. A qualifying trace must record each accepted dab's selected
vertex IDs and post-dab position rows, plus the selected IDs and position rows
for Undo/Redo. This script never guesses selections from final geometry.
"""

import argparse
import hashlib
import json
import math
import struct
import tempfile
from pathlib import Path

CORNER = 0xFFFFFFFF
SCHEMA = "obrush-source-display-replay-v1"
STAGES = ("before", "after", "stable", "undo", "redo")
MAX_TRACE_BYTES = 128 * 1024 * 1024
MAX_VERTICES = 1_000_000
MAX_FACES = 2_000_000


def f32(value):
    return struct.unpack("<f", struct.pack("<f", value))[0]


def bits3(row):
    return tuple(struct.unpack("<III", struct.pack("<fff", *row)))


def digest_rows(rows):
    h = hashlib.sha256()
    for row in rows:
        h.update(struct.pack("<fff", *row))
    return h.hexdigest()


def face_normal(positions, face):
    a, b, c, d = face
    pa, pb, pc = positions[a], positions[b], positions[c]
    ab = [pb[i] - pa[i] for i in range(3)]
    ac = [pc[i] - pa[i] for i in range(3)]
    normal = [ab[1] * ac[2] - ab[2] * ac[1],
              ab[2] * ac[0] - ab[0] * ac[2],
              ab[0] * ac[1] - ab[1] * ac[0]]
    if d != CORNER:
        pd = positions[d]
        cd = [pc[i] - pd[i] for i in range(3)]
        extra = [cd[1] * ac[2] - cd[2] * ac[1],
                 cd[2] * ac[0] - cd[0] * ac[2],
                 cd[0] * ac[1] - cd[1] * ac[0]]
        normal = [normal[i] + extra[i] for i in range(3)]
    result = tuple(f32(x) for x in normal)
    if not all(math.isfinite(x) for x in result):
        raise ValueError("nonfinite computed face normal")
    return result


def checked_rows(rows, count, name):
    if not isinstance(rows, list) or len(rows) != count:
        raise ValueError(f"{name} count is invalid")
    out = []
    for row in rows:
        if not isinstance(row, list) or len(row) != 3 or not all(
            isinstance(x, (int, float)) and math.isfinite(x) for x in row
        ):
            raise ValueError(f"{name} contains an invalid row")
        out.append(tuple(f32(x) for x in row))
    return out


class Replay:
    def __init__(self, mesh):
        self.positions = checked_rows(mesh["positions"], len(mesh["positions"]), "positions")
        count = len(self.positions)
        if not 3 <= count <= MAX_VERTICES:
            raise ValueError("vertex count exceeds replay bound")
        surface = mesh.get("surface")
        if surface is not None:
            self.faces = [tuple(face) for face in surface["faces"]]
        else:
            self.faces = [(*face, CORNER) for face in mesh["triangles"]]
        if not 1 <= len(self.faces) <= MAX_FACES:
            raise ValueError("face count exceeds replay bound")
        self.incident = [[] for _ in self.positions]
        for face_id in range(len(self.faces) - 1, -1, -1):
            face = self.faces[face_id]
            if len(face) != 4 or any(type(v) is not int or not 0 <= v < count
                                     for v in face[:3]) or (face[3] != CORNER and
                                     (type(face[3]) is not int or not 0 <= face[3] < count)):
                raise ValueError("invalid original face")
            for vertex in face[:3 if face[3] == CORNER else 4]:
                self.incident[vertex].append(face_id)
        self.face_normals = [face_normal(self.positions, face) for face in self.faces]
        self.vertex_normals = [self._mean(v) for v in range(count)]

    def _mean(self, vertex):
        incident = self.incident[vertex]
        sums = [0.0, 0.0, 0.0]
        for face_id in incident:
            normal = self.face_normals[face_id]
            for i in range(3):
                sums[i] += normal[i]
        factor = 0.0 if not incident else 1.0 / len(incident)
        return tuple(f32(x * factor) for x in sums)

    def apply(self, event):
        selected = event.get("selected")
        changes = event.get("positions")
        if not isinstance(selected, list) or not isinstance(changes, list):
            raise ValueError("missing selected IDs or post-operation position rows")
        if any(type(v) is not int or not 0 <= v < len(self.positions) for v in selected):
            raise ValueError("invalid selected vertex ID")
        if selected != sorted(set(selected)):
            raise ValueError("selected IDs must be sorted and unique")
        changed_ids = []
        for row in changes:
            if not isinstance(row, list) or len(row) != 4 or type(row[0]) is not int:
                raise ValueError("invalid changed position row")
            index = row[0]
            if not 0 <= index < len(self.positions):
                raise ValueError("changed position ID out of range")
            point = checked_rows([row[1:]], 1, "changed position")[0]
            self.positions[index] = point
            changed_ids.append(index)
        if changed_ids != sorted(set(changed_ids)) or not set(changed_ids) <= set(selected):
            raise ValueError("position rows must be sorted, unique and selected")
        affected = {face for vertex in selected for face in self.incident[vertex]}
        for face in affected:
            self.face_normals[face] = face_normal(self.positions, self.faces[face])
        for vertex in selected:
            self.vertex_normals[vertex] = self._mean(vertex)


def load_bounded(path, limit):
    path = Path(path)
    if path.stat().st_size > limit:
        raise ValueError(f"{path.name} exceeds byte cap")
    return json.loads(path.read_bytes())


def inspect(receipt, trace_path=None):
    receipt = Path(receipt)
    result_path = receipt / "result.json"
    result_bytes = result_path.read_bytes()
    result = json.loads(result_bytes)
    output = {"schema": SCHEMA, "receipt": str(receipt),
              "result_sha256": hashlib.sha256(result_bytes).hexdigest(),
              "status": "missing_replay_evidence", "missing": [], "stages": {}}
    if result.get("fixture", {}).get("vertices", 0) > MAX_VERTICES:
        raise ValueError("fixture exceeds replay vertex bound")
    if trace_path is None:
        trace_path = receipt / "source-display-replay-trace-v1.json"
    trace_path = Path(trace_path)
    if not trace_path.is_file():
        output["missing"] = ["ordered accepted dab sequence with selected vertex IDs",
                             "post-dab position rows for every accepted dab",
                             "Undo/Redo selected IDs and restored position rows",
                             "trace-to-result and build provenance"]
        return output
    trace = load_bounded(trace_path, MAX_TRACE_BYTES)
    if trace.get("schema") != SCHEMA or trace.get("result_sha256") != output["result_sha256"]:
        raise ValueError("replay trace schema or result identity differs")
    if trace.get("product_binding") != result.get("product_binding"):
        raise ValueError("replay trace does not bind the served product bytes")
    before_path = Path(result["before"]["path"])
    if trace.get("before_sha256") != hashlib.sha256(before_path.read_bytes()).hexdigest():
        raise ValueError("replay trace does not bind the Before project")
    if trace.get("accepted_dabs") != result.get("worker_trial", {}).get("acknowledgedDabs"):
        raise ValueError("trace accepted dab count differs from Worker receipt")
    events = trace.get("events")
    if not isinstance(events, list) or len(events) > 100_000:
        raise ValueError("replay event list is missing or exceeds bound")
    acknowledgments = [message for message in result.get("worker_trace", {}).get("messages", [])
                       if message.get("direction") == "worker-to-main"
                       and message.get("kind") == "apply" and message.get("success") is True]
    if len(acknowledgments) != trace["accepted_dabs"]:
        raise ValueError("successful Worker apply reply count differs from replay trace")
    gpu = load_bounded(receipt / "gpu-browser.json", 32 * 1024 * 1024)
    before = load_bounded(before_path, 512 * 1024 * 1024)
    replay = Replay(before["mesh"])
    def checkpoint(stage):
        public_bytes = Path(result[stage]["path"]).read_bytes()
        public_sha256 = hashlib.sha256(public_bytes).hexdigest()
        if public_sha256 != result[stage].get("sha256"):
            raise ValueError(f"{stage} public Save hash differs from receipt")
        project = json.loads(public_bytes)
        expected = checked_rows(project["mesh"]["positions"], len(replay.positions), "public positions")
        position_exact = all(bits3(a) == bits3(b) for a, b in zip(replay.positions, expected))
        observed = gpu["stages"][stage]
        gpu_normal = observed.get("normal_words_sha256")
        row = {"position_words_exact": position_exact,
               "public_save_sha256": public_sha256,
               "gpu_position_words_exact": observed.get("position_words_sha256") == digest_rows(expected),
               "display_normal_words_exact": gpu_normal == digest_rows(replay.vertex_normals),
               "replay_display_normal_words_sha256": digest_rows(replay.vertex_normals),
               "gpu_display_normal_words_sha256": gpu_normal,
               "active_vbo_candidates": observed.get("active_vbo_candidates"),
               "gl_error": observed.get("gl_error")}
        output["stages"][stage] = row
    checkpoint("before")
    seen = []
    dabs = 0
    history = []
    for event in events:
        if not isinstance(event, dict) or event.get("kind") not in ("dab", "history", "checkpoint"):
            raise ValueError("invalid replay event")
        kind = event["kind"]
        if kind == "dab":
            if seen:
                raise ValueError("dab appears after After checkpoint")
            dabs += 1
            if event.get("sequence") != dabs:
                raise ValueError("dab sequence is missing or out of order")
            if dabs > len(acknowledgments):
                raise ValueError("trace has more dabs than accepted Worker replies")
            if event.get("worker_message_id") != acknowledgments[dabs - 1].get("id"):
                raise ValueError("dab does not bind its accepted Worker reply")
            replay.apply(event)
        elif kind == "history":
            expected = "undo" if seen == ["after", "stable"] else (
                "redo" if seen == ["after", "stable", "undo"] else None)
            if event.get("action") != expected or expected in history:
                raise ValueError("invalid history action")
            replay.apply(event)
            history.append(expected)
        else:
            stage = event.get("stage")
            if stage not in STAGES[1:] or stage in seen or stage != STAGES[len(seen) + 1]:
                raise ValueError("stage checkpoint is missing or duplicated")
            if ((stage == "undo" and history != ["undo"])
                    or (stage == "redo" and history != ["undo", "redo"])):
                raise ValueError("history checkpoint lacks the required transition")
            checkpoint(stage)
            seen.append(stage)
    if dabs != trace["accepted_dabs"] or seen != list(STAGES[1:]):
        raise ValueError("accepted dabs or stage checkpoint order is incomplete")
    output["status"] = "replay_completed" if all(
        row["position_words_exact"] and row["gpu_position_words_exact"]
        and row["display_normal_words_exact"] and row["active_vbo_candidates"] == 1
        and row["gl_error"] == 0 for row in output["stages"].values()
    ) else "replay_mismatch"
    return output


def self_test():
    mesh = {"positions": [[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1]],
            "triangles": [[0, 1, 2], [0, 2, 3]], "normals": [[0, 0, 0]] * 4}
    replay = Replay(mesh)
    original = list(replay.vertex_normals)
    assert original[1] == (0.0, 0.0, 1.0)
    replay.apply({"selected": [0], "positions": [[0, 0, 0, 0.5]]})
    assert replay.vertex_normals[1] == original[1]  # unselected row stays stale
    assert replay.vertex_normals[0] != original[0]
    replay.apply({"selected": [0, 1], "positions": [[0, 0, 0, 0]]})
    assert bits3(replay.positions[0]) == bits3((0, 0, 0))
    assert replay.vertex_normals[0] == original[0]
    try:
        replay.apply({"selected": [1, 1], "positions": []})
    except ValueError:
        pass
    else:
        raise AssertionError("duplicate selected IDs were accepted")
    with tempfile.TemporaryDirectory() as temporary:
        directory = Path(temporary)
        oracle = Replay(mesh)
        product = {"wasm_sha256": "synthetic-self-test"}
        result = {"fixture": {"vertices": 4}, "product_binding": product,
                  "worker_trial": {"acknowledgedDabs": 1},
                  "worker_trace": {"messages": [{"direction": "worker-to-main",
                                                "kind": "apply", "success": True, "id": 2}]}}
        gpu = {"stages": {}}
        def save(stage):
            path = directory / f"{stage}.obrush"
            snapshot = dict(mesh)
            snapshot["positions"] = [list(row) for row in oracle.positions]
            payload = json.dumps({"mesh": snapshot}).encode()
            path.write_bytes(payload)
            result[stage] = {"path": str(path), "sha256": hashlib.sha256(payload).hexdigest()}
            gpu["stages"][stage] = {
                "position_words_sha256": digest_rows(oracle.positions),
                "normal_words_sha256": digest_rows(oracle.vertex_normals),
                "active_vbo_candidates": 1, "gl_error": 0}
        save("before")
        dab = {"kind": "dab", "sequence": 1, "worker_message_id": 2,
               "selected": [0], "positions": [[0, 0, 0, 0.5]]}
        undo = {"kind": "history", "action": "undo", "selected": [0],
                "positions": [[0, 0, 0, 0]]}
        redo = {"kind": "history", "action": "redo", "selected": [0],
                "positions": [[0, 0, 0, 0.5]]}
        oracle.apply(dab)
        save("after")
        save("stable")
        oracle.apply(undo)
        save("undo")
        oracle.apply(redo)
        save("redo")
        (directory / "result.json").write_text(json.dumps(result))
        (directory / "gpu-browser.json").write_text(json.dumps(gpu))
        trace = {"schema": SCHEMA,
                 "result_sha256": hashlib.sha256((directory / "result.json").read_bytes()).hexdigest(),
                 "before_sha256": hashlib.sha256((directory / "before.obrush").read_bytes()).hexdigest(),
                 "product_binding": product, "accepted_dabs": 1,
                 "events": [dab, {"kind": "checkpoint", "stage": "after"},
                            {"kind": "checkpoint", "stage": "stable"}, undo,
                            {"kind": "checkpoint", "stage": "undo"}, redo,
                            {"kind": "checkpoint", "stage": "redo"}]}
        trace_path = directory / "source-display-replay-trace-v1.json"
        trace_path.write_text(json.dumps(trace))
        assert inspect(directory)["status"] == "replay_completed"
        trace["events"][0]["selected"] = [0, 0]
        trace_path.write_text(json.dumps(trace))
        try:
            inspect(directory)
        except ValueError:
            pass
        else:
            raise AssertionError("duplicate replay selection was accepted")
    return {"status": "self_test_passed", "initial_hash": digest_rows(original),
            "final_hash": digest_rows(replay.vertex_normals),
            "complete_synthetic_receipt": True, "tampered_selection_rejected": True}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("receipt", nargs="?", type=Path)
    parser.add_argument("--trace", type=Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    try:
        report = self_test() if args.self_test else inspect(args.receipt, args.trace)
    except (OSError, KeyError, TypeError, ValueError) as error:
        report = {"schema": SCHEMA, "status": "invalid_replay_evidence", "error": str(error)}
    print(json.dumps(report, indent=2, allow_nan=False))
    return 0 if report["status"] in ("self_test_passed", "replay_completed") else 1


if __name__ == "__main__":
    raise SystemExit(main())
