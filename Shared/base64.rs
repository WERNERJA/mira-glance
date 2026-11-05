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
