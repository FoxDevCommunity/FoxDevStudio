//! Reading and writing the XML Visual FoxPro puts a cursor in and takes one out of.
//!
//! The reading is `roxmltree`'s: it checks the document is well formed, resolves namespaces,
//! and reads entities and CDATA the way the specification says. What is added here is the shape
//! the rest of the runtime wants - an owned tree that outlives the text it was read from, the
//! byte-level decoding the XML declaration asks for, and the small subset of XPath the
//! XMLAdapter's `XMLNameIsXPath` needs.

/// One element: what it is called, what it was given, and what is inside it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Element {
    /// The name as written, prefix and all: `ns:price`.
    pub name: String,
    /// The namespace that prefix stands for, where the document bound one.
    pub namespace: String,
    /// In the order they were set, not sorted by name - `CURSORTOXML()`'s attribute-centric
    /// output writes a row's fields in field order, and a `BTreeMap` would alphabetize them.
    pub attributes: Vec<(String, String)>,
    pub children: Vec<Element>,
    /// The text directly inside it, entities and CDATA already read.
    pub text: String,
}

impl Element {
    pub fn new(name: &str) -> Element {
        Element { name: name.to_string(), ..Element::default() }
    }

    /// Sets an attribute, in place if it is already there, appended after the rest otherwise -
    /// what a `BTreeMap`'s `insert` did, kept in the order the caller set them in.
    pub fn set_attr(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        let value = value.into();
        match self.attributes.iter_mut().find(|(k, _)| *k == key) {
            Some(existing) => existing.1 = value,
            None => self.attributes.push((key, value)),
        }
    }

    /// The part of the name after the colon: `ns:price` is `price`.
    pub fn local_name(&self) -> &str {
        local_name(&self.name)
    }

    /// The child of that name, ignoring any namespace prefix, or nothing.
    pub fn child(&self, name: &str) -> Option<&Element> {
        self.children.iter().find(|e| e.local_name().eq_ignore_ascii_case(name))
    }

    /// An attribute by name, ignoring any prefix and the case of it.
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(key, _)| local_name(key).eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// Everything the element holds that is not another element: its text, and the text of
    /// anything inside it that holds no elements of its own.
    pub fn content(&self) -> String {
        if !self.text.is_empty() || self.children.is_empty() {
            return self.text.clone();
        }
        self.children.iter().map(Element::content).collect::<Vec<_>>().join("")
    }

    /// The elements a path picks out. The subset is what an XMLAdapter uses: `a/b`, `/a/b`,
    /// `//b` for one at any depth, `*` for any name, `a[@id='1']` for one with an attribute,
    /// and `@name` for an attribute's own value.
    pub fn select(&self, path: &str) -> Vec<&Element> {
        let path = path.trim();
        let (path, any_depth) = match path.strip_prefix("//") {
            Some(rest) => (rest, true),
            None => (path.trim_start_matches('/'), false),
        };
        if path.is_empty() {
            return vec![self];
        }
        let mut here: Vec<&Element> = vec![self];
        for (i, step) in path.split('/').enumerate() {
            let deep = any_depth && i == 0;
            let mut next: Vec<&Element> = Vec::new();
            for element in here {
                element.step(step, deep, &mut next);
            }
            here = next;
        }
        here
    }

    /// One step of that path, from one element.
    fn step<'a>(&'a self, step: &str, deep: bool, out: &mut Vec<&'a Element>) {
        let (name, want) = match step.split_once('[') {
            Some((name, rest)) => (name, attribute_test(rest.trim_end_matches(']'))),
            None => (step, None),
        };
        for child in &self.children {
            let matches = name == "*" || child.local_name().eq_ignore_ascii_case(name);
            let passes = match &want {
                None => true,
                Some((key, value)) => child.attribute(key) == Some(value.as_str()),
            };
            if matches && passes {
                out.push(child);
            }
            if deep {
                child.step(step, true, out);
            }
        }
    }
}

/// `@id='1'` as the attribute and the value it must have.
fn attribute_test(test: &str) -> Option<(String, String)> {
    let (key, value) = test.trim().strip_prefix('@')?.split_once('=')?;
    let value = value.trim().trim_matches(['\'', '"']);
    Some((key.trim().to_string(), value.to_string()))
}

/// The part of a name after the colon: `ns:price` is `price`.
pub fn local_name(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

/// Reads a document and answers its outermost element. A document that is not well formed is
/// an error saying why, which is what the caller turns into a Visual FoxPro one.
pub fn parse(source: &str) -> Result<Element, String> {
    let options = roxmltree::ParsingOptions { allow_dtd: true, ..roxmltree::ParsingOptions::default() };
    let document = roxmltree::Document::parse_with_options(source, options).map_err(|e| e.to_string())?;
    Ok(own(document.root_element()))
}

/// The same from the bytes of a file, decoded as the XML declaration says it is written.
pub fn parse_bytes(bytes: &[u8]) -> Result<Element, String> {
    parse(&decode(bytes))
}

/// Bytes as text, following the byte order mark or the encoding the declaration names. XML
/// says a document with neither is UTF-8.
pub fn decode(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(rest).into_owned();
    }
    if bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]) {
        let (text, _, _) = encoding_rs::UTF_16LE.decode(bytes);
        return text.into_owned();
    }
    let declared = declared_encoding(bytes).and_then(|name| encoding_rs::Encoding::for_label(name.as_bytes()));
    let (text, _, _) = declared.unwrap_or(encoding_rs::UTF_8).decode(bytes);
    text.into_owned()
}

/// The `encoding="..."` of the declaration, read from the first line as ASCII.
fn declared_encoding(bytes: &[u8]) -> Option<String> {
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(200)]).to_string();
    let start = head.find("encoding")? + "encoding".len();
    let rest = head[start..].trim_start().strip_prefix('=')?.trim_start();
    let quote = rest.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    let value = &rest[1..];
    Some(value[..value.find(quote)?].to_string())
}

/// A `roxmltree` element as one that owns what it holds.
fn own(node: roxmltree::Node<'_, '_>) -> Element {
    let name = match node.tag_name().namespace().and_then(|uri| node.lookup_prefix(uri)) {
        Some(prefix) if !prefix.is_empty() => format!("{prefix}:{}", node.tag_name().name()),
        _ => node.tag_name().name().to_string(),
    };
    let mut element = Element {
        name,
        namespace: node.tag_name().namespace().unwrap_or_default().to_string(),
        attributes: Vec::new(),
        children: Vec::new(),
        text: String::new(),
    };
    for attribute in node.attributes() {
        let key = match attribute.namespace().and_then(|uri| node.lookup_prefix(uri)) {
            Some(prefix) if !prefix.is_empty() => format!("{prefix}:{}", attribute.name()),
            _ => attribute.name().to_string(),
        };
        element.set_attr(key, attribute.value().to_string());
    }
    for child in node.children() {
        if child.is_element() {
            element.children.push(own(child));
        } else if let Some(text) = child.text() {
            // an element that holds other elements holds no text of its own: what is between
            // them is the indentation the document was written with
            element.text.push_str(text);
        }
    }
    if !element.children.is_empty() && element.text.trim().is_empty() {
        element.text.clear();
    }
    element
}

/// The five entities XML defines, going out.
pub fn escape(text: &str) -> String {
    quick_xml::escape::escape(text).into_owned()
}

/// Writes an element and everything inside it. `indent` is how many levels in it starts, or
/// nothing at all when the document is written on one line.
pub fn write(element: &Element, indent: Option<usize>) -> String {
    let mut out = String::new();
    write_into(&mut out, element, indent);
    out
}

fn write_into(out: &mut String, element: &Element, indent: Option<usize>) {
    // measured against the product: eight spaces a level, not a tab
    let pad = indent.map(|n| " ".repeat(n * 8)).unwrap_or_default();
    let newline = if indent.is_some() { "\r\n" } else { "" };
    out.push_str(&pad);
    out.push('<');
    out.push_str(&element.name);
    for (key, value) in &element.attributes {
        out.push_str(&format!(" {key}=\"{}\"", escape(value)));
    }
    if element.children.is_empty() && element.text.is_empty() {
        out.push_str("/>");
        out.push_str(newline);
        return;
    }
    out.push('>');
    if element.children.is_empty() {
        out.push_str(&escape(&element.text));
    } else {
        out.push_str(newline);
        for child in &element.children {
            write_into(out, child, indent.map(|n| n + 1));
        }
        out.push_str(&pad);
    }
    out.push_str(&format!("</{}>", element.name));
    out.push_str(newline);
}

/// The namespaces the formats a Visual FoxPro document is written in are declared with. An
/// element belongs to a format because of its namespace, never because of what it is called.
pub const XSD_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema";
pub const DIFFGRAM_NAMESPACE: &str = "urn:schemas-microsoft-com:xml-diffgram-v1";
pub const MSDATA_NAMESPACE: &str = "urn:schemas-microsoft-com:xml-msdata";

/// Whether an element is schema rather than data. A document written by Visual FoxPro or by
/// ADO.NET puts an inline schema in front of the rows.
pub fn is_schema(element: &Element) -> bool {
    element.namespace == XSD_NAMESPACE
}

/// Whether it is the wrapper a diffgram puts round the rows it carries.
fn is_diffgram(element: &Element) -> bool {
    element.namespace == DIFFGRAM_NAMESPACE && element.local_name().eq_ignore_ascii_case("diffgram")
}

/// What the schema says the dataset element is called: the one it marks .
/// A document that declares one wraps its rows in it; one that does not holds them directly.
fn dataset_name(schema: &Element) -> Option<String> {
    fn look(element: &Element) -> Option<String> {
        let marked = element
            .attributes
            .iter()
            .any(|(key, value)| local_name(key).eq_ignore_ascii_case("IsDataSet") && value.eq_ignore_ascii_case("true"));
        if marked {
            return element.attribute("name").map(str::to_string);
        }
        element.children.iter().find_map(look)
    }
    look(schema)
}

/// One kind of row a document holds, and the fields those rows carry between them.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TableShape {
    pub name: String,
    pub fields: Vec<FieldShape>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FieldShape {
    pub name: String,
    /// Whether the row carries it as an attribute rather than as an element inside it.
    pub attribute: bool,
}

/// A whole document as the XMLAdapter reads it: what the outermost element is called, whether
/// it is a diffgram, and the tables under it.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DocumentShape {
    pub root: String,
    pub diffgram: bool,
    pub tables: Vec<TableShape>,
}

/// Where a document keeps the rows it is carrying now.
///
/// Three things can stand between the outermost element and a row, and each of them says so
/// in the document rather than by being called anything in particular: an inline schema, the
/// wrapper a diffgram puts round its two images, and the dataset element the schema names.
pub fn rows_of(root: &Element) -> (&Element, bool) {
    let (data, diffgram) = match root.children.iter().find(|c| is_diffgram(c)) {
        // inside a diffgram the rows as they are now come first;  is the image
        // they were changed from, and  what could not be applied
        Some(wrapper) => (
            wrapper.children.iter().find(|c| c.namespace != DIFFGRAM_NAMESPACE).unwrap_or(wrapper),
            true,
        ),
        None => (root, false),
    };
    let dataset = root.children.iter().find(|c| is_schema(c)).and_then(dataset_name);
    let holder = match dataset {
        Some(name) if !data.local_name().eq_ignore_ascii_case(&name) => data
            .children
            .iter()
            .find(|c| c.local_name().eq_ignore_ascii_case(&name))
            .unwrap_or(data),
        _ => data,
    };
    (holder, diffgram)
}

/// What a document holds, without making anything of it: the rows it carries, gathered by
/// name, and the fields each kind of row was seen to have.
pub fn shape_of(text: &str) -> DocumentShape {
    let Ok(root) = parse(text) else { return DocumentShape::default() };
    let (holder, diffgram) = rows_of(&root);
    let mut tables: Vec<TableShape> = Vec::new();
    for row in holder.children.iter().filter(|c| !is_schema(c) && c.namespace != DIFFGRAM_NAMESPACE) {
        let at = match tables.iter().position(|t| t.name == row.name) {
            Some(at) => at,
            None => {
                tables.push(TableShape { name: row.name.clone(), fields: Vec::new() });
                tables.len() - 1
            }
        };
        let mut seen: Vec<FieldShape> = row
            .attributes
            .iter()
            .filter(|(key, _)| !key.contains(':'))
            .map(|(name, _)| FieldShape { name: name.clone(), attribute: true })
            .collect();
        seen.extend(row.children.iter().map(|c| FieldShape { name: c.name.clone(), attribute: false }));
        for field in seen {
            if !tables[at].fields.iter().any(|f| f.name == field.name) {
                tables[at].fields.push(field);
            }
        }
    }
    DocumentShape { root: root.name.clone(), diffgram, tables }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_document_reads_back_as_the_tree_it_describes() {
        let doc = parse("<?xml version=\"1.0\"?>\n<rows><row id=\"1\"><name>Bolt</name><price>2.50</price></row></rows>")
            .expect("it reads");
        assert_eq!(doc.name, "rows");
        assert_eq!(doc.children.len(), 1);
        let row = &doc.children[0];
        assert_eq!(row.attribute("id"), Some("1"));
        assert_eq!(row.child("name").map(Element::content), Some("Bolt".to_string()));
        assert_eq!(row.child("price").map(Element::content), Some("2.50".to_string()));
    }

    #[test]
    fn entities_comments_and_cdata_are_read_the_way_xml_says() {
        let doc = parse("<a><!-- skip me --><b>x &amp; y &#65;</b><c><![CDATA[<not a tag>]]></c></a>").expect("it reads");
        assert_eq!(doc.child("b").map(Element::content), Some("x & y A".to_string()));
        assert_eq!(doc.child("c").map(Element::content), Some("<not a tag>".to_string()));
    }

    #[test]
    fn an_empty_element_closes_itself() {
        let doc = parse("<rows><row/><row a=\"1\"/></rows>").expect("it reads");
        assert_eq!(doc.children.len(), 2);
        assert_eq!(doc.children[1].attribute("a"), Some("1"));
    }

    #[test]
    fn a_document_that_is_not_well_formed_says_so() {
        let err = parse("<a><b></a>").expect_err("mismatched tags are not a document");
        assert!(err.to_lowercase().contains("expected"), "{err}");
        assert!(parse("not xml at all").is_err());
        assert!(parse("<a><b/>").is_err(), "an element that is never closed is not a document");
    }

    #[test]
    fn a_namespace_is_resolved_and_the_prefix_kept() {
        let doc = parse("<p:rows xmlns:p=\"urn:parts\"><p:row><p:name>Bolt</p:name></p:row></p:rows>").expect("it reads");
        assert_eq!(doc.name, "p:rows");
        assert_eq!(doc.namespace, "urn:parts");
        assert_eq!(doc.local_name(), "rows");
        assert_eq!(doc.child("row").and_then(|r| r.child("name")).map(Element::content), Some("Bolt".to_string()));
    }

    #[test]
    fn bytes_are_decoded_as_the_declaration_says_they_are_written() {
        // a pound sign in Windows-1252 is one byte, and is not valid UTF-8 on its own
        let mut bytes = b"<?xml version=\"1.0\" encoding=\"windows-1252\"?><a>".to_vec();
        bytes.push(0xA3);
        bytes.extend_from_slice(b"5</a>");
        let doc = parse_bytes(&bytes).expect("it reads");
        assert_eq!(doc.content(), "\u{a3}5");

        let utf8 = "<?xml version=\"1.0\" encoding=\"utf-8\"?><a>\u{a3}5</a>".as_bytes().to_vec();
        assert_eq!(parse_bytes(&utf8).expect("it reads").content(), "\u{a3}5");
    }

    #[test]
    fn a_path_picks_out_the_elements_it_names() {
        let doc = parse(
            "<catalogue><part id=\"1\"><name>Bolt</name></part><part id=\"2\"><name>Nut</name></part></catalogue>",
        )
        .expect("it reads");
        assert_eq!(doc.select("part").len(), 2);
        assert_eq!(doc.select("part/name").iter().map(|e| e.content()).collect::<Vec<_>>(), vec!["Bolt", "Nut"]);
        assert_eq!(doc.select("//name").len(), 2);
        assert_eq!(doc.select("part[@id='2']/name").first().map(|e| e.content()), Some("Nut".to_string()));
        assert_eq!(doc.select("*/name").len(), 2);
    }

    #[test]
    fn what_is_written_reads_back_the_same() {
        let doc = parse("<rows><row><name>a &lt; b</name></row></rows>").expect("it reads");
        let text = write(&doc, Some(0));
        let back = parse(&text).expect("it reads again");
        assert_eq!(back, doc);
    }
}
