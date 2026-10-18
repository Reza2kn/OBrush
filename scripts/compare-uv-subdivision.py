"""Compare actual reference and OBrush browser face-corner UV outputs.

Usage: python3 scripts/compare-uv-subdivision.py REFERENCE_RESULT HOST_RESULT
Both inputs are independent, immutable harness receipts. UV array order and
imported vertex IDs are intentionally irrelevant; physical face/corner mapping
is compared instead. This does not inspect texture pixels or latency.
"""
import collections
import hashlib
import json
import pathlib
import struct
import sys


def digest(path):
    return hashlib.sha256(pathlib.Path(path).read_bytes()).hexdigest()


def source_mesh(item):
    if digest(item["path"]) != item["sha256"]:
        raise ValueError("Reference snapshot hash changed")
    data = json.loads(pathlib.Path(item["path"]).read_text())
    mesh = data["meshes"][0]
    return {
        "positions": [mesh["positions"][i:i + 3] for i in range(0, len(mesh["positions"]), 3)],
        "faces": [mesh["faces"][i:i + 4] for i in range(0, len(mesh["faces"]), 4)],
        "uvs": [mesh["uvs"][i:i + 2] for i in range(0, len(mesh["uvs"]), 2)],
        "face_uvs": [mesh["face_uvs"][i:i + 4] for i in range(0, len(mesh["face_uvs"]), 4)],
        "colors": [mesh["colors"][i:i + 3] for i in range(0, len(mesh["colors"]), 3)],
        "materials": [mesh["materials"][i:i + 3] for i in range(0, len(mesh["materials"]), 3)],
    }


def host_mesh(item):
    if digest(item["path"]) != item["sha256"]:
        raise ValueError("OBRush browser project hash changed")
    document = json.loads(pathlib.Path(item["path"]).read_text())
    mesh = document["mesh"]
    return {
        "positions": mesh["positions"],
        "faces": mesh["surface"]["faces"],
        "uvs": mesh["surface"]["uv"]["coordinates"],
        "face_uvs": mesh["surface"]["uv"]["face_indices"],
        "colors": mesh["colors"],
        "materials": mesh.get("materials"),
        "masks": mesh["masks"],
        "mask_convention": mesh.get("mask_convention", "protected_f32_v1"),
    }


def rows(mesh):
    result = collections.defaultdict(list)
    for face, face_uv in zip(mesh["faces"], mesh["face_uvs"]):
        corners = [(mesh["positions"][point], mesh["uvs"][uv])
                   for point, uv in zip(face, face_uv) if point != 4294967295]
        key = tuple(sorted(tuple(round(x, 5) for x in point) for point, _ in corners))
        result[key].append(corners)
    return result


def compare(reference, host):
    if len(reference["uvs"]) != len(host["uvs"]):
        raise ValueError("UV coordinate counts differ")
    ref_rows, host_rows = rows(reference), rows(host)
    if set(ref_rows) != set(host_rows):
        raise ValueError(f"Physical face sets differ: {len(set(ref_rows) - set(host_rows))} reference-only, {len(set(host_rows) - set(ref_rows))} host-only")
    maximum_position = maximum_uv = 0.0
    corners_compared = 0
    for key in ref_rows:
        if len(ref_rows[key]) != 1 or len(host_rows[key]) != 1:
            raise ValueError("Ambiguous duplicate geometric face; comparator requires a unique face key")
        reference_corners = {tuple(round(x, 5) for x in point): (point, uv)
                             for point, uv in ref_rows[key][0]}
        host_corners = {tuple(round(x, 5) for x in point): (point, uv)
                        for point, uv in host_rows[key][0]}
        if set(reference_corners) != set(host_corners):
            raise ValueError("Matched face has different physical corner keys")
        for corner in reference_corners:
            rp, ru = reference_corners[corner]
            hp, hu = host_corners[corner]
            maximum_position = max(maximum_position, *(abs(a - b) for a, b in zip(rp, hp)))
            maximum_uv = max(maximum_uv, *(abs(a - b) for a, b in zip(ru, hu)))
            corners_compared += 1
    return {"faces": len(ref_rows), "corners": corners_compared,
            "reference_uv_coordinates": len(reference["uvs"]),
            "host_uv_coordinates": len(host["uvs"]),
            "max_position_component_error": maximum_position,
            "max_uv_component_error": maximum_uv}


def compare_vertex_channels(reference, host):
    def index(mesh):
        result = {}
        for vertex, position in enumerate(mesh["positions"]):
            key = tuple(round(value, 5) for value in position)
            if key in result:
                raise ValueError("Duplicate physical vertex key in channel comparison")
            result[key] = vertex
        return result

    source, target = index(reference), index(host)
    if set(source) != set(target):
        raise ValueError("Channel comparison vertex sets differ")
    errors = {"rgb": 0.0, "roughness": 0.0, "metalness": 0.0, "unmasked": 0.0}
    for key, i in source.items():
        j = target[key]
        errors["rgb"] = max(errors["rgb"], *(abs(a - b) for a, b in zip(reference["colors"][i], host["colors"][j])))
        material = host["materials"][j] if host["materials"] is not None else [0.18, 0.08]
        errors["roughness"] = max(errors["roughness"], abs(reference["materials"][i][0] - material[0]))
        errors["metalness"] = max(errors["metalness"], abs(reference["materials"][i][1] - material[1]))
        unmasked = host["masks"][j] if host["mask_convention"] == "unprotected_f32_v1" else 1 - host["masks"][j]
        errors["unmasked"] = max(errors["unmasked"], abs(reference["materials"][i][2] - unmasked))
    return {"vertices": len(source), "max_component_error": errors}


def navigation(reference_case):
    for stage in ("before", "fine", "coarse", "returned"):
        item = reference_case[stage]
        if digest(item["path"]) != item["sha256"]:
            raise ValueError("Reference navigation snapshot hash changed: " + stage)
    before = json.loads(pathlib.Path(reference_case["before"]["path"]).read_text())["meshes"][0]
    fine = json.loads(pathlib.Path(reference_case["fine"]["path"]).read_text())["meshes"][0]
    coarse = json.loads(pathlib.Path(reference_case["coarse"]["path"]).read_text())["meshes"][0]
    returned = json.loads(pathlib.Path(reference_case["returned"]["path"]).read_text())["meshes"][0]

    def differences(a, b, name):
        if len(a) != len(b):
            raise ValueError(f"Reference {name} changed array length")
        return {"different_components": sum(x != y for x, y in zip(a, b)),
                "max_component_error": max((abs(x - y) for x, y in zip(a, b)), default=0)}

    return {"lower_positions": differences(before["positions"], coarse["positions"], "lower positions"),
            "return_positions": differences(fine["positions"], returned["positions"], "return positions"),
            "lower_uv_exact": before["uvs"] == coarse["uvs"] and before["face_uvs"] == coarse["face_uvs"],
            "return_uv_exact": fine["uvs"] == returned["uvs"] and fine["face_uvs"] == returned["face_uvs"]}


def native_browser_words(native, browser):
    results = {}
    for field in ("positions", "uvs", "colors", "materials", "masks"):
        def words(values):
            return [struct.pack("<f", value) for row in values
                    for value in (row if isinstance(row, list) else [row])]
        left = words(native[field]) if native[field] is not None else None
        right = words(browser[field]) if browser[field] is not None else None
        results[field + "_f32_words_exact"] = left == right
    for field in ("faces", "face_uvs"):
        results[field + "_indices_exact"] = native[field] == browser[field]
    return results


def main():
    if len(sys.argv) not in (3, 4, 5):
        raise SystemExit(__doc__)
    native_dir = next((arg for arg in sys.argv[3:] if arg != "--navigation"), None)
    navigation_requested = "--navigation" in sys.argv[3:]
    reference_path, host_path = map(pathlib.Path, sys.argv[1:3])
    reference = json.loads(reference_path.read_text())
    host = json.loads(host_path.read_text())
    host_cases = {case["name"]: case for case in host["cases"]}
    output = {"reference_result_sha256": digest(reference_path),
              "host_result_sha256": digest(host_path), "cases": []}
    for case in reference["cases"]:
        if not case["name"].startswith("reference-uv-subdivision-"):
            continue
        name = "paired-uv-" + pathlib.Path(case["fixture"]["path"]).stem
        paired = host_cases[name]
        input_comparison = compare(source_mesh(case["before"]), host_mesh(paired["before"]))
        child_comparison = compare(source_mesh(case["fine"]), host_mesh(paired["fine"]))
        item = {"name": name, "fixture_sha256": case["fixture"]["sha256"],
                                "input": input_comparison, "child": child_comparison,
                                "input_channels": compare_vertex_channels(source_mesh(case["before"]), host_mesh(paired["before"])),
                                "child_channels": compare_vertex_channels(source_mesh(case["fine"]), host_mesh(paired["fine"])),
                                "reference_navigation": navigation(case),
                                "host_gates": paired["gates"]}
        if navigation_requested:
            item["coarse"] = compare(source_mesh(case["coarse"]), host_mesh(paired["coarse"]))
            item["returned"] = compare(source_mesh(case["returned"]), host_mesh(paired["returned"]))
            item["coarse_channels"] = compare_vertex_channels(source_mesh(case["coarse"]), host_mesh(paired["coarse"]))
            item["returned_channels"] = compare_vertex_channels(source_mesh(case["returned"]), host_mesh(paired["returned"]))
        if native_dir:
            native_path = pathlib.Path(native_dir) / (pathlib.Path(case["fixture"]["path"]).stem + "-obrush.json")
            native = json.loads(native_path.read_text())
            item["native_probe_sha256"] = digest(native_path)
            item["native_browser"] = {
                "parent": native_browser_words(native["parent"], host_mesh(paired["before"])),
                "child": native_browser_words(native["child"], host_mesh(paired["fine"])),
            }
            if not all(all(check.values()) for check in item["native_browser"].values()):
                raise ValueError("Native and browser Rust word arrays differ: " + name)
        output["cases"].append(item)
    if len(output["cases"]) != 3:
        raise ValueError("Expected three paired UV subdivision fixtures")
    print(json.dumps(output, indent=2))


if __name__ == "__main__":
    main()
