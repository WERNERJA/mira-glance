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
