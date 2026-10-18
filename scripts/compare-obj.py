#!/usr/bin/env python3
"""Compare an ordered OBJ round-trip. This does not infer vertex correspondence."""
import argparse
import hashlib
import json
import math
from pathlib import Path


def read_mesh(path):
    positions, faces, colors = [], [], []
    mrgb = ''
    for line in path.read_text().splitlines():
        fields = line.split()
        if not fields:
            continue
        if fields[0] == 'v':
            positions.append(tuple(map(float, fields[1:4])))
            if len(fields) >= 7:
                colors.append(tuple(map(float, fields[4:7])))
        elif fields[0] == 'f':
            # This fixture uses positive indices. Normalize legal negatives too.
            indices = [int(value.split('/')[0]) for value in fields[1:]]
            faces.append(tuple(value - 1 if value > 0 else len(positions) + value for value in indices))
        elif fields[0] == '#MRGB':
            mrgb += ''.join(fields[1:])
    if mrgb:
        if len(mrgb) != len(positions) * 8:
            raise ValueError('MRGB stream length does not match vertex count')
        colors = [tuple(int(mrgb[i + offset:i + offset + 2], 16) / 255 for offset in (2, 4, 6)) for i in range(0, len(mrgb), 8)]
    return positions, faces, colors


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('source', type=Path)
parser.add_argument('roundtrip', type=Path)
args = parser.parse_args()
a, af, ac = read_mesh(args.source)
b, bf, bc = read_mesh(args.roundtrip)
result = {
    'source_sha256': hashlib.sha256(args.source.read_bytes()).hexdigest(),
    'roundtrip_sha256': hashlib.sha256(args.roundtrip.read_bytes()).hexdigest(),
    'source_vertices': len(a), 'roundtrip_vertices': len(b),
    'source_faces': len(af), 'roundtrip_faces': len(bf),
    'topology_indices_identical': af == bf,
    'source_colors': len(ac), 'roundtrip_colors': len(bc),
    'assumption': 'Vertex order corresponds; no reordering or registration is inferred.',
}
if len(a) == len(b) and a:
    errors = [math.dist(x, y) for x, y in zip(a, b)]
    result['max_position_error'] = max(errors)
    result['rms_position_error'] = math.sqrt(sum(value * value for value in errors) / len(errors))
if len(ac) == len(bc) and ac:
    result['max_color_channel_error'] = max(abs(x - y) for first, second in zip(ac, bc) for x, y in zip(first, second))
print(json.dumps(result, indent=2))
