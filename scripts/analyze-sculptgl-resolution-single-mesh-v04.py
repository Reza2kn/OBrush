#!/usr/bin/env python3
"""Frozen V04 CSS-pixel analysis of ordinary browser screenshots.

Requires Pillow, NumPy and OpenCV on the local verification host. The browser
runner writes untouched screenshots and scene rectangles to result.json.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from tempfile import TemporaryDirectory
import traceback

import cv2
import numpy as np
from PIL import Image


def check(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def file_sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def normalize(path: Path, dpr: int, rect: list[float], output: Path) -> np.ndarray:
    image = Image.open(path).convert("RGB")
    check(image.size == (1600 * dpr, 1000 * dpr), f"{path.name}: screenshot dimensions {image.size}")
    if dpr == 2:
        image = image.resize((1600, 1000), Image.Resampling.BOX)
    x, y, width, height = [int(round(value)) for value in rect]
    check(width > 0 and height > 0 and 0 <= x < x + width <= 1600 and 0 <= y < y + height <= 1000,
          f"{path.name}: scene rectangle {rect}")
    image = image.crop((x, y, x + width, y + height))
    image.save(output)
    return np.asarray(image, dtype=np.uint8)


def foreground(rgb: np.ndarray, contact: tuple[int, int], output: Path) -> tuple[np.ndarray, dict]:
    h, w, _ = rgb.shape
    check(h >= 40 and w >= 40, "scene too small for fixed corner samples")
    corners = np.concatenate((rgb[:20, :20].reshape(-1, 3), rgb[:20, -20:].reshape(-1, 3),
                              rgb[-20:, :20].reshape(-1, 3), rgb[-20:, -20:].reshape(-1, 3)))
    bg = np.median(corners, axis=0)
    distance = np.linalg.norm(rgb.astype(np.float32) - bg, axis=2)
    binary = np.uint8(distance > 20)
    count, labels, stats, _ = cv2.connectedComponentsWithStats(binary, 8)
    check(count >= 2, "no foreground component")
    chosen = 1 + int(np.argmax(stats[1:, cv2.CC_STAT_AREA]))
    area = int(stats[chosen, cv2.CC_STAT_AREA])
    large_other = [int(stats[i, cv2.CC_STAT_AREA]) for i in range(1, count)
                   if i != chosen and stats[i, cv2.CC_STAT_AREA] > area * 0.005]
    x, y = contact
    mask = labels == chosen
    ratio = area / (h * w)
    check(0.15 <= ratio <= 0.75, f"foreground area ratio {ratio:.6f} outside [0.15,0.75]")
    check(0 <= x < w and 0 <= y < h and bool(mask[y, x]), "contact outside largest foreground")
    check(not large_other, f"second large foreground component(s) {large_other}")
    eroded = cv2.erode(mask.astype(np.uint8), np.ones((25, 25), np.uint8)) != 0
    interior = int(eroded.sum())
    check(interior >= 20_000, f"eroded interior only {interior} pixels")
    Image.fromarray(np.uint8(mask) * 255, "L").save(output)
    return mask, {"background_median_rgb": bg.tolist(), "area": area, "area_fraction": ratio,
                  "other_large_areas": large_other, "eroded_area": interior}


def color_metric(a: np.ndarray, b: np.ndarray, region: np.ndarray, outlier: int) -> dict:
    check(a.shape == b.shape and region.shape == a.shape[:2], "image/region dimensions differ")
    check(bool(np.any(region)), "empty image comparison region")
    delta = np.max(np.abs(a.astype(np.int16) - b.astype(np.int16)), axis=2)[region]
    return {"pixels": int(delta.size), "mean": float(np.mean(delta)),
            "p99": float(np.percentile(delta, 99)),
            f"fraction_gt_{outlier}": float(np.mean(delta > outlier))}


def boundary(mask: np.ndarray) -> np.ndarray:
    return mask & (cv2.erode(mask.astype(np.uint8), np.ones((3, 3), np.uint8)) == 0)


def silhouette_metric(a: np.ndarray, b: np.ndarray) -> dict:
    check(a.shape == b.shape, "silhouette dimensions differ")
    ba, bb = boundary(a), boundary(b)
    check(bool(np.any(ba)) and bool(np.any(bb)), "missing silhouette boundary")
    da = cv2.distanceTransform(np.uint8(~ba), cv2.DIST_L2, cv2.DIST_MASK_PRECISE)[bb]
    db = cv2.distanceTransform(np.uint8(~bb), cv2.DIST_L2, cv2.DIST_MASK_PRECISE)[ba]
    distances = np.concatenate((da, db))
    return {"samples": int(distances.size), "p95_css_px": float(np.percentile(distances, 95)),
            "max_css_px": float(np.max(distances)), "area_a": int(a.sum()), "area_b": int(b.sum())}


def self_check() -> None:
    rgb = np.full((400, 400, 3), 24, np.uint8)
    cv2.circle(rgb, (200, 200), 120, (140, 110, 90), -1)
    with TemporaryDirectory() as tmp:
        full = np.full((1000, 1600, 3), 24, np.uint8)
        full[40:440, 100:500] = rgb
        input_path, output_path = Path(tmp) / "input.png", Path(tmp) / "css.png"
        Image.fromarray(full, "RGB").save(input_path)
        normalized = normalize(input_path, 1, [100, 40, 400, 400], output_path)
        check(np.array_equal(normalized, rgb), "scene crop normalization")
        mask, info = foreground(rgb, (200, 200), Path(tmp) / "mask.png")
        check(info["area"] > 40_000, "synthetic sphere area")
        check(color_metric(rgb, rgb.copy(), mask, 64)["mean"] == 0, "identity color")
        shifted = np.roll(mask, 2, axis=1)
        metric = silhouette_metric(mask, shifted)
        check(1.9 <= metric["max_css_px"] <= 2.1, f"synthetic boundary distance {metric}")
    print("V04 pixel-analysis self-check passed")


def analyze(receipt: Path) -> dict:
    result = json.loads(receipt.read_text())
    dpr = result["dpr"]
    check(dpr in (1, 2), "DPR outside frozen matrix")
    states = {case["name"]: case["state"] for case in result["cases"] if "name" in case}
    images: dict[str, np.ndarray] = {}
    masks: dict[str, np.ndarray] = {}
    details: dict[str, dict] = {}
    for app in ("reference", "obrush"):
        for ratio in ("1", "0.5", "2"):
            before_name = f"{app}-{ratio}-before"
            state = states[before_name]
            if app == "reference":
                rect = state["rect"]
                pointer = [round(rect[2] / 2), round(rect[3] / 2)]
            else:
                vp = state["camera"]["viewport"]
                rect = [vp[0], vp[1], vp[2] - vp[0], vp[3] - vp[1]]
                pointer = [round(rect[2] / 2), round(rect[3] / 2)]
            for phase in ("before", "after", "stable"):
                name = f"{app}-{ratio}-{phase}"
                normalized_path = receipt.parent / f"{name}-css.png"
                mask_path = receipt.parent / f"{name}-mask.png"
                image = normalize(receipt.parent / f"{name}.png", dpr, rect, normalized_path)
                images[name] = image
                mask, info = foreground(image, tuple(pointer), mask_path)
                masks[name] = mask
                details[name] = {**info, "normalized_sha256": file_sha(normalized_path),
                                 "mask_sha256": file_sha(mask_path)}
    comparisons = []
    for app in ("reference", "obrush"):
        for ratio in ("0.5", "2"):
            a_mask = masks[f"{app}-1-before"]
            b_mask = masks[f"{app}-{ratio}-before"]
            check(a_mask.shape == b_mask.shape, f"{app} scene crop differs across ratio")
            region = cv2.erode(np.uint8(a_mask & b_mask), np.ones((25, 25), np.uint8)) != 0
            check(int(region.sum()) >= 20_000, f"{app} {ratio} pair interior below 20,000")
            for phase in ("before", "after"):
                metric = color_metric(images[f"{app}-1-{phase}"], images[f"{app}-{ratio}-{phase}"], region, 64)
                limit = (16, 64, .02) if ratio == "0.5" else (8, 40, .005)
                check(metric["mean"] <= limit[0] and metric["p99"] <= limit[1] and metric["fraction_gt_64"] <= limit[2],
                      f"{app} {ratio} {phase} color bound {metric} > {limit}")
                comparisons.append({"kind": "scale_color", "app": app, "ratio": ratio, "phase": phase, **metric})
        for ratio in ("1", "0.5"):
            for phase in ("before", "after"):
                silhouette = silhouette_metric(masks[f"{app}-{ratio}-{phase}"], masks[f"{app}-2-{phase}"])
                boundary_limit = (4, 10) if ratio == "0.5" else (2, 6)
                check(silhouette["p95_css_px"] <= boundary_limit[0] and silhouette["max_css_px"] <= boundary_limit[1],
                      f"{app} {ratio} {phase} silhouette bound {silhouette} > {boundary_limit}")
                comparisons.append({"kind": "silhouette", "app": app, "ratio": ratio, "phase": phase, **silhouette})
        for ratio in ("1", "0.5", "2"):
            region = cv2.erode(masks[f"{app}-{ratio}-before"].astype(np.uint8),
                               np.ones((25, 25), np.uint8)) != 0
            metric = color_metric(images[f"{app}-{ratio}-after"], images[f"{app}-{ratio}-stable"], region, 32)
            check(metric["mean"] <= 2 and metric["p99"] <= 8 and metric["fraction_gt_32"] <= .001,
                  f"{app} {ratio} static stability bound {metric}")
            comparisons.append({"kind": "static_color", "app": app, "ratio": ratio, **metric})
    return {"status": "passed V04 pixel bounds", "dpr": dpr, "screenshots": details, "comparisons": comparisons}


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("receipt", type=Path, nargs="?")
    parser.add_argument("--self-check", action="store_true")
    args = parser.parse_args()
    if args.self_check:
        self_check()
    else:
        check(args.receipt is not None, "pass browser result.json")
        try:
            output = analyze(args.receipt)
        except Exception as error:
            output = {"status": "failed V04 pixel bounds", "error": str(error), "traceback": traceback.format_exc()}
        target = args.receipt.parent / "pixel-analysis.json"
        target.write_text(json.dumps(output, indent=2) + "\n")
        print(json.dumps({"status": output["status"], "output": str(target)}))
        if output["status"].startswith("failed"):
            raise SystemExit(1)
