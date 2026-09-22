//! Visual FoxPro compound indexes: the `.cdx` beside a table.
//!
//! A compound index is a file of 512-byte pages. Page 0 is the header of the tag directory,
//! which is itself an index whose keys are tag names and whose "record numbers" are the pages
//! the tags' own headers sit on. Each tag is a B+ tree: interior pages hold a key, a record
//! number and a child page per entry; leaf pages hold the keys compressed against each other -
//! a count of bytes shared with the previous key and a count of trailing pad bytes are stored
//! per entry, and only what is left of the key is written, from the end of the page backwards.
//!
//! Reading one means walking the leaves left to right, which gives every key in order with the
//! record it belongs to. That list is what SET ORDER and SEEK work on; the tree structure is
//! not kept, because a table small enough to matter here has a list small enough to hold.
//!
//! Writing one builds the tree back from that list, compact leaves and all, so a table this
//! runtime indexes can be opened by Visual FoxPro itself.

use std::fmt;

pub const PAGE: usize = 512;

/// The attributes word of a page: bit 1 marks a root, bit 2 a leaf.
const ATTR_ROOT: u16 = 1;
const ATTR_LEAF: u16 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CdxError {
    pub message: String,
}

impl fmt::Display for CdxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

fn err(message: impl Into<String>) -> CdxError {
    CdxError { message: message.into() }
}

/// One entry of a tag: the key as bytes in the order the index sorts them, and the record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub key: Vec<u8>,
    pub recno: u32,
}

/// One tag of a compound index, read in full.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    /// The tag name, upper-cased and trimmed, as VFP shows it.
    pub name: String,
    /// The key expression, as the index was built from.
    pub key_expr: String,
    /// The FOR condition, empty when there is none.
    pub for_expr: String,
    pub key_len: usize,
    pub descending: bool,
    pub unique: bool,
    /// The candidate flag, which is what a primary key is stored as.
    pub candidate: bool,
    /// True when the keys are numbers or dates: eight bytes each, stored as they are.
    pub numeric: bool,
    /// Every key with its record, in index order.
    pub entries: Vec<Entry>,
}

// ---------------------------------------------------------------- reading

fn u16_at(p: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([p[at], p[at + 1]])
}

fn u32_at(p: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([p[at], p[at + 1], p[at + 2], p[at + 3]])
}

fn u32_be_at(p: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([p[at], p[at + 1], p[at + 2], p[at + 3]])
}

fn page(bytes: &[u8], at: u32) -> Result<&[u8], CdxError> {
    let start = at as usize;
    bytes.get(start..start + PAGE).ok_or_else(|| err(format!("index page {at} is past the end of the file")))
}

/// An expression as Visual FoxPro writes it down when it keeps one - an index key, an index FOR,
/// a filter, a relation.
///
/// It is not the text the program typed. The product keeps the expression it compiled and prints
/// it back out from that, so the spaces between the pieces are gone: `INDEX ON code TAG t FOR
/// price > 1` is kept as `price>1`. The names keep the case they were written in, which is what
/// the index files Visual FoxPro wrote for this crate's fixtures show and what `ATAGINFO()` hands
/// back.
pub fn stored_expr(text: &str) -> String {
    squeezed(text, false)
}

/// The same expression as the functions that ask about one answer with it: every name upper-cased.
///
/// `KEY()`, `FOR()`, `FILTER()` and `RELATION()` all do this, and so does the key column of
/// `ATAGINFO()`; only ATAGINFO's filter column hands back what the file holds. What is inside
/// quotes is part of a value rather than a name, so it is left exactly as written -
/// `INDEX ON UPPER(name) + "tail"` reads back as `UPPER(NAME)+"tail"`.
pub fn compiled_expr(text: &str) -> String {
    squeezed(text, true)
}

/// True of the characters that would run into each other if the space between them went.
fn runs_on(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '.'
}

fn squeezed(text: &str, upper: bool) -> String {
    let chars: Vec<char> = text.trim().chars().collect();
    let mut out = String::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            let next = chars[i..].iter().position(|c| !c.is_whitespace()).map(|n| i + n);
            match next {
                // a space is only worth keeping where it holds two words apart
                Some(at) => {
                    if out.chars().next_back().is_some_and(runs_on) && runs_on(chars[at]) {
                        out.push(' ');
                    }
                    i = at;
                }
                None => break,
            }
            continue;
        }
        // `[` opens a string unless it subscripts what came just before it, which is how the
        // lexer tells the two apart as well
        let opens_string = matches!(c, '"' | '\'')
            || (c == '[' && !out.chars().next_back().is_some_and(|p| runs_on(p) || p == ')' || p == ']'));
        if opens_string {
            let close = if c == '[' { ']' } else { c };
            let end = chars[i + 1..].iter().position(|&x| x == close).map_or(chars.len(), |n| i + n + 2);
            out.extend(&chars[i..end]);
            i = end;
            continue;
        }
        out.push(if upper { c.to_ascii_uppercase() } else { c });
        i += 1;
    }
    out
}

/// The header of a tag, or of the tag directory: where its root is and what its keys are.
#[derive(Debug, Clone)]
pub(crate) struct TagHeader {
    pub root: u32,
    pub key_len: usize,
    pub options: u8,
    pub descending: bool,
    pub key_expr: String,
    pub for_expr: String,
}

/// Where the header keeps how long each of the two expressions is, counting the null after it.
/// A tag with no FOR condition still has a length of one there, for that null alone.
const KEY_LEN_AT: usize = 510;
const FOR_LEN_AT: usize = 506;

pub(crate) fn read_tag_header(bytes: &[u8], at: u32) -> Result<TagHeader, CdxError> {
    let start = at as usize;
    let p = bytes.get(start..start + 2 * PAGE).ok_or_else(|| err(format!("index header {at} is past the end of the file")))?;
    let key_len = u16_at(p, 12) as usize;
    let for_len = u16_at(p, FOR_LEN_AT) as usize;
    let key_expr_len = (u16_at(p, KEY_LEN_AT) as usize).min(PAGE);
    // the expressions sit on the page after the header, each with a null after it: the key
    // first, then the FOR condition where the key's own length says the key ended
    let text = &p[PAGE..2 * PAGE];
    let key_expr = c_string(&text[..key_expr_len]);
    let for_expr = if for_len > 1 { c_string(&text[key_expr_len..(key_expr_len + for_len).min(PAGE)]) } else { String::new() };
    Ok(TagHeader {
        root: u32_at(p, 0),
        key_len,
        options: p[14],
        descending: u16_at(p, 502) == 1,
        key_expr,
        for_expr,
    })
}

fn c_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    bytes[..end].iter().map(|&b| b as char).collect::<String>().trim().to_string()
}

/// One entry as a leaf stores it: how much it shares with the key before it and the bytes in
/// between, which is all that is written for it.
struct Stored {
    dup: usize,
    bytes: Vec<u8>,
    recno: u32,
}

/// Every entry under a root, in order, and whether its keys are numbers: the leaves are found
/// by descending the leftmost path, then followed along their right pointers.
pub(crate) fn read_entries(bytes: &[u8], root: u32, key_len: usize) -> Result<(Vec<Entry>, bool), CdxError> {
    let mut at = root;
    // down the left edge to the first leaf
    loop {
        let p = page(bytes, at)?;
        let attr = u16_at(p, 0);
        if attr & ATTR_LEAF != 0 {
            break;
        }
        let count = u16_at(p, 2) as usize;
        if count == 0 {
            return Ok((Vec::new(), false));
        }
        // interior entry: key, record number, child page
        at = u32_be_at(p, 12 + key_len + 4);
    }
    let mut raw: Vec<Stored> = Vec::new();
    let mut seen = 0usize;
    loop {
        let p = page(bytes, at)?;
        read_leaf(p, key_len, &mut raw)?;
        seen += 1;
        if seen > bytes.len() / PAGE + 1 {
            return Err(err("the index's leaves loop"));
        }
        let right = u32_at(p, 8);
        if right == u32::MAX {
            break;
        }
        at = right;
    }
    let pad = pad_byte(key_len, &raw);
    Ok((materialize(&raw, key_len, pad), pad == 0))
}

/// Whether a tag's keys are numbers. Nothing in the file says so - Visual FoxPro knows it from
/// the type of the key expression - so it is read off the keys themselves: a number's eight
/// bytes carry the sign bit, or the flipped bits of a negative one, and text does not.
pub(crate) fn numeric_keys<'a>(key_len: usize, mut keys: impl Iterator<Item = &'a [u8]>) -> bool {
    key_len == 8 && keys.any(|k| k.iter().any(|b| !(0x20..0x80).contains(b)))
}

/// The byte a tag's keys are padded out to their full length with: spaces for text, and zero
/// bytes for the eight of a number or a date, which are the number itself.
fn pad_byte(key_len: usize, stored: &[Stored]) -> u8 {
    if numeric_keys(key_len, stored.iter().map(|s| s.bytes.as_slice())) { 0 } else { b' ' }
}

/// The keys the stored entries stand for, each grown back to the full key length.
fn materialize(stored: &[Stored], key_len: usize, pad: u8) -> Vec<Entry> {
    let mut out = Vec::with_capacity(stored.len());
    let mut previous: Vec<u8> = Vec::new();
    for s in stored {
        let mut key = Vec::with_capacity(key_len);
        key.extend_from_slice(&previous[..s.dup.min(previous.len())]);
        key.extend_from_slice(&s.bytes);
        key.resize(key_len, pad);
        previous = key.clone();
        out.push(Entry { key, recno: s.recno });
    }
    out
}

/// Unpacks a compact leaf: the entries, then the keys stored backwards from the end.
fn read_leaf(p: &[u8], key_len: usize, out: &mut Vec<Stored>) -> Result<(), CdxError> {
    let count = u16_at(p, 2) as usize;
    let recno_mask = u32_at(p, 14);
    let dup_mask = p[18] as u32;
    let trail_mask = p[19] as u32;
    let recno_bits = p[20] as u32;
    let dup_bits = p[21] as u32;
    let holding = p[23] as usize;
    if holding == 0 || holding > 8 {
        return Err(err("an index leaf holds no entries"));
    }
    let mut end = PAGE;
    for i in 0..count {
        let at = 24 + i * holding;
        let mut packed = 0u64;
        for (k, b) in p[at..at + holding].iter().enumerate() {
            packed |= (*b as u64) << (8 * k);
        }
        let recno = (packed & recno_mask as u64) as u32;
        let dup = ((packed >> recno_bits) & dup_mask as u64) as usize;
        let trail = ((packed >> (recno_bits + dup_bits)) & trail_mask as u64) as usize;
        let stored = key_len.saturating_sub(dup + trail);
        if end < stored {
            return Err(err("an index leaf's keys overrun the page"));
        }
        end -= stored;
        out.push(Stored { dup, bytes: p[end..end + stored].to_vec(), recno });
    }
    Ok(())
}

/// Reads every tag of a compound index.
pub fn read_cdx(bytes: &[u8]) -> Result<Vec<Tag>, CdxError> {
    if bytes.len() < 2 * PAGE {
        return Err(err("not a compound index: the file is too short"));
    }
    let directory = read_tag_header(bytes, 0)?;
    let (mut names, _) = read_entries(bytes, directory.root, directory.key_len)?;
    // the directory holds the names in sorted order, but a tag is numbered by where its header
    // sits in the file, which is the order the tags were made in: what TAG(n) and SET ORDER TO
    // n count down
    names.sort_by_key(|e| e.recno);
    let mut tags = Vec::with_capacity(names.len());
    for entry in names {
        let name = c_string(&entry.key);
        let header = read_tag_header(bytes, entry.recno)?;
        let (entries, numeric) = read_entries(bytes, header.root, header.key_len)?;
        tags.push(Tag {
            name: name.to_ascii_uppercase(),
            key_expr: header.key_expr,
            for_expr: header.for_expr,
            key_len: header.key_len,
            descending: header.descending,
            unique: header.options & 0x01 != 0,
            candidate: header.options & 0x04 != 0,
            numeric,
            entries,
        });
    }
    Ok(tags)
}

// ---------------------------------------------------------------- key encoding

/// A number as an index key: the IEEE double in big-endian order, with the sign bit flipped for
/// a positive number and every bit flipped for a negative one, so that bytes sort as numbers.
pub fn encode_number(n: f64) -> [u8; 8] {
    let mut bytes = n.to_be_bytes();
    if n < 0.0 || (n == 0.0 && n.is_sign_negative() && false) {
        for b in &mut bytes {
            *b = !*b;
        }
    } else {
        bytes[0] |= 0x80;
    }
    bytes
}

/// The number an index key stands for: the inverse of `encode_number`.
pub fn decode_number(key: &[u8]) -> f64 {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&key[..8]);
    if bytes[0] & 0x80 != 0 {
        bytes[0] &= 0x7f;
    } else {
        for b in &mut bytes {
            *b = !*b;
        }
    }
    f64::from_be_bytes(bytes)
}

/// The Julian day number of 1970-01-01, which is how a date becomes a number in an index.
const JULIAN_1970: f64 = 2_440_588.0;

/// A date as an index key: its Julian day number, encoded as a number.
pub fn encode_date(days_since_1970: i32) -> [u8; 8] {
    encode_number(f64::from(days_since_1970) + JULIAN_1970)
}

/// A datetime as an index key: the Julian day with the time of day as its fraction.
pub fn encode_datetime(seconds_since_1970: f64) -> [u8; 8] {
    encode_number(seconds_since_1970 / 86_400.0 + JULIAN_1970)
}

/// A character key: the text, one byte per character, padded with spaces to the key length.
pub fn encode_text(text: &str, key_len: usize) -> Vec<u8> {
    let mut key: Vec<u8> = text.chars().map(|c| (c as u32 & 0xff) as u8).take(key_len).collect();
    key.resize(key_len, b' ');
    key
}

// ---------------------------------------------------------------- writing

/// Bits needed to hold `n`.
fn bits_for(n: u64) -> u32 {
    if n == 0 { 1 } else { 64 - n.leading_zeros() }
}

/// Packs entries into compact leaves, each holding as many as fit, and links them left to right.
/// Returns the pages written, in order.
fn build_leaves(entries: &[Entry], key_len: usize, pad: u8, pages: &mut Vec<Vec<u8>>, first_page: u32) -> Vec<(Vec<u8>, u32, u32)> {
    // an entry takes whole bytes, and the record number is given every bit the counts of shared
    // and trailing bytes leave over in them, which is what Visual FoxPro's own writer does: a
    // table of five records in a ten-byte key still gets eight bits of record number
    let max_recno = entries.iter().map(|e| e.recno as u64).max().unwrap_or(1);
    let dup_bits = bits_for(key_len as u64);
    let trail_bits = bits_for(key_len as u64);
    let holding = (bits_for(max_recno).max(1) + dup_bits + trail_bits).div_ceil(8) as usize;
    let recno_bits = (holding as u32 * 8 - dup_bits - trail_bits).min(32);

    let mut leaves: Vec<(Vec<u8>, u32, u32)> = Vec::new(); // (last key, last recno, page number)
    let mut i = 0;
    while i < entries.len() || leaves.is_empty() {
        let page_no = first_page + (pages.len() as u32) * PAGE as u32;
        let mut p = vec![0u8; PAGE];
        let mut used_head = 24usize;
        let mut used_tail = 0usize;
        let mut count = 0usize;
        let mut previous: Vec<u8> = Vec::new();
        let start = i;
        while i < entries.len() {
            let key = &entries[i].key;
            let dup = previous.iter().zip(key).take_while(|(a, b)| a == b).count().min(key_len);
            let trail = key.iter().rev().take_while(|&&b| b == pad).count().min(key_len - dup);
            let stored = key_len - dup - trail;
            if used_head + holding + used_tail + stored > PAGE {
                break;
            }
            let packed: u64 = entries[i].recno as u64 | ((dup as u64) << recno_bits) | ((trail as u64) << (recno_bits + dup_bits));
            for k in 0..holding {
                p[used_head + k] = ((packed >> (8 * k)) & 0xff) as u8;
            }
            used_head += holding;
            used_tail += stored;
            let tail_at = PAGE - used_tail;
            p[tail_at..tail_at + stored].copy_from_slice(&key[dup..dup + stored]);
            previous = key.clone();
            count += 1;
            i += 1;
        }
        if count == 0 && i < entries.len() {
            // a key too long for a page: a defect in the caller, not something to loop on
            break;
        }
        p[0..2].copy_from_slice(&ATTR_LEAF.to_le_bytes());
        p[2..4].copy_from_slice(&(count as u16).to_le_bytes());
        p[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        p[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        p[12..14].copy_from_slice(&((PAGE - used_head - used_tail) as u16).to_le_bytes());
        let recno_mask = if recno_bits >= 32 { u32::MAX } else { (1u32 << recno_bits) - 1 };
        p[14..18].copy_from_slice(&recno_mask.to_le_bytes());
        p[18] = ((1u32 << dup_bits) - 1) as u8;
        p[19] = ((1u32 << trail_bits) - 1) as u8;
        p[20] = recno_bits as u8;
        p[21] = dup_bits as u8;
        p[22] = trail_bits as u8;
        p[23] = holding as u8;
        let last = entries.get(i.saturating_sub(1)).filter(|_| count > 0);
        leaves.push((last.map(|e| e.key.clone()).unwrap_or_else(|| vec![pad; key_len]), last.map_or(0, |e| e.recno), page_no));
        pages.push(p);
        if i == start && entries.is_empty() {
            break;
        }
    }
    // link the leaves to their neighbours
    let n = leaves.len();
    for k in 0..n {
        let page_index = (leaves[k].2 - first_page) as usize / PAGE;
        let p = &mut pages[page_index];
        let left = if k > 0 { leaves[k - 1].2 } else { u32::MAX };
        let right = if k + 1 < n { leaves[k + 1].2 } else { u32::MAX };
        p[4..8].copy_from_slice(&left.to_le_bytes());
        p[8..12].copy_from_slice(&right.to_le_bytes());
    }
    leaves
}

/// Builds interior pages over `children` (last key, last recno, page) until one root is left.
/// Returns the root page number.
fn build_interior(mut children: Vec<(Vec<u8>, u32, u32)>, key_len: usize, pages: &mut Vec<Vec<u8>>, first_page: u32) -> u32 {
    if children.len() == 1 {
        let page_index = (children[0].2 - first_page) as usize / PAGE;
        let p = &mut pages[page_index];
        let attr = u16_at(p, 0) | ATTR_ROOT;
        p[0..2].copy_from_slice(&attr.to_le_bytes());
        return children[0].2;
    }
    let per_page = (PAGE - 12) / (key_len + 8);
    let mut parents: Vec<(Vec<u8>, u32, u32)> = Vec::new();
    while !children.is_empty() {
        let take = children.len().min(per_page);
        let group: Vec<(Vec<u8>, u32, u32)> = children.drain(..take).collect();
        let page_no = first_page + (pages.len() as u32) * PAGE as u32;
        let mut p = vec![0u8; PAGE];
        p[0..2].copy_from_slice(&0u16.to_le_bytes());
        p[2..4].copy_from_slice(&(group.len() as u16).to_le_bytes());
        p[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        p[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        for (k, (key, recno, child)) in group.iter().enumerate() {
            let at = 12 + k * (key_len + 8);
            p[at..at + key_len].copy_from_slice(&key[..key_len]);
            p[at + key_len..at + key_len + 4].copy_from_slice(&recno.to_be_bytes());
            p[at + key_len + 4..at + key_len + 8].copy_from_slice(&child.to_be_bytes());
        }
        let last = group.last().unwrap();
        parents.push((last.0.clone(), last.1, page_no));
        pages.push(p);
    }
    // link this level's pages to their neighbours
    for k in 0..parents.len() {
        let page_index = (parents[k].2 - first_page) as usize / PAGE;
        let left = if k > 0 { parents[k - 1].2 } else { u32::MAX };
        let right = if k + 1 < parents.len() { parents[k + 1].2 } else { u32::MAX };
        let p = &mut pages[page_index];
        p[4..8].copy_from_slice(&left.to_le_bytes());
        p[8..12].copy_from_slice(&right.to_le_bytes());
    }
    build_interior(parents, key_len, pages, first_page)
}

/// Writes one tag - its two header pages, then its tree - and returns the page its header is on.
pub(crate) fn write_tag(out: &mut Vec<u8>, base: u32, key_len: usize, key_expr: &str, for_expr: &str, options: u8, descending: bool, entries: &[Entry], pad: u8) -> u32 {
    // pages are numbered from the start of the file, and `out` may be a buffer that will be
    // placed at `base`
    let header_page = base + out.len() as u32;
    // the header takes two pages: the numbers, then the expressions
    let mut header = vec![0u8; 2 * PAGE];
    let tree_start = header_page + 2 * PAGE as u32;
    let mut pages: Vec<Vec<u8>> = Vec::new();
    let leaves = build_leaves(entries, key_len, pad, &mut pages, tree_start);
    let root = build_interior(leaves, key_len, &mut pages, tree_start);

    header[0..4].copy_from_slice(&root.to_le_bytes());
    header[12..14].copy_from_slice(&(key_len as u16).to_le_bytes());
    header[14] = options;
    header[15] = 1;
    // how many keys the tag holds and how many of them differ: what Rushmore is told about a
    // tag before it opens it
    let distinct = entries.windows(2).filter(|w| w[0].key != w[1].key).count() + usize::from(!entries.is_empty());
    header[16..20].copy_from_slice(&(entries.len() as u32).to_le_bytes());
    header[20..24].copy_from_slice(&(distinct as u32).to_le_bytes());
    header[502..504].copy_from_slice(&(u16::from(descending)).to_le_bytes());
    let key_bytes: Vec<u8> = key_expr.bytes().collect();
    let for_bytes: Vec<u8> = for_expr.bytes().collect();
    let key_room = (key_bytes.len() + 1) as u16;
    header[504..506].copy_from_slice(&key_room.to_le_bytes());
    header[FOR_LEN_AT..FOR_LEN_AT + 2].copy_from_slice(&(for_bytes.len() as u16 + 1).to_le_bytes());
    header[KEY_LEN_AT..KEY_LEN_AT + 2].copy_from_slice(&key_room.to_le_bytes());
    header[PAGE..PAGE + key_bytes.len()].copy_from_slice(&key_bytes);
    if !for_bytes.is_empty() {
        let at = PAGE + key_bytes.len() + 1;
        header[at..at + for_bytes.len()].copy_from_slice(&for_bytes);
    }
    out.extend_from_slice(&header);
    for p in pages {
        out.extend_from_slice(&p);
    }
    header_page
}

/// Writes a compound index holding `tags`, as Visual FoxPro would read it: the tag directory
/// first, then each tag. A structural index is one named after its table, which VFP opens with
/// it; the flag is set because that is the only kind this runtime makes.
pub fn write_cdx(tags: &[Tag]) -> Vec<u8> {
    // the directory comes first, but its entries point at pages written after it, so how long
    // it is has to be known before the tags are laid out. The guess is made again with what
    // the last one produced until it is big enough, which takes a second pass only when the
    // pages the tags landed on needed wider entries than the first guess allowed for.
    let mut room = 3 * PAGE as u32;
    loop {
        let (body, names) = write_tag_bodies(tags, room);
        let mut out: Vec<u8> = Vec::new();
        // the directory itself: a structural, compound, compact index of the tag names
        write_tag(&mut out, 0, 10, "", "", 0xE0, false, &names, b' ');
        if out.len() as u32 <= room {
            out.resize(room as usize, 0);
            out.extend_from_slice(&body);
            return out;
        }
        room = out.len() as u32;
    }
}

/// Every tag written one after another, starting `room` bytes into the file, and the directory
/// entries that name them: the tag name, and the page its header landed on.
fn write_tag_bodies(tags: &[Tag], room: u32) -> (Vec<u8>, Vec<Entry>) {
    let mut body: Vec<u8> = Vec::new();
    let mut names: Vec<Entry> = Vec::new();
    for tag in tags {
        // 0x40 says the tag lives in a compound index, which is what tells it from the same
        // header at the front of a single-entry .idx
        let options = 0x60
            | (if tag.unique { 0x01 } else { 0 })
            | (if tag.candidate { 0x04 } else { 0 })
            | (if tag.for_expr.is_empty() { 0 } else { 0x08 });
        let pad = if tag.numeric { 0 } else { b' ' };
        let at = write_tag(&mut body, room, tag.key_len, &tag.key_expr, &tag.for_expr, options, tag.descending, &tag.entries, pad);
        names.push(Entry { key: encode_text(&tag.name.to_ascii_uppercase(), 10), recno: at });
    }
    names.sort_by(|a, b| a.key.cmp(&b.key));
    (body, names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_sort_as_their_keys() {
        let values = [-100.5, -1.0, -0.0, 0.0, 0.5, 1.0, 2.0, 1e9];
        let keys: Vec<[u8; 8]> = values.iter().map(|v| encode_number(*v)).collect();
        for w in keys.windows(2) {
            assert!(w[0] <= w[1], "{:?} <= {:?}", w[0], w[1]);
        }
        for v in values {
            assert_eq!(decode_number(&encode_number(v)), v);
        }
    }

    #[test]
    fn a_written_index_reads_back() {
        let mut entries: Vec<Entry> = (1..=300u32).map(|n| Entry { key: encode_text(&format!("KEY{n:05}"), 10), recno: n }).collect();
        entries.sort_by(|a, b| a.key.cmp(&b.key));
        let tags = vec![
            Tag { name: "BYKEY".into(), key_expr: "UPPER(key)".into(), for_expr: String::new(), key_len: 10, descending: false, unique: false, candidate: false, numeric: false, entries: entries.clone() },
            Tag { name: "BYNUM".into(), key_expr: "amount".into(), for_expr: "!DELETED()".into(), key_len: 8, descending: true, unique: true, candidate: false, numeric: true, entries: (1..=5u32).map(|n| Entry { key: encode_number(f64::from(n)).to_vec(), recno: n }).collect() },
        ];
        let bytes = write_cdx(&tags);
        assert_eq!(bytes.len() % PAGE, 0);
        let back = read_cdx(&bytes).expect("reads");
        assert_eq!(back.len(), 2);
        let bykey = back.iter().find(|t| t.name == "BYKEY").expect("BYKEY");
        assert_eq!(bykey.key_expr, "UPPER(key)");
        assert_eq!(bykey.entries, entries);
        let bynum = back.iter().find(|t| t.name == "BYNUM").expect("BYNUM");
        assert_eq!(bynum.for_expr, "!DELETED()");
        assert!(bynum.descending && bynum.unique);
        assert_eq!(bynum.entries.len(), 5);
        assert_eq!(decode_number(&bynum.entries[4].key), 5.0);
    }
}
