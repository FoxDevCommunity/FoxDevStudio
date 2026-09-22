//! Visual FoxPro single-entry indexes: the `.idx` a table has beside it, one expression each.
//!
//! There are two of them. The older **standard** index is a file of 512-byte nodes with a
//! header in front, and it stores every key whole. The **compact** index, which `COMPACT` and
//! every `.cdx` tag ask for, is the same tree with the keys packed against each other; its
//! header and its nodes are the ones [`crate::cdx`] already reads, so a compact `.idx` is a
//! `.cdx` with one tag in it and no tag directory in front. Nothing here is written down, so
//! both layouts are read off files Visual FoxPro 9 itself wrote.
//!
//! The standard header, 512 bytes, and then the nodes:
//!
//! | offset | what |
//! |---|---|
//! | 0-3 | where the root node starts, counted in bytes from the start of the file |
//! | 4-7 | the first node of the free list, `0xFFFFFFFF` when there is none |
//! | 8-11 | how long the file is |
//! | 12-13 | how long a key is |
//! | 14 | what kind: 1 unique, 8 a FOR condition |
//! | 15 | 1, which is what says the file is an index |
//! | 16-235 | the key expression, with a null after it |
//! | 236-455 | the FOR condition, the same way |
//!
//! | offset in a node | what |
//! |---|---|
//! | 0-1 | 1 marks a root, 2 a leaf, so a tree of one node is 3 |
//! | 2-3 | how many entries the node holds |
//! | 4-7 | the node to the left, `0xFFFFFFFF` when there is none |
//! | 8-11 | the node to the right |
//! | 12- | the entries: the key, then four bytes big-endian - the record number in a leaf, and
//!   in every other node where the child holding keys up to that one starts |
//!
//! A compact index is the `.cdx` tag header at the front of the file, its expressions on the
//! page after it, and its tree from byte 1024 on. What is left over of the header there is
//! three counters Visual FoxPro's own sort leaves behind - how many comparisons it made and how
//! its runs fell out. Nothing reads them back, and they are written as zero.
//!
//! A single-entry index has no direction of its own: Visual FoxPro's `COPY TAG` writes an
//! ascending file from a descending tag, and `SET INDEX ... DESCENDING` is what turns one round
//! when it is opened.

use crate::cdx::{CdxError, Entry, PAGE, Tag};

/// Where the expressions of a standard index sit, and how much room each is given.
const KEY_EXPR_AT: usize = 16;
const FOR_EXPR_AT: usize = 236;
const EXPR_ROOM: usize = 220;

/// The byte that says a file is an index, in both layouts.
const SIGNATURE: u8 = 1;
/// The option bits shared with a compound index: a unique index, and one with a FOR condition.
const OPT_UNIQUE: u8 = 0x01;
const OPT_FOR: u8 = 0x08;
/// The bit that tells a compact index from a standard one.
const OPT_COMPACT: u8 = 0x20;

/// The attributes of a node: a root, and a leaf.
const ATTR_ROOT: u16 = 1;
const ATTR_LEAF: u16 = 2;

/// Whether a file is the compact kind. A standard index has the free list at 4 and its own
/// length at 8, both of which a compact one leaves at zero, and it says so in its options.
pub fn is_compact(bytes: &[u8]) -> bool {
    bytes.len() > 15 && bytes[14] & OPT_COMPACT != 0
}

/// Reads a single-entry index, either kind. The tag comes back unnamed: what an `.idx` is
/// called is the name of the file it is in, which only the caller knows.
pub fn read(bytes: &[u8]) -> Result<Tag, CdxError> {
    if bytes.len() < 2 * PAGE {
        return Err(CdxError { message: "not an index: the file is too short".into() });
    }
    if is_compact(bytes) { read_compact(bytes) } else { read_standard(bytes) }
}

/// Writes one, `compact` deciding which of the two layouts it is written in.
pub fn write(tag: &Tag, compact: bool) -> Vec<u8> {
    if compact { write_compact(tag) } else { write_standard(tag) }
}

// ---------------------------------------------------------------- the compact kind

fn read_compact(bytes: &[u8]) -> Result<Tag, CdxError> {
    let header = crate::cdx::read_tag_header(bytes, 0)?;
    let (entries, numeric) = crate::cdx::read_entries(bytes, header.root, header.key_len)?;
    Ok(Tag {
        name: String::new(),
        key_expr: header.key_expr,
        for_expr: header.for_expr,
        key_len: header.key_len,
        descending: header.descending,
        unique: header.options & OPT_UNIQUE != 0,
        candidate: false,
        numeric,
        entries,
    })
}

fn write_compact(tag: &Tag) -> Vec<u8> {
    let options = OPT_COMPACT | if tag.unique { OPT_UNIQUE } else { 0 } | if tag.for_expr.is_empty() { 0 } else { OPT_FOR };
    let mut out = Vec::new();
    crate::cdx::write_tag(
        &mut out,
        0,
        tag.key_len,
        &tag.key_expr,
        &tag.for_expr,
        options,
        false,
        &tag.entries,
        if tag.numeric { 0 } else { b' ' },
    );
    out
}

// ---------------------------------------------------------------- the standard kind

/// How many entries a node of a standard index holds: they are written whole, key and the four
/// bytes after it, from byte 12 of the node on.
fn per_node(key_len: usize) -> usize {
    (PAGE - 12) / (key_len + 4)
}

fn u16_at(p: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([p[at], p[at + 1]])
}

fn u32_at(p: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([p[at], p[at + 1], p[at + 2], p[at + 3]])
}

fn u32_be_at(p: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([p[at], p[at + 1], p[at + 2], p[at + 3]])
}

fn c_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    bytes[..end].iter().map(|&b| b as char).collect::<String>().trim().to_string()
}

fn node(bytes: &[u8], at: u32) -> Result<&[u8], CdxError> {
    let start = at as usize;
    bytes
        .get(start..start + PAGE)
        .ok_or_else(|| CdxError { message: format!("index node {at} is past the end of the file") })
}

fn read_standard(bytes: &[u8]) -> Result<Tag, CdxError> {
    let key_len = u16_at(bytes, 12) as usize;
    if key_len == 0 || key_len + 4 > PAGE - 12 {
        return Err(CdxError { message: format!("an index key of {key_len} bytes cannot be right") });
    }
    let options = bytes[14];
    let key_expr = c_string(&bytes[KEY_EXPR_AT..KEY_EXPR_AT + EXPR_ROOM]);
    let for_expr = c_string(&bytes[FOR_EXPR_AT..FOR_EXPR_AT + EXPR_ROOM]);
    let entries = read_standard_entries(bytes, u32_at(bytes, 0), key_len)?;
    let numeric = crate::cdx::numeric_keys(key_len, entries.iter().map(|e| e.key.as_slice()));
    Ok(Tag {
        name: String::new(),
        key_expr,
        for_expr,
        key_len,
        descending: false,
        unique: options & OPT_UNIQUE != 0,
        candidate: false,
        numeric,
        entries,
    })
}

/// Every entry under a root, in order: down the left edge to the first leaf, then along the
/// leaves' right pointers.
fn read_standard_entries(bytes: &[u8], root: u32, key_len: usize) -> Result<Vec<Entry>, CdxError> {
    let mut at = root;
    loop {
        let p = node(bytes, at)?;
        if u16_at(p, 0) & ATTR_LEAF != 0 {
            break;
        }
        if u16_at(p, 2) == 0 {
            return Ok(Vec::new());
        }
        at = u32_be_at(p, 12 + key_len);
    }
    let mut out = Vec::new();
    let mut seen = 0usize;
    loop {
        let p = node(bytes, at)?;
        let count = u16_at(p, 2) as usize;
        for i in 0..count.min(per_node(key_len)) {
            let start = 12 + i * (key_len + 4);
            out.push(Entry { key: p[start..start + key_len].to_vec(), recno: u32_be_at(p, start + key_len) });
        }
        seen += 1;
        if seen > bytes.len() / PAGE + 1 {
            return Err(CdxError { message: "the index's leaves loop".into() });
        }
        let right = u32_at(p, 8);
        if right == u32::MAX {
            break;
        }
        at = right;
    }
    Ok(out)
}

fn write_standard(tag: &Tag) -> Vec<u8> {
    let key_len = tag.key_len.max(1);
    let pad = if tag.numeric { 0 } else { b' ' };
    let mut nodes: Vec<Vec<u8>> = Vec::new();
    // the nodes start after the header, and a node is known by where it starts in the file
    let start = PAGE as u32;
    let leaves = build_standard_leaves(&tag.entries, key_len, pad, &mut nodes, start);
    let root = build_standard_interior(leaves, key_len, &mut nodes, start);

    let mut out = vec![0u8; PAGE];
    out[0..4].copy_from_slice(&root.to_le_bytes());
    out[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
    out[8..12].copy_from_slice(&(((nodes.len() + 1) * PAGE) as u32).to_le_bytes());
    out[12..14].copy_from_slice(&(key_len as u16).to_le_bytes());
    out[14] = if tag.unique { OPT_UNIQUE } else { 0 } | if tag.for_expr.is_empty() { 0 } else { OPT_FOR };
    out[15] = SIGNATURE;
    put_expr(&mut out, KEY_EXPR_AT, &tag.key_expr);
    put_expr(&mut out, FOR_EXPR_AT, &tag.for_expr);
    for p in nodes {
        out.extend_from_slice(&p);
    }
    out
}

/// One expression in the room the header gives it, with the null after it. Anything longer is
/// cut, because there is nowhere else for it to go.
fn put_expr(out: &mut [u8], at: usize, text: &str) {
    let bytes: Vec<u8> = text.bytes().take(EXPR_ROOM - 1).collect();
    out[at..at + bytes.len()].copy_from_slice(&bytes);
}

/// Fills leaves in key order and links them left to right. Answers the last key, the last
/// record number and where each one starts, which is what the level above it is built from.
fn build_standard_leaves(
    entries: &[Entry],
    key_len: usize,
    pad: u8,
    nodes: &mut Vec<Vec<u8>>,
    start: u32,
) -> Vec<(Vec<u8>, u32, u32)> {
    let per = per_node(key_len).max(1);
    let mut leaves: Vec<(Vec<u8>, u32, u32)> = Vec::new();
    // an index with nothing in it still has a root, which is an empty leaf
    let groups: Vec<&[Entry]> = if entries.is_empty() { vec![&[]] } else { entries.chunks(per).collect() };
    for group in groups {
        let at = start + (nodes.len() as u32) * PAGE as u32;
        let mut p = vec![0u8; PAGE];
        p[0..2].copy_from_slice(&ATTR_LEAF.to_le_bytes());
        p[2..4].copy_from_slice(&(group.len() as u16).to_le_bytes());
        p[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        p[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        for (i, entry) in group.iter().enumerate() {
            let mut key = entry.key.clone();
            key.resize(key_len, pad);
            let field = 12 + i * (key_len + 4);
            p[field..field + key_len].copy_from_slice(&key);
            p[field + key_len..field + key_len + 4].copy_from_slice(&entry.recno.to_be_bytes());
        }
        let last = group.last();
        leaves.push((
            last.map(|e| {
                let mut key = e.key.clone();
                key.resize(key_len, pad);
                key
            })
            .unwrap_or_else(|| vec![pad; key_len]),
            last.map_or(0, |e| e.recno),
            at,
        ));
        nodes.push(p);
    }
    link(&leaves, nodes, start);
    leaves
}

/// Builds the levels above `children` until one node is left, and answers where it starts.
fn build_standard_interior(
    mut children: Vec<(Vec<u8>, u32, u32)>,
    key_len: usize,
    nodes: &mut Vec<Vec<u8>>,
    start: u32,
) -> u32 {
    if children.len() == 1 {
        let which = (children[0].2 - start) as usize / PAGE;
        let attr = u16_at(&nodes[which], 0) | ATTR_ROOT;
        nodes[which][0..2].copy_from_slice(&attr.to_le_bytes());
        return children[0].2;
    }
    let per = per_node(key_len).max(1);
    let mut parents: Vec<(Vec<u8>, u32, u32)> = Vec::new();
    while !children.is_empty() {
        let group: Vec<(Vec<u8>, u32, u32)> = children.drain(..children.len().min(per)).collect();
        let at = start + (nodes.len() as u32) * PAGE as u32;
        let mut p = vec![0u8; PAGE];
        p[2..4].copy_from_slice(&(group.len() as u16).to_le_bytes());
        p[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        p[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        for (i, (key, _, child)) in group.iter().enumerate() {
            let field = 12 + i * (key_len + 4);
            p[field..field + key_len].copy_from_slice(&key[..key_len]);
            p[field + key_len..field + key_len + 4].copy_from_slice(&child.to_be_bytes());
        }
        let last = group.last().expect("a group has an entry");
        parents.push((last.0.clone(), last.1, at));
        nodes.push(p);
    }
    link(&parents, nodes, start);
    build_standard_interior(parents, key_len, nodes, start)
}

/// Points every node of one level at its neighbours.
fn link(level: &[(Vec<u8>, u32, u32)], nodes: &mut [Vec<u8>], start: u32) {
    for k in 0..level.len() {
        let which = (level[k].2 - start) as usize / PAGE;
        let left = if k > 0 { level[k - 1].2 } else { u32::MAX };
        let right = if k + 1 < level.len() { level[k + 1].2 } else { u32::MAX };
        nodes[which][4..8].copy_from_slice(&left.to_le_bytes());
        nodes[which][8..12].copy_from_slice(&right.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cdx::{encode_number, encode_text};

    fn sample(key_len: usize, count: u32) -> Tag {
        let mut entries: Vec<Entry> =
            (1..=count).map(|n| Entry { key: encode_text(&format!("K{n:06}"), key_len), recno: n }).collect();
        entries.sort_by(|a, b| (&a.key, a.recno).cmp(&(&b.key, b.recno)));
        Tag {
            name: String::new(),
            key_expr: "UPPER(name)".into(),
            for_expr: String::new(),
            key_len,
            descending: false,
            unique: false,
            candidate: false,
            numeric: false,
            entries,
        }
    }

    #[test]
    fn a_standard_index_reads_back_whole() {
        for count in [1u32, 5, 40, 500] {
            let tag = sample(10, count);
            let bytes = write(&tag, false);
            assert_eq!(bytes.len() % PAGE, 0);
            assert!(!is_compact(&bytes));
            let back = read(&bytes).expect("reads");
            assert_eq!(back.entries, tag.entries, "{count} entries");
            assert_eq!(back.key_expr, "UPPER(name)");
            assert_eq!(back.key_len, 10);
        }
    }

    #[test]
    fn a_compact_index_reads_back_whole() {
        for count in [1u32, 5, 40, 500] {
            let tag = sample(10, count);
            let bytes = write(&tag, true);
            assert!(is_compact(&bytes));
            let back = read(&bytes).expect("reads");
            assert_eq!(back.entries, tag.entries, "{count} entries");
            assert_eq!(back.key_expr, "UPPER(name)");
        }
    }

    #[test]
    fn a_number_key_comes_back_as_a_number_in_either_kind() {
        let mut tag = sample(8, 0);
        tag.numeric = true;
        tag.key_expr = "amount".into();
        tag.entries = (1..=6u32).map(|n| Entry { key: encode_number(f64::from(n) - 3.0).to_vec(), recno: n }).collect();
        for compact in [false, true] {
            let back = read(&write(&tag, compact)).expect("reads");
            assert!(back.numeric, "compact {compact}");
            assert_eq!(back.entries, tag.entries);
        }
    }

    #[test]
    fn a_unique_index_with_a_condition_says_so_in_either_kind() {
        let mut tag = sample(10, 3);
        tag.unique = true;
        tag.for_expr = "amount>0".into();
        for compact in [false, true] {
            let bytes = write(&tag, compact);
            let back = read(&bytes).expect("reads");
            assert!(back.unique, "compact {compact}");
            assert_eq!(back.for_expr, "amount>0", "compact {compact}");
        }
    }
}
