//! Compound indexes, read from the files Visual FoxPro ships and written back the same way.
//!
//! The sample tables come with the indexes VFP built for them, and those are the ground truth
//! for a reader: a tag's keys must come out in order, one per record, with the expression the
//! index was built from. The tests skip themselves where Visual FoxPro is not installed.

use std::path::Path;

use foxvm::cdx::{decode_number, read_cdx, write_cdx};
use foxvm::dbf::read_table;

const DATA: &str = "C:/Program Files (x86)/Microsoft Visual FoxPro 9/Samples/Data";

fn installed() -> bool {
    Path::new(DATA).join("customer.cdx").exists()
}

#[test]
fn reads_every_tag_of_the_customer_index() {
    if !installed() {
        return;
    }
    let bytes = std::fs::read(Path::new(DATA).join("customer.cdx")).unwrap();
    let tags = read_cdx(&bytes).expect("customer.cdx reads");
    let names: Vec<&str> = tags.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"CUST_ID"), "{names:?}");
    assert!(names.contains(&"COMPANY"), "{names:?}");

    // one key per record, and the keys in the order the index sorts them
    let table = read_table(&std::fs::read(Path::new(DATA).join("customer.dbf")).unwrap(), None).unwrap();
    for tag in &tags {
        assert_eq!(tag.entries.len(), table.records.len(), "{} has a key per record", tag.name);
        assert!(!tag.key_expr.is_empty(), "{} has an expression", tag.name);
        for w in tag.entries.windows(2) {
            assert!(w[0].key <= w[1].key, "{}: keys are in order", tag.name);
        }
        for e in &tag.entries {
            assert!(e.recno >= 1 && e.recno as usize <= table.records.len(), "{}: record {} exists", tag.name, e.recno);
        }
    }

    // the CUST_ID tag is the customer id, so its keys are the field's values sorted
    let cust = tags.iter().find(|t| t.name == "CUST_ID").unwrap();
    let id_col = table.fields.iter().position(|f| f.name.eq_ignore_ascii_case("cust_id")).unwrap();
    let mut ids: Vec<String> = table
        .records
        .iter()
        .map(|r| match &r.values[id_col] {
            foxvm::dbf::DbfValue::Text(s) => s.trim_end().to_string(),
            other => panic!("{other:?}"),
        })
        .collect();
    ids.sort();
    let keys: Vec<String> = cust.entries.iter().map(|e| String::from_utf8_lossy(&e.key).trim_end().to_string()).collect();
    assert_eq!(keys, ids);
}

#[test]
fn reads_a_numeric_key_as_the_number_it_was() {
    if !installed() {
        return;
    }
    let bytes = std::fs::read(Path::new(DATA).join("orders.cdx")).unwrap();
    let tags = read_cdx(&bytes).expect("orders.cdx reads");
    // every tag with 8-byte keys and a numeric-looking expression decodes to sorted numbers
    for tag in tags.iter().filter(|t| t.key_len == 8) {
        let numbers: Vec<f64> = tag.entries.iter().map(|e| decode_number(&e.key)).collect();
        for w in numbers.windows(2) {
            assert!(w[0] <= w[1], "{}: {} <= {}", tag.name, w[0], w[1]);
        }
    }
}

#[test]
fn what_we_write_is_what_we_read_for_a_real_index() {
    if !installed() {
        return;
    }
    let bytes = std::fs::read(Path::new(DATA).join("customer.cdx")).unwrap();
    let tags = read_cdx(&bytes).unwrap();
    let ours = write_cdx(&tags);
    let back = read_cdx(&ours).expect("our own index reads");
    assert_eq!(back.len(), tags.len());
    for (a, b) in tags.iter().zip(&back) {
        assert_eq!(a.name, b.name);
        assert_eq!(a.key_expr, b.key_expr);
        assert_eq!(a.entries, b.entries, "{}", a.name);
    }
}

#[test]
fn tells_a_number_key_from_a_text_one() {
    let log = Path::new("C:/Program Files (x86)/Microsoft Visual FoxPro 9/Samples/Solution/Coverage/Demos/SavedCovLog.cdx");
    if !log.exists() {
        return;
    }
    let tags = read_cdx(&std::fs::read(log).unwrap()).expect("SavedCovLog.cdx reads");
    // the coverage log's AVG and TOTAL are durations, its OBJCLASS and SOURCE are text
    for name in ["AVG", "TOTAL"] {
        let tag = tags.iter().find(|t| t.name == name).unwrap_or_else(|| panic!("{name} is a tag"));
        assert!(tag.numeric, "{name} holds numbers");
        let numbers: Vec<f64> = tag.entries.iter().map(|e| decode_number(&e.key)).collect();
        assert!(numbers.iter().all(|n| n.is_finite() && *n >= 0.0), "{name}: {:?}", &numbers[..numbers.len().min(4)]);
        for w in numbers.windows(2) {
            assert!(w[0] <= w[1], "{name}: {} <= {}", w[0], w[1]);
        }
    }
    for name in ["OBJCLASS", "SOURCE"] {
        let tag = tags.iter().find(|t| t.name == name).unwrap_or_else(|| panic!("{name} is a tag"));
        assert!(!tag.numeric, "{name} holds text");
    }
    // and what we write of it comes back the same
    let back = read_cdx(&write_cdx(&tags)).expect("our own copy reads");
    for (a, b) in tags.iter().zip(&back) {
        assert_eq!(a.numeric, b.numeric, "{}", a.name);
        assert_eq!(a.entries, b.entries, "{}", a.name);
    }
}
