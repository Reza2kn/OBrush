#!/usr/bin/env python3
"""Derived fixed-ROI visual diagnostics for the retained CR01 browser receipts."""
import json
import math
import sys
from pathlib import Path
from PIL import Image


def analyze(receipt_path):
    receipt_path = Path(receipt_path).resolve()
    directory = receipt_path.parent
    data = json.loads(receipt_path.read_text())
    side = 'source' if data['side'] == 'source' else 'obrush'
    center = data['path'][6]
    radius = data['frozen']['pixel_roi_radius']
    baseline = Image.open(directory / f'{side}-baseline.png').convert('RGB')
    final = Image.open(directory / f'{side}-stroke-8.png').convert('RGB')
    fourth = Image.open(directory / f'{side}-stroke-4.png').convert('RGB')
    x, y = center
    bounds = (math.floor(x-radius), math.floor(y-radius), math.ceil(x+radius), math.ceil(y+radius))
    background = baseline.getpixel((300, 500))
    total = sum_sq = max_abs = leaks_before = leaks_after = changed = 0
    for row in range(bounds[1], bounds[3]):
        for col in range(bounds[0], bounds[2]):
            a, b = baseline.getpixel((col,row)), final.getpixel((col,row))
            delta = [abs(a[k]-b[k]) for k in range(3)]
            max_abs = max(max_abs,*delta)
            sum_sq += sum(v*v for v in delta)
            total += 3
            changed += max(delta)>4
            if (col-x)**2+(row-y)**2<=150**2:
                leaks_before += max(abs(a[k]-background[k]) for k in range(3))<=8
                leaks_after += max(abs(b[k]-background[k]) for k in range(3))<=8
    crops = []
    for name,image in [('baseline',baseline),('stroke-4',fourth),('stroke-8',final)]:
        output=directory/f'{side}-{name}-roi.png'
        image.crop(bounds).resize(((bounds[2]-bounds[0])*2,(bounds[3]-bounds[1])*2),Image.Resampling.NEAREST).save(output)
        crops.append(str(output))
    return {'side':side,'receipt':str(receipt_path),'center_css':center,'roi':bounds,'changed_pixels_gt4':changed,'max_channel_delta':max_abs,'rgb_rms':math.sqrt(sum_sq/total),'interior_background_like_pixels_baseline':leaks_before,'interior_background_like_pixels_final':leaks_after,'background_sample_rgb':background,'crops':crops}


if __name__=='__main__':
    if len(sys.argv)!=3:
        raise SystemExit('Usage: analyze-sculptgl-crease-overlap-cr01.py SOURCE_RESULT.json OBRUSH_RESULT.json')
    for arg in sys.argv[1:]:
        q=analyze(arg)
        output=Path(arg).parent/'derived-visual-analysis.json'
        output.write_text(json.dumps(q,indent=2)+'\n')
        print(json.dumps(q))
