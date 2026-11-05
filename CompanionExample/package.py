#!/usr/bin/env python3
"""Build a standalone author example, not firmware or a bundled production app."""
from pathlib import Path
import base64,subprocess,sys,tempfile
from validate import validate_html
root=Path(__file__).resolve().parent
output=Path(sys.argv[1] if len(sys.argv)>1 else '/tmp/CompanionCounter.html')
with tempfile.TemporaryDirectory(prefix='wasm-companion-example-') as folder:
 module=Path(folder)/'counter.wasm'
 subprocess.run(['rustc',str(root/'counter.rs'),'--edition=2021','--target=wasm32-unknown-unknown','--crate-type=cdylib','-C','opt-level=z','-C','panic=abort','-C','lto=fat','-C','codegen-units=1','-C','link-arg=--no-entry','-C','link-arg=--strip-all','-C','target-feature=-bulk-memory,-bulk-memory-opt,-reference-types','-C','link-arg=-zstack-size=8192','-C','link-arg=--initial-memory=65536','-C','link-arg=--max-memory=65536','-o',str(module)],check=True)
 encoded=base64.b64encode(module.read_bytes()).decode()
 html='''<!doctype html><html><head><meta name="viewport" content="width=device-width,initial-scale=1">
<script type="application/mira-manifest+json">{"title":"Companion Counter"}</script>
<style>body{font:18px system-ui;padding:24px}button{font:inherit;padding:12px}</style></head>
<body><main id="mira-companion-root"><h1>Companion Counter</h1><p id="count">Connect and open on glasses</p><button id="bump">Add one</button></main>
<script>miraCompanion.onFromApp(function(m){if(m.type==='count')document.getElementById('count').textContent='Count '+m.n;});document.getElementById('bump').onclick=function(){miraCompanion.sendToApp({type:'bump'});};miraCompanion.sendToApp({type:'sync'});</script>
<script type="application/mira-wasm">'''+encoded+'</script></body></html>'
 validate_html(html)
 output.write_text(html)
print(output)
