#!/usr/bin/env python3
"""Serve the local Rust/WASM build on localhost without exposing the repository."""
import argparse
import http.server
from functools import partial
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--port', type=int, default=8787)
args = parser.parse_args()
root = Path(__file__).resolve().parents[1] / 'web' / 'dist'
if not (root / 'index.html').is_file():
    parser.error('Browser build missing. Run scripts/build-web.sh first.')
handler = partial(http.server.SimpleHTTPRequestHandler, directory=str(root))
with http.server.ThreadingHTTPServer(('127.0.0.1', args.port), handler) as server:
    print(f'OBrush: http://127.0.0.1:{args.port}', flush=True)
    server.serve_forever()
