//! What an object of a Visual FoxPro base class is made of: the properties it starts out
//! holding, and the events and methods it answers to.
//!
//! `CREATEOBJECT("CommandButton")` has to give back a button with a Caption of "Command" and a
//! Height of 17, or a program that reads one is being told a story. In the running application
//! that comes from `src/shared/registry`; here it comes from `base_classes.tsv`, which is
//! written from that same registry so the two cannot drift. Regenerate it with
//! `node scripts/gen-base-classes.mjs`.
//!
//! `tests/reference/vfp-base-classes.tsv` is the product's own answer to the same question, and
//! it is what the registry is generated from in the first place.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::value::Value;

/// What a property starts out holding. Kept apart from `Value`, which is built on `Rc` and so
/// cannot live in a static.
#[derive(Debug, Clone)]
pub enum PropDefault {
    Str(String),
    Num(f64),
    Log(bool),
    Null,
}

impl PropDefault {
    pub fn to_value(&self) -> Value {
        match self {
            PropDefault::Str(s) => Value::str(s),
            PropDefault::Num(n) => Value::number(*n),
            PropDefault::Log(b) => Value::Logical(*b),
            PropDefault::Null => Value::Null,
        }
    }
}

/// One base class, as the registry declares it.
pub struct BaseClass {
    /// Property name (upper-cased) and what a new object holds, in the order the table lists.
    pub properties: Vec<(String, PropDefault)>,
    /// Properties the object answers to whose value is not a constant: `Parent`, `Application`,
    /// a container's `Controls`. They are members; reading one is worked out elsewhere.
    pub computed: Vec<String>,
    /// Properties the product refuses to have a program write, upper-cased, with the error it
    /// raises for each.
    pub read_only: Vec<(String, u32)>,
    /// Event names, upper-cased.
    pub events: Vec<String>,
    /// Method names, upper-cased.
    pub methods: Vec<String>,
}

impl BaseClass {
    /// What one property of a new object holds, or `None` when the class has no such property.
    pub fn property(&self, name: &str) -> Option<Value> {
        let upper = name.to_ascii_uppercase();
        self.properties.iter().find(|(n, _)| *n == upper).map(|(_, v)| v.to_value())
    }
}

const TABLE: &str = include_str!("base_classes.tsv");

fn parse() -> HashMap<String, BaseClass> {
    let mut classes: HashMap<String, BaseClass> = HashMap::new();
    for line in TABLE.lines() {
        if line.starts_with('#') || line.trim().is_empty() || line.starts_with("class\t") {
            continue;
        }
        let mut fields = line.split('\t');
        let (Some(class), Some(kind), Some(name)) = (fields.next(), fields.next(), fields.next()) else {
            continue;
        };
        let of = fields.next().unwrap_or("-");
        let text = fields.next().unwrap_or("");
        let refuses: Option<u32> = fields.next().unwrap_or("").parse().ok();
        let entry = classes.entry(class.to_ascii_uppercase()).or_insert_with(|| BaseClass {
            properties: Vec::new(),
            computed: Vec::new(),
            read_only: Vec::new(),
            events: Vec::new(),
            methods: Vec::new(),
        });
        let upper = name.to_ascii_uppercase();
        if let Some(code) = refuses {
            entry.read_only.push((upper.clone(), code));
        }
        match kind {
            // a row that only says the class exists, for one that answers to nothing at all
            "class" => {}
            "event" => entry.events.push(upper),
            "method" => entry.methods.push(upper),
            _ => match of {
                "C" => entry.properties.push((upper, PropDefault::Str(text.to_string()))),
                "N" => entry.properties.push((upper, PropDefault::Num(text.parse().unwrap_or(0.0)))),
                "L" => entry.properties.push((upper, PropDefault::Log(text == ".T."))),
                "X" => entry.properties.push((upper, PropDefault::Null)),
                _ => entry.computed.push(upper),
            },
        }
    }
    classes
}

/// The base class of that name, or `None` when the name is not one of Visual FoxPro's.
pub fn find(class: &str) -> Option<&'static BaseClass> {
    static CLASSES: OnceLock<HashMap<String, BaseClass>> = OnceLock::new();
    CLASSES.get_or_init(parse).get(&class.to_ascii_uppercase())
}
