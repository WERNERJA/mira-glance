#!/usr/bin/env python3
"""Check the final Mira HTML using Python stdlib and Node's parsers."""
from pathlib import Path
import base64
import json
import os
import re
import shutil
import subprocess
import sys

class InvalidApp(ValueError):
    pass

def find_node():
    configured = os.environ.get('NODE_BINARY')
    candidates = [
        configured,
        shutil.which('node'),
        '/Applications/Codex.app/Contents/Resources/cua_node/bin/node',
        '/Applications/ChatGPT.app/Contents/Resources/cua_node/bin/node',
    ]
    return next((candidate for candidate in candidates
                 if candidate and os.path.isfile(candidate) and os.access(candidate, os.X_OK)), None)

def require(ok, message):
    if not ok:
        raise InvalidApp(message)

class Reader:
    def __init__(self, data):
        self.data, self.offset = data, 0
    def take(self, n):
        require(0 <= n <= len(self.data)-self.offset, 'Truncated WASM section.')
        value = self.data[self.offset:self.offset+n]
        self.offset += n
        return value
    def byte(self):
        return self.take(1)[0]
    def uint(self):
        value = 0
        for shift in range(0, 35, 7):
            b = self.byte()
            require(shift != 28 or b <= 15, 'Invalid WASM integer.')
            value |= (b & 127) << shift
            if b < 128:
                return value
        raise InvalidApp('Invalid WASM integer.')
    def string(self):
        return self.take(self.uint()).decode('utf-8')
    def vector(self):
        return list(self.take(self.uint()))
    def done(self):
        return self.offset == len(self.data)

def validate_html(html):
    require(len(html.encode('utf-8')) <= 2_000_000, 'HTML exceeds 2 MB.')
    def block(kind):
        pattern = r'<script\b[^>]*\btype\s*=\s*[\"\x27]' + re.escape(kind) + r'[\"\x27][^>]*>(.*?)</script\s*>'
        matches = re.findall(pattern, html, re.I | re.S)
        require(len(matches) == 1, 'Include exactly one '+kind+' script block.')
        return matches[0]
    manifest = json.loads(block('application/mira-manifest+json'))
    require(isinstance(manifest,dict), 'Manifest must be a JSON object.')
    title = manifest.get('title')
    require(isinstance(title,str) and 1 <= len(title) <= 31 and all(32 <= ord(c) < 127 for c in title), 'Title must be 1–31 printable ASCII characters.')
    module = base64.b64decode(''.join(block('application/mira-wasm').split()), validate=True)
    require(len(module) <= 128*1024 and module[:8] == b'\0asm\x01\0\0\0', 'Embed a real compiled WASM module up to 128 KB.')
    abi = json.loads((Path(__file__).resolve().parent.parent/'SDK-ABI.json').read_text())
    reader = Reader(module[8:])
    types, functions, imports, exports, seen = [], [], set(), {}, set()
    memories = defined = bodies = 0
    while not reader.done():
        section_id = reader.byte()
        section = Reader(reader.take(reader.uint()))
        if section_id == 0:
            continue
        require(section_id not in seen, 'Duplicate WASM section.')
        seen.add(section_id)
        if section_id == 1:
            for _ in range(section.uint()):
                require(section.byte() == 0x60, 'Unsupported function type.')
                types.append((section.vector(), section.vector()))
        elif section_id == 2:
            for _ in range(section.uint()):
                namespace, name, kind = section.string(), section.string(), section.byte()
                require(namespace == 'mira' and kind == 0 and name in abi, 'Unsupported import: '+namespace+'.'+name)
                index = section.uint()
                count, result = abi[name]
                require(index < len(types) and types[index] == ([0x7f]*count, [0x7f] if result else []), 'Wrong signature for mira.'+name)
                functions.append(index)
                imports.add(name)
        elif section_id == 3:
            defined = section.uint()
            for _ in range(defined):
                index = section.uint()
                require(index < len(types), 'Invalid function type index.')
                functions.append(index)
        elif section_id == 5:
            memories = section.uint()
            require(memories == 1, 'Export exactly one memory.')
            flags, minimum = section.uint(), section.uint()
            maximum = section.uint() if flags & 1 else None
            require(flags == 1 and minimum == 1 and maximum == 1,
                    f'Memory declares minimum={minimum}, maximum={maximum} pages. Recompile with --initial-memory=65536 AND --max-memory=65536; an omitted maximum is invalid.')
        elif section_id == 7:
            for _ in range(section.uint()):
                name, kind, index = section.string(), section.byte(), section.uint()
                require(name not in exports, 'Duplicate export: '+name)
                exports[name] = (kind,index)
        elif section_id == 8:
            raise InvalidApp('Use app_init, not a WASM start function.')
        elif section_id == 10:
            bodies = section.uint()
            for _ in range(bodies):
                body = section.take(section.uint())
                require(body and body[-1] == 0x0b, 'Incomplete function body.')
        elif section_id in (4,6,9,11):
            continue  # Node validates these sections and the instruction bodies below.
        else:
            raise InvalidApp('Unsupported WASM section; use the supplied compiler flags.')
        require(section.done(), 'Unexpected bytes in a WASM section.')
    require(memories == 1 and bodies == defined and defined > 0 and exports.get('memory') == (2,0), 'Export memory and the app entry points.')
    for name,count in [('app_init',0),('app_render',1),('app_event',1),('app_stop',0)]:
        kind,index = exports.get(name,(-1,-1))
        require(kind == 0 and 0 <= index < len(functions) and types[functions[index]] == ([0x7f]*count,[]), 'Wrong or missing export: '+name)
    node = find_node()
    require(node is not None, 'Install Node.js to validate companion JavaScript and WASM before delivery.')
    script_blocks = re.findall(r'<script\b([^>]*)>(.*?)</script\s*>', html, re.I | re.S)
    executable_types = {'', 'text/javascript', 'application/javascript', 'text/ecmascript', 'application/ecmascript', 'module'}
    for index, (attributes, source) in enumerate(script_blocks, 1):
        match = re.search(r'\btype\s*=\s*["\x27]([^"\x27]+)["\x27]', attributes, re.I)
        script_type = match.group(1).strip().lower() if match else ''
        if script_type not in executable_types:
            continue
        command = [node, '--check'] if script_type != 'module' else [node, '--input-type=module', '--check']
        result = subprocess.run(command, input=source.encode(), capture_output=True, timeout=15)
        require(result.returncode == 0, 'Companion JavaScript syntax error in script block '+str(index)+': '+result.stderr.decode('utf-8',errors='replace')[:1500])
    result = subprocess.run([node,'-e',"new WebAssembly.Module(require('fs').readFileSync(0));"], input=module, capture_output=True, timeout=15)
    require(result.returncode == 0, 'WASM engine rejected the module: '+result.stderr.decode('utf-8',errors='replace')[:1500])
    return title, len(module)

if __name__ == '__main__':
    try:
        title, size = validate_html(Path(sys.argv[1]).read_text())
        print(f'PASS package/ABI: {title}, {size} WASM bytes, initial=max=65536. App behavior still requires smoke tests.')
    except (ValueError, OSError, IndexError, subprocess.TimeoutExpired) as error:
        sys.exit('INVALID MIRA APP: '+str(error))
