use crate::{SectionInfo, StringRef};

fn printable(b: u8) -> bool {
    (0x20..0x7f).contains(&b) || b == b'\t'
}

/// Maps a file offset to `(vaddr, section name)` when it is inside a section.
fn locate(sections: &[SectionInfo], off: u64) -> Option<(u64, String)> {
    sections.iter().find_map(|s| {
        let start = s.file_offset?;
        (off >= start && off < start + s.size).then(|| (s.addr + (off - start), s.name.clone()))
    })
}

fn push(out: &mut Vec<StringRef>, sections: &[SectionInfo], off: usize, value: String, enc: &str) {
    let loc = locate(sections, off as u64);
    out.push(StringRef {
        addr: loc.as_ref().map_or(off as u64, |l| l.0),
        mapped: loc.is_some(),
        value,
        encoding: enc.into(),
        section: loc.map(|l| l.1),
    });
}

/// Scans the whole file for ASCII and UTF-16LE strings of `min_len`+ chars.
pub(crate) fn scan(data: &[u8], sections: &[SectionInfo], min_len: usize) -> Vec<StringRef> {
    let mut out = Vec::new();
    let mut start = 0usize;
    for i in 0..=data.len() {
        if i < data.len() && printable(data[i]) {
            continue;
        }
        if i - start >= min_len {
            let value = String::from_utf8_lossy(&data[start..i]).into_owned();
            push(&mut out, sections, start, value, "ascii");
        }
        start = i + 1;
    }
    let mut i = 0usize;
    while i + 1 < data.len() {
        let begin = i;
        let mut s = String::new();
        while i + 1 < data.len() && data[i + 1] == 0 && printable(data[i]) {
            s.push(data[i] as char);
            i += 2;
        }
        if s.len() >= min_len {
            push(&mut out, sections, begin, s, "utf16le");
        }
        i = if i == begin { i + 1 } else { i };
    }
    out.sort_by_key(|s| s.addr);
    out
}
