"""Compare two- or three-level controlled coarse edit on independent UV fixtures.

Usage: python3 scripts/compare-uv-two-level.py REF_RESULT PROBE_DIR OUTPUT
The direct coarse edit isolates hierarchy synthesis from brush response.
"""
import hashlib
import json
import math
import pathlib
import sys


def sha(path):
    return hashlib.sha256(pathlib.Path(path).read_bytes()).hexdigest()


def positions(record):
    path = pathlib.Path(record["path"])
    if sha(path) != record["sha256"]:
        raise ValueError(f"Reference snapshot changed: {path}")
    mesh = json.loads(path.read_text())["meshes"][0]
    values = mesh["positions"]
    return [values[i:i + 3] for i in range(0, len(values), 3)]


def pair(reference, host):
    if len(reference) != len(host):
        raise ValueError("Vertex counts differ")
    by_position = {}
    for index, point in enumerate(host):
        key = tuple(round(value, 5) for value in point)
        if key in by_position:
            raise ValueError(f"Ambiguous host vertex position: {key}")
        by_position[key] = index
    mapping = []
    for index, point in enumerate(reference):
        key = tuple(round(value, 5) for value in point)
        if key not in by_position:
            raise ValueError(f"Reference vertex {index} lacks host match: {point}")
        mapping.append(by_position[key])
    if len(set(mapping)) != len(mapping):
        raise ValueError("Physical mapping is not bijective")
    return mapping


def error(ref_before, ref_after, host_before, host_after, mapping):
    initial, final, displacement = [], [], []
    for ref_id, host_id in enumerate(mapping):
        for component in range(3):
            initial.append(abs(ref_before[ref_id][component] - host_before[host_id][component]))
            final.append(abs(ref_after[ref_id][component] - host_after[host_id][component]))
            displacement.append(abs(
                (ref_after[ref_id][component] - ref_before[ref_id][component])
                - (host_after[host_id][component] - host_before[host_id][component])
            ))
    return {name: {
        "max": max(values),
        "rms": math.sqrt(sum(value * value for value in values) / len(values)),
    } for name, values in (("before", initial), ("after", final), ("displacement", displacement))}


def main():
    reference_path, probe_dir, output_path = map(pathlib.Path, sys.argv[1:])
    reference = json.loads(reference_path.read_text())
    three = reference["status"] == "completed reference-only controlled three-level edit fixtures"
    if not three and reference["status"] != "completed reference-only controlled two-level edit fixtures":
        raise ValueError("Reference run did not complete")
    reports = []
    for case in reference["cases"]:
        prefix = "reference-uv-three-level-edit-independent-seamed-" if three else "reference-uv-two-level-edit-independent-seamed-"
        if not case["name"].startswith(prefix):
            continue
        name = case["name"].removeprefix(prefix)
        probe = probe_dir / f"{name}.json"
        host = json.loads(probe.read_text())
        coarse_mapping = pair(positions(case["coarse"]), host["coarse_before"])
        reference_edit_vertex = case["edit"]["vertex"]
        host_edit_vertex = host["vertex"]
        if coarse_mapping[reference_edit_vertex] != host_edit_vertex:
            raise ValueError(
                f"{name}: physical coarse edit differs: reference vertex "
                f"{reference_edit_vertex} maps to host {coarse_mapping[reference_edit_vertex]}, "
                f"but host edited {host_edit_vertex}"
            )
        if case["edit"]["delta"] != host["delta"]:
            raise ValueError(f"{name}: coarse edit delta differs")
        levels = []
        level_keys = [
            ("first", "fine", "edited_fine"),
            ("second", "second_fine", "edited_second_fine"),
        ]
        if three:
            level_keys.append(("third", "third_fine", "edited_third_fine"))
        for label, before_key, after_key in level_keys:
            ref_before = positions(case[before_key])
            ref_after = positions(case[after_key])
            host_before, host_after = host[f"{label}_before"], host[f"{label}_after"]
            mapping = pair(ref_before, host_before)
            level = {"level": label, "vertices": len(mapping),
                     "errors": error(ref_before, ref_after, host_before, host_after, mapping)}
            if label in ("second", "third"):
                reference_neighbors = case[f"{label}_source_frame"]["first_neighbor"]
                host_neighbors = host[f"{label}_first_neighbor"]
                if len(reference_neighbors) != len(mapping) or len(host_neighbors) != len(mapping):
                    raise ValueError(f"{name}: {label} neighbor count differs")
                mismatches = sum(mapping[reference_neighbors[i]] != host_neighbors[host_id]
                                 for i, host_id in enumerate(mapping))
                level["first_neighbor_mismatches"] = mismatches
            levels.append(level)
        reports.append({"fixture": name, "reference_fixture_sha256": case["fixture"]["sha256"],
                        "host_probe_sha256": sha(probe),
                        "mapped_coarse_edit": {"reference_vertex": reference_edit_vertex,
                                               "host_vertex": host_edit_vertex,
                                               "delta": host["delta"]},
                        "levels": levels})
    if len(reports) != 3:
        raise ValueError("Expected all three independent fixtures")
    result = {"reference_result_sha256": sha(reference_path), "cases": reports,
              "scope": f"{3 if three else 2} source-static levels, one direct coarse vertex edit, no brush/history claim"}
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
