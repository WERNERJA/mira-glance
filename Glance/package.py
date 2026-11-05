#!/usr/bin/env python3
"""Build Glance: compile the Rust module and embed it with the phone companion."""
from pathlib import Path
import base64, subprocess, sys, tempfile
from validate import validate_html
root = Path(__file__).resolve().parent
output = Path(sys.argv[1] if len(sys.argv) > 1 else 'Glance.html')
with tempfile.TemporaryDirectory(prefix='wasm-glance-') as folder:
    module = Path(folder) / 'glance.wasm'
    subprocess.run(['rustc', str(root / 'glance.rs'), '--edition=2021', '--target=wasm32-unknown-unknown',
        '--crate-type=cdylib', '-C', 'opt-level=z', '-C', 'panic=abort', '-C', 'lto=fat', '-C', 'codegen-units=1',
        '-C', 'link-arg=--no-entry', '-C', 'link-arg=--strip-all',
        '-C', 'target-feature=-bulk-memory,-bulk-memory-opt,-reference-types',
        '-C', 'link-arg=-zstack-size=8192', '-C', 'link-arg=--initial-memory=65536',
        '-C', 'link-arg=--max-memory=65536', '-o', str(module)], check=True)
    encoded = base64.b64encode(module.read_bytes()).decode()
    companion = (root / 'companion.html').read_text(encoding='utf-8')
    html = ('<!doctype html><html lang="nl"><head><meta charset="utf-8">'
        '<meta name="viewport" content="width=device-width,initial-scale=1">\n'
        '<script type="application/mira-manifest+json">{"title":"Glance"}</script>\n'
        '<title>Glance</title></head><body>\n' + companion +
        '\n<script type="application/mira-wasm">' + encoded + '</script></body></html>')
    validate_html(html)
    output.write_text(html, encoding='utf-8')
print(output)
