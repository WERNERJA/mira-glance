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
