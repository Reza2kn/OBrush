"""Compare independently captured SculptGL and OBrush two-step seam Reverse.

Usage: python3 scripts/compare-reverse-seam.py REFERENCE_RESULT HOST_RESULT
The source's first reversed level currently exposes one out-of-range face UV
index; report that failure instead of silently accepting partial UV parity.
"""
import collections
import hashlib
import json
import pathlib
import sys


def read_record(record):
    raw = pathlib.Path(record["path"]).read_bytes()
    if hashlib.sha256(raw).hexdigest() != record["sha256"]:
        raise ValueError(f"Snapshot hash changed: {record['path']}")
    return json.loads(raw)


def fixture(receipt):
    target = "independent-reverse-two-step-seam-grid.obj"
    return next(item for item in receipt["fixtures"]
                if pathlib.Path(item["path"]).name == target)


def source(record):
    mesh = read_record(record)["meshes"][0]
    return {
        "positions": [mesh["positions"][i:i + 3] for i in range(0, len(mesh["positions"]), 3)],
        "faces": [mesh["faces"][i:i + 4] for i in range(0, len(mesh["faces"]), 4)],
        "uvs": [mesh["uvs"][i:i + 2] for i in range(0, len(mesh["uvs"]), 2)],
        "face_uvs": [mesh["face_uvs"][i:i + 4] for i in range(0, len(mesh["face_uvs"]), 4)],
    }


def host(record):
    mesh = read_record(record)["mesh"]
    return {
        "positions": mesh["positions"],
        "faces": mesh["surface"]["faces"],
        "uvs": mesh["surface"]["uv"]["coordinates"],
        "face_uvs": mesh["surface"]["uv"]["face_indices"],
    }


def cycles(mesh):
    rows = {}
    partition = collections.defaultdict(set)
    invalid = []
    for face_id, (face, uv_row) in enumerate(zip(mesh["faces"], mesh["face_uvs"])):
        points = [mesh["positions"][vertex] for vertex in face]
        xy = [tuple(round(value, 6) for value in point[:2]) for point in points]
        key = min(tuple(xy[offset:] + xy[:offset]) for offset in range(4))
        if key in rows:
            raise ValueError("Duplicate physical face")
        corners = {}
        for corner, (point, uv_id) in enumerate(zip(points, uv_row)):
            corner_key = (key, xy[corner])
            if uv_id >= len(mesh["uvs"]):
                invalid.append({"face": face_id, "corner": corner, "uv_id": uv_id})
                corners[xy[corner]] = (point, None)
            else:
                corners[xy[corner]] = (point, mesh["uvs"][uv_id])
                partition[uv_id].add(corner_key)
        rows[key] = corners
    return rows, invalid, {frozenset(value) for value in partition.values()}


def compare(reference, obrush):
    ref_rows, invalid, ref_partition = cycles(reference)
    host_rows, host_invalid, host_partition = cycles(obrush)
    if set(ref_rows) != set(host_rows):
        raise ValueError("Oriented physical face sets differ")
    if host_invalid:
        raise ValueError("OBRush saved an out-of-range face UV index")
    max_position = max_uv = 0.0
    matched_uv = 0
    uv_differences = []
    for face in ref_rows:
        if set(ref_rows[face]) != set(host_rows[face]):
            raise ValueError("Physical face corner sets differ")
        for corner in ref_rows[face]:
            ref_point, ref_uv = ref_rows[face][corner]
            host_point, host_uv = host_rows[face][corner]
            max_position = max(max_position, *(abs(a - b) for a, b in zip(ref_point, host_point)))
            if ref_uv is not None:
                difference = max(abs(a - b) for a, b in zip(ref_uv, host_uv))
                max_uv = max(max_uv, difference)
                if difference:
                    uv_differences.append({"face": face, "corner": corner,
                                           "reference_uv": ref_uv, "obrush_uv": host_uv,
                                           "max_component_error": difference})
                matched_uv += 1
    return {
        "faces": len(ref_rows), "corners": len(ref_rows) * 4,
        "matched_uv_corners": matched_uv, "reference_invalid_uv_indices": invalid,
        "reference_uv_coordinates": len(reference["uvs"]),
        "obrush_uv_coordinates": len(obrush["uvs"]),
        "uv_sharing_partition_equal": ref_partition == host_partition,
        "max_position_component_error": max_position,
        "max_valid_uv_component_error": max_uv,
        "uv_differences": uv_differences,
        "all_uv_corners_compared": not invalid,
    }


def main():
    ref_path, host_path = map(pathlib.Path, sys.argv[1:3])
    ref = json.loads(ref_path.read_text())
    ob = json.loads(host_path.read_text())
    ref_fixture, host_fixture = fixture(ref), fixture(ob)
    if ref_fixture["sha256"] != host_fixture["sha256"]:
        raise ValueError("Fixture bytes differ")
    ref_case = next(case for case in ref["cases"] if case["name"] == "reference-reverse-two-step-seam-grid")
    host_case = next(case for case in ob["cases"] if case["name"] == "reverse-two-step-seam-grid-normal-browser")
    stage_pairs = (("before", "before"), ("first", "first"),
                   ("second", "second"), ("returned", "fine"))
    stages = {name: compare(source(ref_case[name]), host(host_case[host_name]))
              for name, host_name in stage_pairs}
    print(json.dumps({
        "fixture_sha256": ref_fixture["sha256"],
        "reference_result_sha256": hashlib.sha256(ref_path.read_bytes()).hexdigest(),
        "host_result_sha256": hashlib.sha256(host_path.read_bytes()).hexdigest(),
        "stages": stages,
    }, indent=2))


if __name__ == "__main__":
    main()
