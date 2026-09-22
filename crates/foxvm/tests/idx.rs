//! `.idx` files, against ones Visual FoxPro 9 wrote itself.
//!
//! The fixtures were produced by running this in Visual FoxPro 9:
//!
//! ```foxpro
//! CREATE TABLE people (name C(10), age N(3,0))
//! INSERT INTO people (name, age) VALUES ("delta", 40)
//! INSERT INTO people (name, age) VALUES ("alpha", 10)
//! INSERT INTO people (name, age) VALUES ("charlie", 30)
//! INSERT INTO people (name, age) VALUES ("bravo", 20)
//! INSERT INTO people (name, age) VALUES ("echo", 50)
//! USE people EXCLUSIVE
//! INDEX ON name TO idxstd
//! INDEX ON name TO idxcmp COMPACT
//! INDEX ON age TO idxnum COMPACT
//! INDEX ON UPPER(name) TAG upname FOR age>20
//! COPY TAG upname TO idxfor
//! USE people EXCLUSIVE INDEX idxcmp, idxnum
//! COPY INDEXES idxcmp, idxnum TO idxtags.cdx
//! ```
//!
//! What is compared is everything the format puts a meaning on. Three things are left out of
//! the comparison, and each is scrubbed from both sides so that what is left has to match byte
//! for byte:
//!
//! - the three counters at 24, 28 and 32 of a compact header. They are statistics Visual
//!   FoxPro's sort leaves behind - the same key set arriving in a different record order gives
//!   different numbers - and nothing reads them back;
//! - the attributes word of a compact node, where Visual FoxPro sets a bit above the root and
//!   leaf ones. What it distinguishes is not documented, and its own writer sets it on some
//!   nodes of a tree and not others;
//! - the free space of a node, between the end of its entries and the keys packed against the
//!   end of the page. Visual FoxPro leaves scratch there - the tail of the same entry list as
//!   it stood before the node was packed down, and in a standard index whatever the buffer
//!   held last.

use foxvm::cdx::{Entry, PAGE, Tag, encode_number, encode_text, read_cdx};
use foxvm::idx;

const STANDARD: &[u8] = include_bytes!("fixtures/vfp9-standard.idx");
const COMPACT: &[u8] = include_bytes!("fixtures/vfp9-compact.idx");
const NUMBER: &[u8] = include_bytes!("fixtures/vfp9-number.idx");
const FOR: &[u8] = include_bytes!("fixtures/vfp9-for.idx");
const COPIED: &[u8] = include_bytes!("fixtures/vfp9-copied-tags.cdx");

/// The people table the fixtures were built from: the name, the age and the record it is in.
const PEOPLE: [(&str, f64, u32); 5] =
    [("delta", 40.0, 1), ("alpha", 10.0, 2), ("charlie", 30.0, 3), ("bravo", 20.0, 4), ("echo", 50.0, 5)];

fn sorted(mut entries: Vec<Entry>) -> Vec<Entry> {
    entries.sort_by(|a, b| (&a.key, a.recno).cmp(&(&b.key, b.recno)));
    entries
}

fn by_name() -> Tag {
    Tag {
        name: String::new(),
        key_expr: "name".into(),
        for_expr: String::new(),
        key_len: 10,
        descending: false,
        unique: false,
        candidate: false,
        numeric: false,
        entries: sorted(PEOPLE.iter().map(|(n, _, r)| Entry { key: encode_text(n, 10), recno: *r }).collect()),
    }
}

fn by_age() -> Tag {
    Tag {
        key_expr: "age".into(),
        key_len: 8,
        numeric: true,
        entries: sorted(PEOPLE.iter().map(|(_, a, r)| Entry { key: encode_number(*a).to_vec(), recno: *r }).collect()),
        ..by_name()
    }
}

fn by_upper_name_over_twenty() -> Tag {
    Tag {
        key_expr: "UPPER(name)".into(),
        for_expr: "age>20".into(),
        entries: sorted(
            PEOPLE
                .iter()
                .filter(|(_, a, _)| *a > 20.0)
                .map(|(n, _, r)| Entry { key: encode_text(&n.to_uppercase(), 10), recno: *r })
                .collect(),
        ),
        ..by_name()
    }
}

/// Blanks what the format leaves undefined, so that what is left is only what it means.
fn defined_only(bytes: &[u8]) -> Vec<u8> {
    let mut out = bytes.to_vec();
    let compact = idx::is_compact(&out);
    let key_len = u16::from_le_bytes([out[12], out[13]]) as usize;
    // the header's own scratch, and the page the expressions sit on, which is defined in full
    let first_node = if compact {
        out[24..36].fill(0);
        2 * PAGE
    } else {
        PAGE
    };
    let mut at = first_node;
    while at + PAGE <= out.len() {
        let node = &mut out[at..at + PAGE];
        let leaf = u16::from_le_bytes([node[0], node[1]]) & 2 != 0;
        let count = u16::from_le_bytes([node[2], node[3]]) as usize;
        let (used, keys) = if !compact {
            (12 + count * (key_len + 4), 0)
        } else if leaf {
            let free = u16::from_le_bytes([node[12], node[13]]) as usize;
            (24 + count * node[23] as usize, PAGE - 24 - count * node[23] as usize - free)
        } else {
            (12 + count * (key_len + 8), 0)
        };
        if compact {
            node[0..2].fill(0);
        }
        node[used..PAGE - keys].fill(0);
        at += PAGE;
    }
    out
}

fn same_bytes(mine: &[u8], theirs: &[u8], what: &str) {
    assert_eq!(mine.len(), theirs.len(), "{what}: the file is as long as Visual FoxPro's");
    let (mine, theirs) = (defined_only(mine), defined_only(theirs));
    for (page, (a, b)) in mine.chunks(PAGE).zip(theirs.chunks(PAGE)).enumerate() {
        assert_eq!(a, b, "{what}: page {page} is what Visual FoxPro wrote");
    }
}

#[test]
fn writes_the_standard_index_visual_foxpro_wrote() {
    same_bytes(&idx::write(&by_name(), false), STANDARD, "a standard index over a character key");
}

#[test]
fn writes_the_compact_indexes_visual_foxpro_wrote() {
    same_bytes(&idx::write(&by_name(), true), COMPACT, "a compact index over a character key");
    same_bytes(&idx::write(&by_age(), true), NUMBER, "a compact index over a number");
    same_bytes(&idx::write(&by_upper_name_over_twenty(), true), FOR, "a compact index with a FOR condition");
}

#[test]
fn reads_what_visual_foxpro_wrote() {
    for (bytes, what) in [(STANDARD, "standard"), (COMPACT, "compact")] {
        let tag = idx::read(bytes).unwrap_or_else(|e| panic!("{what} reads: {e}"));
        assert_eq!(tag.key_expr, "name", "{what}");
        assert_eq!(tag.key_len, 10, "{what}");
        assert!(!tag.numeric && !tag.unique, "{what}");
        assert_eq!(tag.entries, by_name().entries, "{what}");
    }
    let ages = idx::read(NUMBER).expect("the number index reads");
    assert!(ages.numeric);
    assert_eq!(ages.entries, by_age().entries);
    let over = idx::read(FOR).expect("the FOR index reads");
    assert_eq!(over.key_expr, "UPPER(name)");
    assert_eq!(over.for_expr, "age>20");
    assert_eq!(over.entries, by_upper_name_over_twenty().entries);
}

#[test]
fn the_standard_and_the_compact_kind_are_told_apart() {
    assert!(!idx::is_compact(STANDARD));
    assert!(idx::is_compact(COMPACT));
    assert!(idx::is_compact(&idx::write(&by_name(), true)));
    assert!(!idx::is_compact(&idx::write(&by_name(), false)));
}

/// The program the fixtures were made from, run against the runtime instead of Visual FoxPro.
fn built_here() -> foxvm::mock_host::MockHost {
    let mut host = foxvm::mock_host::MockHost::new();
    let src = r#"
CREATE TABLE people (name C(10), age N(3,0))
INSERT INTO people (name, age) VALUES ("delta", 40)
INSERT INTO people (name, age) VALUES ("alpha", 10)
INSERT INTO people (name, age) VALUES ("charlie", 30)
INSERT INTO people (name, age) VALUES ("bravo", 20)
INSERT INTO people (name, age) VALUES ("echo", 50)
INDEX ON name TO idxstd
INDEX ON name TO idxcmp COMPACT
INDEX ON age TO idxnum COMPACT
INDEX ON UPPER(name) TAG upname FOR age>20
COPY TAG upname TO idxfor
"#;
    if let Err(e) = foxvm::mock_host::run_program(src, &mut host) {
        panic!("{} at line {}: {}", e.code, e.line, e.message);
    }
    host
}

#[test]
fn the_commands_write_the_files_visual_foxpro_writes() {
    let host = built_here();
    for (name, theirs, what) in [
        ("IDXSTD.IDX", STANDARD, "INDEX ON ... TO"),
        ("IDXCMP.IDX", COMPACT, "INDEX ON ... TO ... COMPACT"),
        ("IDXNUM.IDX", NUMBER, "INDEX ON a number TO ... COMPACT"),
        ("IDXFOR.IDX", FOR, "COPY TAG"),
    ] {
        let mine = host.files.get(name).unwrap_or_else(|| panic!("{what} wrote {name}"));
        same_bytes(mine, theirs, what);
    }
}

#[test]
fn the_tags_copy_indexes_makes_are_the_indexes_it_was_given() {
    // the compound index Visual FoxPro built out of idxcmp.idx and idxnum.idx: a tag per file,
    // named after the file, holding what that file held
    let tags = read_cdx(COPIED).expect("the copied index reads");
    let mut names: Vec<&str> = tags.iter().map(|t| t.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, ["IDXCMP", "IDXNUM"]);
    let named = |name: &str| tags.iter().find(|t| t.name == name).unwrap_or_else(|| panic!("{name} is there"));
    assert_eq!(named("IDXCMP").key_expr, "name");
    assert_eq!(named("IDXCMP").entries, by_name().entries);
    assert_eq!(named("IDXNUM").key_expr, "age");
    assert_eq!(named("IDXNUM").entries, by_age().entries);
}
