fn main() {
    for src in [
        "WITH THISFORM\n  DIMENSION .aChoices[3]\nENDWITH\n",
        "DIMENSION THIS.aChoices[3]\n",
        "WITH THISFORM\n  DIMENSION .aChoices[lnCount]\nENDWITH\n",
    ] {
        let out = foxvm::parser::parse_program(src);
        println!("{:?}", out.diagnostics.iter().map(|d| format!("{}: {}", d.line, d.message)).collect::<Vec<_>>());
    }
}
