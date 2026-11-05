---
name: mira-wasm-app
description: Compile Rust apps for Mira glasses and deliver one HTML file with embedded WASM and an optional phone companion.
---

# Build a Mira app

**Do not use ChatGPT.com for this workflow.** Use **Codex, Claude on
claude.com, or Claude Code** to build your Mira app. Give the assistant this
file and describe your app. It should return **one compiled HTML file** to
upload into Mira; no coding or file assembly is required from you.

On Claude.com, use a session with code execution and file creation. The first
build may need compiler setup. If Claude shows **Continue** after reaching its
tool-use limit, press it in the same chat. A progress report or Rust source is
not the finished app: wait for the downloadable HTML and validation result.
If that session cannot install the required compiler, use Codex or Claude Code
with this same guide.

## For the assistant

If running in ChatGPT.com, tell the user upfront: "ChatGPT.com is not supported
for this Mira app-building workflow. Please use Codex, Claude.com, or Claude
Code, and give it this same guide and your app description." Hand off the work
instead of attempting the build there.

If running outside the three supported tools, direct the user to one of them
before building. Identify the actual environment, not just the model name;
if the environment is unknown, say so rather than guessing.

Use a session with code execution and file creation available. A text-only chat
cannot build this app. **Check the build environment before reading the full
embedded API reference or designing the app.** Check Rust, Python 3, Node.js, and the
`wasm32-unknown-unknown` target. Install missing tools where permitted. With
rustup, check `rustup target list --installed` and add the target if needed.
If compilation is unavailable, hand off this guide, the app description, and
existing source to Codex or Claude Code. **Do not substitute hand-written WAT,
hand-assembled WASM, invented base64, or an HTML-only demo.**

This file includes the SDK, API reference, and example. Save it as `SKILL.md`
and extract the labeled source blocks into an empty working directory:

```python
from pathlib import Path
import re
text = Path("SKILL.md").read_text(encoding="utf-8")
files = re.findall(r"<!-- mira-sdk-file: ([^\n]+) -->\n(`{3,})[^\n]*\n(.*?)\n\2\n<!-- /mira-sdk-file -->", text, re.S)
if not files:
    raise SystemExit("No SDK sources found; use the complete guide.")
for name, _, content in files:
    path = Path(name)
    if path.is_absolute() or ".." in path.parts:
        raise SystemExit("Invalid SDK path")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")
```

## Build from the working example

1. Extract the files above and compile the **unchanged example first**. Do this
   before designing or writing the custom app, so toolchain problems surface early.
2. Keep the supplied `Shared/*.rs` files and the example's shell and companion
   integration. Adapt `CompanionExample/counter.rs` for the requested behavior;
   edit the title and phone UI in `package.py`. Do not replace the SDK with a
   smaller imitation or implement its Bluetooth packet protocol yourself.
3. Run the packager again after every final source change. It compiles Rust,
   embeds the resulting bytes, and validates the HTML before saving it.

```sh
rustup target add wasm32-unknown-unknown
python3 CompanionExample/package.py MyApp.html
python3 CompanionExample/validate.py MyApp.html
```

Keep the packager's exact compiler flags, including
`-C link-arg=--initial-memory=65536`, `-C link-arg=--max-memory=65536`,
`-C link-arg=-zstack-size=8192`, and
`-C target-feature=-bulk-memory,-bulk-memory-opt,-reference-types`.
Update the source path if renaming `counter.rs`. Both memory limits must be
present in the **compiled binary**: starting at 64 KB with no declared maximum
is invalid. If the linker says the app exceeds 64 KB, reduce buffers/static data
and rebuild; do not raise the limit or remove the maximum to silence the error.

## App requirements

- **Runtime:** Rust/WASM runs on glasses; HTML/CSS/JavaScript runs on the phone.
  Use `#![no_std]`, a panic handler, and fixed buffers. Export `memory`,
  `app_init()`, `app_render(i32)`, `app_event(i32)`, and `app_stop()`; functions return void.
  WASM is limited to 128 KiB and one 64 KiB memory page including an 8 KiB stack.
  The bundled resident Casa module is separately built with two fixed memory
  pages (128 KiB); this does not change the limit for imported content apps.
  No WASI, threads, SIMD, bulk memory, reference types, or start section.
- **Name:** use the same 1–31-byte printable ASCII name in the manifest,
  and `companion::Companion::new`. Avoid bundled app names,
  Casa Mira, and Mira Intro. Keep it consistent when updating an app.
- **Navigation:** apps are content modules beneath one resident Casa Shell.
  Define your sidebar icon with `app_icon!(content::app_icon::Icon::new()... .0)`
  or a 128-byte, 32 × 32 MSB-first bitmap. The starter includes an example.
  Do not include Casa UI, Casa icons, or `casa_shell.rs`. The host calls your event
  handler only after Shell declines the event. Use `content::double_tap_local`
  for contextual double-tap handling; overlays and an open sidebar take priority.
  Use `content::holds_local` when the app owns long-click and hold input. Always
  handle hold release; check chrome visibility before starting a held action.
  The host clears claims and module state on every app switch. Publish the
  cursor anchor with `content::cursor(x,y,true)`. Casa starts the shared cursor
  glide on its next local tick. Position the anchor beside the current selection
  using the same state and scroll offset as its highlight, and update it on every
  focus change, including frames that reuse retained content. Avoid covering
  labels. Pass `false` on surfaces without a focused control.
  Sidebar, dashboard, and notification controls show their own pointer.
  Shell owns drawing and animating the cursor.
- **Rendering:** import only documented `mira` functions from `SDK-API.md`.
  Draw between `begin` and `present`; there is no `clear` or `end`. Canvas:
  640×480, grayscale 0–15. Font IDs 0/1/2/3 mean 24/18/32/40 px; alignment
  0/1/2 means left/center/right. Keep callbacks short and BLE asynchronous.
  For text-heavy apps, use `Shared/retained_frame.rs` to avoid redrawing
  unchanged app content while navigating the shell.
- **Companion:** use `Shared/companion.rs` and the example's `miraCompanion`
  bridge. Do not call `0x0062`/`0x0063` directly: the supplied code handles
  registration, paging, acknowledgements, and message framing. Process received messages once; retry a busy `send` on a later tick.
  Incoming JSON is limited to 3500 UTF-8 bytes; outgoing JSON to 4096.
  Keep networking and credentials on the phone. For HTTP(S), call the standard
  global `fetch(...)`; the companion bridge routes it through the phone. The
  bridge APIs are `sendToApp`, `setSnapshotForApp`, `setLatestForApp`, `onFromApp` (returns unsubscribe), `storeGet`,
  `storeSet`, `storeRemove`, and WebSocket.
  For replaceable paged state up to 3500 UTF-8 bytes, use
  `miraCompanion.setSnapshotForApp(value)` with `Companion`. It preserves the
  message being read and replaces pending snapshots without dropping ordered
  commands or replies. `miraApp.publish` uses this path automatically.
  For small replaceable realtime state, use `companion::Realtime::new(APP_NAME)`
  with `miraCompanion.setLatestForApp(value)`. Controls are JSON up to 379 bytes;
  scene JSON is up to 380 bytes. One duplex BLE exchange carries both directions
  without a FIFO backlog. Ordered commands/events still use `Companion`.
- **Phone capabilities:** use documented Mira services, not assumed browser
  capabilities. For live captions, use the WASM transcription APIs `0x0001`
  (start), `0x0002` (stop), and `0x0003` (state/final/text snapshot). Connect phone
  UI buttons through `miraCompanion` and `Shared/companion.rs`. Do not use
  `window.SpeechRecognition`/`webkitSpeechRecognition` as the speech backend of
  a Mira companion. If a requested capability is not documented, explain the
  limitation instead of inventing an API.
- **Device snapshots on the phone:** `miraCompanion.getMediaState()`,
  `mediaCommand(code)`, `getIMU()`, and `stopIMU()` are public asynchronous APIs
  for any running companion. See SDK-API.md for fields, units, lifecycle, and
  command codes. Use local WASM IMU getters for latency-sensitive head control.
- **Microphone audio:** use the phone companion's capture APIs below. Keep raw
  audio processing on the phone and send compact results to the glasses.
- **Package:** one HTML file under 2 MB, containing the JSON manifest
  (`application/mira-manifest+json`), optional phone UI, and actual base64 WASM
  (`application/mira-wasm`). Even glasses-only apps use this wrapper.
  The old `<script type="mira-app">` cannot replace WASM.

## Use the glasses microphone

The running app's phone companion can receive decoded glasses microphone audio
through `miraCompanion`. The phone injects this API; it needs no extra firmware
imports or browser microphone permission.

- `await miraCompanion.startCapture()` starts or shares capture and returns
  `{sampleRate: 16000, channels: 1, format: "s16le"}` when ready. Repeated starts
  reuse the capture. Handle rejection when the glasses microphone is unavailable
  or startup is cancelled.
- `miraCompanion.onCapture(handler)` receives `{samples: Int16Array, sampleRate:
  16000, channels: 1, sampleOffset}` and returns an unsubscribe function. Register
  the handler before starting. PCM is signed 16-bit mono at 16 kHz, with the
  phone's microphone gain already applied; divide samples by 32768 to normalize.
- `await miraCompanion.stopCapture()` releases this app's capture and returns
  `{ok: true}`. Other microphone consumers keep running. Unsubscribing only
  removes the handler; it does not stop capture.

This phone UI fragment demonstrates incoming audio with a live peak meter:

```html
<button id="mic-start">Start mic</button>
<button id="mic-stop">Stop mic</button>
<output id="mic-level">0%</output>
<script>
const level = document.getElementById('mic-level');
const unsubscribe = miraCompanion.onCapture(({samples}) => {
  let peak = 0;
  for (const sample of samples) peak = Math.max(peak, Math.abs(sample));
  level.textContent = Math.round(peak / 32768 * 100) + '%';
});
document.getElementById('mic-start').onclick = () => miraCompanion.startCapture()
  .catch(error => { level.textContent = error.message || String(error); });
document.getElementById('mic-stop').onclick = () => miraCompanion.stopCapture()
  .then(() => { level.textContent = '0%'; })
  .catch(error => { level.textContent = error.message || String(error); });
window.addEventListener('pagehide', unsubscribe);
</script>
```

Keep handlers short: delivery batches contain roughly 100–256 ms of audio, and
the host retains at most 256 ms while the WebView is busy. `sampleOffset` is the
first sample's index in the capture stream; detect gaps before assembling a
recording. Microphone mute suppresses delivery. Page teardown, app replacement,
and glasses disconnect release capture automatically. After reconnect, explicitly
start capture again. Closing the phone sheet leaves the running companion alive;
stop capture when the app's recording or analysis ends.

Render waveforms and levels in the companion. Use `setLatestForApp` with
`Shared/companion.rs` to send small summaries to the glasses instead of forwarding
PCM through the glasses module's 64 KiB memory or the JSON message bridge.

## Check and deliver

Run `python3 CompanionExample/validate.py MyApp.html` on the **exact final
HTML you will attach**, including after any manual HTML edits. It checks the
embedded module's memory limits, import signatures against `SDK-ABI.json`,
entry points, shared-shell imports, and real WebAssembly engine validation.
Fix every failure and rerun it. Never deliver a file that failed this check.
A successful build of the unchanged counter proves the toolchain only; it does
not validate the custom app. Check the final custom HTML independently.
The validator is included in this guide and needs only Python 3 and Node.js.

Then manually check startup, the app’s main interaction, any phone/glasses
message exchange, and double-tap → sidebar → Home on connected glasses.
For captions, verify that spoken words appear in the rendered text.
Report the device checks performed.

Return only the finished HTML as the installable deliverable; keep editable
sources in the workspace. Tell the user: **Apps → Create an app → Upload/Paste
→ Check code → Add → Install → Open**. For updates: long-press the app card →
**Edit App → Upload → Save → Update → Open**.

## Included SDK sources

Everything below is part of this same guide. Extract the labeled blocks using
the script above; preserve their paths so Rust can resolve shared modules.


### CompanionExample/counter.rs

<!-- mira-sdk-file: CompanionExample/counter.rs -->
```rust
#![no_std]
#![allow(static_mut_refs)]
#[path = "../Shared/companion.rs"] mod companion;
#[path = "../Shared/content.rs"] mod content;

app_icon!(content::app_icon::Icon::new()
    .circle(16,16,12).line(10,16,22,16).line(16,10,16,22).0);

#[link(wasm_import_module="mira")]
extern "C" {fn begin();fn text(x:i32,y:i32,w:i32,h:i32,c:i32,font:i32,a:i32,s:i32,p:*const u8,n:i32);fn present();}
#[panic_handler]fn panic(_: &core::panic::PanicInfo)->!{loop{}}
static mut BRIDGE:companion::Companion=companion::Companion::new(b"Companion Counter");
static mut N:u32=0;
static mut NEED_SEND:bool=false;
unsafe fn render(){let mut label=[0u8;32];label[..6].copy_from_slice(b"Count ");let n=digits(N,&mut label[6..]);begin();text(24,100,592,40,15,1,0,24,label.as_ptr(),(n+6) as i32);content::cursor(614, 440, true);present();}
fn digits(mut n:u32,out:&mut[u8])->usize{let mut reversed=[0u8;10];let mut count=0;loop{reversed[count]=b'0'+(n%10) as u8;count+=1;n/=10;if n==0{break;}}for i in 0..count{out[i]=reversed[count-1-i];}count}
#[no_mangle]pub unsafe extern "C" fn app_init(){N=0;BRIDGE=companion::Companion::new(b"Companion Counter");NEED_SEND=true;render();}
#[no_mangle]pub unsafe extern "C" fn app_render(ms:i32){
 let shell_dirty=content::context_changed();
 if let Some(message)=BRIDGE.tick(ms.max(0) as u32){
  if message==b"{\"type\":\"bump\"}" {N=N.saturating_add(1);NEED_SEND=true;render();}
  if message==b"{\"type\":\"sync\"}" {NEED_SEND=true;}
 }
 if shell_dirty{render();}
 if NEED_SEND{let mut json=[0u8;64];let head=b"{\"type\":\"count\",\"n\":";json[..head.len()].copy_from_slice(head);let n=digits(N,&mut json[head.len()..]);let end=head.len()+n;json[end]=b'}';if BRIDGE.send(&json[..end+1]){NEED_SEND=false;}}
}
#[no_mangle]pub unsafe extern "C" fn app_event(e:i32){if e==0{N=N.saturating_add(1);NEED_SEND=true;render();}}

// The host releases this instance and its owned resources after this callback.
#[no_mangle] pub extern "C" fn app_stop() {}

```
<!-- /mira-sdk-file -->


### CompanionExample/package.py

<!-- mira-sdk-file: CompanionExample/package.py -->
```python
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

```
<!-- /mira-sdk-file -->


### CompanionExample/validate.py

<!-- mira-sdk-file: CompanionExample/validate.py -->
```python
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

```
<!-- /mira-sdk-file -->


### SDK-ABI.json

<!-- mira-sdk-file: SDK-ABI.json -->
```json
{"app_launch":[2,true],"app_registry":[3,true],"app_resource":[5,true],"audio_level":[1,true],"begin":[0,false],"bezier":[10,false],"bitmap1":[7,false],"bitmap4":[6,false],"bitmap4_rotated":[7,false],"ble_request":[3,true],"ble_response_len":[1,true],"ble_response_read":[3,true],"ble_send":[3,true],"ble_stream_read":[3,true],"content_context":[1,false],"content_cursor":[3,false],"content_id":[2,true],"content_info":[1,true],"content_shade":[1,false],"device_id":[2,true],"device_status":[2,true],"display_brightness_get":[0,true],"display_brightness_set":[1,true],"display_power_get":[0,true],"display_power_set":[1,false],"fill_circle":[4,false],"fill_rect":[5,false],"fill_round_rect":[6,false],"fill_triangle":[7,false],"firmware_version":[2,true],"frame_checkpoint":[0,true],"frame_restore":[0,true],"gesture_policy":[2,true],"imu_pitch_cdeg":[0,true],"imu_raw_read":[1,true],"imu_ready":[0,true],"imu_roll_cdeg":[0,true],"imu_stop":[0,false],"imu_threshold":[2,true],"imu_yaw_cdeg":[0,true],"line":[6,false],"media_command":[1,true],"media_state":[2,true],"now_ms":[0,true],"present":[0,false],"registry_revision":[0,true],"stroke_circle":[5,false],"stroke_rect":[6,false],"stroke_round_rect":[7,false],"text":[10,false]}

```
<!-- /mira-sdk-file -->


### SDK-API.md

<!-- mira-sdk-file: SDK-API.md -->
````markdown
# On-glasses WebAssembly apps

Casa Mira is bundled with the phone app. Phone startup, foreground return and
glasses connection check its installed package and install it through the SDK
when missing or different from the bundle. Matching packages do not transfer or
restart. The phone then restores the running app or opens Home. Firmware contains
no Casa bytecode, UI or product logic; installed Casa can boot offline from flash.

The phone Apps drawer installs optional apps into the same external flash store.
Installed apps survive phone disconnects and glasses reboots. Ordinary navigation
opens an installed name locally. Casa is hidden from app lists and remains the
protected Home destination.
Casa is one resident WASM controller with `shell_init`, `shell_render`, and
`shell_event`. It keeps running while one content module is switched beneath it.
Home and Settings are destinations inside that controller. Updating its installed
package replaces the resident; ordinary app switches never reload it. This is
SDK ABI 2; bundled apps migrate together and no per-app Shell path remains.

Each optional app contains a WebAssembly 1.0 module up to 160 KiB. The persistent catalog supports 30 apps including Casa, subject
to storage capacity. The partition is 3 MiB on the 32 MiB flash configuration,
and 1,216 KiB on the 16 MiB configuration. Two catalog sectors and sector-rounded
payloads consume part of that budget; updates need space for the new payload
alongside the current one. Audio, networking, AI, transcription, credentials and
HTML companions still live on the phone. Optional app downloads and updates remain explicit.

The store writes a new payload into free sectors, verifies its CRC, then publishes
a new catalog in the inactive metadata sector, writing its commit marker last.
An interrupted download or metadata write leaves the prior installation valid.
The original volatile `MWPK` app-pack path has been removed.

`build-wasm-casa.sh` builds only the phone-bundled `casa_mira.wasm`. Casa changes
ship with the phone app; no firmware resource export or firmware update is needed
unless a change requires a new generic SDK capability.
Firmware source and hardware validation details are in
[Persistent app library](../../Firmware/Halo/metaglass-ap4_Halo/boards/meta_bounds/glass/metaglass_ambiq4/mira_sdk/wasm/APP-LIBRARY.md).

The retired Bluetooth UI SDK, JavaScript runtime, HTML apps and companion
loaders are archived under `Archive/BluetoothSDK` and excluded from active
app builds and backend routing. There is no native Casa or legacy-firmware
fallback. Firmware source changes require the user's cloud firmware updater;
do not compile, package or flash firmware locally.

### Shared Casa state

Home and the dashboard above content apps share one resident dashboard cache,
subscription, visibility state, and controls. App switches preserve that feed;
hidden dashboard updates continue draining. Full IMU ownership still suppresses
look-up until the content app releases it. After building Casa, run
`node MiraApp/scripts/test-wasm-dashboard.mjs` from the repository root to check
the bundled WASM against the host's subscription replacement semantics.

`MiraPhoneServices.casaHybridTranscriptSnapshot()` preserves the existing
conversation projection: translated captions, message IDs, the latest user/AI
pair, cumulative streaming answers, confirmation state, and tool progress.
WASM receives text wrapped for its viewport through pushed field updates. The
phone retains full message history. Captions, streaming AI answers, status and
quick-skill labels publish when their source changes; the shared response cursor
also subscribes. There is no periodic BLE request for these values. A received
caption redraws on the next app tick, without waiting for the render interval.

`Shared/push_data.rs` provides reusable SDK subscriptions. Apps call `ble_request`
with method `0x0070` and a topic u16 plus mode u8, then consume the ordinary reply.
The returned request ID is also the subscription token. This declaration and its
reply are local, including while offline. Firmware owns the role (content 0,
resident 1) and app identity. Subscriptions are scoped to that owner; content
replacement preserves resident tokens. Supported phone topics are conversation
(`0x0050`, mode 1 for Casa captions/skills), assistant presentation (`0x0035`), dashboard
(`0x0020`, mode 1 when visible), languages (`0x0051`, mode 1 for source, 0 for target),
notification overlay (`0x0053`), and onboarding (`0x0056`). Mode is
otherwise zero. Initial dashboard visibility is restored by its subscription;
subsequent open/close gestures send the explicit visibility command below.

Dashboard field 8 is one byte indicating an active, user-enabled Listen session.
Casa uses it for the wake-word hint; temporary AI microphone sessions leave it off.

Assistant presentation field 12 is `1` while an active Soniox or Gemini connection
is poor, otherwise `0`. It is independent of assistant visibility and drives the
Home status-bar text: `Poor connection` replaces the `Talk to me` hint. The phone owns
latency, upload-backlog and connection checks; firmware only transports the field.
Gemini's `connection.ping`/`connection.pong` exchange measures the phone-to-bridge
round trip through the audio-forwarding loop without making a model request.

Turning Listen off from the glasses dims the background and shows a titled
box, "End listening?", with two equal buttons: End, then Keep Listening, which
is selected initially. Listening stays
active until End is selected; double tap dismisses the box without stopping.
The same confirmation covers Home, the dashboard above apps, and the glasses
Listen shortcut. Phone controls and explicit voice requests keep their existing behavior.

Each subscription starts with current values. Later updates contain only changed
fields: field u8, byte length u16, then opaque bytes; an empty value clears a field.
The phone retains only the newest pending value per field and prioritizes captions.
One batch may already be in flight. A batch is fragmented into at most 368 bytes per
RPC `0x040e`, preceded by session u32, token u32, revision u32, offset u16 and total
u16 (all integers little-endian). Logical batches are capped at 8192 bytes.
Firmware treats topics and data as opaque, with sixteen subscription slots and a
four-packet mailbox. `ble_stream_read(token, out, capacity)` drains that local
mailbox; it sends nothing over Bluetooth and still returns revision/offset/total
followed by bytes. The SDK publishes a complete batch atomically and ignores
retries. Every live subscription must be drained even when its UI is hidden.

RPC `0x040f` reconciles the complete registry without invalidating SDK tokens or
restarting apps. Its request is the nonzero host session u32. Its response is:
session u32, registry revision u32, last accepted stream revision u32, count u8,
two NUL-padded 32-byte owner names (content then resident), and `count` records of
token u32, topic u16, mode u8, owner u8. At most sixteen records fit one response.
A new session clears incomplete mailbox traffic; packets from older sessions are
rejected. The host continues stream revisions beyond the returned watermark and
publishes current state for every feed. Within the same session, unchanged feeds
keep their queues; app switches reconcile only the changed declarations.

Firmware emits one-way SDK event `0x0072` containing the registry revision when
declarations change. An uncollected change is retried at most once per second
until the host reads the registry; unchanged state emits no traffic. Phone and
watch share `WatchShared/MiraWasmStateSync.swift` for the handshake, bounded retries,
session identity and framing. Connection, foreground and stream-failure recovery
use this same path, independently of package checks and display ownership.

This protocol change requires matching current phone/watch and firmware builds;
firmware builds and deployment use the cloud updater. The guest SDK imports and
subscription calls are unchanged. Product topics and UI remain in phone-delivered
SDK apps. Hardware latency and reconnect behavior still need device validation.

Notification arrivals use phone-owned card data and action handlers. Casa owns
expiry and display state locally: it captures the initial screen state once,
selects the backdrop before drawing, and restores sleep after the card exits.
Replacements inherit that state. The phone pushes card changes to the resident WASM Shell,
which presents the card above Casa or a bundled child app. Swipes choose actions/scroll controls,
tap activates, and double tap uses the pushed AI-output state locally. A modal/card consumes double tap first, then an open sidebar, then a valid
local app claim. Otherwise the default global action interrupts active AI or
toggles the sidebar. Interrupt acknowledgements only release
transport slots and never gate navigation or other input.
Interacting stops an ordinary card's automatic dismissal. Confirmation decisions
carry the card's ID and snapshot revision; stale or repeated decisions are
rejected. Lost action replies are not automatically retried. Long bodies use
the existing scroll increments and body-line layout.

Language pickers use `SonioxTranslationLanguages` and the shared recent-first
ordering. Auto detect is source-only, followed by up to eight unique recent
languages, then the remaining catalog. Selection uses the existing validated
settings writer and `LanguagePreferencesService` persistence.

Language, overlay, and onboarding subscriptions publish field 0 as a complete
snapshot: revision u32, zero offset u16, byte count u16, then UTF-8/NUL-delimited
fields (up to 4096 bytes). `Snapshot::push_tick` assembles transport fragments
atomically using the existing snapshot buffers. The revision identifies the
card shown when an action is sent. No read, unchanged reply, or continuation
request is sent by the glasses. Conversation uses independent changed fields
through `Fields::tick` instead.

- Topic `0x0035`: Casa's single assistant presentation, owned by the phone's
  `MiraAssistantPresentation`. Field 0 is session ID, 1 output revision, 2 activity
  (`0` idle, `1` connecting, `2` listening, `3` thinking, `4` speaking, `5` disconnected),
  3 content (`0` empty, `1` answer, `2` confirmation), 4 answer text, 5 activity label,
  6 visibility (`0`/`1`), 7 monotonic turn number, 8 the NUL-separated rich-page fields
  (key, title, lead, body, style, icon), 9 the NUL-separated confirmation fields,
  10 the tool icon (`0` none, `1` search, `2` memory, `3` mail, `4` calendar,
  `5` web, `6` contact, `7` document), and 11 whether the pending confirmation
  was approved on the phone's side (`0`/`1`, as a spoken "approve" is): Casa
  then shows the same check as a tap on Approve and closes the exchange itself.
  The backend derives the icon from tool
  identity/source metadata; Casa preserves its morph animations without parsing labels.
  Metadata uses UTF-8 text. The SDK sends changed fields in one atomic batch,
  including all clears on replacement; activity changes do not resend the page.
  Casa renders this state and owns only animation, scroll and control focus.
  New content resets scrolling; activity changes and form edits preserve it.
  Dismissal closes the session, so delayed output cannot reopen it. Backend voice
  events carry `presentation_turn`; rich-page jobs retain their originating turn.
- Topic `0x0050`: shared conversation and quick-skill labels. Casa uses mode `1`,
  which sends only caption state (field 0), caption text (9) and skill labels
  (12–14). It does not select assistant content from this feed.
- Topic `0x0051`: ordered language code/name pairs. Source and target pickers
  share one subscription slot; switching mode replaces it, and the SDK keeps
  draining its local mailbox after the picker closes.
- Topic `0x0053`: ordinary notifications. Confirmations arrive only on `0x0035`.
- `0x0054`: action u8 followed by the displayed card's UTF-8 ID; `0..2`
  select an action and `255` dismisses/denies. Delayed actions cannot affect a
  replacement card.
  A confirmation is a form instead of a card: fields are id, `1`, app name,
  heading, rows, choices, approve label, deny label. Rows and choices are `0x1e`
  separated; a row is `label\tchoice count\tselected\tdetail\tvalue` and a choice
  is `label\tdetail`, grouped in row order. Detail draws dimmer under the value
  (a recipient's number). A row with choices shows its position, takes focus and
  opens a numbered list; otherwise every row is drawn the same. Picking one sends action `0x40 | row << 4 | choice`, which rewrites that
  argument on the staged tool call and leaves the form up; `1` approves and `0`
  denies. New forms focus the first selectable control in display order,
  skipping read-only rows.
- `0x0055`: interrupt active AI output. Empty request; one-byte response is `1`
  when consumed and `0` otherwise. This acknowledgement never triggers navigation.
- `0x0058`: wake Mira, sent by Casa on tap-and-hold. Same path as the spoken wake
  word. With Listen off the phone starts it for this one conversation and stops it
  when the conversation closes. Empty request; one-byte response is `1` when
  accepted and `0` when blocked or a conversation is already open.
- Topic `0x0056`: pushed onboarding state: epoch, enabled, start stage, completed voice
  token, voice failure, last accepted command token, whether response audio is active, and current phase.
  Phase 31 belongs to the separate intro app; Casa waits until it reports phase 1.
  The WASM controllers own the phase transitions.
- `0x0057`: onboarding service command: epoch u32, monotonic token u32, step u8,
  action u8, phase u8. Actions are speak (0), prepare continuous voice input (1),
  finish (4), report phase (5), or deliver a timed reminder (6). Repeated tokens are
  acknowledged without repeating effects, including the final finish command.

Ask Mira uses the Listen session. Its voice phases are hidden (0), connecting (1),
Listening (2), thinking (3), and answer (4). The shell never starts voice input
from a hold gesture; app-owned holds such as Mira Code dictation remain available.

### Onboarding

The phone account, permission and connection setup prepares Casa and Maps through
the standard SDK install flow, then opens Casa's spoken tour directly.
Ink Reel and Mira Intro used the same video; both are retired. Their source and
assets remain for reference, but firmware no longer supports movie playback.

The tour prepares hands-free spoken turns for the restaurant question,
name/hobby, personalized question, and translation language. Gemini detects each
turn boundary; no touch-bar practice or hold gesture is required. The dashboard,
Listen and translation lessons retain their prompts and the 1.5-second pause after
looking down. Double-tap does not interrupt narration. During translation Home
remains visible with only the assistant answer row hidden.

The app lesson selects Maps, narrates a nearby destination connected to the
user's interest, then launches the installed Maps SDK app. Its companion reports
actual navigation start. The phone SDK host preserves the original voice session,
waits three seconds, releases Maps audio and rearms Casa after 350 ms. The original
personalized sendoff plays over the route; drained playback returns to Home.
Debug entry stages, including Maps, retain the BLE assumed-conversation prompts.

Physical verification still requires paired glasses and an iPhone: receive a
real notification, scroll a long body, approve/deny a
tool confirmation, resolve it on the phone while its card is open, and repeat
with the phone locked and through a BLE disconnect. Run the first-run tour on
physical glasses and verify direct Watch navigation with the phone disconnected.

Modules import drawing calls from `mira`:

```text
mira.begin()                                      -> void
mira.frame_checkpoint()                           -> i32 saved
mira.frame_restore()                              -> i32 restored
mira.line(x0, y0, x1, y1, width, gray)           -> void
mira.fill_rect(x, y, width, height, gray)         -> void
mira.stroke_rect(x, y, width, height, line, gray) -> void
mira.fill_circle(cx, cy, radius, gray)             -> void
mira.stroke_circle(cx, cy, radius, line, gray)     -> void
mira.fill_round_rect(x, y, w, h, radius, gray)    -> void
mira.stroke_round_rect(x, y, w, h, r, line, gray) -> void
mira.fill_triangle(x0, y0, x1, y1, x2, y2, gray) -> void
mira.text(x, y, w, h, gray, font, align,
          lineSpacing, utf8Pointer, utf8Length)    -> void
mira.bitmap4(x, y, w, h, pixelsPointer, pixelsLen) -> void
mira.bitmap4_rotated(x, y, w, h, pixelsPointer, pixelsLen, degrees) -> void
mira.bitmap1(x, y, w, h, maskPointer, maskLen, gray) -> void
mira.present()                                    -> void
mira.now_ms()                                     -> i32
mira.display_brightness_get()                      -> i32 percent
mira.display_brightness_set(percent)               -> i32 appliedPercent
mira.display_power_get()                           -> i32 active (0 or 1)
mira.display_power_set(on)                         -> void
mira.device_status(outPointer, capacity)            -> i32 byteCount
mira.media_state(outPointer, capacity)              -> i32 byteCount
mira.media_command(command)                         -> i32 accepted
mira.imu_ready()                                  -> i32
mira.imu_threshold(aboveCdeg, belowCdeg)            -> i32
mira.imu_stop()
mira.imu_pitch_cdeg()                             -> i32
mira.imu_roll_cdeg()                              -> i32
mira.imu_yaw_cdeg()                               -> i32
mira.imu_raw_read(outPointer)                    -> i32 ready
mira.ble_request(method, payloadPointer, length)  -> i32 requestId
mira.ble_response_len(requestId)                  -> i32
mira.ble_response_read(requestId, out, capacity)  -> i32
```

`display_brightness_get` reads the glasses' current brightness. `display_brightness_set`
clamps to 0–100 and applies brightness immediately. Both work without a phone
connection or pending phone API slots.
Casa reads brightness directly for drawing and slider input, with no mirrored brightness
state or refresh hooks; phone settings snapshots do not override it. The phone reads brightness on reconnect.
The updated Casa package requires Apollo firmware containing these imports.

`display_power_get` reads local active/asleep state, including the dimmed state.
`display_power_set(1)` wakes or refreshes the idle timer; an already-active display
stays visible. On wake the LCD stays black until the first complete frame has
transferred. `display_power_set(0)` sleeps it. Casa chooses its notification
backdrop and requests wake in the same render tick, so neither a BLE acknowledgment
nor a timer guesses when the card is ready. These imports require the matching
firmware source update through the cloud updater.

Coordinates target the 640x480 canvas. `gray` is 0 (black) through 15 (white).
`begin` starts a replacement frame on the caller's retained surface; `present`
composes content then chrome. Identical primitive payloads remain in place.
Changed and removed primitives contribute their old and new bounds to a clipped
partial repaint. Each surface has its own dirty accumulator and checkpoint.
`frame_checkpoint` saves a vector/text prefix; `frame_restore` discards only that
surface's suffix. It returns zero after a canvas reset, module replacement, or
an unavailable checkpoint. Bitmap4 prefixes cannot be checkpointed. Copied
bitmap1 masks can be retained. The helper in `Shared/retained_frame.rs` manages
local content invalidation; shell changes no longer require content repainting.

`bitmap4` copies a 4-bit grayscale sprite into the retained frame using the
same native image renderer and transparency mask as the BLE SDK. Each row is
`ceil(w / 2)` bytes, with the left pixel in the high nibble; odd-width rows pad
the low nibble. Gray 15 is transparent, so use 14 for the brightest opaque
pixel. `pixelsLen` must equal `ceil(w / 2) * h`. Width is 1–640 and height is
1–480. A frame permits at most 64 bitmap calls and 131,072 total bitmap pixels,
within the shared 512-primitive limit. Each call uses one retained primitive.
Unrotated single-color images are retained as packed 1-bit masks; multicolor
and rotated images use 4-bit pixelmaps.
Invalid dimensions/lengths, exhausted budgets and unavailable image storage
skip the call; invalid guest memory traps through WAMR validation. Coordinates
are clamped to −1024…1023 and rendering clips to the canvas. There is no scaling.

`bitmap4_rotated` uses the same bitmap format, pool, validation and limits.
It rotates around the image center with the native BLE image renderer, normalizing
integer degrees into 0–359. Rotated bounds are invalidated conservatively.
The original `bitmap4` remains an unrotated wrapper. Modules using the new import
(such as Uber's spinner) require a cloud firmware update containing it; app-specific
screens and animation logic remain in WASM.

`bitmap1` copies a packed, MSB-first monochrome mask into one of sixteen retained
32×32 slots, then draws its set pixels in the supplied 4-bit grayscale color.
Each row occupies `ceil(w / 8)` bytes and `maskLen` must equal that stride times
height. It accepts widths and heights from 1–32; invalid dimensions, lengths,
or an exhausted sixteen-mask budget shared by content and the resident skip
the call. Unlike `bitmap4`, it does not use the image pool and works with `frame_checkpoint`/`frame_restore`.

Bitmap1 uses sixteen 128-byte native mask slots, separate from the bitmap4 pool.
Bitmap4 pixels and transparency masks share a 32 KiB data budget across the
resident and content surfaces, plus 2 KiB of allocator headers. Allow for both
row strides: `ceil(w/2)*h + ceil(w/8)*h`, rounded to four bytes per allocation.
An unrotated image with one opaque gray uses only `ceil(w/8)*h` bytes; its clear
bits remain transparent and its original gray is preserved. These frame masks
share the image pool and bitmap work limit, not the sixteen retained icon slots,
and prevent checkpoints just like other frame-owned images.
A full 32-piece Chess board plus Casa's cursor uses about 21 KiB including headers.
Phone-streamed image updates have an additional 8 KiB staging buffer; a target
whose pixel capacity exceeds 8 KiB is rejected before staging. WASM bitmap4 calls
copy directly into the retained pool and do not use that staging buffer.
Large imported sprites must fit the shared budget; unavailable storage skips
the draw. The preview protocol streams the retained bitmap bytes; compact
frame masks use its existing bitmap1 record format.

The shared status bar shows a phone outline with a check when the local BLE
connection flag is set and a slash when it is disconnected or not yet known.
Classic audio pairing alone does not set this indicator. This reports the BLE
link only: it does not certify that the phone host is responding or has internet.
Battery and time continue to render locally while disconnected. The icon lives
in the SDK and uses the existing `device_status` import.

`media_state` reads the glasses' local AVRCP cache immediately, without a phone
app request or request-table slot. It returns available and playing bytes followed
by NUL-terminated UTF-8 title and artist (up to 80 bytes each; use a 164-byte buffer).
No track returns `[0,0]`; insufficient capacity returns `-1` without writing.
`media_command` sends directly to the Bluetooth chip: 0 previous, 1 toggle play/pause,
2 next, 3 volume up, 4 volume down. Returns 1 if accepted for transport, 0 if sending
failed, -1 for an invalid command. Acceptance is not playback confirmation.
Use `Shared/media.rs`; sample the local cache when opening a media view and every
250 ms while visible. Metadata arrives from the player via AVRCP independently of
the Mira phone app. These imports require the matching Apollo firmware.

`device_status` reads local hardware without Bluetooth, phone requests, or API
slots. Capacity must be at least 10 bytes; success writes and returns 10:
battery percent (0–100; other values mean unavailable), charging flag, classic
Bluetooth connected flag, BLE connected flag, RTC-valid flag, hour (0–23),
minute, month (1–12), day (1–31), and weekday (Sunday=0). Invalid capacity returns
-1; WAMR validates guest memory. Battery/charging use the local battery cache;
date/time use Apollo's RTC, which continues after phone disconnection.
Connection warnings must use this local BLE flag. A slow, expired, or rejected
phone request is not evidence of disconnection. The shared shell checks the flag
when a phone action starts or fails; its five-second warning timer only controls
how long the message stays visible.

Casa and the shared app dashboard sample these values at most once per second.
Their SDK code formats battery, clock, and date even before any phone response;
phone dashboard fields cannot overwrite them. Weather/calendar still use phone
services. This import needs an Apollo update; Airoha routing is unchanged.

Orientation and screen coordinates use these conventions across SDK apps:

- Angles are centidegrees (`100` = 1 degree). Positive yaw turns right;
  compute the shortest signed yaw difference across the -180°/180° boundary.
- Positive pitch looks up. Screen X increases rightward and screen Y increases
  downward, so head-controlled screen positions use `x = centerX + yawDelta * scale`
  and `y = centerY - pitchDelta * scale`. Apply user axis-inversion settings after
  this conversion, including the inverse mapping used for edge recentering.
  Raw phone-streamed AHRS `attitude.y_pitch` has the opposite pitch sign; the
  IMU Visualizer displays those raw sensor fields. Do not copy its raw pitch sign
  into WASM head controls. Roll and acceleration vectors are sensor axes, not
  screen-coordinate deltas.

The shared SDK shell defaults to local pitch thresholds: looking up past 27°
opens the notification dashboard inside an app, and lowering to 24° closes it.
Casa and other shell apps include the same dashboard layout and text handling.
Threshold monitoring sleeps when the screen is off and resumes on screen wake;
it does not fill the motion ring or send continuous IMU data over BLE.

`imu_threshold(aboveCdeg, belowCdeg)` returns 0 below, 1 above, -1 while asleep
or awaiting calibrated data, and -2 while a full WASM or phone IMU stream owns
the sensor. Threshold values and dashboard behavior belong to the SDK.

The orientation getters and `imu_raw_read` claim full IMU sampling, which
suppresses the look-up dashboard. `imu_ready` stays zero until a calibrated
sample arrives. Call `mira.imu_stop()` (Rust: `content::imu::stop()`) to release
that claim and restore threshold monitoring without leaving the app. App exit
also releases its claims. Head-controlled apps such as Mirafold remain full-IMU
consumers; ordinary shell apps get the dashboard by default. Full mode retains
the old continuous-sampling behavior. Phone streaming independently suppresses
the dashboard until it stops. These primitives require a cloud firmware update;
the dashboard itself remains phone-delivered SDK code.

`imu_raw_read` writes nine native float32 sensor values (36 bytes):
accelerometer X/Y/Z, gyroscope X/Y/Z, then magnetometer X/Y/Z. These are
copies of the existing glasses driver outputs, with no additional SDK scaling,
axis changes, filtering, fusion, or motion calculations. Returns 1 when fresh
(at most 500 ms old), 0 when unavailable/stale, or -1 for an invalid buffer.
Only use the values when the result is 1. Reading claims local IMU sampling;
`imu_stop` or app teardown releases it. This snapshot is non-consuming and
requires no phone, BLE subscription, or calibrated attitude. Raw reads do not
request sensor fusion; orientation processing runs only for other active consumers.

Pre-rendered movie assets, the four `animation_*` imports, and their bundled
audio playback APIs are no longer supported. Apps still animate normally by
updating drawing primitives and small sprites in their own render loops.

`ble_send(0x0065, payload, 4)` is a local usage-journal command, available offline.
Payload: little-endian event u16 (nonzero), value u16. Returns 0 when enqueued,
-1 for invalid input, -2 when the journal cannot accept it. Do not retry from a
render loop. The collector adds app identity, time, and phone BLE connection state.
Shared shell codes: 1 sidebar opened, 2 sidebar closed (value: 0 app, 1 double tap,
2 selection); 3 look-up dashboard opened, 4 button-opened dashboard (value:
cached notification count, 65535 unknown). Codes 1–255 are reserved for shared
SDK behavior; custom app events should use 256–65535. This marks a UI transition,
not proof that a notification was read. Hooks belong at transitions, not draws.

For other methods, `ble_send` sends a one-way phone command with request ID zero. It allocates no
response slot; the phone executes the command without sending an API response.
Success means the command was handed to transport, not that the phone performed
it. It is not queued for reconnect or retried. Subscriptions require
`ble_request`, because they need a response token. Local UI state must not wait
for a phone command.

`ble_request` is the asynchronous escape hatch for work that belongs on the
phone, including transcription, account-backed data, and AI. Up to eight
requests may be in flight, request and response payloads are capped at 384
bytes, and unanswered requests expire after eight seconds. Poll
`ble_response_len`: `-1` means pending, `-2` unknown, and `-3` timed out. A
non-negative value is the response size and can be consumed once with
`ble_response_read`; `-4` means the supplied output buffer is too small.
Phone-side API failures are returned as `-100 - status`.

Casa uses these phone service API methods:

- `0x0001` / `0x0002` — start or stop transcription.
- `0x0003` — transcription snapshot: state byte, final byte, then UTF-8 text.
- `0x0010` — three NUL-delimited quick-skill labels.
- `0x0011` — trigger the quick-skill slot in the one-byte request payload.
- `0x0012` — AI state byte followed by UTF-8 response text.
- `0x0020` — dashboard visibility command: one byte, `1` when open, `0` when
  closed. The response has no payload. Send only when visibility changes.
  Dashboard content uses subscription topic `0x0020`: field 0 contains three
  empty placeholders (time/date/battery come from local hardware), weather,
  calendar state (`0` loading, `1` available, `2` unavailable), then up to two
  calendar title/when/place triples. Field 1 contains a reserved byte (formerly phone brightness) and a captions byte
  (always `1`) followed by NUL-delimited source/target
  language names and codes, then Mute AI (`0` or `1`). Field 2 contains transcription state and final bytes.
  Fields 3–6 contain one inbox row each: count byte, index byte, then NUL-delimited
  source, title, body, and relative time. Field 7 contains ring paired, connected,
  battery percent (`255` unknown), and charging bytes. It covers phone-owned
  JCorp and glasses-owned AIZO rings, pushes on ring state changes, and refreshes
  the battery when the dashboard opens. Only the look-up dashboard status bar
  shows a ring icon and percentage beside the glasses battery; the normal app
  status bar hides it. Unknown/disconnected power shows
  `--`, and no paired ring or no phone BLE link hides the ring indicator.
  Each dashboard field fits one transport
  packet and is applied directly to renderer state by `Events`, without keeping
  a second full copy of the dashboard. Hidden dashboards receive cleared inbox
  rows; opening publishes current rows. There are no settings or inbox read RPCs.
- `0x0022` — change a phone-owned setting;
  the first byte is the setting key (`3` for source, `4` for target) and the
  remainder is its UTF-8 language code. Key `5` sets Mute AI with exactly one
  value byte (`0` off, `1` on), sharing the phone's saved preference. The settings
  snapshot appends its state as NUL-delimited field 4 (`0` or `1`) after the four
  language fields. All other keys are rejected. Brightness
  uses the local display imports; captions are always shown while listening.
- Installed packages are queried through `app_registry(index,out,capacity)` and
  `registry_revision()`, never through a phone catalog. Index 0 is the protected
  resident; indices 1 onward are content packages. Each 40-byte record contains
  revision u32 LE, flags u32 LE (`1` launchable content, `2` protected resident),
  and a NUL-padded 32-byte printable ASCII ID/name. Return 40 means success,
  0 means end, and negative means invalid arguments or a changed revision.
  Names are stable IDs across replacement. Shell keeps only sorted indices and
  refreshes them when the store revision changes. `app_launch(id,length)` queues
  a launch by ID locally, returning 1 if accepted, 0 if rejected, -1 if invalid.
- `app_resource(index,name,name_len,out,capacity)` reads an installed module's
  named WASM custom section without launching it. It uses the same indices as
  `app_registry`. Names are 1–64 bytes. Returns the payload length, 0 if absent,
  -1 for invalid arguments, malformed data or read failure, -2 if the catalog
  changed, and -3 if the output is too small. Only the module is scanned;
  only module bytes are read. The caller checks `registry_revision`
  around a multi-app snapshot. This import requires the current SDK firmware.

### App sidebar icons

Apps define their own 32 × 32 monochrome icon in Rust, alongside their code:

```rust
#[path = "../Shared/content.rs"] mod content;
app_icon!(content::app_icon::Icon::new()
    .circle(16, 16, 12).line(10, 16, 22, 16).line(16, 10, 16, 22).0);
```

The const helper offers two-pixel lines, rectangle outlines and circles.
Alternatively pass a `[u8;128]` mask directly: four bytes per row, MSB first,
one bit per pixel. Set bits are tinted to match the sidebar row. `app_icon!`
retains the mask in a `mira.icon` custom section, including with the SDK's
`--strip-all` build flags. Define it once per module.

Casa reads the icons directly from installed packages via `app_resource`,
caches them until the catalog revision changes, and draws them next to app
names. Imported apps use the same mechanism. No phone connection or running
app instance is required. Missing or invalid masks get a generic app icon.
Firmware provides the resource reader and bitmap storage; all icon definitions,
fallbacks, sorting and sidebar rendering remain in SDK apps.

### Other phone services

- `0x0031` — open an installed app locally, keeping the shared Mira page attached. A NUL after the app name may be
  followed by an SDK-defined launch context. No BLE transfer is required.
- `mira.audio_level(channel: i32) -> i32` — local PCM loudness, 0–255.
  Channel `0` reads the microphone before LC3 encoding; `1` reads the assistant
  speaker stream after decoding at playback consumption. Values approximate
  RMS energy over 50 ms, mapped from −48 dBFS (0) to −12 dBFS (255).
  Invalid channels and readings older than 400 ms return 0; stream close also
  clears the level. Internal DSP → MCU → display SPI telemetry never goes over
  phone BLE. Apps read it during animation ticks and own their smoothing and UI.
- Topic `0x0032` exposes the SDK voice status for app consumers. Casa uses
  `0x0035` for both the sheet and the tour's narration; it does not subscribe to
  separate voice-state or rich-page feeds.
- `0x0033` — consume the launch context supplied with the most recent local
  app switch. Casa uses this to open Settings directly from a child app.
- Topic `0x0059` — pushed Home route request: field 0 is a token that changes per
  request, field 1 the route (`home` or `settings`). The phone publishes it when it re-selects
  the running Home, which the firmware does not resume; a fresh start ignores the retained
  request and takes its route from the `0x0033` launch context instead.

Font values are `0` for 24 px, `1` for 18 px, `2` for 32 px, and `3` for 40
px. Text alignment is `0` left, `1` center, or `2` right. All text is copied
into firmware-owned bounded storage before the WASM call returns.
An app must export:

```text
app_init()          -> void
app_render(elapsedMs) -> void
app_stop()          -> void
app_event(event)    -> void
```

WASM input event values are `0` click, `1` swipe forward, `2` swipe backward,
`3` double click, `9` long click, `10` hold start, and `12` hold end.
For menus and readers, event `1` selects the previous/up/left item or earlier
content; event `2` selects the next/down/right item or later content. The wire
names `scrollTop`/`scrollBottom` and touch-bar forward/backward refer to this same
pair. Phone-preview and Watch up/left gestures send `1`; down/right sends `2`.
Game-specific actions may use the pair differently (e.g. Mine Walk forward/back,
Dino jump, or Music volume up/down), but shared sidebar/card navigation always
uses the same previous/next rule. Do not invert the shared events per app.
Firmware routes every gesture to `shell_event(event) -> i32` first: 1 consumes,
0 forwards to the active content module. Shell alone imports Casa UI code from
`WasmApps/CasaMira`; content apps import `Shared/content.rs` for generic services.
Double-tap precedence is modal/notification overlay, open sidebar, active app's
valid local claim, then Shell's default global action. Double tap also requests
AI cancellation; neither phone state nor its reply delays navigation.
`gesture_policy(event,local)` accepts events 3, 9, 10 and 12 with local 0 or 1
only from the active content instance. Claims reset on app switch. Maps and
Tetris use `content::holds_local(true)` for long-click and hold edges. A claimed
hold release always reaches the app; the app checks chrome visibility before
starting a held action. Home's contextual controls are handled inside Shell.
A content module that claims hold edges owns its own held-action presentation;
Shell adds nothing beside the pointer. A conversation with Mira still opens the
shared answer sheet above it.

Casa Home uses the liquid orb presentation prototyped in the retired AI app.
This is presentation only: Casa's established hold routing, conversation state,
confirmation/onboarding gates, notification behavior and quick-skill APIs remain
in charge. The shared cursor dissolves into a particle-sphere orb while
Casa reports held input, thinking or active output: 140 dots on a rotating sphere
that slowly breathes between a hollow halo and a filled ball. It follows the shared
cursor position and glide on every frame, including during release and playback,
then morphs back at the current hover target. It has no separate movement state.
Near display edges, the center moves only enough to fit the loudest halo. The orb uses the existing
realtime topic's input level while held and output level afterward; stale samples
expire after 400 ms. The shared glasses meter uses the transport's normalized
loudness directly, without boosting quiet speech, with a smoothed 140 ms
attack / 220 ms release that grows the sphere from 20 to 30 px, pushes it out to a
halo, adds up to 15% radial shimmer and speeds its spin. Rim dots are brightest
and largest in the halo, near-side dots in the ball, over a dark backing disc. Playback timing remains
an estimate from the phone's LC3 pacer, without firmware playback timestamps.

Listen and available insight buttons form one bottom row, ending at y=477.
There is no Ask Mira hold gesture. Soniox mentions of “Mira” or “mirror” wake a
fresh Gemini session with the previous 10 seconds of buffered microphone audio.
Mira has one presentation on every screen: the answer sheet. It rises from the
bottom edge over Home, Settings and any content app alike, with the same
geometry, timing, gestures and timeout; there is no cursor bubble beside an
app's pointer. Over a content app the Shell draws the sheet on its chrome
surface, shades the app to the sidebar's level, reports it as covered through
`content_context` bit 1, and blocks the app pointer. Opening the sheet closes
the sidebar and notification dashboard. The dashboard stays closed while the
sheet is visible; after dismissal, looking down and up reopens it. The guided
tour keeps its narration bubble.
On Home the sheet covers the captions and controls, with the orb centered on its
top border. The AI transcript comes first, followed by rich output underneath in
the same continuous column. Swipes smoothly scroll both; one more forward swipe
at the end dismisses the sheet and closes Gemini, stopping playback. Double-tap
does the same from anywhere. Soniox remains armed for the next wake word.
Dismissal and timeout slide the sheet out with the same 240 ms easing as its entrance.

Pending confirmations replace the previous transcript and rich output, reset
scrolling, and occupy the sheet on their own. Late reply updates cannot replace
a pending form. They reuse notification content without the outer card border,
bell icon, or app-name header. The orb becomes the selection
cursor for the Deny/Approve buttons and long-text controls. Swipes select confirmation
controls, bringing them into view; taps activate them. Approving draws a check
in the sheet for about a second before it lowers; Cancel lowers it at once.
Pending confirmations pause
the reply timeout. Double-tap cancels the confirmation and closes the conversation.
Confirmations use the sheet inside content apps and Settings too, and lower an
open dashboard so they never appear as a floating card. Ordinary notifications
retain their top overlay.

The sheet crown renders the phone-selected activity label beside the orb.
Listening is hidden during generation and playback and returns when microphone
input is available, while the last answer remains visible. One 25-second inactivity timer owns
both the sheet and Gemini: it pauses during generation/playback, renews on new
output, and closes both on expiry. Scrolling does not change the timer. Empty or declined wakes use the
same timer; there is no separate follow-up deadline. Closing the voice session
for other reasons does not clear an existing reply. A fixed ring in the upper-right loses segments as time runs out, matching notifications.
Status and countdown draw above the retained content frame, so their animation
does not re-layout the rich reply. Settings, the sidebar, notifications and the look-up dashboard
retain their established controls and voice behavior.

Build with `scripts/build-wasm-casa.sh`.
The orb uses a fixed particle count and bounded triangle discs, never per-pixel WASM rasterization.
The separate AI bundle is removed; the existing withdrawn-app cleanup removes
previously installed bundled copies on reconnection, preserving imported apps.

The host retains two independent primitive surfaces in one bounded arena:
content is composed first and chrome last. The existing 512 primitive slots,
64 text slots, four 1-bit mask slots and image workspace are shared, not doubled.
Each surface retains its own ordered slot map, frame/checkpoint state and dirty
bounds. Updating one surface cannot truncate or reset the other. Repaint uses
only the union of changed old/new bounds; unchanged primitive frames do not
trigger a redraw. A content module draws only its own layer.

`content_cursor(x,y,flags)` publishes a target; bit 0 makes the content pointer
visible. Clear it on surfaces without a focused control, such as navigation.
The resident picks up target and visibility changes on its next local tick;
cursor animation does not depend on BLE or content redraw frequency.
Sidebar, dashboard, and notification controls show their
own pointer. Menu transitions glide to and from the app's anchor during AI
calls as well as while idle.
`content_info(field)` returns active-content presence (0), valid double-tap
claim (1), cursor x/y/flags (2/3/4), opaque Shell context (5), last host error
(6), or valid local gesture bitmask (7, bit `1 << event`).
`content_id(out,capacity)` copies the active ID and returns its byte length,
or 0 at Home. Only the resident may call `content_context(flags)` to publish
occlusion hints; Casa defines bit 0 as sidebar and bit 1 as overlay/dashboard.
Firmware does not interpret those UI meanings.
Only the resident may call `content_shade(level)`: the compositor then draws
every retained content primitive with its color scaled by `level/15` (15 is
unchanged, 0 is off) without the app redrawing. Fractional palette levels round
up so nonblack details stay visible at nonzero shade levels; black stays black. Chrome is unaffected, and 4bpp
images keep their raw pixels. The runtime restores 15 whenever content stops.
Casa shades content to 2 while its sidebar or a notification card is visible
(cards that hide the background paint the frame themselves), and Home, which
the resident paints itself, scales its own colors to the same level.

The host sends a generic `0x0071` launch notification with the active ID to the
phone so companions follow launches even when content never makes a BLE request.
Content stop releases only its own requests, subscriptions, images and input
policy. The resident's conversation, overlays, icons and global gesture routing
continue in the same execution context.

Each callback has a firmware-enforced instruction budget. WASI, threads,
filesystem, sockets, and dynamic code generation are intentionally unavailable.
Modules with a WebAssembly start section or legacy `__post_instantiate` hook are
rejected; put initialization in the metered `app_init` export instead.

The firmware runtime also disables the bulk-memory and reference-types
proposals. Keep Rust builds on the flags used by the checked-in build scripts;
in particular, both `bulk-memory` and LLVM's separate `bulk-memory-opt`
code-generation feature must be disabled so slice copies do not emit
`memory.copy` (`0xfc 0x0a`).

The repository includes thirteen reference experiences. Build their
binaries with:

```sh
MiraApp/scripts/build-wasm-tetris.sh
MiraApp/scripts/build-wasm-dino.sh
MiraApp/scripts/build-wasm-jelly-bounce.sh
MiraApp/scripts/build-wasm-geo-dash.sh
MiraApp/scripts/build-wasm-showcases.sh
MiraApp/scripts/build-wasm-casa.sh
MiraApp/scripts/build-wasm-mirafold.sh
MiraApp/scripts/build-wasm-mine-walk.sh
MiraApp/scripts/build-wasm-physics-games.sh
```

Tetris controls on the glasses are: tap to rotate and swipe forward/backward to
move left/right. Double-tap is reserved for the shared Casa sidebar. Opening
Tetris again starts a fresh game.

Dino Run advances automatically. Tap or swipe to jump; use the same gestures
after a collision to restart. Double-tap opens the shared Casa sidebar.

Jelly Bounce is a scrolling platformer built to showcase fluid, physics-driven
vector animation. Look left or right to move (the further you turn, the faster
the jelly runs), nod up or tap to jump, and swipe forward or backward to dash
left or right, respectively. Its 16-point soft-body membrane runs an 8 ms spring-and-volume
solver: impacts, jumps, acceleration, dashes, and pickups send ripples through
the blob while a 60 Hz-ready render path redraws every host frame. The jelly
also throws particle splashes, collects droplets, activates checkpoints, and
reforms after a collision. If fused orientation is unavailable, swipes set its run direction.
Landing pads are broad, moving platforms carry the jelly, and neutral head input
brakes promptly in the air. The first checkpoint has a clear runway; later spike
groups are smaller and collision follows the teeth with a forgiving inner body.
Six courses ramp the difficulty: levels four to six add spiked balls that patrol
floors, bob over landing pads, or orbit the gaps on a chain, plus platforms that
slide sideways and carry the jelly. Every course is verified solvable on the
real physics. The title screen is a level picker: swipe to choose any level and
tap to play; levels cleared in the current session show a tick. The game uses
only the on-glasses SDK and keeps no state between launches. It is an optional
install from the phone Apps drawer.

Jumping Jam (developer app) is an auto-running rhythm platformer in the style of Geometry Dash.
The cube runs right at a fixed speed per level; nod up or tap to jump, and
swipe to jump as well. Spikes and block faces kill, block tops carry the cube,
and every death explodes the cube and restarts the level from the beginning
with the next attempt number after a short pause. Jumping is one rule: a quick
upward flick of the head (4° within 160 ms) jumps, with a 300 ms cooldown so a
single nod fires once. There is no baseline, arming state, or hold behaviour. A jump
buffer and a short coyote window absorb IMU latency. The cube spins while
airborne and snaps to a quarter turn on landing; a progress bar and percentage
sit at the top. Levels are one byte per tile (`.` ground, `^` spike, `1`-`3`
block heights, `A`-`C` spiked blocks). An optional parallel `roof` string marks
rocket tiles (`0` bare ceiling, `1`-`6` hanging blocks, `v` ceiling spike): a
portal swaps the cube for a rocket that flies a nine-tile corridor. Holding the
head above or below its resting pitch sets the climb rate (1.5° dead zone, full
300 px/s at 9°); the resting pitch tracks slowly in cube mode and freezes in
rocket mode. The rocket slides along the floor, ceiling and block tops and dies
on block faces and spikes; without the IMU a tap flaps against gravity. The
title screen is a level picker: turn the head left or right past 10° (or
swipe) to step through five courses of rising speed and tap to play; the turn
re-arms within 4° of rest and repeats every 700 ms while held. Levels 4 and 5
mix tighter cube patterns with rocket sections. Cleared levels show a tick. Double tap or long click returns to the picker; double tap
from the picker opens the sidebar. The first three courses are composed
from patterns whose jump windows were measured on the real integer physics
with a buffered-input solver, and no forced jump has a window under 136 ms;
levels 4 and 5 were solved on the same physics with inputs limited to a 64 ms grid.
The game uses only the on-glasses SDK and keeps no state between launches.
Enable developer apps to install it.

The additional showcase experiences demonstrate different parts of the ABI:

- **Casa Mira** — a WASM reimplementation of the regular glasses shell that
  also draws the AI's visual pages (`CasaMira/visual_page.rs`): when the
  backend composes a structured page beside a spoken answer, the phone pushes
  it as part of topic `0x35`. Swipes scroll the answer sheet; the page
  is read-only, and double tap dismisses the exchange. It is a WASM
  reimplementation of the regular glasses shell that
  mirrors the BLE SDK `CasaMira.html` layout and the native Casa chrome:
  status bar with translation badge, the Listen toggle, wrapped live
  captions, the request bubble and answer thread, three phone-backed quick
  skills beside Listen, a gliding cursor, the settings
  page with brightness slider and translation pickers, the sidebar, and the
  look-up dashboard with date, weather, calendar, media
  controls, and wrapped notifications. Text is wrapped and ellipsized inside
  the module with the firmware glyph advance tables, because the firmware
  paints each row literally. Double-tap opens or closes the sidebar; looking
  up (27°) opens the dashboard and looking down (below 24°) closes it.
  The sidebar slides in from the left edge (240 ms ease-out) and slides back
  out on close or app switch. It is a card hugging the left edge with rounded
  right corners, sized to its rows rather than the full display height, holding
  one scrolling list of 32 px rows without
  a header or row borders: Mira, Settings, then installed apps with the three
  most recently opened first, newest at the top, and the rest alphabetically.
  Seven rows are visible; the window follows the focused row, which is full
  ink while other rows are a darker shade. A dot marks the running
  destination. Recents are resident state and reset with Casa.

- **Mirafold** — MIRA rendered in one centered row of four close-set animated
  letters. Swipe between letters and tap to cycle variants; head movement bends
  the focused letter. Folding joints, twisting ribbons, flowing ink trails, and
  opening fans animate continuously at a target 30 fps, with no extra control
  panel. The 13 variants include pixel, turtle, ribbon, catenoid, planetary,
  Pong, rough, griddy, fan, and string studies. Double-tap opens the shared sidebar.

- **Breakout** — real-time collision physics, a dense 48-brick scene, and fused
  IMU input. Look left or right to move the paddle; the launch heading becomes
  center. Tap to launch/pause. Double-tap opens the shared sidebar.
  Swipes remain available if calibrated orientation is unavailable.
- **Starfall** — a hands-free auto-firing space shooter. Small yaw/pitch motions
  steer, close dodges charge overdrive, and every third sector brings a boss
  with weapon and hull rewards. Play starts automatically; look down to pause
  and hold left to retry. Tracking loss, notifications, and
  the shared sidebar freeze play. Tap/swipes provide manual controls without
  IMU.
- **Tether** — a two-player co-op obstacle course with an elastic rope. Turn/look
  to steer, time moving hazards, and open a pressure-switch gate.
  Both players reach the exit together and retry hands-free. Install/Open Tether
  from Mira's built-in app list. Its bundled SDK HTML companion creates/joins a
  room through standard SDK messaging and WebSockets. No game-specific Swift service or firmware is
  needed. Disconnects, tracking loss, and shared overlays pause play. See
  [Tether](Tether/README.md) for the package, deployment, and verification.
- **Mine Walk** — a first-person voxel landscape with raised tree canopies,
  bark, leaf, grass, dirt, stone, sand and water textures, distance shading, and
  clipped 3D faces. Stand still for about one second to calibrate footsteps, then
  step to move 0.72 blocks in the heading measured at the impact. Motion records
  are buffered independently of rendering. Swipes remain manual forward/back
  steps; tap recenters, long-click pauses/resumes walking, and double-click
  resets. Movement sweeps a player radius, blocks tree trunks, and climbs at most
  one block. Terrain remains procedural with no fixed edge. The phone WASM
  preview's Step button injects an acceleration waveform through the same ABI.
  The frame caps geometry plus texture to 470 triangles with room for the HUD;
  textures prioritize large visible surfaces within that fixed display budget.
  Scene construction is split into bounded callbacks while motion processing
  continues; the camera is held consistent for each build and only complete
  frames are published. This preserves the 500,000-instruction callback limit.

Developer Settings also exposes a glasses runtime log. It reads the bounded
Apollo-side event ring over the existing Mira BLE RPC and reports upload/CRC,
WAMR initialization, module linking, first-frame primitive counts and bounds,
input events, IMU activation/readiness, shutdowns, and the callback stage for
any trap.

The connected-device **Glasses Preview** is hardware-backed for a running
WASM app. While that screen is open, the phone pages the retained WAMR vector
display list from Apollo (`0x040B`) and paints those exact primitives in the
normal preview canvas. Physical touch-bar input, resident app switches, and
IMU-driven frames therefore appear without running a second copy of the module.
Gestures on the phone preview are injected into the same on-glasses runtime
through `0x040C`. The mirror is built to stay off the link: each poll first asks
for a header-only page (`slot == total`) and downloads nothing while the present
generation is unchanged, polls back off from 120 ms to 1 s as the display rests,
bitmaps already seen (matched by the record's size and hash) are skipped by
resuming the slot at its end, and reads use the bulk lane so OS and app traffic
go first. Polling stops when the preview closes or the app leaves the foreground.

The same Developer Settings card has **Preview WASM on Phone**. It executes
the selected bundled binary in a hidden WebKit WebAssembly runtime and renders
the resulting display list with the existing glasses preview canvas. Touch
gestures map to the normal input ABI, orientation buttons simulate pitch/yaw,
and deterministic local BLE fixtures exercise transcription, AI, skills, and
dashboard responses without connected glasses. This is a functional layout
and behavior simulator; firmware instruction metering, memory pressure, BLE
latency, and the optical appearance of the waveguide still require hardware.

The shared assistant hold and transcription start controls show “Connect your phone and open Mira” when the local BLE phone link is unavailable. Blocked actions are not queued for reconnection. Request failures or timeouts do not imply disconnection; the prompt is gated only by the local BLE flag. Existing phone internet and service errors remain phone-owned.

Ring taps, swipes, hold-start, and release use the same local input events as the
touch bar. AIZO delivers these directly from the Bluetooth MCU to Apollo; its phone
copy is telemetry only. JCorp/X5B must remain phone-based. The phone forwards gesture
codes in receive order without interpreting holds or starting AI. WASM owns gesture
behavior and requests phone services only for actions that need them.


### Public companion device access

All installed HTML/WASM companions, including imported apps, can call:

- `await miraCompanion.getMediaState()` returns `{available, title, artist,
  playing, elapsedMs, durationMs, volume}`. Volume is the phone output level in
  percent; elapsed/duration are milliseconds. The read uses the current Bluetooth
  now-playing service. `available: false` means no metadata was returned.
- `await miraCompanion.mediaCommand(code)` accepts the same codes as the local
  WASM media API: 0 previous, 1 toggle, 2 next, 3 volume up, 4 volume down. Returns
  `{accepted}`; transport acceptance is not playback confirmation.
- `await miraCompanion.getIMU()` starts/reuses glasses streaming and returns
  `{available, active, source, sampleCount, receivedAt, ageMs, accelerometer,
  gyroscope, magnetometer, attitude}`. Missing vectors/fields must be treated as
  unavailable. Vectors contain x/y/z and sensor timestamp metadata. Attitude uses
  x_roll/y_pitch/z_yaw and can include a stale flag. These are the existing raw
  phone-stream units/conventions (attitude degrees), distinct from the WASM
  orientation getters' centidegrees. Use the local WASM IMU getters for games
  that need minimal latency. Poll raw diagnostics at a bounded rate (e.g. 400 ms).
- `await miraCompanion.stopIMU()` releases that companion's streaming claim.
  Stop, app switching, and companion replacement also release it automatically.
- `await miraCompanion.startCapture()` starts or shares the glasses microphone
  and returns `{sampleRate: 16000, channels: 1, format: "s16le"}` when ready.
  Repeated starts reuse the capture. Startup rejects if glasses are unavailable,
  microphone enable fails, or the capture is stopped before it becomes ready.
- `miraCompanion.onCapture(handler)` registers a PCM handler and returns an
  unsubscribe function. Each frame is `{samples: Int16Array, sampleRate: 16000,
  channels: 1, sampleOffset}`. Samples are signed 16-bit mono, decoded from the
  glasses' LC3 stream with the phone's microphone gain applied. Delivery batches
  contain roughly 100–256 ms of audio. The host buffers at most 256 ms and drops
  the oldest waiting samples when WebKit falls behind; `sampleOffset` identifies
  the first sample since capture started, so gaps reveal dropped samples.
  Service-level microphone mute suppresses delivery. Audio stays on the phone;
  use the companion to process it or upload it to your own service.
- `await miraCompanion.stopCapture()` releases only the companion's microphone
  claim and returns `{ok: true}`. It discards pending audio and leaves other
  microphone consumers active. Unsubscribing removes a handler without stopping
  capture. App switching, page replacement/termination, and glasses disconnect
  also release capture; after reconnect, call `startCapture()` again.

Register the handler before starting capture:

```js
const unsubscribe = miraCompanion.onCapture(({samples, sampleOffset}) => {
  // Process signed PCM samples here; use sampleOffset to detect delivery gaps.
});
await miraCompanion.startCapture();
// When finished:
await miraCompanion.stopCapture();
unsubscribe();
```

These methods require the companion's app to be running. They reuse existing
phone hardware services and require no new firmware imports. They are available
through the same host bridge for every package; there are no app-name privileges.
`sendToApp` returns a promise that resolves when the host accepts the message,
not when WASM consumes it; rejection can be used for bounded retry/backpressure.

For phone text entry, `miraCompanion.bindPasteButton(button, onPaste)` displays an
iOS system Paste control in the HTML button's bounds. The HTML button remains a
hidden layout anchor; the existing web editor is unchanged. `onPaste(text)`
receives plain text from the user's paste action, with multiple text items joined
by a blank line. The app decides where to insert it. This avoids programmatic web
clipboard reads and their additional confirmation. Hidden/disabled button state
and bounds are synchronized automatically. Multiple Paste controls are supported.
The binding exposes `ready` and `dispose()`; controls are also removed on page
replacement. This phone capability requires no glasses connection or WASM change.

`Shared/companion.rs` accepts optional compile-time input/output buffer sizes:
`Companion::<512, 512>::new(APP_NAME)`. Defaults retain the full 4096-byte buffers.
The input capacity must fit the mailbox's sequence field and the entire incoming
JSON message, not just the decoded application payload. Size messages to fit
these app-chosen bounds and the public 3500/4096 byte limits.

The five ordinary packages and their build workflow are described in
`SDKApps/README.md`.


For compact realtime state, use `companion::Realtime::new(APP_NAME)` and
`miraCompanion.setLatestForApp(value)`. `send` replaces pending controls (JSON up
to 379 bytes); call `tick` every frame. The host retains only the latest scene
(JSON up to 380 bytes), and returns it in the same BLE round trip as the controls.
`0x0063` operation 2 carries an input ID (u32 LE) followed by JSON; its response
is a scene revision (u32 LE) followed by JSON. Old/duplicate input IDs are ignored within a registered stream.
The helper registers the companion via `0x0064`, limits requests to one in
flight, and returns changed scenes only. Pending realtime messages in the phone
host are coalesced; ordinary ordered messages are preserved. Use this channel
for replaceable state, never transactions or events that must all be delivered.
Tether demonstrates this path and three-second head-pose calibration.

Phone request tokens use bit 30 for resident ownership and the low 30 bits for
a nonzero sequence. Tokens remain positive i32 values. Content switches invalidate
content requests on both sides while resident requests and subscriptions survive.

### Rich-output icons

Casa draws a compact 60-name catalog in `fastapi_backend/app/ai/glasses_visual.py`
as distinct Lucide vectors covering places, travel, people, communication,
news, education, fitness, security, technology, work, finance and everyday needs.
Keep this catalog small: every entry adds code to the resident app's RAM footprint.
The editor prompt advertises the same complete catalog. Unknown wire names use
the help icon; the editor chooses the closest core symbol or leaves it empty.

`CasaMira/visual_icons.rs` is generated from pinned `lucide-static@1.45.0`
(ISC / Feather MIT; attribution is in `CasaMira/visual-icons.LICENSE`). The
generator snaps to the 24 px grid and simplifies subpixel corner arcs. A packed
path stream in code constants avoids a bitmap atlas and fits the existing
128 KiB resident memory and 160 KiB executable budgets. Content apps retain
their 64 KiB memory limit. The firmware runtime pool reserves 288 KiB for the
resident module, one content module, and interpreter overhead; its separate
24 KiB free-heap guard remains in place. All strokes use Home's shading
wrappers; no bitmap-mask slots or firmware changes are needed.

Casa prepares rich-page text once after measuring it, retaining wrapped bytes,
text widths, table columns, and detail-card metrics in resident memory. Scrolling
reuses those values; vertical viewport changes do not reflow the fixed-width
column. Content changes invalidate the cache even when a page ID is reused.
Installing the 128 KiB Casa build requires firmware with the 288 KiB runtime
pool; firmware compilation and deployment use the cloud updater workflow.

Regenerate after changing the canonical catalog:

```sh
npm install --prefix /tmp/mira-visual-icon-tools --ignore-scripts --no-audit --no-fund lucide-static@1.45.0 svgpath@2.6.0
node MiraApp/scripts/gen-visual-icons.mjs /tmp/mira-visual-icon-tools
bash MiraApp/scripts/build-wasm-casa.sh
```

`CasaMira/visual-icons.json` records the source geometry for review and catalog
parity checks; it is not shipped into WASM linear memory. The icon check runs
the generated renderer at 24/32/36 px, checks representative families for
distinct geometry, and enforces the device module-size limit. The rich-page
check additionally renders dense pages with the most complex icons.

````
<!-- /mira-sdk-file -->


### Shared/app_icon.rs

<!-- mira-sdk-file: Shared/app_icon.rs -->
```rust
//! App-owned, 32 × 32 monochrome sidebar icons. No runtime code is executed
//! when the launcher reads the `mira.icon` custom section from an installed app.
#![allow(dead_code)]

#[macro_export]
macro_rules! app_icon {
    ($pixels:expr) => {
        #[used]
        #[link_section = "mira.icon"]
        static APP_ICON: [u8; 128] = $pixels;
    };
}

/// Small const drawing helper; authors can also pass their own MSB-first mask.
pub struct Icon(pub [u8; 128]);
impl Icon {
    pub const fn new() -> Self { Self([0; 128]) }
    const fn pixel(mut self, x: i32, y: i32) -> Self {
        if x >= 0 && x < 32 && y >= 0 && y < 32 {
            self.0[(y * 4 + x / 8) as usize] |= 0x80 >> (x % 8);
        }
        self
    }
    /// Two-pixel stroke, with coordinates in the 32 px icon cell.
    pub const fn line(mut self, mut x: i32, mut y: i32, x1: i32, y1: i32) -> Self {
        let dx = (x1 - x).abs(); let dy = -(y1 - y).abs();
        let sx = if x < x1 { 1 } else { -1 }; let sy = if y < y1 { 1 } else { -1 };
        let mut error = dx + dy;
        loop {
            self = self.pixel(x, y).pixel(x + 1, y).pixel(x, y + 1);
            if x == x1 && y == y1 { break; }
            let twice = error * 2;
            if twice >= dy { error += dy; x += sx; }
            if twice <= dx { error += dx; y += sy; }
        }
        self
    }
    pub const fn rect(self, x: i32, y: i32, w: i32, h: i32) -> Self {
        self.line(x,y,x+w,y).line(x+w,y,x+w,y+h)
            .line(x+w,y+h,x,y+h).line(x,y+h,x,y)
    }
    pub const fn circle(mut self, cx: i32, cy: i32, r: i32) -> Self {
        let mut y = -r;
        while y <= r {
            let mut x = -r;
            while x <= r {
                let d = x*x + y*y;
                if d <= r*r && d >= (r-2)*(r-2) { self = self.pixel(cx+x,cy+y); }
                x += 1;
            }
            y += 1;
        }
        self
    }
}

```
<!-- /mira-sdk-file -->


### Shared/base64.rs

<!-- mira-sdk-file: Shared/base64.rs -->
```rust
//! Bounded standard base64 decoder for phone-delivered binary content.
fn digit(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c-b'A'), b'a'..=b'z' => Some(c-b'a'+26),
        b'0'..=b'9' => Some(c-b'0'+52), b'+' => Some(62), b'/' => Some(63), _ => None,
    }
}
pub fn decode(input: &[u8], output: &mut [u8]) -> Option<usize> {
    if input.len() % 4 != 0 { return None; }
    let mut n = 0;
    for (i, block) in input.chunks_exact(4).enumerate() {
        let a = digit(block[0])?; let b = digit(block[1])?;
        let padding = if block[2] == b'=' { 2 } else if block[3] == b'=' { 1 } else { 0 };
        if padding > 0 && (i+1)*4 != input.len() { return None; }
        let c = if padding == 2 { 0 } else { digit(block[2])? };
        let d = if padding > 0 { 0 } else { digit(block[3])? };
        if (padding == 2 && (block[3] != b'=' || b & 15 != 0)) || (padding == 1 && c & 3 != 0) { return None; }
        let count = 3-padding;
        if n+count > output.len() { return None; }
        let bytes = [(a<<2)|(b>>4), (b<<4)|(c>>2), (c<<6)|d];
        output[n..n+count].copy_from_slice(&bytes[..count]); n += count;
    }
    Some(n)
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn valid_and_invalid() {
        for (encoded, raw) in [("", ""),("Zg==","f"),("Zm8=","fo"),("Zm9v","foo"),("Zm9vYmFy","foobar")] {
            let mut out=[0;16]; let n=decode(encoded.as_bytes(),&mut out).unwrap(); assert_eq!(&out[..n],raw.as_bytes());
        }
        for bad in ["Z", "====", "Zg=a", "Zg==AAAA", "Zh==", "Zm9=", "!!!!"] { assert!(decode(bad.as_bytes(),&mut [0;16]).is_none()); }
        assert!(decode(b"Zm9v",&mut [0;2]).is_none());
    }
}

```
<!-- /mira-sdk-file -->


### Shared/buffered_text.rs

<!-- mira-sdk-file: Shared/buffered_text.rs -->
```rust
//! Buffered immutable text for local paging and scrolling. Content loads in
//! the background; input never waits on a phone navigation command.
#[link(wasm_import_module = "mira")]
extern "C" {
    fn ble_request(method: i32, payload: *const u8, size: i32) -> i32;
    fn ble_response_len(id: i32) -> i32;
    fn ble_response_read(id: i32, out: *mut u8, capacity: i32) -> i32;
}
const CAPACITY: usize = 6144;
struct Section {
    bytes: [u8; CAPACITY],
    starts: [u16; 129],
    first: u32,
    count: usize,
    indexed: usize,
}
impl Section {
    const fn new() -> Self {
        Self {
            bytes: [0; CAPACITY],
            starts: [0; 129],
            first: 0,
            count: 0,
            indexed: 0,
        }
    }
    fn contains(&self, page: u32) -> bool {
        page >= self.first && page - self.first < self.count as u32
    }
    // Index only the newly received bytes. Validating a full 6 KiB section
    // in its final callback creates an avoidable input-latency spike.
    fn index(&mut self, offset: usize, end: usize, count: usize, complete: bool) -> bool {
        if offset == 0 {
            self.indexed = 0;
            self.starts[0] = 0;
        }
        for i in offset..end {
            if self.bytes[i] == 0 {
                if self.indexed >= count
                    || core::str::from_utf8(&self.bytes[self.starts[self.indexed] as usize..i])
                        .is_err()
                {
                    return false;
                }
                self.indexed += 1;
                self.starts[self.indexed] = (i + 1) as u16;
            }
        }
        if complete {
            if self.indexed != count || self.starts[self.indexed] as usize != end {
                return false;
            }
            self.count = count;
        }
        true
    }
}
pub struct Reader<const METHOD: i32> {
    sections: [Section; 2],
    pub session: u32,
    pub page: u32,
    pub count: u32,
    visible: u32,
    backwards: bool,
    active: usize,
    loading: usize,
    target: u32,
    request_session: u32,
    request_backwards: bool,
    offset: usize,
    total: usize,
    section_count: usize,
    pending: i32,
    age: u32,
    retry: u32,
    pub failed: bool,
}
fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}
fn u16_at(bytes: &[u8], offset: usize) -> usize {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap()) as usize
}
impl<const METHOD: i32> Reader<METHOD> {
    pub fn retain_lines(&mut self, count: u32) {
        self.visible = count;
    }
    pub fn direction(&mut self, backwards: bool) {
        self.backwards = backwards;
        self.retry = 0;
    }
    pub const fn new() -> Self {
        Self {
            sections: [Section::new(), Section::new()],
            session: 0,
            page: 0,
            count: 0,
            visible: 0,
            backwards: false,
            active: 0,
            loading: 0,
            target: 0,
            request_session: 0,
            request_backwards: false,
            offset: 0,
            total: 0,
            section_count: 0,
            pending: 0,
            age: 0,
            retry: 0,
            failed: false,
        }
    }
    pub fn open(&mut self, session: u32, page: u32, count: u32) {
        if self.session == session {
            return;
        }
        self.session = session;
        self.count = count;
        self.page = page.min(count.saturating_sub(1));
        self.backwards = false;
        for section in &mut self.sections {
            section.count = 0;
        }
        self.offset = 0;
        self.total = 0;
        self.failed = false;
        self.retry = 0;
        // Outstanding replies are drained in tick, but cannot publish into a
        // new book because every response is bound to its reader session.
    }
    pub fn turn(&mut self, backwards: bool) -> bool {
        let next = if backwards {
            self.page.saturating_sub(1)
        } else {
            self.page
                .saturating_add(1)
                .min(self.count.saturating_sub(1))
        };
        if next == self.page {
            return false;
        }
        self.page = next;
        self.backwards = backwards;
        self.retry = 0;
        if self.sections[1 - self.active].contains(next) {
            self.active = 1 - self.active;
        }
        true
    }
    pub fn text(&self) -> Option<&[u8]> {
        self.text_at(self.page)
    }
    pub fn has_range(&self, start: u32, end: u32) -> bool {
        (start..end.min(self.count)).all(|i| self.text_at(i).is_some())
    }
    pub fn seek(&mut self, page: u32) -> bool {
        let next = page.min(self.count.saturating_sub(1));
        if next == self.page {
            return false;
        }
        self.backwards = next < self.page;
        self.page = next;
        self.retry = 0;
        if self.sections[1 - self.active].contains(next) {
            self.active = 1 - self.active;
        }
        true
    }
    pub fn text_at(&self, page: u32) -> Option<&[u8]> {
        self.sections.iter().find(|s| s.contains(page)).map(|s| {
            let index = (page - s.first) as usize;
            &s.bytes[s.starts[index] as usize..s.starts[index + 1] as usize - 1]
        })
    }
    fn fail(&mut self) {
        self.sections[self.loading].count = 0;
        self.pending = 0;
        self.offset = 0;
        self.total = 0;
        self.retry = 1000;
        self.failed = true;
    }
    fn accept(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() <= 14 || u32_at(bytes, 0) != self.session {
            return false;
        }
        let first = u32_at(bytes, 4);
        let count = u16_at(bytes, 8);
        let offset = u16_at(bytes, 10);
        let total = u16_at(bytes, 12);
        if count == 0
            || count > 128
            || total == 0
            || total > CAPACITY
            || offset != self.offset
            || offset + bytes.len() - 14 > total
            || first >= self.count
            || count as u32 > self.count - first
            || self.target < first
            || self.target - first >= count as u32
        {
            return false;
        }
        let section = &mut self.sections[self.loading];
        if offset == 0 {
            section.first = first;
            self.total = total;
            self.section_count = count;
        } else if section.first != first || self.total != total || self.section_count != count {
            return false;
        }
        let end = offset + bytes.len() - 14;
        section.bytes[offset..end].copy_from_slice(&bytes[14..]);
        self.offset = end;
        if !section.index(offset, end, count, end == total) {
            return false;
        }
        if end == total {
            if section.contains(self.page) {
                self.active = self.loading;
            }
            self.offset = 0;
            self.total = 0;
        }
        true
    }
    pub unsafe fn tick(&mut self, ms: u32) -> bool {
        self.tick_with::<384>(ms, 3000, ble_request, ble_response_len, ble_response_read)
    }
    pub unsafe fn tick_with<const RESPONSE: usize>(&mut self, ms: u32, timeout_ms: u32,
        ble_request: unsafe extern "C" fn(i32,*const u8,i32)->i32,
        ble_response_len: unsafe extern "C" fn(i32)->i32,
        ble_response_read: unsafe extern "C" fn(i32,*mut u8,i32)->i32) -> bool {
        let was_ready = self.text().is_some();
        let was_failed = self.failed;
        self.retry = self.retry.saturating_sub(ms);
        if self.pending > 0 {
            self.age = self.age.saturating_add(ms);
            let n = ble_response_len(self.pending);
            if n >= 0 {
                let mut out = [0; RESPONSE];
                let n = ble_response_read(self.pending, out.as_mut_ptr(), RESPONSE as i32);
                self.pending = 0;
                if self.request_session == self.session {
                    if n < 0 || n > RESPONSE as i32 || !self.accept(&out[..n as usize]) {
                        self.fail();
                    } else {
                        self.failed = false;
                    }
                }
            } else if n != -1 || self.age >= timeout_ms {
                self.fail();
            }
        }
        if self.session != 0 && self.count > 0 && self.pending <= 0 && self.retry == 0 {
            if self.total == 0 {
                // Preserve the visible section while filling the other bank.
                let active = &self.sections[self.active];
                let target = if !active.contains(self.page) {
                    Some(self.page)
                } else if self.backwards {
                    active.first.checked_sub(1)
                } else {
                    let next = active.first + active.count as u32;
                    if next < self.count {
                        Some(next)
                    } else {
                        None
                    }
                };
                if let Some(target) = target {
                    let other = &self.sections[1 - self.active];
                    // A scrolling viewport can span both banks. Keep its
                    // trailing lines until they leave the screen before
                    // repurposing that bank for the next prefetch.
                    let visible = other.count > 0
                        && self.page < other.first + other.count as u32
                        && other.first < self.page.saturating_add(self.visible.max(1));
                    if !visible && !self.sections.iter().any(|s| s.contains(target)) {
                        self.loading = 1 - self.active;
                        self.sections[self.loading].count = 0;
                        self.target = target;
                        self.request_backwards = self.backwards;
                        self.offset = 0;
                        // Nonzero until the initial response supplies the size.
                        self.total = 1;
                    }
                }
            }
            if self.total > 0 {
                let mut request = [0; 11];
                request[..4].copy_from_slice(&self.session.to_le_bytes());
                request[4..8].copy_from_slice(&self.target.to_le_bytes());
                request[8] = self.request_backwards as u8;
                request[9..11].copy_from_slice(&(self.offset as u16).to_le_bytes());
                self.request_session = self.session;
                self.age = 0;
                self.pending = ble_request(METHOD, request.as_ptr(), 11);
                // A package transport returns zero while its outgoing slot is
                // busy. Keep the section and retry next tick, without reporting
                // a broken connection or backing off for a second.
                if self.pending < 0 {
                    self.fail();
                }
            }
        }
        was_ready != self.text().is_some() || was_failed != self.failed
    }
}

```
<!-- /mira-sdk-file -->


### Shared/casa_data.rs

<!-- mira-sdk-file: Shared/casa_data.rs -->
```rust
//! Bounded, atomic paged snapshots shared by Casa and its shell.
#[link(wasm_import_module = "mira")]
extern "C" {
    fn ble_request(method: i32, payload: *const u8, payload_len: i32) -> i32;
    fn ble_response_len(request_id: i32) -> i32;
    fn ble_response_read(request_id: i32, output: *mut u8, output_cap: i32) -> i32;
    fn ble_stream_read(token: i32, output: *mut u8, capacity: i32) -> i32;
}

pub fn field(bytes: &[u8], index: usize) -> &[u8] {
    bytes.split(|b| *b == 0).nth(index).unwrap_or(&[])
}
pub fn number(bytes: &[u8]) -> usize {
    bytes.iter().take_while(|b| b.is_ascii_digit()).fold(0usize,
        |n, b| n.saturating_mul(10).saturating_add((b - b'0') as usize))
}

/// Validate the complete block before mutating published storage. Backreferences
/// may overlap (runs), but must always point into already decoded bytes.
#[inline(never)]
fn unpack(input: &[u8], output: &mut [u8]) -> Result<(), ()> {
    unpack_pass(input, output, false)?;
    unpack_pass(input, output, true)
}

#[inline(never)]
fn unpack_pass(input: &[u8], output: &mut [u8], write: bool) -> Result<(), ()> {
    let (mut i, mut n) = (0, 0);
    while i < input.len() {
        let token = input[i]; i += 1;
        if token < 128 {
            let len = token as usize + 1;
            if i + len > input.len() || n + len > output.len() { return Err(()); }
            if write { output[n..n+len].copy_from_slice(&input[i..i+len]); }
            i += len; n += len;
        } else {
            let len = (token as usize & 127) + 3;
            if i + 2 > input.len() || n + len > output.len() { return Err(()); }
            let distance = u16::from_le_bytes([input[i], input[i+1]]) as usize;
            if distance == 0 || distance > n { return Err(()); }
            if write { for j in n..n+len { output[j] = output[j-distance]; } }
            i += 2; n += len;
        }
    }
    if n != output.len() { return Err(()); }
    Ok(())
}

// Shared across capacities: keep one copy in the resident rather than inlining
// this scan into every subscription's reader.
#[inline(never)]
fn index_fields(bytes: &[u8], starts: &mut [u16; 24]) -> usize {
    starts[0] = 0;
    let (mut indexed, mut field_index) = (1, 0);
    for (i, byte) in bytes.iter().enumerate() {
        if *byte == 0 {
            field_index += 1;
            if field_index % 4 == 0 {
                starts[indexed] = (i + 1) as u16;
                indexed += 1;
                if indexed == starts.len() { break; }
            }
        }
    }
    indexed
}

/// Compression is a compile-time opt-in: only the Maps state stream uses it.
/// Other apps must not carry decoder code for every snapshot capacity.
pub struct Snapshot<const N: usize, const COMPRESSED: bool = false> {
    pub revision: u32,
    pub ready: bool,
    pub failed: bool,
    data: [u8; N],
    staging: [u8; N],
    len: usize,
    // Index once per publication instead of rescanning text on every UI frame.
    field_starts: [u16; 24],
    indexed_fields: usize,
    offset: usize,
    total: usize,
    loading_revision: u32,
    pending: i32,
    age: u32,
    cadence: u32,
    refresh_requested: bool,
    push_ack: bool,
    push_loading: u32,
    push_complete: u32,
    push_decoded: Option<usize>,
}
impl<const N: usize, const COMPRESSED: bool> Snapshot<N, COMPRESSED> {
    pub const fn new() -> Self {
        Self { revision: 0, ready: false, failed: false, data: [0; N], staging: [0; N],
            len: 0, field_starts: [0; 24], indexed_fields: 0, offset: 0, total: 0, loading_revision: 0, pending: 0, age: 0, cadence: 0, refresh_requested: false, push_ack: false, push_loading: 0, push_complete: 0, push_decoded: None }
    }
    /// Supply a local view of an already atomically received SDK snapshot.
    pub fn replace(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() > N || (self.ready && self.bytes() == bytes) { return false; }
        if !bytes.is_empty() { self.data[..bytes.len()].copy_from_slice(bytes); }
        self.len = bytes.len(); self.index_fields(); self.ready = true;
        true
    }
    pub fn bytes(&self) -> &[u8] { &self.data[..self.len] }
    /// Empty snapshots clear visibility without overwriting published storage.
    /// A renderer may retain the preceding length for its exit animation; a
    /// nonempty publication replaces these bytes atomically on the same tick.
    pub fn retained_bytes(&self, len: usize) -> &[u8] { &self.data[..len.min(N)] }
    pub fn field(&self, index: usize) -> &[u8] {
        let block = index / 4;
        if block < self.indexed_fields {
            return field(&self.data[self.field_starts[block] as usize..self.len], index % 4);
        }
        // Preserve support for snapshots with more fields than the small index.
        field(self.bytes(), index)
    }
    fn index_fields(&mut self) {
        self.indexed_fields = index_fields(&self.data[..self.len], &mut self.field_starts);
    }
    pub fn refresh(&mut self) { self.refresh_requested = true; }
    /// Background commands must let a complete snapshot arrive first. A
    /// command's refresh otherwise repeatedly discards continuation packets.
    pub fn idle(&self) -> bool {
        self.pending <= 0 && self.offset == 0 && !self.refresh_requested
    }
    pub fn reset(&mut self) {
        self.revision = 0; self.ready = false; self.failed = false;
        self.len = 0; self.indexed_fields = 0; self.offset = 0; self.pending = 0; self.cadence = 60_000;
        self.refresh_requested = false; self.push_ack = false; self.push_loading = 0; self.push_complete = 0; self.push_decoded = None;
    }
    fn fail(&mut self) {
        self.failed = true; self.pending = 0; self.offset = 0;
        self.revision = 0; self.cadence = 0;
    }
    /// False leaves the previously published snapshot intact. A new revision
    /// becomes visible only after every page passes bounds and order checks.
    fn accept(&mut self, bytes: &[u8]) -> Result<bool, ()> {
        if bytes.len() < 8 { return Err(()); }
        let revision = u32::from_le_bytes(bytes[..4].try_into().unwrap());
        let offset = u16::from_le_bytes(bytes[4..6].try_into().unwrap()) as usize;
        let total = u16::from_le_bytes(bytes[6..8].try_into().unwrap()) as usize;
        if revision == 0 || offset != self.offset { return Err(()); }
        if total == 0xffff {
            if offset != 0 || revision != self.revision || bytes.len() != 8 { return Err(()); }
            self.failed = false;
            return Ok(false);
        }
        if total > N || offset > total || bytes.len() - 8 > total - offset { return Err(()); }
        if offset == 0 { self.loading_revision = revision; self.total = total; }
        if revision != self.loading_revision || total != self.total { return Err(()); }
        let end = offset + bytes.len() - 8;
        if end == offset && end < total { return Err(()); }
        self.staging[offset..end].copy_from_slice(&bytes[8..]);
        self.offset = end;
        if end == total {
            self.data[..total].copy_from_slice(&self.staging[..total]);
            self.len = total; self.index_fields(); self.revision = revision; self.offset = 0;
            self.ready = true; self.failed = false;
            return Ok(true);
        }
        Ok(false)
    }
    /// Subscribe once. The phone pushes complete, revision-bound snapshots as
    /// field 0; old data stays intact until every chunk has arrived.
    #[inline(never)]
    pub unsafe fn push_tick(&mut self, topic: u16, mode: u8, elapsed: u32) -> bool {
        self.cadence = self.cadence.saturating_add(elapsed);
        self.refresh_requested = false; // Commands publish through the same subscription.
        if self.pending <= 0 {
            if self.failed && self.cadence < 1000 { return false; }
            let payload = [(topic & 255) as u8, (topic >> 8) as u8, mode];
            self.pending = ble_request(0x70, payload.as_ptr(), 3);
            self.push_ack = self.pending > 0;
            self.push_loading = 0; self.push_complete = 0; self.push_decoded = None; self.offset = 0;
            self.cadence = 0;
            if self.pending <= 0 { self.failed = true; }
            return false;
        }
        if self.push_ack {
            let n = ble_response_len(self.pending);
            // The firmware registers the mailbox before asking the phone. Data
            // can arrive before its separate API reply; consume it immediately.
            if n != -1 {
                let mut reply = [0u8; 384];
                let failed = n < 0 || ble_response_read(self.pending, reply.as_mut_ptr(), 384) < 0;
                self.push_ack = false;
                // A received publication proves the subscription is active even
                // if its acknowledgment was lost. Mailbox reads detect resets.
                if failed && self.push_loading == 0 { self.fail(); return false; }
                self.failed = false;
            }
        }
        let mut changed = false;
        for _ in 0..4 {
            let mut packet = [0u8; 384];
            let n = ble_stream_read(self.pending, packet.as_mut_ptr(), 384);
            if n == -1 { break; }
            if n < 8 || n > 384 { self.fail(); break; }
            let stream = u32::from_le_bytes(packet[..4].try_into().unwrap());
            if self.push_complete != 0 && stream.wrapping_sub(self.push_complete) as i32 <= 0 { continue; }
            let offset = u16::from_le_bytes(packet[4..6].try_into().unwrap()) as usize;
            let total = u16::from_le_bytes(packet[6..8].try_into().unwrap()) as usize;
            if total < 11 || total > N + 11 { self.fail(); break; }
            let skip;
            if offset == 0 {
                if n < 19 || packet[8] != 0 || u16::from_le_bytes([packet[9],packet[10]]) as usize != total - 3 {
                    self.fail(); break;
                }
                if self.push_loading == stream && self.offset > 0 { continue; }
                self.push_loading = stream; self.offset = 0; self.total = total - 11;
                skip = 11; // field header plus embedded snapshot header
                self.loading_revision = u32::from_le_bytes(packet[11..15].try_into().unwrap());
                if COMPRESSED {
                    let encoding = u16::from_le_bytes([packet[15], packet[16]]);
                    let decoded = u16::from_le_bytes([packet[17], packet[18]]) as usize;
                    self.push_decoded = if encoding == 0x8000 { Some(decoded) } else { None };
                    if self.loading_revision == 0 || decoded > N
                        || (encoding != 0 && encoding != 0x8000)
                        || (encoding == 0 && decoded != total - 11) {
                        self.fail(); break;
                    }
                } else if packet[15] != 0 || packet[16] != 0
                    || u16::from_le_bytes([packet[17], packet[18]]) as usize != total - 11 {
                    self.fail(); break;
                }
            } else {
                if stream != self.push_loading || offset < 11 || total - 11 != self.total { self.fail(); break; }
                if offset - 11 < self.offset { continue; }
                if offset - 11 != self.offset { self.fail(); break; }
                skip = 0;
            }
            let size = n as usize - 8 - skip;
            if COMPRESSED && self.push_decoded.is_some() {
                let decoded = self.push_decoded.unwrap();
                let end = self.offset + size;
                if end > total - 11 || end > N { self.fail(); break; }
                self.staging[self.offset..end].copy_from_slice(&packet[8+skip..n as usize]);
                self.offset = end;
                if end == total - 11 {
                    if unpack(&self.staging[..end], &mut self.data[..decoded]).is_err() { self.fail(); break; }
                    self.len = decoded; self.index_fields(); self.revision = self.loading_revision;
                    self.offset = 0; self.ready = true; self.failed = false;
                    self.push_complete = stream; changed = true;
                }
                continue;
            }
            let mut page = [0u8; 384];
            page[..4].copy_from_slice(&self.loading_revision.to_le_bytes());
            page[4..6].copy_from_slice(&(self.offset as u16).to_le_bytes());
            page[6..8].copy_from_slice(&((total - 11) as u16).to_le_bytes());
            page[8..8+size].copy_from_slice(&packet[8+skip..n as usize]);
            match self.accept(&page[..8+size]) {
                Ok(done) => { changed |= done; if done { self.push_complete = stream; } },
                Err(()) => { self.fail(); break; }
            }
        }
        changed
    }

    pub unsafe fn tick(&mut self, method: i32, elapsed: u32, interval: u32) -> bool {
        self.cadence = self.cadence.saturating_add(elapsed);
        let mut changed = false;
        if self.pending > 0 {
            self.age = self.age.saturating_add(elapsed);
            let count = ble_response_len(self.pending);
            if count >= 0 {
                let mut buffer = [0u8; 384];
                let read = ble_response_read(self.pending, buffer.as_mut_ptr(), buffer.len() as i32);
                self.pending = 0;
                // A completed command makes any older in-flight snapshot stale.
                // Drain its reply, then request current state immediately.
                if self.refresh_requested { self.offset = 0; }
                else if read < 0 || read as usize > buffer.len() { self.fail(); }
                else { match self.accept(&buffer[..read as usize]) {
                    Ok(value) => { changed = value; self.cadence = 0; },
                    Err(()) => self.fail(),
                } }
            } else if count != -1 || self.age >= 3_000 { self.fail(); }
        }
        if self.pending <= 0 && (self.refresh_requested || (!self.ready && !self.failed) || self.offset > 0 || self.cadence >= interval) {
            if self.refresh_requested { self.offset = 0; self.refresh_requested = false; }
            let revision = if self.offset > 0 { self.loading_revision } else { self.revision };
            let mut payload = [0u8; 6];
            payload[..4].copy_from_slice(&revision.to_le_bytes());
            payload[4..].copy_from_slice(&(self.offset as u16).to_le_bytes());
            self.pending = ble_request(method, payload.as_ptr(), 6);
            self.age = 0; self.cadence = 0;
            if self.pending <= 0 { self.fail(); }
        }
        changed
    }
}

```
<!-- /mira-sdk-file -->


### Shared/companion.rs

<!-- mira-sdk-file: Shared/companion.rs -->
```rust
//! The HTML companion's existing JSON event API over the WASM phone bridge.
//! Call tick each frame; handle its returned message once. send queues one JSON
//! response (up to 4096 bytes), fragmented and retried with a stable message ID.
#[path = "casa_data.rs"] mod data;
#[link(wasm_import_module = "mira")]
extern "C" {
    fn ble_request(method:i32, payload:*const u8, len:i32)->i32;
    fn ble_response_len(id:i32)->i32;
    fn ble_response_read(id:i32, output:*mut u8, cap:i32)->i32;
}
pub struct Companion<const IN:usize=4096,const OUT:usize=4096> {
    incoming:data::Snapshot<IN>, delivered:u32, ack:u32,
    name:&'static [u8], epoch:u32, identity_pending:i32, identity_age:u32,
    outgoing:[u8;OUT], len:usize, offset:usize, id:u32,
    pending:i32, pending_ack:bool, pending_count:usize, age:u32,
    pub failed:bool,
}
impl<const IN:usize,const OUT:usize> Companion<IN,OUT> {
    pub const fn new(name:&'static [u8])->Self { Self {incoming:data::Snapshot::new(),delivered:0,ack:0,
        name,epoch:0,identity_pending:0,identity_age:5_000,
        outgoing:[0;OUT],len:0,offset:0,id:0,pending:0,pending_ack:false,pending_count:0,age:0,failed:false} }
    pub fn send(&mut self,json:&[u8])->bool {
        if self.len>0 || json.is_empty() || json.len()>OUT || json.len()>4096 {return false;}
        self.outgoing[..json.len()].copy_from_slice(json); self.len=json.len();self.offset=0;
        self.id=self.id.wrapping_add(1).max(1);true
    }
    pub unsafe fn tick(&mut self,elapsed:u32)->Option<&[u8]> {
        self.identity_age=self.identity_age.saturating_add(elapsed);
        if self.identity_pending>0 {
            let available=ble_response_len(self.identity_pending);
            if available>=0 {
                let mut reply=[0u8;384];
                let n=ble_response_read(self.identity_pending,reply.as_mut_ptr(),384);
                self.identity_pending=0;
                if n==4 {
                    let epoch=u32::from_le_bytes(reply[..4].try_into().unwrap());
                    if epoch!=self.epoch {
                        self.incoming.reset();self.delivered=0;self.ack=0;
                        self.pending=0;self.offset=0;self.epoch=epoch;
                    }
                }
            } else if available != -1 || self.identity_age>=3_000 {self.identity_pending=0;}
        }
        if self.identity_pending<=0 && self.identity_age>=5_000 {
            self.identity_pending=ble_request(0x0064,self.name.as_ptr(),self.name.len() as i32);
            self.identity_age=0;
        }
        if self.epoch==0 {return None;}
        self.incoming.tick(0x0062,elapsed,100);
        if self.pending>0 {
            self.age=self.age.saturating_add(elapsed);
            let n=ble_response_len(self.pending);
            if n>=0 {
                let mut reply=[0u8;384];let n=ble_response_read(self.pending,reply.as_mut_ptr(),384);
                self.pending=0;
                if n>=0 {
                    self.failed=false;
                    if self.pending_ack {self.ack=0;self.incoming.refresh();}
                    else {self.offset+=self.pending_count;if self.offset==self.len{self.len=0;self.offset=0;}}
                } else {self.failed=true;}
            } else if n != -1 || self.age>=3_000 {self.pending=0;self.failed=true;}
        }
        let sequence=data::number(self.incoming.field(0)) as u32;
        let fresh=sequence!=0 && sequence!=self.delivered && self.ack==0;
        if fresh {self.delivered=sequence;self.ack=sequence;}
        if self.pending<=0 && (self.ack!=0 || self.len>0) {
            let mut packet=[0u8;384];let count;
            self.pending_ack=self.ack!=0;
            if self.pending_ack {packet[0]=0;packet[1..5].copy_from_slice(&self.ack.to_le_bytes());count=5;}
            else {
                packet[0]=1;packet[1..5].copy_from_slice(&self.id.to_le_bytes());
                packet[5..7].copy_from_slice(&(self.offset as u16).to_le_bytes());
                packet[7..9].copy_from_slice(&(self.len as u16).to_le_bytes());
                self.pending_count=(self.len-self.offset).min(375);count=9+self.pending_count;
                packet[9..count].copy_from_slice(&self.outgoing[self.offset..self.offset+self.pending_count]);
            }
            self.pending=ble_request(0x0063,packet.as_ptr(),count as i32);self.age=0;
            if self.pending<=0 {self.failed=true;}
        }
        if fresh {Some(self.incoming.field(1))} else {None}
    }
}

/// Latest-value duplex channel for compact, time-sensitive controls/state.
/// Pair with miraCompanion.setLatestForApp (380-byte JSON limit). Replaces
/// unsent controls and allows one BLE exchange in flight, so delays do not
/// build a backlog. Ordered messages should continue to use Companion.
pub struct Realtime {
    name: &'static [u8], epoch: u32, identity: i32, identity_age: u32,
    pending: i32, age: u32, cadence: u32, id: u32, revision: u32,
    outgoing: [u8;379], len: usize, incoming: [u8;384],
    pub failed: bool,
}
impl Realtime {
    pub const fn new(name: &'static [u8]) -> Self {
        Self { name, epoch:0, identity:0, identity_age:5000, pending:0, age:0,
            cadence:50, id:0, revision:0, outgoing:[0;379], len:0, incoming:[0;384], failed:false }
    }
    pub fn send(&mut self, json: &[u8]) -> bool {
        if json.is_empty() || json.len()>379 {return false;}
        self.outgoing[..json.len()].copy_from_slice(json);self.len=json.len();true
    }
    pub unsafe fn tick(&mut self, elapsed:u32) -> Option<&[u8]> {
        self.identity_age=self.identity_age.saturating_add(elapsed);
        self.cadence=self.cadence.saturating_add(elapsed);
        if self.identity>0 {
            let n=ble_response_len(self.identity);
            if n>=0 {
                let mut reply=[0u8;384];let n=ble_response_read(self.identity,reply.as_mut_ptr(),384);self.identity=0;
                if n==4 {
                    let epoch=u32::from_le_bytes(reply[..4].try_into().unwrap());
                    if epoch!=self.epoch {self.epoch=epoch;self.revision=0;self.pending=0;}
                }
            } else if n != -1 || self.identity_age>=1500 {self.identity=0;}
        }
        if self.identity<=0 && self.identity_age>=5000 {
            self.identity=ble_request(0x64,self.name.as_ptr(),self.name.len() as i32);self.identity_age=0;
        }
        if self.epoch==0 {return None;}
        let mut fresh=0;
        if self.pending>0 {
            self.age=self.age.saturating_add(elapsed);
            let n=ble_response_len(self.pending);
            if n>=0 {
                let n=ble_response_read(self.pending,self.incoming.as_mut_ptr(),384);self.pending=0;
                self.failed=n<4;
                if n>=4 {
                    let revision=u32::from_le_bytes(self.incoming[..4].try_into().unwrap());
                    if revision!=0 && revision!=self.revision && n>4 {self.revision=revision;fresh=n as usize;}
                }
            } else if n != -1 || self.age>=1000 {self.pending=0;self.failed=true;}
        }
        if self.pending<=0 && self.len>0 && self.cadence>=50 {
            let mut packet=[0u8;384];packet[0]=2;self.id=self.id.wrapping_add(1).max(1);
            packet[1..5].copy_from_slice(&self.id.to_le_bytes());packet[5..5+self.len].copy_from_slice(&self.outgoing[..self.len]);
            self.pending=ble_request(0x63,packet.as_ptr(),(self.len+5) as i32);self.age=0;self.cadence=0;
            if self.pending<=0 {self.failed=true;}
        }
        if fresh>0 {Some(&self.incoming[4..fresh])}else{None}
    }
}

```
<!-- /mira-sdk-file -->


### Shared/content.rs

<!-- mira-sdk-file: Shared/content.rs -->
```rust
//! Services for the active content module. The resident owns all global UI.
#![allow(dead_code)]
#[path="app_icon.rs"] pub mod app_icon;
#[path="casa_data.rs"] pub mod data;
pub mod push_data;
pub mod media;
pub mod imu;
#[link(wasm_import_module="mira")]
extern "C" {
    fn gesture_policy(gesture:i32,local:i32)->i32;
    fn content_cursor(x:i32,y:i32,flags:i32);
    fn content_info(field:i32)->i32;
    fn app_launch(id:*const u8,len:i32)->i32;
}
static mut LAST_CONTEXT:i32=-1;
/// Sets the cursor anchor until replaced or app switch.
/// Visibility controls the content pointer; resident controls keep their own.
pub unsafe fn cursor(x:i32,y:i32,visible:bool) { content_cursor(x,y,visible as i32); }
/// Claims double tap for this active instance. Switching apps revokes the claim.
pub unsafe fn double_tap_local(local:bool) { gesture_policy(3,local as i32); }
/// Claims long-click and hold edges for this instance; app switches revoke them.
/// Check chrome visibility before starting a hold, but always handle its release.
pub unsafe fn holds_local(local:bool) {
    for event in [9,10,12] { gesture_policy(event,local as i32); }
}
/// Returns the resident's sidebar/overlay occlusion hints for pacing local UI.
pub unsafe fn sidebar_open()->bool { content_info(5)&1!=0 }
pub unsafe fn overlay_visible()->bool { content_info(5)&2!=0 }
pub unsafe fn context_changed()->bool {
    let current=content_info(5); let changed=current!=LAST_CONTEXT; LAST_CONTEXT=current; changed
}
pub unsafe fn launch_app(id:&[u8]) { app_launch(id.as_ptr(),id.len() as i32); }

```
<!-- /mira-sdk-file -->


### Shared/cursor_bubble.rs

<!-- mira-sdk-file: Shared/cursor_bubble.rs -->
```rust
//! Casa's cursor-adjacent text bubble, shared with apps that draw their own cursor.
#[path = "replica_ui.rs"]
mod typography;

#[link(wasm_import_module = "mira")]
extern "C" {
    fn fill_round_rect(x: i32, y: i32, w: i32, h: i32, radius: i32, ink: i32);
    fn stroke_round_rect(x: i32, y: i32, w: i32, h: i32, radius: i32, border: i32, ink: i32);
    fn text(x: i32, y: i32, w: i32, h: i32, ink: i32, font: i32, align: i32,
            spacing: i32, value: *const u8, length: i32);
}

// Width is a maximum; short text hugs its longest line with 11 px side insets.
// Preserve the resident's bubble geometry, screen clamping and 24 px typography.
pub unsafe fn draw(cursor_x: i32, cursor_y: i32, width: i32, border_width: i32, value: &[u8]) {
    draw_with_clearance(cursor_x, cursor_y, width, border_width, value, 0);
}

// Larger cursor shapes reuse the same bubble, leaving room for their contour.
pub unsafe fn draw_with_clearance(cursor_x: i32, cursor_y: i32, width: i32, border_width: i32, value: &[u8], clearance: i32) {
    if value.is_empty() { return; }
    let width = core::str::from_utf8(value).map(|value| {
        let text_width = value.split('\n').take(3)
            .map(|line| typography::width(line, 24)).max().unwrap_or(0);
        (text_width + 22).min(width)
    }).unwrap_or(width);
    draw_fixed_with_clearance(cursor_x, cursor_y, width, border_width, value, clearance);
}

// Animated labels can reserve their full width while the visible text changes.
pub unsafe fn draw_fixed_with_clearance(cursor_x: i32, cursor_y: i32, width: i32, border_width: i32, value: &[u8], clearance: i32) {
    if value.is_empty() { return; }
    let lines = (value.iter().filter(|byte| **byte == b'\n').count() as i32 + 1).min(3);
    let height = 47 + (lines - 1) * 27;
    let x = (if cursor_x >= 320 { cursor_x - width - 18 - clearance } else { cursor_x + 24 + clearance })
        .clamp(8, 640 - width - 8);
    let y = (cursor_y + 10 - height / 2).clamp(42, 480 - height - 2);
    fill_round_rect(x, y, width, height, 10, 0);
    stroke_round_rect(x, y, width, height, 10, border_width, 15);
    text(x + 11, y + 7, width - 22, height - 14, 15, 0, 0, 27, value.as_ptr(), value.len() as i32);
}

```
<!-- /mira-sdk-file -->


### Shared/head_game.rs

<!-- mira-sdk-file: Shared/head_game.rs -->
```rust
//! Automatic IMU setup and interruption handling for head-controlled games.
pub const STEP: i32 = 20;
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Phase { Waiting, Live, Paused }
#[derive(Clone, Copy)]
pub struct Pose { pub horizontal: i32, pub pitch: i32, pub ready: bool }
pub fn delta(angle: i32, center: i32) -> i32 {
    (angle - center + 54_000) % 36_000 - 18_000
}
pub struct Control {
    pub phase: Phase, pub x: i32, pub y: i32,
    pub center_horizontal: i32, pub center_pitch: i32,
    pub head_pause: bool,
    initialized: bool, blocked: bool, down: i32, pause_pitch: i32,
}
impl Control {
    pub const fn new() -> Self {
        Self { phase: Phase::Waiting, x: 0, y: 0, center_horizontal: 0,
            center_pitch: 0, head_pause: true, initialized: false, blocked: false, down: 0, pause_pitch: 0 }
    }
    // Games can reserve the full head-motion range for play. Only deliberate
    // taps may pause/resume; looking down must not freeze the simulation.
    pub const fn tap_only() -> Self {
        Self { head_pause: false, ..Self::new() }
    }
    pub fn recenter(&mut self, pose: Pose) {
        if !pose.ready { return; }
        self.center_horizontal = pose.horizontal; self.center_pitch = pose.pitch;
        self.x = 0; self.y = 0; self.down = 0; self.initialized = true;
    }
    pub fn tap(&mut self, pose: Pose) {
        if self.blocked || !pose.ready { return; }
        if self.phase == Phase::Live {
            self.phase = Phase::Paused; self.pause_pitch = pose.pitch;
        } else {
            self.recenter(pose); self.phase = Phase::Live;
        }
    }
    pub fn step(&mut self, pose: Pose, blocked: bool) -> bool {
        self.blocked = blocked;
        if !pose.ready {
            self.initialized = false;
            if self.phase != Phase::Paused { self.phase = Phase::Waiting; }
            return false;
        }
        // Acquire a resting pose once on startup/recovery, with no user task.
        // Menus, landings and room changes preserve it during ordinary play.
        if !self.initialized { self.recenter(pose); }
        if self.phase == Phase::Waiting { self.phase = Phase::Live; }
        self.x = delta(pose.horizontal, self.center_horizontal);
        self.y = pose.pitch - self.center_pitch;
        if blocked { return false; }
        if self.phase == Phase::Paused {
            if self.head_pause && pose.pitch - self.pause_pitch >= 600 {
                self.recenter(pose); self.phase = Phase::Live;
            }
            return false;
        }
        if self.head_pause && self.y < -2200 {
            self.down += STEP;
            if self.down >= 400 { self.phase = Phase::Paused; self.pause_pitch = pose.pitch; }
            return false;
        }
        self.down = 0;
        true
    }
}

```
<!-- /mira-sdk-file -->


### Shared/head_game_ui.rs

<!-- mira-sdk-file: Shared/head_game_ui.rs -->
```rust
//! Drawing helpers for head-controlled SDK games.
use super::head_game::{Control, Phase, Pose};
#[link(wasm_import_module = "mira")]
extern "C" {
    pub fn begin(); pub fn present();
    pub fn line(x0:i32,y0:i32,x1:i32,y1:i32,width:i32,gray:i32);
    pub fn fill_rect(x:i32,y:i32,w:i32,h:i32,gray:i32);
    pub fn stroke_rect(x:i32,y:i32,w:i32,h:i32,width:i32,gray:i32);
    pub fn fill_circle(x:i32,y:i32,r:i32,gray:i32);
    pub fn stroke_circle(x:i32,y:i32,r:i32,width:i32,gray:i32);
    pub fn fill_triangle(x0:i32,y0:i32,x1:i32,y1:i32,x2:i32,y2:i32,gray:i32);
    fn text(x:i32,y:i32,w:i32,h:i32,gray:i32,font:i32,align:i32,spacing:i32,p:*const u8,n:i32);
    fn imu_yaw_cdeg()->i32; fn imu_pitch_cdeg()->i32; fn imu_ready()->i32;
}
pub unsafe fn pose() -> Pose {
    // Getters claim sampling even before the first calibrated sample exists.
    let yaw = imu_yaw_cdeg(); let pitch = imu_pitch_cdeg();
    Pose { horizontal: yaw, pitch, ready: imu_ready() != 0 }
}
pub unsafe fn label(x:i32,y:i32,w:i32,gray:i32,font:i32,align:i32,s:&[u8]) {
    text(x,y,w,32,gray,font,align,28,s.as_ptr(),s.len() as i32);
}
pub unsafe fn center(y:i32,s:&[u8]) { label(82,y,476,15,1,1,s); }
pub unsafe fn number(x:i32,y:i32,w:i32,n:u32) {
    let mut bytes=[0;10]; let mut pos=10; let mut v=n;
    loop { pos-=1; bytes[pos]=b'0'+(v%10) as u8; v/=10; if v==0 { break; } }
    label(x,y,w,15,1,0,&bytes[pos..]);
}
pub unsafe fn meter(x:i32,y:i32,w:i32,value:i32,max:i32) {
    fill_rect(x,y,w,4,4);
    let filled=value.clamp(0,max)*w/max;
    if filled>0 { fill_rect(x,y,filled,4,15); }
}
pub unsafe fn card(title:&[u8], subtitle:&[u8]) {
    fill_rect(92,173,456,134,0);
    label(110,188,420,15,0,1,title);
    center(238,subtitle);
}
pub unsafe fn overlay(c:&Control,title:&[u8],_hint:&[u8]) {
    match c.phase {
        Phase::Waiting => card(title,b"Waiting for head tracking"),
        Phase::Paused => card(b"PAUSED",if c.head_pause {b"Lift your head to resume"} else {b"Tap to resume"}),
        Phase::Live => {}
    }
}

```
<!-- /mira-sdk-file -->


### Shared/imu.rs

<!-- mira-sdk-file: Shared/imu.rs -->
```rust
//! SDK IMU ownership. Orientation/motion getters claim full IMU; stop releases
//! that claim. Threshold-only apps keep the shared look-up dashboard available.
#[link(wasm_import_module = "mira")]
extern "C" {
    fn imu_threshold(above_cdeg:i32,below_cdeg:i32)->i32;
    fn imu_stop();
}

/// 0 below, 1 above, -1 asleep/unavailable, -2 full IMU requested by the app/phone.
pub unsafe fn threshold_state()->i32 { imu_threshold(2_700,2_400) }

#[allow(dead_code)]
pub unsafe fn stop() { imu_stop(); }

```
<!-- /mira-sdk-file -->


### Shared/media.rs

<!-- mira-sdk-file: Shared/media.rs -->
```rust
//! Local media primitives. The Bluetooth chip owns AVRCP; the SDK owns the UI.
#[link(wasm_import_module = "mira")]
extern "C" {
    fn media_state(out: *mut u8, capacity: i32) -> i32;
    fn media_command(command: i32) -> i32;
}

/// Available, playing, then NUL-delimited title and artist. No phone RPC.
pub unsafe fn read(out: &mut [u8]) -> i32 {
    let count = media_state(out.as_mut_ptr(), out.len() as i32);
    if count > out.len() as i32 { return -1; }
    // Keep song metadata on one line; rendering/truncation stays in the SDK.
    if count > 2 {
        for byte in &mut out[2..count as usize] {
            if *byte == b'\n' || *byte == b'\r' || *byte == b'\t' { *byte = b' '; }
        }
    }
    count
}

/// Previous, play/pause, next, volume up, volume down (0..4).
/// True means accepted by the glasses transport, not confirmed by the player.
pub unsafe fn command(command: u8) -> bool { media_command(command as i32) == 1 }

```
<!-- /mira-sdk-file -->


### Shared/push_data.rs

<!-- mira-sdk-file: Shared/push_data.rs -->
```rust
//! Generic phone-pushed fields. Reading the mailbox is local; only subscribing
//! or reconnecting sends a BLE request. A complete field batch applies atomically.
#[link(wasm_import_module = "mira")]
extern "C" {
    fn ble_request(method: i32, payload: *const u8, length: i32) -> i32;
    fn ble_response_len(id: i32) -> i32;
    fn ble_response_read(id: i32, out: *mut u8, capacity: i32) -> i32;
    fn ble_stream_read(token: i32, out: *mut u8, capacity: i32) -> i32;
}

/// Small independent updates (dashboard fields) need no duplicate state buffers.
/// Each batch must fit one transport packet; the renderer owns the stored values.
pub struct Events { token: i32, pending: bool, retry: u32, revision: u32 }
impl Events {
    pub const fn new() -> Self { Self { token: 0, pending: false, retry: 1000, revision: 0 } }
    pub fn reset(&mut self) { *self = Self::new(); }
    pub unsafe fn next(&mut self, topic: u16, mode: u8, elapsed: u32, out: &mut [u8;384]) -> i32 {
        self.retry = self.retry.saturating_add(elapsed);
        if self.token <= 0 {
            if self.retry < 1000 { return -1; }
            let request = [(topic & 255) as u8, (topic >> 8) as u8, mode];
            self.token = ble_request(0x70, request.as_ptr(), 3);
            self.pending = self.token > 0; self.retry = 0; self.revision = 0;
            return -1;
        }
        if self.pending {
            let n = ble_response_len(self.token);
            if n == -1 { return -1; }
            if n < 0 || ble_response_read(self.token, out.as_mut_ptr(), 384) < 0 { self.token = 0; return -1; }
            self.pending = false;
        }
        let n = ble_stream_read(self.token, out.as_mut_ptr(), 384);
        if n == -2 { self.reset(); return -1; }
        if n < 8 || n > 384 { return -1; }
        let revision = u32::from_le_bytes(out[..4].try_into().unwrap());
        if self.revision != 0 && revision.wrapping_sub(self.revision) as i32 <= 0 { return 0; }
        if out[4] != 0 || out[5] != 0 || u16::from_le_bytes([out[6],out[7]]) as i32 != n - 8 { self.reset(); return -1; }
        self.revision = revision;
        n
    }
}

pub struct Fields<const N: usize> {
    data: [u8; N],
    lengths: [usize; 32],
    staging: [u8; N],
    offset: usize,
    total: usize,
    loading_revision: u32,
    revision: u32,
    token: i32,
    pending: bool,
    retry: u32,
    pub ready: bool,
}
impl<const N: usize> Fields<N> {
    pub const fn new() -> Self {
        Self { data: [0; N], lengths: [0; 32], staging: [0; N], offset: 0, total: 0,
            loading_revision: 0, revision: 0, token: 0, pending: false,
            retry: 1000, ready: false }
    }
    pub fn field(&self, index: usize) -> &[u8] {
        if index >= 32 { return &[]; }
        let start: usize = self.lengths[..index].iter().sum();
        &self.data[start..start + self.lengths[index]]
    }
    pub fn refresh(&mut self) {
        self.token = 0; self.pending = false; self.offset = 0; self.revision = 0;
        self.loading_revision = 0; self.retry = 1000;
    }
    pub fn reset(&mut self) { self.refresh(); self.ready = false; self.lengths = [0; 32]; }
    #[inline(never)]
    fn apply(&mut self) -> Result<(), ()> {
        let mut positions = [usize::MAX; 32];
        let mut sizes = self.lengths;
        let mut cursor = 0;
        while cursor < self.total {
            if self.total - cursor < 3 { return Err(()); }
            let key = self.staging[cursor] as usize;
            let length = u16::from_le_bytes([self.staging[cursor+1], self.staging[cursor+2]]) as usize;
            cursor += 3;
            if key >= 32 || positions[key] != usize::MAX || length > self.total - cursor { return Err(()); }
            positions[key] = cursor; sizes[key] = length; cursor += length;
        }
        if sizes.iter().sum::<usize>() > N { return Err(()); }
        // Merge in place: a merge buffer the size of the feed overflowed Casa's
        // 8 KiB stack. Kept fields sliding toward the start move in key order,
        // then those sliding toward the end in reverse, so none overwrites another.
        let (mut old_at, mut new_at, mut old, mut new) = ([0usize; 32], [0usize; 32], 0, 0);
        for key in 0..32 { old_at[key] = old; new_at[key] = new; old += self.lengths[key]; new += sizes[key]; }
        let kept = |key: usize| positions[key] == usize::MAX;
        for key in (0..32).filter(|&k| kept(k) && new_at[k] < old_at[k]).chain((0..32).rev().filter(|&k| kept(k) && new_at[k] > old_at[k])) {
            self.data.copy_within(old_at[key]..old_at[key]+sizes[key], new_at[key]);
        }
        for key in (0..32).filter(|&k| !kept(k)) {
            self.data[new_at[key]..new_at[key]+sizes[key]].copy_from_slice(&self.staging[positions[key]..positions[key]+sizes[key]]);
        }
        self.lengths = sizes; self.ready = true;
        Ok(())
    }
    fn accept(&mut self, packet: &[u8]) -> Result<bool, ()> {
        if packet.len() <= 8 { return Err(()); }
        let revision = u32::from_le_bytes(packet[..4].try_into().unwrap());
        let offset = u16::from_le_bytes(packet[4..6].try_into().unwrap()) as usize;
        let total = u16::from_le_bytes(packet[6..8].try_into().unwrap()) as usize;
        let bytes = &packet[8..];
        if revision == 0 || total > N || offset >= total || bytes.len() > total-offset { return Err(()); }
        if self.revision != 0 && revision.wrapping_sub(self.revision) as i32 <= 0 { return Ok(false); }
        if revision == self.loading_revision && offset < self.offset {
            // An acknowledgement can be lost after delivery; retries are inert.
            if total == self.total && offset+bytes.len() <= self.offset
                && &self.staging[offset..offset+bytes.len()] == bytes { return Ok(false); }
            return Err(());
        }
        if offset == 0 { self.offset = 0; self.total = total; self.loading_revision = revision; }
        if revision != self.loading_revision || total != self.total || offset != self.offset { return Err(()); }
        self.staging[offset..offset+bytes.len()].copy_from_slice(bytes);
        self.offset += bytes.len();
        if self.offset != total { return Ok(false); }
        self.apply()?;
        self.revision = revision; self.offset = 0;
        Ok(true)
    }
    pub unsafe fn tick(&mut self, topic: u16, mode: u8, elapsed: u32) -> bool {
        self.retry = self.retry.saturating_add(elapsed);
        if self.token <= 0 {
            if self.retry < 1000 { return false; }
            let request = [(topic & 255) as u8, (topic >> 8) as u8, mode];
            self.token = ble_request(0x0070, request.as_ptr(), 3);
            self.pending = self.token > 0; self.retry = 0;
            return false;
        }
        if self.pending {
            let length = ble_response_len(self.token);
            if length >= 0 {
                let mut reply = [0u8; 384];
                if ble_response_read(self.token, reply.as_mut_ptr(), 384) < 0 { self.token = 0; return false; }
                self.pending = false;
            } else if length != -1 { self.token = 0; self.pending = false; return false; }
            else { return false; }
        }
        let mut changed = false;
        for _ in 0..4 {
            let mut packet = [0u8; 384];
            let length = ble_stream_read(self.token, packet.as_mut_ptr(), 384);
            if length == -2 { self.refresh(); break; }
            if length < 0 { break; }
            if length as usize > packet.len() { self.refresh(); break; }
            match self.accept(&packet[..length as usize]) {
                Ok(value) => changed |= value,
                Err(()) => { self.refresh(); break; }
            }
        }
        changed
    }
}

```
<!-- /mira-sdk-file -->


### Shared/reader.rs

<!-- mira-sdk-file: Shared/reader.rs -->
```rust
// Shared WASM document reader. The phone supplies text chunks, not layout.
use super::*;
const ADV24: [u8; 95] = [
    6, 7, 8, 15, 13, 19, 17, 4, 8, 8, 12, 14, 5, 7, 5, 8, 15, 10, 13, 12, 15, 14, 14, 13, 14, 14,
    7, 7, 14, 14, 14, 12, 24, 16, 15, 15, 17, 14, 13, 17, 18, 6, 11, 15, 13, 21, 18, 18, 15, 18,
    16, 14, 14, 18, 16, 24, 15, 14, 15, 8, 8, 8, 14, 8, 6, 13, 15, 12, 15, 13, 9, 14, 13, 5, 6,
    12, 4, 22, 14, 14, 15, 14, 9, 11, 10, 14, 13, 20, 12, 13, 12, 8, 6, 8, 14,
];

static mut DOC:content::data::Snapshot<4096>=content::data::Snapshot::new();
static mut BYTES:[u8;2400]=[0;2400];static mut SIZE:usize=0;
static mut STARTS:[u16;2048]=[0;2048];static mut LINES:usize=0;
static mut TOP:usize=0;static mut CURRENT:i32=-1;static mut VERSION:[u8;64]=[0;64];static mut VLEN:usize=0;
static mut REVERSE:bool=false;static mut WAIT:i32=0;static mut WAIT_MS:u32=0;static mut ERROR:bool=false;
static mut TIMER_MS:u32=0;static mut RUNNING:bool=false;
static mut POSITION:i32=0;
fn advance(b:u8)->i32 {if (32..=126).contains(&b){ADV24[(b-32) as usize] as i32}else if b&0xc0==0x80 || b<32 {0}else{24}}
unsafe fn wrap(){
 LINES=0;let mut start=0;
 while start<SIZE && LINES<STARTS.len() {
  STARTS[LINES]=start as u16;LINES+=1;
  let mut i=start;let mut width=0;let mut space=None;
  while i<SIZE {
   let b=BYTES[i];if b==b'\n' {i+=1;break;}
   if b==b' ' {space=Some(i);}
   let added=advance(b);
   if width+added>608 && i>start {i=space.filter(|p|*p>start).map(|p|p+1).unwrap_or(i);break;}
   width+=added;i+=1;
  }
  start=i.max(start+1);
 }
}
unsafe fn apply(){
 let chunk=content::data::number(DOC.field(4)) as i32;let version=DOC.field(0);
 if chunk==CURRENT && version==&VERSION[..VLEN] {return;}
 let same=version==&VERSION[..VLEN];VLEN=version.len().min(64);VERSION[..VLEN].copy_from_slice(&version[..VLEN]);
 CURRENT=chunk;SIZE=DOC.field(3).len().min(BYTES.len());BYTES[..SIZE].copy_from_slice(&DOC.field(3)[..SIZE]);wrap();
 TOP=if same && REVERSE {LINES.saturating_sub(11)}else{0};POSITION=TOP as i32*28;REVERSE=false;
 if !same {TIMER_MS=0;RUNNING=false;}
}
unsafe fn tx(x:i32,y:i32,w:i32,h:i32,font:i32,value:&[u8]){let mut n=0;let mut width=0;
 while n<value.len(){let b=value[n];let len=if b<128{1}else if b&0xe0==0xc0{2}else if b&0xf0==0xe0{3}else{4};let add=advance(b);if width+add>w || b==b'\n'{break;}width+=add;n=(n+len).min(value.len());}
 text(x,y,w,h,15,font,0,28,value.as_ptr(),n as i32);}
unsafe fn number(mut n:u32,out:&mut[u8])->usize {let mut temp=[0u8;12];let mut len=0;loop{temp[len]=b'0'+(n%10) as u8;len+=1;n/=10;if n==0{break;}}for i in 0..len{out[i]=temp[len-1-i];}len}
unsafe fn render(){
 begin();tx(20,15,600,38,0,DOC.field(1));tx(20,52,600,28,1,DOC.field(2));
 if SIZE==0 {tx(24,175,592,100,0,b"Choose content on your phone");}
 else {
  let first=(POSITION/28).max(0) as usize;
  for index in first..(first+13).min(LINES){let from=STARTS[index] as usize;let end=if index+1<LINES{STARTS[index+1] as usize}else{SIZE};let mut to=end;while to>from && (BYTES[to-1]==b'\n'||BYTES[to-1]==b' '){to-=1;}
   let y=88+index as i32*28-POSITION;if y>=84 && y<404 {tx(16,y,608,28,0,&BYTES[from..to]);}
  }
 }
 let mut label=[0u8;96];let mut n=0;
 for b in b"Part " {label[n]=*b;n+=1;}n+=number((CURRENT+1).max(0) as u32,&mut label[n..]);
 label[n]=b'/';n+=1;n+=number(content::data::number(DOC.field(5)).max(1) as u32,&mut label[n..]);
 if TIMED {for b in b"  Time "{label[n]=*b;n+=1;}n+=number(TIMER_MS/1000,&mut label[n..]);label[n]=b's';n+=1;}
 tx(20,416,600,28,1,&label[..n]);
 tx(20,449,600,24,1,if ERROR{b"Request failed; swipe to retry"}else if DOC.failed{b"Phone disconnected"}else if WAIT>0{b"Loading next part..."}else if DOC.field(6)!=b"Ready"{DOC.field(6)}else if TIMED {if RUNNING{b"Swipe to scroll / tap to pause timer"}else{b"Swipe to scroll / tap to start timer"}}else{b"Swipe to read / tap for next page"});
 content::cursor(610, 420, true);present();
}
unsafe fn shift(direction:i32){
 if WAIT>0{return;}
 let max=LINES.saturating_sub(11);
 let next=(TOP as i32+direction*6).clamp(0,max as i32) as usize;
 if next!=TOP {TOP=next;return;}
 let part=CURRENT+direction;let count=content::data::number(DOC.field(5)) as i32;
 if part>=0 && part<count {let mut body=[0u8;69];body[1..5].copy_from_slice(&(part as u32).to_le_bytes());body[5..5+VLEN].copy_from_slice(&VERSION[..VLEN]);WAIT=ble_request(METHOD+1,body.as_ptr(),(5+VLEN) as i32);WAIT_MS=0;ERROR=WAIT<=0;REVERSE=direction<0;}
}
pub unsafe fn init(){render();}
pub unsafe fn tick(ms:i32){
 content::context_changed();let dt=ms.clamp(0,250) as u32;
 if DOC.tick(METHOD,dt,250){apply();}
 if WAIT>0 {WAIT_MS+=dt;let len=ble_response_len(WAIT);if len>=0{let mut out=[0u8;384];ERROR=ble_response_read(WAIT,out.as_mut_ptr(),384)<0;WAIT=0;DOC.refresh();}else if len != -1 || WAIT_MS>=3000 {ERROR=true;WAIT=0;}}
 if RUNNING && !content::sidebar_open() && !content::overlay_visible(){TIMER_MS=TIMER_MS.saturating_add(dt);}
 let target=TOP as i32*28;let speed=(dt as i32*560/1000).max(1);POSITION+= (target-POSITION).clamp(-speed,speed);render();
}
pub unsafe fn event(e:i32){
match e{1=>shift(-1),2=>shift(1),0 if TIMED=>RUNNING=!RUNNING,0=>shift(1),_=>{}}render();}

```
<!-- /mira-sdk-file -->


### Shared/replica_commands.rs

<!-- mira-sdk-file: Shared/replica_commands.rs -->
```rust
//! Ordered app actions. Never repeat a possibly completed side effect.
#[link(wasm_import_module="mira")]extern "C" { fn ble_request(m:i32,p:*const u8,n:i32)->i32; fn ble_response_len(i:i32)->i32; fn ble_response_read(i:i32,p:*mut u8,n:i32)->i32; }
pub struct Commands { queue:[u8;32], read:usize, count:usize, pending:i32, age:u32 }
impl Commands {
 pub const fn new()->Self{Self{queue:[0;32],read:0,count:0,pending:0,age:0}}
 pub fn push(&mut self,value:u8){if self.count<32{self.queue[(self.read+self.count)%32]=value;self.count+=1;}}
 pub unsafe fn tick(&mut self,method:i32,ms:u32)->bool {
  self.tick_with(method,ms,ble_request,ble_response_len,ble_response_read)
 }
 pub unsafe fn tick_with(&mut self,method:i32,ms:u32,
  ble_request:unsafe extern "C" fn(i32,*const u8,i32)->i32,
  ble_response_len:unsafe extern "C" fn(i32)->i32,
  ble_response_read:unsafe extern "C" fn(i32,*mut u8,i32)->i32)->bool {
  let mut done=false;
  if self.pending>0{self.age=self.age.saturating_add(ms);let n=ble_response_len(self.pending);if n>=0{let mut b=[0;384];ble_response_read(self.pending,b.as_mut_ptr(),384);self.pending=0;done=true;}else if n != -1 || self.age>3000{self.pending=0;done=true;}}
  if self.pending<=0 && self.count>0{self.pending=ble_request(method,&self.queue[self.read],1);if self.pending>0{self.read=(self.read+1)%32;self.count-=1;self.age=0;}}
  done
 }
}

```
<!-- /mira-sdk-file -->


### Shared/replica_ui.rs

<!-- mira-sdk-file: Shared/replica_ui.rs -->
```rust
//! Typography and drawing for replicas of the archived canvas apps.
#![allow(dead_code)]
const ADV18: [u8; 95] = [
    6, 7, 8, 15, 11, 17, 13, 4, 7, 7, 9, 15, 6, 7, 6, 6, 11, 11, 11, 11, 11, 11, 11, 11, 11, 11, 6,
    6, 15, 15, 15, 10, 18, 12, 12, 13, 14, 11, 10, 14, 14, 6, 6, 12, 10, 16, 14, 14, 11, 14, 13,
    11, 12, 14, 12, 19, 13, 12, 13, 7, 6, 7, 15, 9, 9, 10, 11, 9, 11, 11, 6, 11, 11, 5, 5, 10, 5,
    17, 11, 11, 11, 11, 8, 8, 7, 11, 11, 16, 11, 11, 9, 11, 6, 11, 15,
];
const ADV24: [u8; 95] = [
    6, 7, 8, 15, 13, 19, 17, 4, 8, 8, 12, 14, 5, 7, 5, 8, 15, 10, 13, 12, 15, 14, 14, 13, 14, 14,
    7, 7, 14, 14, 14, 12, 24, 16, 15, 15, 17, 14, 13, 17, 18, 6, 11, 15, 13, 21, 18, 18, 15, 18,
    16, 14, 14, 18, 16, 24, 15, 14, 15, 8, 8, 8, 14, 8, 6, 13, 15, 12, 15, 13, 9, 14, 13, 5, 6, 12,
    4, 22, 14, 14, 15, 14, 9, 11, 10, 14, 13, 20, 12, 13, 12, 8, 6, 8, 14,
];
const ADV32: [u8; 95] = [
    8, 8, 10, 19, 18, 25, 22, 6, 10, 10, 17, 18, 7, 11, 7, 10, 19, 12, 17, 17, 19, 18, 19, 18, 20,
    18, 7, 7, 18, 18, 18, 15, 31, 22, 20, 20, 22, 17, 17, 22, 23, 7, 15, 20, 16, 27, 23, 23, 19,
    23, 20, 18, 18, 23, 21, 32, 20, 20, 20, 11, 10, 10, 18, 10, 8, 18, 19, 16, 19, 18, 11, 19, 19,
    7, 7, 16, 7, 29, 19, 18, 19, 19, 11, 14, 12, 19, 17, 27, 16, 17, 16, 10, 8, 11, 18,
];
const ADV40: [u8; 95] = [
    10, 10, 13, 23, 23, 31, 28, 7, 13, 13, 21, 23, 8, 13, 8, 13, 25, 16, 21, 21, 23, 23, 23, 23,
    24, 22, 10, 10, 23, 23, 23, 20, 38, 27, 25, 26, 28, 22, 21, 28, 29, 9, 18, 25, 21, 35, 29, 30,
    23, 30, 25, 23, 22, 29, 27, 40, 25, 25, 24, 13, 13, 13, 23, 13, 10, 22, 25, 21, 25, 22, 14, 25,
    23, 8, 8, 20, 8, 36, 24, 24, 25, 25, 14, 19, 15, 23, 22, 33, 20, 22, 20, 14, 11, 13, 23,
];
const LATIN_BASE: &[u8] = &[
    192, 0, 65, 193, 0, 65, 194, 0, 65, 195, 0, 65, 196, 0, 65, 197, 0, 65, 199, 0, 67, 200, 0, 69,
    201, 0, 69, 202, 0, 69, 203, 0, 69, 204, 0, 73, 205, 0, 73, 206, 0, 73, 207, 0, 73, 209, 0, 78,
    210, 0, 79, 211, 0, 79, 212, 0, 79, 213, 0, 79, 214, 0, 79, 217, 0, 85, 218, 0, 85, 219, 0, 85,
    220, 0, 85, 221, 0, 89, 224, 0, 97, 225, 0, 97, 226, 0, 97, 227, 0, 97, 228, 0, 97, 229, 0, 97,
    231, 0, 99, 232, 0, 101, 233, 0, 101, 234, 0, 101, 235, 0, 101, 236, 0, 105, 237, 0, 105, 238,
    0, 105, 239, 0, 105, 241, 0, 110, 242, 0, 111, 243, 0, 111, 244, 0, 111, 245, 0, 111, 246, 0,
    111, 249, 0, 117, 250, 0, 117, 251, 0, 117, 252, 0, 117, 253, 0, 121, 255, 0, 121, 0, 1, 65, 1,
    1, 97, 2, 1, 65, 3, 1, 97, 4, 1, 65, 5, 1, 97, 6, 1, 67, 7, 1, 99, 8, 1, 67, 9, 1, 99, 10, 1,
    67, 11, 1, 99, 12, 1, 67, 13, 1, 99, 14, 1, 68, 15, 1, 100, 18, 1, 69, 19, 1, 101, 20, 1, 69,
    21, 1, 101, 22, 1, 69, 23, 1, 101, 24, 1, 69, 25, 1, 101, 26, 1, 69, 27, 1, 101, 28, 1, 71, 29,
    1, 103, 30, 1, 71, 31, 1, 103, 32, 1, 71, 33, 1, 103, 34, 1, 71, 35, 1, 103, 36, 1, 72, 37, 1,
    104, 40, 1, 73, 41, 1, 105, 42, 1, 73, 43, 1, 105, 44, 1, 73, 45, 1, 105, 46, 1, 73, 47, 1,
    105, 48, 1, 73, 52, 1, 74, 53, 1, 106, 54, 1, 75, 55, 1, 107, 57, 1, 76, 58, 1, 108, 59, 1, 76,
    60, 1, 108, 61, 1, 76, 62, 1, 108, 67, 1, 78, 68, 1, 110, 69, 1, 78, 70, 1, 110, 71, 1, 78, 72,
    1, 110, 76, 1, 79, 77, 1, 111, 78, 1, 79, 79, 1, 111, 80, 1, 79, 81, 1, 111, 84, 1, 82, 85, 1,
    114, 86, 1, 82, 87, 1, 114, 88, 1, 82, 89, 1, 114, 90, 1, 83, 91, 1, 115, 92, 1, 83, 93, 1,
    115, 94, 1, 83, 95, 1, 115, 96, 1, 83, 97, 1, 115, 98, 1, 84, 99, 1, 116, 100, 1, 84, 101, 1,
    116, 104, 1, 85, 105, 1, 117, 106, 1, 85, 107, 1, 117, 108, 1, 85, 109, 1, 117, 110, 1, 85,
    111, 1, 117, 112, 1, 85, 113, 1, 117, 114, 1, 85, 115, 1, 117, 116, 1, 87, 117, 1, 119, 118, 1,
    89, 119, 1, 121, 120, 1, 89, 121, 1, 90, 122, 1, 122, 123, 1, 90, 124, 1, 122, 125, 1, 90, 126,
    1, 122, 160, 1, 79, 161, 1, 111, 175, 1, 85, 176, 1, 117, 205, 1, 65, 206, 1, 97, 207, 1, 73,
    208, 1, 105, 209, 1, 79, 210, 1, 111, 211, 1, 85, 212, 1, 117, 213, 1, 85, 214, 1, 117, 215, 1,
    85, 216, 1, 117, 217, 1, 85, 218, 1, 117, 219, 1, 85, 220, 1, 117, 222, 1, 65, 223, 1, 97, 224,
    1, 65, 225, 1, 97, 230, 1, 71, 231, 1, 103, 232, 1, 75, 233, 1, 107, 234, 1, 79, 235, 1, 111,
    236, 1, 79, 237, 1, 111, 240, 1, 106, 244, 1, 71, 245, 1, 103, 248, 1, 78, 249, 1, 110, 250, 1,
    65, 251, 1, 97, 0, 2, 65, 1, 2, 97, 2, 2, 65, 3, 2, 97, 4, 2, 69, 5, 2, 101, 6, 2, 69, 7, 2,
    101, 8, 2, 73, 9, 2, 105, 10, 2, 73, 11, 2, 105, 12, 2, 79, 13, 2, 111, 14, 2, 79, 15, 2, 111,
    16, 2, 82, 17, 2, 114, 18, 2, 82, 19, 2, 114, 20, 2, 85, 21, 2, 117, 22, 2, 85, 23, 2, 117, 24,
    2, 83, 25, 2, 115, 26, 2, 84, 27, 2, 116, 30, 2, 72, 31, 2, 104, 38, 2, 65, 39, 2, 97, 40, 2,
    69, 41, 2, 101, 42, 2, 79, 43, 2, 111, 44, 2, 79, 45, 2, 111, 46, 2, 79, 47, 2, 111, 48, 2, 79,
    49, 2, 111, 50, 2, 89, 51, 2, 121, 126, 3, 59, 0, 30, 65, 1, 30, 97, 2, 30, 66, 3, 30, 98, 4,
    30, 66, 5, 30, 98, 6, 30, 66, 7, 30, 98, 8, 30, 67, 9, 30, 99, 10, 30, 68, 11, 30, 100, 12, 30,
    68, 13, 30, 100, 14, 30, 68, 15, 30, 100, 16, 30, 68, 17, 30, 100, 18, 30, 68, 19, 30, 100, 20,
    30, 69, 21, 30, 101, 22, 30, 69, 23, 30, 101, 24, 30, 69, 25, 30, 101, 26, 30, 69, 27, 30, 101,
    28, 30, 69, 29, 30, 101, 30, 30, 70, 31, 30, 102, 32, 30, 71, 33, 30, 103, 34, 30, 72, 35, 30,
    104, 36, 30, 72, 37, 30, 104, 38, 30, 72, 39, 30, 104, 40, 30, 72, 41, 30, 104, 42, 30, 72, 43,
    30, 104, 44, 30, 73, 45, 30, 105, 46, 30, 73, 47, 30, 105, 48, 30, 75, 49, 30, 107, 50, 30, 75,
    51, 30, 107, 52, 30, 75, 53, 30, 107, 54, 30, 76, 55, 30, 108, 56, 30, 76, 57, 30, 108, 58, 30,
    76, 59, 30, 108, 60, 30, 76, 61, 30, 108, 62, 30, 77, 63, 30, 109, 64, 30, 77, 65, 30, 109, 66,
    30, 77, 67, 30, 109, 68, 30, 78, 69, 30, 110, 70, 30, 78, 71, 30, 110, 72, 30, 78, 73, 30, 110,
    74, 30, 78, 75, 30, 110, 76, 30, 79, 77, 30, 111, 78, 30, 79, 79, 30, 111, 80, 30, 79, 81, 30,
    111, 82, 30, 79, 83, 30, 111, 84, 30, 80, 85, 30, 112, 86, 30, 80, 87, 30, 112, 88, 30, 82, 89,
    30, 114, 90, 30, 82, 91, 30, 114, 92, 30, 82, 93, 30, 114, 94, 30, 82, 95, 30, 114, 96, 30, 83,
    97, 30, 115, 98, 30, 83, 99, 30, 115, 100, 30, 83, 101, 30, 115, 102, 30, 83, 103, 30, 115,
    104, 30, 83, 105, 30, 115, 106, 30, 84, 107, 30, 116, 108, 30, 84, 109, 30, 116, 110, 30, 84,
    111, 30, 116, 112, 30, 84, 113, 30, 116, 114, 30, 85, 115, 30, 117, 116, 30, 85, 117, 30, 117,
    118, 30, 85, 119, 30, 117, 120, 30, 85, 121, 30, 117, 122, 30, 85, 123, 30, 117, 124, 30, 86,
    125, 30, 118, 126, 30, 86, 127, 30, 118, 128, 30, 87, 129, 30, 119, 130, 30, 87, 131, 30, 119,
    132, 30, 87, 133, 30, 119, 134, 30, 87, 135, 30, 119, 136, 30, 87, 137, 30, 119, 138, 30, 88,
    139, 30, 120, 140, 30, 88, 141, 30, 120, 142, 30, 89, 143, 30, 121, 144, 30, 90, 145, 30, 122,
    146, 30, 90, 147, 30, 122, 148, 30, 90, 149, 30, 122, 150, 30, 104, 151, 30, 116, 152, 30, 119,
    153, 30, 121, 160, 30, 65, 161, 30, 97, 162, 30, 65, 163, 30, 97, 164, 30, 65, 165, 30, 97,
    166, 30, 65, 167, 30, 97, 168, 30, 65, 169, 30, 97, 170, 30, 65, 171, 30, 97, 172, 30, 65, 173,
    30, 97, 174, 30, 65, 175, 30, 97, 176, 30, 65, 177, 30, 97, 178, 30, 65, 179, 30, 97, 180, 30,
    65, 181, 30, 97, 182, 30, 65, 183, 30, 97, 184, 30, 69, 185, 30, 101, 186, 30, 69, 187, 30,
    101, 188, 30, 69, 189, 30, 101, 190, 30, 69, 191, 30, 101, 192, 30, 69, 193, 30, 101, 194, 30,
    69, 195, 30, 101, 196, 30, 69, 197, 30, 101, 198, 30, 69, 199, 30, 101, 200, 30, 73, 201, 30,
    105, 202, 30, 73, 203, 30, 105, 204, 30, 79, 205, 30, 111, 206, 30, 79, 207, 30, 111, 208, 30,
    79, 209, 30, 111, 210, 30, 79, 211, 30, 111, 212, 30, 79, 213, 30, 111, 214, 30, 79, 215, 30,
    111, 216, 30, 79, 217, 30, 111, 218, 30, 79, 219, 30, 111, 220, 30, 79, 221, 30, 111, 222, 30,
    79, 223, 30, 111, 224, 30, 79, 225, 30, 111, 226, 30, 79, 227, 30, 111, 228, 30, 85, 229, 30,
    117, 230, 30, 85, 231, 30, 117, 232, 30, 85, 233, 30, 117, 234, 30, 85, 235, 30, 117, 236, 30,
    85, 237, 30, 117, 238, 30, 85, 239, 30, 117, 240, 30, 85, 241, 30, 117, 242, 30, 89, 243, 30,
    121, 244, 30, 89, 245, 30, 121, 246, 30, 89, 247, 30, 121, 248, 30, 89, 249, 30, 121, 239, 31,
    96, 42, 33, 75, 43, 33, 65, 96, 34, 61, 110, 34, 60, 111, 34, 62,
];
fn advance(c: char, font: i32) -> i32 {
    let code = c as u32;
    if (0x300..=0x36f).contains(&code) {
        return 0;
    }
    let mut base = code;
    if code > 126 && code <= 65535 {
        let mut low = 0;
        let mut high = LATIN_BASE.len() / 3;
        while low < high {
            let mid = (low + high) / 2;
            let key = LATIN_BASE[mid * 3] as u32 | ((LATIN_BASE[mid * 3 + 1] as u32) << 8);
            if key < code {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        if low < LATIN_BASE.len() / 3
            && (LATIN_BASE[low * 3] as u32 | ((LATIN_BASE[low * 3 + 1] as u32) << 8)) == code
        {
            base = LATIN_BASE[low * 3 + 2] as u32;
        }
    }
    if (32..=126).contains(&base) {
        if font == 18 {
            ADV18[base as usize - 32] as i32
        } else {
            (match font {
                32 => ADV32[base as usize - 32],
                40 => ADV40[base as usize - 32],
                _ => ADV24[base as usize - 32],
            }) as i32
        }
    } else {
        font
    }
}
#[link(wasm_import_module = "mira")]
extern "C" {
    pub fn begin();
    pub fn present();
    pub fn line(x: i32, y: i32, x2: i32, y2: i32, s: i32, c: i32);
    pub fn stroke_round_rect(x: i32, y: i32, w: i32, h: i32, r: i32, s: i32, c: i32);
    pub fn fill_round_rect(x: i32, y: i32, w: i32, h: i32, r: i32, c: i32);
    pub fn fill_rect(x: i32, y: i32, w: i32, h: i32, c: i32);
    pub fn stroke_rect(x: i32, y: i32, w: i32, h: i32, s: i32, c: i32);
    pub fn text(
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        c: i32,
        f: i32,
        a: i32,
        s: i32,
        p: *const u8,
        n: i32,
    );
}
pub fn width(s: &str, font: i32) -> i32 {
    s.chars().map(|c| advance(c, font)).sum()
}
#[derive(Clone, Copy)]
struct Row {
    b: [u8; 768],
    n: usize,
    width: i32,
}
impl Row {
    const fn new() -> Self {
        Self { b: [0; 768], n: 0, width: 0 }
    }
    fn add(&mut self, s: &str, font: i32) {
        let mut n = s.len().min(self.b.len() - self.n);
        while !s.is_char_boundary(n) { n -= 1; }
        self.b[self.n..self.n + n].copy_from_slice(&s.as_bytes()[..n]);
        self.n += n;
        self.width += width(&s[..n], font);
    }
    fn value(&self) -> &str {
        core::str::from_utf8(&self.b[..self.n]).unwrap_or("")
    }
}
static mut ROWS: [Row; 4] = [Row::new(); 4];
/// Same word wrap, three-dot ellipsis and glyph advances as MiraCanvasShim.
pub unsafe fn label(
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    ink: i32,
    font: i32,
    align: i32,
    spacing: i32,
    max: usize,
    ellipsis: bool,
    break_word: bool,
    value: &[u8],
) {
    let v = core::str::from_utf8(value).unwrap_or("");
    if v.is_empty() {
        return;
    }
    for r in &mut ROWS {
        r.n = 0; r.width = 0;
    }
    let spacing = if spacing > 0 { spacing } else { font };
    let mut limit = if max > 0 { max.min(4) } else { 4 };
    if ellipsis {
        limit = limit.min(((h + spacing - 1) / spacing).max(1) as usize);
    }
    let mut row = 0usize;
    let mut overflow = false;
    'paragraphs: for (pi, p) in v.split('\n').enumerate() {
        if pi > 0 {
            row += 1;
            if row >= limit {
                overflow = true;
                break;
            }
        }
        for word in p.split_whitespace() {
            let ww = width(word, font);
            let used = ROWS[row].width;
            if ROWS[row].n > 0 && used + advance(' ', font) + ww > w {
                row += 1;
                if row >= limit {
                    overflow = true;
                    break 'paragraphs;
                }
            }
            if break_word && ww > w {
                for c in word.chars() {
                    if ROWS[row].n > 0 && ROWS[row].width + advance(c, font) > w {
                        row += 1;
                        if row >= limit {
                            overflow = true;
                            break 'paragraphs;
                        }
                    }
                    let mut b = [0; 4];
                    ROWS[row].add(c.encode_utf8(&mut b), font);
                }
            } else {
                if ROWS[row].n > 0 {
                    ROWS[row].add(" ", font);
                }
                ROWS[row].add(word, font);
            }
        }
    }
    let count = (row + 1).min(4).min(limit);
    if ellipsis && (overflow || row + 1 > limit) && count > 0 {
        let last = &mut ROWS[count - 1];
        while last.n > 0 && width(last.value(), font) + width("...", font) > w {
            last.n -= 1;
            while last.n > 0 && (last.b[last.n] & 0xc0) == 0x80 {
                last.n -= 1;
            }
        }
        last.add("...", font);
    }
    for i in 0..count {
        let r = &ROWS[i];
        if r.n == 0 {
            continue;
        }
        let offset = i as i32 * spacing;
        if offset >= h {
            break;
        }
        // Firmware aligns each already-wrapped line within the original text box.
        text(
            x,
            y + offset,
            w,
            (h - offset).min(match font {
                18 => 18,
                24 => 33,
                32 => 47,
                _ => 57,
            }),
            ink,
            match font {
                18 => 1,
                32 => 2,
                40 => 3,
                _ => 0,
            },
            align,
            spacing,
            r.b.as_ptr(),
            r.n as i32,
        );
    }
}
pub unsafe fn single(x: i32, y: i32, w: i32, h: i32, c: i32, f: i32, a: i32, v: &[u8]) {
    label(x, y, w, h, c, f, a, 0, 1, true, false, v)
}
pub struct Buffer {
    pub b: [u8; 256],
    pub n: usize,
}
impl Buffer {
    pub const fn new() -> Self {
        Self { b: [0; 256], n: 0 }
    }
    pub fn add(&mut self, v: &[u8]) {
        let n = v.len().min(256 - self.n);
        self.b[self.n..self.n + n].copy_from_slice(&v[..n]);
        self.n += n;
    }
    pub fn num(&mut self, mut v: usize) {
        let mut a = [0; 20];
        let mut i = 20;
        loop {
            i -= 1;
            a[i] = b'0' + (v % 10) as u8;
            v /= 10;
            if v == 0 {
                break;
            }
        }
        self.add(&a[i..]);
    }
    pub fn bytes(&self) -> &[u8] {
        &self.b[..self.n]
    }
}
#[link(wasm_import_module = "mira")]
extern "C" {
    pub fn fill_circle(x: i32, y: i32, r: i32, c: i32);
    pub fn stroke_circle(x: i32, y: i32, r: i32, s: i32, c: i32);
    pub fn fill_triangle(x: i32, y: i32, x1: i32, y1: i32, x2: i32, y2: i32, c: i32);
    pub fn bezier(
        x: i32,
        y: i32,
        c0x: i32,
        c0y: i32,
        c1x: i32,
        c1y: i32,
        x1: i32,
        y1: i32,
        s: i32,
        c: i32,
    );
    pub fn bitmap4(x: i32, y: i32, w: i32, h: i32, p: *const u8, n: i32);
}

/// Same 16 fixed-point segments as the original GUIX quadratic primitive.
/// Keeping tessellation in the SDK avoids requiring a new firmware import.
pub unsafe fn quadratic(x0:i32,y0:i32,cx:i32,cy:i32,x1:i32,y1:i32,w:i32,ink:i32){
 let(mut px,mut py)=(x0,y0);
 for i in 1..=16{let u=16-i;let x=(u*u*x0+2*u*i*cx+i*i*x1+128)/256;let y=(u*u*y0+2*u*i*cy+i*i*y1+128)/256;line(px,py,x,y,w,ink);px=x;py=y;}
}

```
<!-- /mira-sdk-file -->


### Shared/retained_frame.rs

<!-- mira-sdk-file: Shared/retained_frame.rs -->
```rust
//! Retain app drawing while an independent SDK overlay changes.
//! Call begin(content_changed), draw app content and checkpoint only when true,
//! then draw overlays and present as usual. No duplicate WASM drawing buffer.
#[link(wasm_import_module = "mira")]
extern "C" {
    fn begin();
    fn frame_checkpoint() -> i32;
    fn frame_restore() -> i32;
}
pub unsafe fn start(content_changed: bool) -> bool {
    if !content_changed && frame_restore() != 0 { return false; }
    begin();
    true
}
pub unsafe fn checkpoint() { frame_checkpoint(); }

```
<!-- /mira-sdk-file -->


### Shared/voice_bars.rs

<!-- mira-sdk-file: Shared/voice_bars.rs -->
```rust
//! Shared glasses Listen meter: microphone energy, smoothed level and geometry.
#[link(wasm_import_module = "mira")]
extern "C" {
    fn fill_round_rect(x: i32, y: i32, w: i32, h: i32, r: i32, color: i32);
}

pub fn advance(level: f32, audio: u8, ms: u32) -> f32 {
    // Lift quieter speech while keeping headroom for louder peaks.
    // This only changes the drawing, never the microphone gain.
    let input = audio as f32 / 255.0;
    let target = 5.0 * input / (1.0 + 4.0 * input);
    let response = if target > level { 45.0 } else { 180.0 };
    let next = level + (target - level) * ms as f32 / (response + ms as f32);
    // Settle the animation only at zero input; never gate a nonzero level.
    if target == 0.0 && next < 0.015 { 0.0 } else { next }
}

pub unsafe fn draw_layer(x: i32, y: i32, level: f32, open: f32, grow: i32, ink: i32) {
    for (i, full) in [8u8, 14, 10, 12, 8].into_iter().enumerate() {
        let half = (((2.0 + (full as f32 - 2.0) * level) * open + 0.5) as i32).max(1);
        let bar_x = x + (i as i32 - 2) * 6;
        fill_round_rect(bar_x - 1 - grow, y - half - grow,
            2 + 2 * grow, 2 * half + 2 * grow, 1 + grow, ink);
    }
}

```
<!-- /mira-sdk-file -->
