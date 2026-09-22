//! `FoxScript.Json` and the data bridge.
//!
//! Visual FoxPro has no JSON, so nothing here can be measured. Every rule is written down in
//! docs/foxscript.md, "FoxScript.Json and the data bridge, settled", before it was written here,
//! and these tests are that specification.

use foxvm::mock_host::{MockHost, run_program};

fn run(src: &str) -> Vec<String> {
    match run_program(src, &mut MockHost::new()) {
        Ok((_, output)) => output,
        Err(e) => panic!("{} at line {}: {}", e.code, e.line, e.message),
    }
}

fn fails(src: &str) -> String {
    match run_program(src, &mut MockHost::new()) {
        Ok((_, out)) => panic!("expected an error, got {out:?}"),
        Err(e) => format!("{}: {}", e.code, e.message),
    }
}

// ----- the value -----------------------------------------------------------------------------

#[test]
fn vartype_of_a_json_value_is_j() {
    // FoxScript's own letter, chosen the way F was: measured, the letters vfp9.exe can return
    // are C N Y D T L O G Q X U, so J is free
    let out = run(
        "LOCAL o\n\
         o = FoxScript.Json.Parse('{\"a\": 1}')\n\
         ? VARTYPE(o), TYPE(\"o\")\n",
    );
    assert_eq!(out, vec!["J J"]);
}

#[test]
fn a_member_comes_out_as_the_foxpro_value_json_stands_for() {
    let out = run(
        "LOCAL o\n\
         o = FoxScript.Json.Parse('{\"name\": \"Ada\", \"age\": 36, \"ok\": true, \"nil\": null}')\n\
         ? o.name, VARTYPE(o.name)\n\
         ? o.age, VARTYPE(o.age)\n\
         ? o.ok, VARTYPE(o.ok)\n\
         ? VARTYPE(o.nil)\n",
    );
    assert_eq!(out, vec!["Ada C", "        36 N", ".T. L", "X"]);
}

#[test]
fn a_member_is_found_without_regard_to_case() {
    // every other member read in this language is case-insensitive; a programmer writing
    // oData.Name for a key spelled name is doing the ordinary FoxPro thing
    let out = run("LOCAL o\no = FoxScript.Json.Parse('{\"name\": \"Ada\"}')\n? o.Name, o.NAME\n");
    assert_eq!(out, vec!["Ada Ada"]);
}

#[test]
fn a_name_the_object_has_not_got_is_the_products_unknown_member() {
    assert_eq!(
        fails("LOCAL o\no = FoxScript.Json.Parse('{\"a\": 1}')\n? o.b\n"),
        "1925: Unknown member B."
    );
}

#[test]
fn get_asks_rather_than_insists() {
    let out = run(
        "LOCAL o\n\
         o = FoxScript.Json.Parse('{\"a\": 1}')\n\
         ? VARTYPE(FoxScript.Json.Get(o, \"b\")), FoxScript.Json.Get(o, \"a\")\n\
         ? FoxScript.Json.Has(o, \"a\"), FoxScript.Json.Has(o, \"b\")\n",
    );
    assert_eq!(out, vec!["X          1", ".T. .F."]);
}

#[test]
fn an_array_is_subscripted_one_based_like_every_other_subscript() {
    let out = run(
        "LOCAL o\n\
         o = FoxScript.Json.Parse('[10, 20, 30]')\n\
         ? o[1], o(2), o[3]\n\
         ? FoxScript.Json.Count(o)\n",
    );
    assert_eq!(out, vec!["        10         20         30", "         3"]);
}

#[test]
fn a_subscript_outside_the_array_is_the_products_invalid_subscript() {
    assert_eq!(fails("LOCAL o\no = FoxScript.Json.Parse('[1]')\n? o[2]\n"), "31: Invalid subscript reference.");
}

#[test]
fn reaching_goes_on_through_objects_and_arrays() {
    let out = run(
        "LOCAL o\n\
         o = FoxScript.Json.Parse('{\"items\": [{\"name\": \"one\"}, {\"name\": \"two\"}]}')\n\
         ? VARTYPE(o.items), FoxScript.Json.Count(o.items)\n\
         ? o.items[2].name\n",
    );
    assert_eq!(out, vec!["J          2", "two"]);
}

#[test]
fn keys_answers_the_names_in_document_order() {
    let out = run(
        "LOCAL o, k\n\
         o = FoxScript.Json.Parse('{\"zebra\": 1, \"apple\": 2}')\n\
         k = FoxScript.Json.Keys(o)\n\
         ? k[1], k[2]\n",
    );
    assert_eq!(out, vec!["zebra apple"]);
}

#[test]
fn two_json_values_holding_the_same_thing_are_equal() {
    // unlike a lambda or an object there is nothing to be identical to, so what is in them is
    // what is compared
    let out = run(
        "LOCAL a, b, c\n\
         a = FoxScript.Json.Parse('{\"x\": 1}')\n\
         b = FoxScript.Json.Parse('{\"x\": 1}')\n\
         c = FoxScript.Json.Parse('{\"x\": 2}')\n\
         ? a = b, a = c\n",
    );
    assert_eq!(out, vec![".T. .F."]);
}

#[test]
fn text_that_is_not_json_is_refused_with_the_parsers_own_complaint() {
    let said = fails("? FoxScript.Json.Parse('{oops}')\n");
    assert!(said.starts_with("3001: That is not JSON:"), "{said}");
}

// ----- writing it out ------------------------------------------------------------------------

#[test]
fn stringify_takes_any_foxpro_value() {
    let out = run(
        "? FoxScript.Json.Stringify(\"text\")\n\
         ? FoxScript.Json.Stringify(42)\n\
         ? FoxScript.Json.Stringify(.T.)\n\
         ? FoxScript.Json.Stringify(.NULL.)\n\
         ? FoxScript.Json.Stringify({^2024-01-31})\n",
    );
    assert_eq!(out, vec!["\"text\"", "42", "true", "null", "\"2024-01-31\""]);
}

#[test]
fn stringify_writes_an_array_as_one() {
    let out = run("LOCAL ARRAY a(3)\na(1) = 1\na(2) = \"two\"\na(3) = .F.\n? FoxScript.Json.Stringify(@a)\n");
    assert_eq!(out, vec!["[1,\"two\",false]"]);
}

#[test]
fn stringify_and_parse_are_each_others_undoing() {
    let out = run(
        "LOCAL o\n\
         o = FoxScript.Json.Parse('{\"a\":[1,2],\"b\":\"x\"}')\n\
         ? FoxScript.Json.Stringify(o)\n",
    );
    assert_eq!(out, vec!["{\"a\":[1,2],\"b\":\"x\"}"]);
}

#[test]
fn an_object_cannot_be_written_as_json() {
    // an object is the host's and the host is not here; a program that wants one written builds
    // the JSON for it
    let src = "LOCAL o\no = CREATEOBJECT(\"Empty\")\n? FoxScript.Json.Stringify(o)\n";
    assert_eq!(fails(src), "3001: An object cannot be written as JSON.");
}

// ----- a cursor, both ways -------------------------------------------------------------------

#[test]
fn a_cursor_goes_out_as_an_array_of_objects_with_lower_cased_names() {
    // the same choice Visual FoxPro itself makes when it writes DIF and SYLK, measured
    let out = run(
        "CREATE CURSOR c_cust (cust_id N(6), company C(12), active L, born D)\n\
         INSERT INTO c_cust VALUES (7, \"Ada Ltd\", .T., {^1975-03-04})\n\
         INSERT INTO c_cust VALUES (8, \"Bee Ltd\", .F., {^1980-12-25})\n\
         ? FoxScript.Data.CursorToJson(\"c_cust\")\n",
    );
    assert_eq!(
        out,
        vec![concat!(
            "[{\"cust_id\":7,\"company\":\"Ada Ltd\",\"active\":true,\"born\":\"1975-03-04\"},",
            "{\"cust_id\":8,\"company\":\"Bee Ltd\",\"active\":false,\"born\":\"1980-12-25\"}]"
        )]
    );
}

#[test]
fn an_empty_cursor_goes_out_as_an_empty_array() {
    let out = run("CREATE CURSOR c_none (a N(3))\n? FoxScript.Data.CursorToJson(\"c_none\")\n");
    assert_eq!(out, vec!["[]"]);
}

#[test]
fn a_deleted_record_is_left_out_while_set_deleted_is_on() {
    let out = run(
        "SET DELETED ON\n\
         CREATE CURSOR c_two (a N(3))\n\
         INSERT INTO c_two VALUES (1)\n\
         INSERT INTO c_two VALUES (2)\n\
         GO 1\n\
         DELETE\n\
         ? FoxScript.Data.CursorToJson(\"c_two\")\n\
         SET DELETED OFF\n\
         ? FoxScript.Data.CursorToJson(\"c_two\")\n",
    );
    assert_eq!(out, vec!["[{\"a\":2}]", "[{\"a\":1},{\"a\":2}]"]);
}

#[test]
fn an_alias_nothing_has_open_is_the_products_own_error() {
    assert_eq!(fails("? FoxScript.Data.CursorToJson(\"nosuch\")\n"), "13: Alias 'NOSUCH' is not found.");
}

#[test]
fn json_becomes_a_cursor_the_language_can_query() {
    let out = run(
        "LOCAL n\n\
         n = FoxScript.Data.JsonToCursor('[{\"id\": 1, \"name\": \"one\"}, {\"id\": 2, \"name\": \"two\"}]', \"c_made\")\n\
         ? n, RECCOUNT(\"c_made\")\n\
         SELECT c_made\n\
         GO 2\n\
         ? c_made.id, TRIM(c_made.name)\n\
         ? TYPE(\"c_made.id\"), TYPE(\"c_made.name\")\n",
    );
    assert_eq!(out, vec!["         2          2", "                   2 two", "N C"]);
}

#[test]
fn one_object_becomes_one_record() {
    let out = run(
        "? FoxScript.Data.JsonToCursor('{\"id\": 9}', \"c_one\")\n\
         ? RECCOUNT(\"c_one\")\n",
    );
    assert_eq!(out, vec!["         1", "         1"]);
}

#[test]
fn the_fields_are_the_union_of_the_keys_in_the_order_they_are_first_seen() {
    let out = run(
        "LOCAL ARRAY af(1)\n\
         LOCAL i, cList\n\
         FoxScript.Data.JsonToCursor('[{\"b\": 1}, {\"a\": \"x\", \"b\": 2}]', \"c_mixed\")\n\
         AFIELDS(af, \"c_mixed\")\n\
         cList = \"\"\n\
         FOR i = 1 TO ALEN(af, 1)\n\
           cList = cList + IIF(i > 1, \" \", \"\") + ALLTRIM(af(i, 1)) + \":\" + af(i, 2)\n\
         ENDFOR\n\
         ? cList\n",
    );
    assert_eq!(out, vec!["B:N A:C"]);
}

#[test]
fn a_key_whose_every_value_is_null_becomes_logical() {
    // which is what vfp9.exe says a bare .NULL. is: measured, VARTYPE(.NULL., .T.) answers L
    let out = run(
        "FoxScript.Data.JsonToCursor('[{\"a\": null}]', \"c_null\")\n\
         ? TYPE(\"c_null.a\")\n",
    );
    assert_eq!(out, vec!["L"]);
}

#[test]
fn a_nested_value_becomes_a_memo_holding_its_json_text() {
    // `TYPE()` of a memo field is "C" in Visual FoxPro, measured, so the header is what says it
    let out = run(
        "LOCAL ARRAY af(1)\n\
         FoxScript.Data.JsonToCursor('[{\"tags\": [1, 2]}]', \"c_nest\")\n\
         AFIELDS(af, \"c_nest\")\n\
         ? af(1, 2), c_nest.tags\n",
    );
    assert_eq!(out, vec!["M [1,2]"]);
}

#[test]
fn a_number_keeps_as_many_places_as_the_widest_value_needs() {
    let out = run(
        "FoxScript.Data.JsonToCursor('[{\"n\": 1}, {\"n\": 2.25}]', \"c_num\")\n\
         SELECT c_num\n\
         GO 1\n\
         ? c_num.n\n\
         GO 2\n\
         ? c_num.n\n",
    );
    assert_eq!(out, vec!["                1.00", "                2.25"]);
}

#[test]
fn a_value_that_is_not_an_array_of_objects_is_refused() {
    assert_eq!(
        fails("? FoxScript.Data.JsonToCursor('[1, 2]', \"c_bad\")\n"),
        "3001: A cursor is made from an array of objects."
    );
}

#[test]
fn a_cursor_goes_out_and_comes_back_the_same() {
    let out = run(
        "CREATE CURSOR c_first (id N(4), name C(10))\n\
         INSERT INTO c_first VALUES (1, \"one\")\n\
         INSERT INTO c_first VALUES (2, \"two\")\n\
         LOCAL cJson\n\
         cJson = FoxScript.Data.CursorToJson(\"c_first\")\n\
         FoxScript.Data.JsonToCursor(cJson, \"c_back\")\n\
         ? FoxScript.Data.CursorToJson(\"c_back\") == cJson\n",
    );
    assert_eq!(out, vec![".T."]);
}

// ----- discovery -----------------------------------------------------------------------------

#[test]
fn amembers_lists_the_whole_namespace() {
    let out = run(
        "LOCAL ARRAY am(1)\n\
         LOCAL n, i, cList\n\
         n = AMEMBERS(am, FoxScript, 1)\n\
         cList = \"\"\n\
         FOR i = 1 TO n\n\
           cList = cList + IIF(i > 1, \" \", \"\") + am(i, 1)\n\
         ENDFOR\n\
         ? cList\n\
         n = AMEMBERS(am, FoxScript.Json, 1)\n\
         cList = \"\"\n\
         FOR i = 1 TO n\n\
           cList = cList + IIF(i > 1, \" \", \"\") + am(i, 1)\n\
         ENDFOR\n\
         ? cList\n\
         n = AMEMBERS(am, FoxScript.Data, 1)\n\
         cList = \"\"\n\
         FOR i = 1 TO n\n\
           cList = cList + IIF(i > 1, \" \", \"\") + am(i, 1)\n\
         ENDFOR\n\
         ? cList\n",
    );
    assert_eq!(
        out,
        vec![
            "DATA HTTP JSON VERSION",
            "COUNT GET HAS KEYS PARSE STRINGIFY",
            "CURSORTOJSON JSONTOCURSOR",
        ]
    );
}
