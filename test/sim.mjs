// Minimal Mira host simulator: runs the WASM from the packaged HTML, emulates the
// companion bridge (0x64 identity, 0x62 inbound snapshot, 0x63 outbound), and
// records draw calls per frame as SVG.
import fs from 'fs';
const [,, htmlPath, snapshotPath, outDir] = process.argv;
const html = fs.readFileSync(htmlPath, 'utf8');
const b64 = /<script type="application\/mira-wasm">([^<]+)<\/script>/.exec(html)[1];
const abi = JSON.parse(fs.readFileSync(new URL('../SDK-ABI.json', import.meta.url)));
const snapshotJson = fs.readFileSync(snapshotPath, 'utf8');
let mem; const u8 = () => new Uint8Array(mem.buffer);
const enc = new TextEncoder(), dec = new TextDecoder();
let nextId = 1; const responses = new Map(); const outbound = [];
let inbound = {rev: 0, seq: 0, bytes: new Uint8Array(0)};
function publish(json){ inbound = {rev: inbound.rev + 1, seq: inbound.seq + 1, bytes: enc.encode(String(inbound.seq + 1) + '\0' + json)}; }
let outMsg = {id: 0, chunks: []};
let frame = []; let frames = 0; let traps = 0;
let media = null;
const status = [87, 0, 1, 1, 1, 9, 41, 11, 5, 4];
function le32(n){ return [n & 255, (n >> 8) & 255, (n >> 16) & 255, (n >>> 24) & 255]; }
function le16(n){ return [n & 255, (n >> 8) & 255]; }
const imports = {mira: {}};
for (const [name, [argc, ret]] of Object.entries(abi)) imports.mira[name] = () => ret ? 0 : undefined;
const rec = (op) => (...a) => { frame.push([op, ...a]); };
Object.assign(imports.mira, {
  begin(){ frame = []; },
  present(){ frames++; },
  fill_rect: rec('fill_rect'), stroke_rect: rec('stroke_rect'), fill_circle: rec('fill_circle'), stroke_circle: rec('stroke_circle'),
  fill_round_rect: rec('fill_round_rect'), stroke_round_rect: rec('stroke_round_rect'), fill_triangle: rec('fill_triangle'), line: rec('line'),
  text(x, y, w, h, c, f, a, s, p, n){ frame.push(['text', x, y, w, h, c, f, a, dec.decode(u8().slice(p, p + n))]); },
  frame_checkpoint(){ return 0; }, frame_restore(){ return 0; },
  content_info(){ return 0; }, content_cursor(){}, gesture_policy(){ return 1; },
  device_status(p, cap){ u8().set(status, p); return 10; },
  media_state(p, cap){ if (!media) { u8().set([0, 0], p); return 2; } const b = [1, 1, ...enc.encode(media[0]), 0, ...enc.encode(media[1]), 0]; u8().set(b, p); return b.length; },
  now_ms(){ return Date.now() & 0x7fffffff; },
  ble_request(method, p, n){
    const payload = u8().slice(p, p + n); const id = nextId++;
    let resp = new Uint8Array(0);
    if (method === 0x64) resp = new Uint8Array(le32(7));
    else if (method === 0x62) {
      const dv = new DataView(payload.buffer); const rev = dv.getUint32(0, true), off = dv.getUint16(4, true);
      if (inbound.rev === 0) resp = new Uint8Array([...le32(0), 0, 0, 0xff, 0xff]);
      else if (rev === inbound.rev && off === 0) resp = new Uint8Array([...le32(inbound.rev), 0, 0, 0xff, 0xff]);
      else { const chunk = inbound.bytes.slice(off, off + 376); resp = new Uint8Array([...le32(inbound.rev), ...le16(off), ...le16(inbound.bytes.length), ...chunk]); }
    } else if (method === 0x63) {
      if (payload[0] === 1) {
        const dv = new DataView(payload.buffer); const mid = dv.getUint32(1, true), off = dv.getUint16(5, true), total = dv.getUint16(7, true);
        if (mid !== outMsg.id) outMsg = {id: mid, chunks: []};
        outMsg.chunks.push(payload.slice(9));
        const all = Buffer.concat(outMsg.chunks.map(c => Buffer.from(c)));
        if (all.length === total) outbound.push(all.toString('utf8'));
      }
    }
    responses.set(id, resp); return id;
  },
  ble_response_len(id){ return responses.has(id) ? responses.get(id).length : -2; },
  ble_response_read(id, p, cap){ const r = responses.get(id); if (!r) return -2; if (r.length > cap) return -4; u8().set(r, p); responses.delete(id); return r.length; },
});
const { instance } = await WebAssembly.instantiate(Buffer.from(b64, 'base64'), imports);
mem = instance.exports.memory;
const X = instance.exports;
const tick = (n, ms = 33) => { for (let i = 0; i < n; i++) X.app_render(ms); };
function svg(f){
  const col = g => { const v = Math.round(g / 15 * 255); return `rgb(${Math.round(v*0.55)},${v},${Math.round(v*0.6)})`; };
  const size = [24, 18, 32, 40];
  const out = [`<svg xmlns="http://www.w3.org/2000/svg" width="640" height="480" viewBox="0 0 640 480"><rect width="640" height="480" fill="#000"/>`];
  const esc = s => s.replace(/&/g,'&amp;').replace(/</g,'&lt;');
  for (const [op, ...a] of f) {
    if (op === 'fill_rect') out.push(`<rect x="${a[0]}" y="${a[1]}" width="${a[2]}" height="${a[3]}" fill="${col(a[4])}"/>`);
    else if (op === 'stroke_rect') out.push(`<rect x="${a[0]}" y="${a[1]}" width="${a[2]}" height="${a[3]}" fill="none" stroke-width="${a[4]}" stroke="${col(a[5])}"/>`);
    else if (op === 'fill_round_rect') out.push(`<rect x="${a[0]}" y="${a[1]}" width="${a[2]}" height="${a[3]}" rx="${a[4]}" fill="${col(a[5])}"/>`);
    else if (op === 'stroke_round_rect') out.push(`<rect x="${a[0]}" y="${a[1]}" width="${a[2]}" height="${a[3]}" rx="${a[4]}" fill="none" stroke-width="${a[5]}" stroke="${col(a[6])}"/>`);
    else if (op === 'fill_circle') out.push(`<circle cx="${a[0]}" cy="${a[1]}" r="${a[2]}" fill="${col(a[3])}"/>`);
    else if (op === 'stroke_circle') out.push(`<circle cx="${a[0]}" cy="${a[1]}" r="${a[2]}" fill="none" stroke-width="${a[3]}" stroke="${col(a[4])}"/>`);
    else if (op === 'fill_triangle') out.push(`<polygon points="${a[0]},${a[1]} ${a[2]},${a[3]} ${a[4]},${a[5]}" fill="${col(a[6])}"/>`);
    else if (op === 'line') out.push(`<line x1="${a[0]}" y1="${a[1]}" x2="${a[2]}" y2="${a[3]}" stroke-width="${a[4]}" stroke-linecap="round" stroke="${col(a[5])}"/>`);
    else if (op === 'text') { const [x, y, w, h, c, fnt, al, s] = a; const px = size[fnt] || 24; const tx = al === 1 ? x + w / 2 : al === 2 ? x + w : x; out.push(`<text x="${tx}" y="${y + px * 0.82}" font-family="Inter, Helvetica, Arial, sans-serif" font-size="${px}" text-anchor="${['start','middle','end'][al] || 'start'}" fill="${col(c)}">${esc(s)}</text>`); }
  }
  out.push('</svg>'); return out.join('\n');
}
const report = {};
X.app_init();
tick(5);
fs.writeFileSync(`${outDir}/00-empty.svg`, svg(frame));
publish(snapshotJson);
tick(120);
report.outbound = outbound.slice();
report.textSlots = {};
const names = ['overzicht', 'agenda', 'weer', 'nieuws', 'markten', 'todo'];
for (let i = 0; i < 6; i++) {
  if (i === 0) { media = null; tick(40); }
  fs.writeFileSync(`${outDir}/${String(i + 1).padStart(2, '0')}-${names[i]}.svg`, svg(frame));
  report.textSlots[names[i]] = {texts: frame.filter(f => f[0] === 'text').length, prims: frame.length};
  X.app_event(2); tick(3);
}
// back at overview: now-playing variant
media = ['Clair de Lune', 'Claude Debussy'];
tick(40);
fs.writeFileSync(`${outDir}/07-overzicht-muziek.svg`, svg(frame));
// news tap pages to the next headlines
X.app_event(2); X.app_event(2); X.app_event(2); tick(3); X.app_event(0); tick(3);
fs.writeFileSync(`${outDir}/08-nieuws-meer.svg`, svg(frame));
// tap elsewhere requests refresh
X.app_event(1); tick(3); X.app_event(0); tick(60);
report.outboundAfterTap = outbound.slice();
report.frames = frames;
console.log(JSON.stringify(report, null, 1));
