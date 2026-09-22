//! Colour, font and keyboard built-ins.

mod ctx;

use ctx::{array, err, flag, items, n, num, request, s, text, texts, value};
use foxvm::bytecode::Constant;
use foxvm::compiler::compile_program;
use foxvm::error::RtError;
use foxvm::host::HostRequest;
use foxvm::value::Value;

// ----- RGB ------------------------------------------------------------------------------

#[test]
fn rgb_packs_a_colour() {
    assert_eq!(num("RGB", vec![n(255.0), n(0.0), n(0.0)]), 255.0, "red is the low byte");
    assert_eq!(num("RGB", vec![n(0.0), n(255.0), n(0.0)]), 65280.0);
    assert_eq!(num("RGB", vec![n(0.0), n(0.0), n(255.0)]), 16711680.0);
    assert_eq!(num("RGB", vec![n(0.0), n(0.0), n(0.0)]), 0.0);
    assert_eq!(num("RGB", vec![n(255.0), n(255.0), n(255.0)]), 16777215.0);
    assert_eq!(num("RGB", vec![n(1.0), n(2.0), n(3.0)]), 1.0 + 2.0 * 256.0 + 3.0 * 65536.0);
}

#[test]
fn rgb_clamps_and_truncates_its_components() {
    assert_eq!(num("RGB", vec![n(300.0), n(-5.0), n(999.0)]), 255.0 + 0.0 + 255.0 * 65536.0);
    assert_eq!(num("RGB", vec![n(12.9), n(0.0), n(0.0)]), 12.0, "components truncate toward zero");
    assert_eq!(err("RGB", vec![s("x"), n(0.0), n(0.0)]).code, RtError::FUNCTION_ARG_INVALID);
}

/// The compiler folds `RGB()` inside DEFINE CLASS, where a property must be a constant. The two
/// implementations have to agree or a form's colours change the moment the code moves into a
/// method.
#[test]
fn rgb_matches_the_constant_folded_in_a_class_definition() {
    let src = "DEFINE CLASS c1 AS custom\n\
        BackColor = RGB(12, 34, 56)\n\
        Edge = RGB(300, -5, 255)\n\
        ENDDEFINE\n";
    let r = compile_program(src, "main");
    assert!(!r.diagnostics.iter().any(|d| d.is_error()), "{:#?}", r.diagnostics);
    let module = r.module.expect("module");
    let class = module.classes.iter().find(|c| c.name.eq_ignore_ascii_case("c1")).expect("class c1");
    let folded = |name: &str| match class.properties.iter().find(|(p, _)| p.eq_ignore_ascii_case(name)) {
        Some((_, Constant::Num(v, ..))) => *v,
        other => panic!("{name} folded to {other:?}"),
    };
    assert_eq!(folded("BackColor"), num("RGB", vec![n(12.0), n(34.0), n(56.0)]));
    assert_eq!(folded("Edge"), num("RGB", vec![n(300.0), n(-5.0), n(255.0)]));
}

// ----- dialogs --------------------------------------------------------------------------

#[test]
fn getcolor_yields_a_dialog_request() {
    assert_eq!(request("GETCOLOR", vec![]), HostRequest::GetColor { default: -1.0 }, "no preselected colour");
    assert_eq!(request("GETCOLOR", vec![n(255.0)]), HostRequest::GetColor { default: 255.0 });
}

#[test]
fn getfont_yields_a_dialog_request() {
    assert_eq!(
        request("GETFONT", vec![]),
        HostRequest::GetFont { name: String::new(), size: 0.0, style: String::new() }
    );
    assert_eq!(
        request("GETFONT", vec![s("Arial"), n(12.0), s("BI")]),
        HostRequest::GetFont { name: "Arial".into(), size: 12.0, style: "BI".into() }
    );
}

#[test]
fn getkey_yields_a_request() {
    assert_eq!(request("GETKEY", vec![]), HostRequest::GetKey);
}

// ----- font metrics ---------------------------------------------------------------------

#[test]
fn fontmetric_reports_approximate_metrics() {
    let m = |attr: f64, size: f64| num("FONTMETRIC", vec![n(attr), s("Arial"), n(size)]);
    // 1 is the whole character cell, 2 what stands above the baseline, 3 what hangs below it -
    // the numbering Visual FoxPro uses, measured against Arial 10 there
    let (height, ascent, descent) = (m(1.0, 10.0), m(2.0, 10.0), m(3.0, 10.0));
    assert_eq!(ascent, 8.0);
    assert_eq!(descent, 2.0);
    assert_eq!(height, ascent + descent, "the cell is ascent + descent");
    assert_eq!(m(7.0, 10.0), 10.0, "maximum width is the point size");
    assert_eq!(m(6.0, 10.0), 6.0, "average width is narrower than the maximum");
    assert_eq!(m(8.0, 10.0), 400.0, "an upright face weighs 400");
    // the relationship must hold at any size, and the style argument does not change it
    for size in [1.0, 8.0, 9.0, 11.0, 24.0, 72.0] {
        assert_eq!(m(1.0, size), m(2.0, size) + m(3.0, size), "the cell at {size}pt");
    }
    assert_eq!(num("FONTMETRIC", vec![n(1.0), s("Arial"), n(10.0), s("B")]), 10.0);
    // the attribute alone measures the caller's own font, which here is the shell font
    assert_eq!(num("FONTMETRIC", vec![n(7.0)]), 9.0);
    assert_eq!(m(0.0, 10.0), 0.0, "an unknown attribute returns 0");
    assert_eq!(m(18.0, 10.0), 0.0, "an unknown attribute returns 0");
    assert_eq!(m(1.0, -5.0), 0.0, "a non-positive size has no metrics");
}

#[test]
fn wfont_reports_the_shell_font() {
    assert_eq!(text("WFONT", vec![n(1.0)]), "Segoe UI");
    assert_eq!(value("WFONT", vec![n(2.0)]), Value::number(9.0));
    assert_eq!(text("WFONT", vec![n(3.0)]), "N", "an upright face is spelled N, as VFP spells it");
    assert_eq!(text("WFONT", vec![n(9.0)]), "", "an unknown attribute is empty");
    assert_eq!(text("WFONT", vec![n(1.0), s("Form1")]), "Segoe UI", "every window uses the same font");
}

#[test]
fn afont_fills_an_array_with_font_families() {
    let a = array(vec![Value::Logical(false)]);
    assert_eq!(num("AFONT", vec![a.clone()]), 7.0);
    let all = texts(&a);
    assert_eq!(all.len(), 7);
    assert_eq!(all[0], "Segoe UI");
    assert!(all.contains(&"Courier New".to_string()), "{all:?}");

    // naming a font asks something else about that one font, and answers whether the array
    // was made rather than a count - measured against vfp9.exe, `AFONT(arr, cName) > 0` raises
    // "Operator/operand type mismatch" because the real answer is logical, not numeric
    let b = array(vec![]);
    assert!(flag("AFONT", vec![b.clone(), s("Courier New")]));
    // every family here is a scalable TrueType face, so the one row is what a scalable font
    // always answers on its own: -1, since it has no fixed sizes to list
    assert_eq!(items(&b), vec![Value::number(-1.0)]);

    // a size given too still answers whether the array was made, and the array itself holds
    // whether that size is available - always true for a scalable font
    let c = array(vec![]);
    assert!(flag("AFONT", vec![c.clone(), s("Segoe UI"), n(12.0)]));
    assert_eq!(items(&c), vec![Value::Logical(true)]);

    assert_eq!(err("AFONT", vec![s("not an array")]).code, RtError::FUNCTION_ARG_INVALID);
}

#[test]
fn parsfont_says_it_is_not_a_foxpro_function() {
    let e = err("PARSFONT", vec![]);
    assert_eq!(e.code, RtError::FEATURE_NOT_AVAILABLE);
    assert!(e.message.contains("no such Visual FoxPro function"), "{}", e.message);
    assert!(e.message.contains("GETFONT()"), "the message must point at what to use: {}", e.message);
}

#[test]
fn loadpicture_reads_a_file_and_savepicture_writes_one() {
    // with no file it is the null picture: no handle, no kind, no size
    match ctx::call("LOADPICTURE", vec![]) {
        Ok(foxvm::builtins::BuiltinResult::Suspend(HostRequest::MakePicture {
            picture,
            of_kind,
            width,
            height,
        })) => assert_eq!((picture, of_kind, width, height), (0.0, 0.0, 0.0, 0.0)),
        _ => panic!("LOADPICTURE() did not ask for the null picture"),
    }
    // with one it asks the host for the bytes before it can say anything about them
    assert_eq!(request("LOADPICTURE", vec![s("fox.bmp")]), HostRequest::FileReadBytes { path: "fox.bmp".into() });
    // a picture that was never loaded is nothing to write out
    assert_eq!(value("SAVEPICTURE", vec![Value::Object(foxvm::value::Handle(1)), s("out.bmp")]), Value::Logical(false));
    assert_eq!(err("SAVEPICTURE", vec![n(1.0), s("out.bmp")]).code, RtError::FUNCTION_ARG_INVALID);
}

#[test]
fn a_picture_is_measured_in_himetric() {
    // four pixels across at ninety-six to the inch is 4 x 2540 / 96, rounded
    let bitmap = tiny_bitmap();
    let picture = foxvm::picture::Picture::decode(&bitmap).expect("a bitmap");
    assert_eq!((picture.width(), picture.height(), picture.kind), (106.0, 79.0, 1));
    // and what it writes back reads as the same picture
    let again = foxvm::picture::Picture::decode(&picture.to_bitmap().expect("written")).expect("read back");
    assert_eq!((again.width(), again.height()), (106.0, 79.0));
    // bytes that are not a picture are refused, rather than answered with an empty one
    let refused = foxvm::picture::Picture::decode(b"not a picture").err().expect("not a picture");
    assert_eq!(refused.code, RtError::OLE_ERROR);
}

/// A four-by-three 24-bit bitmap, which is the same file the golden programs load.
fn tiny_bitmap() -> Vec<u8> {
    let (w, h) = (4usize, 3usize);
    let row = w * 3 + (4 - (w * 3) % 4) % 4;
    let mut pixels = vec![0u8; row * h];
    for (y, colour) in [[0, 0, 255], [0, 255, 0], [255, 0, 0]].iter().enumerate() {
        for x in 0..w {
            pixels[y * row + x * 3..y * row + x * 3 + 3].copy_from_slice(colour);
        }
    }
    let mut out = vec![0u8; 54];
    out[0..2].copy_from_slice(b"BM");
    out[2..6].copy_from_slice(&((54 + pixels.len()) as u32).to_le_bytes());
    out[10..14].copy_from_slice(&54u32.to_le_bytes());
    out[14..18].copy_from_slice(&40u32.to_le_bytes());
    out[18..22].copy_from_slice(&(w as i32).to_le_bytes());
    out[22..26].copy_from_slice(&(h as i32).to_le_bytes());
    out[26..28].copy_from_slice(&1u16.to_le_bytes());
    out[28..30].copy_from_slice(&24u16.to_le_bytes());
    out[34..38].copy_from_slice(&(pixels.len() as u32).to_le_bytes());
    out.extend_from_slice(&pixels);
    out
}
