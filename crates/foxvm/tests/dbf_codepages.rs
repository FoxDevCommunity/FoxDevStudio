//! Synthetic international text checks; no application fixtures.
use foxvm::dbf::{
    DbfField,
    encoding::{codepage_for_language_id, decode, encode},
    read_table,
    write::encode_field,
};
use foxvm::value::Value;

fn cases() -> Vec<(u16, u8, Vec<char>)> {
    include_str!("codepage_reference.txt")
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| {
            let mut fields = l.split_whitespace();
            let cp = fields.next().unwrap().parse().unwrap();
            let marker = u8::from_str_radix(fields.next().unwrap(), 16).unwrap();
            let expected = fields.map(|s| char::from_u32(u32::from_str_radix(s, 16).unwrap()).unwrap()).collect();
            (cp, marker, expected)
        })
        .collect()
}

#[test]
fn reference_mappings_decode_and_encode_every_defined_byte() {
    for (cp, marker, expected) in cases() {
        assert_eq!(codepage_for_language_id(marker), Some(cp));
        assert_eq!(expected.len(), 128);
        for byte in 0u8..=255 {
            let want = if byte < 128 { byte as char } else { expected[(byte - 128) as usize] };
            assert_eq!(decode(&[byte], Some(cp)), want.to_string(), "CP{cp} byte {byte:02X}");
            if want != '\u{fffd}' {
                assert_eq!(encode(&want.to_string(), Some(cp)), vec![byte], "CP{cp} byte {byte:02X}");
            }
        }
        assert_eq!(encode("😀", Some(cp)), b"?");
    }
}

// A character field followed by a text/binary memo, using a real DBF/FPT layout.
fn fixture(marker: u8, raw: &[u8], binary: bool) -> (Vec<u8>, Vec<u8>) {
    let width = raw.len();
    let mut dbf = vec![0u8; 97 + 1 + width + 4];
    dbf[0] = 0x30;
    dbf[4..8].copy_from_slice(&1u32.to_le_bytes());
    dbf[8..10].copy_from_slice(&97u16.to_le_bytes());
    dbf[10..12].copy_from_slice(&((1 + width + 4) as u16).to_le_bytes());
    dbf[29] = marker;
    dbf[32..37].copy_from_slice(b"LABEL");
    dbf[43] = b'C';
    dbf[48] = width as u8;
    dbf[64..68].copy_from_slice(b"NOTE");
    dbf[75] = b'M';
    dbf[80] = 4;
    dbf[96] = 0x0d;
    dbf[97] = b' ';
    dbf[98..98 + width].copy_from_slice(raw);
    dbf[98 + width..].copy_from_slice(&1u32.to_le_bytes());
    let mut memo = vec![0u8; 1024];
    memo[0..4].copy_from_slice(&2u32.to_be_bytes());
    memo[6..8].copy_from_slice(&512u16.to_be_bytes());
    memo[512..516].copy_from_slice(&(if binary { 0u32 } else { 1u32 }).to_be_bytes());
    memo[516..520].copy_from_slice(&(width as u32).to_be_bytes());
    memo[520..520 + width].copy_from_slice(raw);
    (dbf, memo)
}

#[test]
fn character_write_reopen_and_memo_read_use_each_tables_page() {
    for (cp, marker, expected) in cases() {
        let pairs: Vec<_> = expected.iter().enumerate().filter(|(_, c)| **c != '\u{fffd}').collect();
        let raw: Vec<_> = pairs.iter().map(|(i, _)| *i as u8 + 128).collect();
        let text: String = pairs.iter().map(|(_, c)| **c).collect();
        let field = DbfField::new("LABEL", 'C', raw.len() as u8, 0);
        let written = encode_field(&field, raw.len(), &Value::str(&text), Some(cp)).unwrap();
        assert_eq!(written, raw, "CP{cp} write bytes");
        let (dbf, memo) = fixture(marker, &written, false);
        let table = read_table(&dbf, Some(&memo)).unwrap();
        for name in ["LABEL", "NOTE"] {
            assert_eq!(table.records[0].get(&table, name).unwrap().as_text(), text, "CP{cp} {name}");
        }
        let short = encode_field(&field, 2, &Value::str(&text), Some(cp)).unwrap();
        assert_eq!(short, &raw[..2]);
        let padded = encode_field(&field, 3, &Value::str(text.chars().next().unwrap().to_string()), Some(cp)).unwrap();
        assert_eq!(padded, vec![raw[0], b' ', b' ']);
        let (dbf, memo) = fixture(marker, &raw, true);
        let table = read_table(&dbf, Some(&memo)).unwrap();
        assert!(matches!(table.records[0].get(&table,"NOTE").unwrap(),foxvm::dbf::DbfValue::Bytes(b) if b==&raw));
    }
}

#[test]
fn unsupported_and_unmarked_pages_keep_the_existing_fallback() {
    for cp in [None, Some(932), Some(936), Some(949), Some(950), Some(10000), Some(9999)] {
        assert_eq!(decode(&[0xe9], cp), "é");
        assert_eq!(encode("é", cp), [0xe9]);
    }
    assert_eq!(codepage_for_language_id(0), None);
}

#[test]
fn language_text_survives_vm_character_and_memo_write_reopen() {
    use foxvm::mock_host::{MockHost, run_program};
    let samples = [
        (437, "Español"),
        (737, "Ελληνικά"),
        (850, "Português"),
        (852, "Čeština"),
        (857, "Türkçe"),
        (860, "Português"),
        (861, "Ísland"),
        (863, "Québec"),
        (865, "Norsk ø"),
        (866, "Привет"),
        (874, "ไทย"),
        (1250, "Zażółć"),
        (1251, "Привет"),
        (1252, "Español"),
        (1253, "Ελληνικά"),
        (1254, "Türkçe"),
        (1255, "שלום"),
        (1256, "مرحبا"),
        (1257, "Ąžuolas"),
    ];
    for (cp, sample) in samples {
        let marker = cases().into_iter().find(|c| c.0 == cp).unwrap().1;
        let (dbf, memo) = fixture(marker, &[b' '; 40], false);
        let mut host = MockHost::new();
        host.tables.insert("TEXT.DBF".into(), dbf);
        host.memo_files.insert("TEXT.DBF".into(), memo);
        let src = format!(
            "USE text.dbf EXCLUSIVE\nREPLACE label WITH \"{sample}\", note WITH \"{sample}\"\nUSE\nUSE text.dbf\n? label\n? note\nUSE"
        );
        let (_, output) = run_program(&src, &mut host).unwrap();
        assert_eq!(
            output,
            vec![format!("{sample}{}", " ".repeat(40 - sample.chars().count())), sample.to_string()],
            "CP{cp}"
        );
        let table = read_table(&host.tables["TEXT.DBF"], Some(&host.memo_files["TEXT.DBF"])).unwrap();
        assert_eq!(table.codepage, Some(cp));
        assert_eq!(table.records[0].get(&table, "NOTE").unwrap().as_text(), sample);
    }
}

#[test]
fn cpconvert_uses_the_requested_source_and_target_pages() {
    use foxvm::mock_host::{MockHost, run_program};
    let (_, out) = run_program(
        "? CPCONVERT(1251,866,CHR(192)+CHR(193))\n? CPCONVERT(737,1253,CHR(128))\n? CPCONVERT(1252,850,CHR(241))",
        &mut MockHost::new(),
    )
    .unwrap();
    assert_eq!(out, vec!["\u{80}\u{81}", "\u{c1}", "\u{a4}"]);
}

#[test]
fn mac_greek_marker_is_distinct_from_mac_roman() {
    assert_eq!(codepage_for_language_id(0x98), Some(10006));
    assert_eq!(codepage_for_language_id(0x04), Some(10000));
}
