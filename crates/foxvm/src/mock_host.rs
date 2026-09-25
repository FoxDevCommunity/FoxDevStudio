//! An in-memory `Host` for tests and the CLI: an object tree with properties, captured `?`
//! output, a fixed clock, a deterministic random sequence and a program table.

use std::collections::{HashMap, VecDeque};

use crate::compiler;
use crate::error::RtError;
use crate::host::{ClassDefOut, Host, HostRequest, JsonValue, Member, MemberInfo};
use crate::value::{Handle, Value};
use crate::vm::{FiberId, Step, Vm};

#[derive(Debug, Clone)]
pub struct MockObject {
    pub class: String,
    pub name: String,
    pub parent: Option<Handle>,
    /// Upper-cased property name -> value.
    pub props: HashMap<String, Value>,
    pub children: Vec<Handle>,
    /// Upper-cased names the object answers `Member::Method` for. Events are in here too:
    /// `PEMSTATUS(o, "Click", 5)` is true for a class that has a Click, whether or not anything
    /// is written in it.
    pub methods: Vec<String>,
    /// Properties the object answers to whose value is worked out rather than stored - its
    /// `Parent`, the application it belongs to, a container's `Controls`.
    pub computed: Vec<String>,
    /// Upper-cased names of the properties the running program has written since the object was
    /// made. What a class declares is not in here, which is what AMEMBERS()' "changed" means.
    pub written: std::collections::BTreeSet<String>,
    /// The same, for what ADDPROPERTY() put there rather than a class declaring it.
    pub added: std::collections::BTreeSet<String>,
}

pub struct MockHost {
    objects: HashMap<u32, MockObject>,
    /// Tables `USE` can open, by upper-cased path: the whole file, as it would be on disk.
    pub tables: HashMap<String, Vec<u8>>,
    /// Files opened so far; the handle is the 1-based position.
    open: Vec<Vec<u8>>,
    /// The table each open handle came from, so the index beside it can be found again.
    open_paths: Vec<String>,
    /// The compound index beside each table, by the table's path.
    pub indexes: HashMap<String, Vec<u8>>,
    /// The memo file beside each table, by the table's path.
    pub memo_files: HashMap<String, Vec<u8>>,
    next_handle: u32,
    /// `?` lines (`??` appends to the last line).
    pub output: Vec<String>,
    pub now: (i32, f64),
    rng: crate::host::SeededRandom,
    /// Upper-cased program name -> module id, for `resolve_program`.
    pub programs: HashMap<String, u32>,
    /// Upper-cased program name -> source, for programs that are not loaded until something
    /// asks: `run_with_requests` compiles one when the VM's `LoadProgram` names it, which is
    /// what the IDE's host does with a `.prg` in the project.
    pub program_sources: HashMap<String, String>,
    /// Every request answered by `run_with_requests`, in order.
    pub requests: Vec<HostRequest>,
    /// Handles released through `ReleaseObject`.
    pub released: Vec<u32>,
    /// Files the low-level functions and the file commands work on, by upper-cased path.
    pub files: HashMap<String, Vec<u8>>,
    /// The class libraries `SET CLASSLIB TO` has loaded: the file, and the name it answers to.
    /// Nothing here reads a `.vcx`; what the list is for is `SET("CLASSLIB")`.
    pub class_libraries: Vec<(String, String)>,
    /// The folders MKDIR has made, upper-cased. A folder holding files counts as made whether
    /// or not it is in here, which is what saves every fixture from having to declare one.
    pub dirs: std::collections::BTreeSet<String>,
    /// The path as it was actually given, by its upper-cased key in `files` or `dirs` - a
    /// Windows file system keeps the case a name was created with even though it compares names
    /// without regard to it, and `ADIR(a, m, c, 1)` is asking for exactly that case back.
    case: HashMap<String, String>,
    /// Upper-cased paths `FCREATE()` made with a nonzero attribute - Windows' own read-only bit,
    /// which blocks a write no matter how the file is reopened, until something outside FCREATE,
    /// FPUTS and FWRITE clears it, which is nothing this runtime does yet.
    readonly: std::collections::BTreeSet<String>,
    /// The character screen as the VM last drew it.
    pub screen: Option<crate::screen::ScreenDoc>,
    /// What a READ answers with, one set of field values per read.
    pub reads: std::collections::VecDeque<Vec<Value>>,
    /// What a MENU TO answers with, one choice per command.
    pub choices: std::collections::VecDeque<f64>,
    /// Where the pointer is over the character screen, for the mouse functions.
    pub pointer: (f64, f64, bool),
    /// The printers the printer functions report; none unless a test says otherwise.
    pub printers: Vec<String>,
    /// Open low-level handles: the file, and where in it the next read or write goes.
    file_handles: HashMap<u32, (String, usize)>,
    next_file_handle: u32,
    /// A request the host refused. Answering one is a value, so an error is left here and
    /// `run_with_requests` raises it into the program instead of resuming.
    pub refused: Option<RtError>,
    /// What BINDEVENT has bound: source handle, event, handler handle, delegate.
    bindings: Vec<(u32, String, u32, String)>,
    /// The running copy of the product, which every object's `Application` answers with. It is
    /// made when something first asks for it, so a program that never mentions `_VFP` is not
    /// charged a handle for it.
    application: Option<Handle>,
}

impl Default for MockHost {
    fn default() -> Self {
        Self::new()
    }
}

const DEFAULT_METHODS: &[&str] =
    &["RELEASE", "SETFOCUS", "REFRESH", "SHOW", "HIDE", "CLICK", "ADDOBJECT", "REMOVEOBJECT"];

impl MockHost {
    /// A host whose only object is `_SCREEN` (handle 0).
    pub fn new() -> Self {
        let mut h = MockHost {
            objects: HashMap::new(),
            tables: HashMap::new(),
            open: Vec::new(),
            open_paths: Vec::new(),
            indexes: HashMap::new(),
            memo_files: HashMap::new(),
            next_handle: 1,
            output: Vec::new(),
            now: (crate::value::days_from_civil(2026, 9, 7), 12.0 * 3600.0 + 30.0 * 60.0),
            rng: crate::host::SeededRandom::default(),
            programs: HashMap::new(),
            program_sources: HashMap::new(),
            requests: Vec::new(),
            released: Vec::new(),
            files: HashMap::new(),
            class_libraries: Vec::new(),
            dirs: std::collections::BTreeSet::new(),
            case: HashMap::new(),
            readonly: std::collections::BTreeSet::new(),
            screen: None,
            reads: std::collections::VecDeque::new(),
            choices: std::collections::VecDeque::new(),
            pointer: (0.0, 0.0, false),
            printers: Vec::new(),
            file_handles: HashMap::new(),
            next_file_handle: 1,
            refused: None,
            bindings: Vec::new(),
            application: None,
        };
        let screen = MockObject {
            class: "Screen".into(),
            name: "_SCREEN".into(),
            parent: None,
            props: HashMap::new(),
            children: Vec::new(),
            methods: DEFAULT_METHODS.iter().map(|s| s.to_string()).collect(),
            computed: Vec::new(),
            written: Default::default(),
            added: Default::default(),
        };
        h.objects.insert(0, screen);
        h
    }

    /// A class name as Visual FoxPro writes it back: first letter up, the rest down.
    ///
    /// Measured - `crates/foxvm/tests/programs/ref_baseclass_spelling.prg` is the program run in
    /// the product: `DEFINE CLASS mybutton AS CommandButton` answers Class "Mybutton" and
    /// BaseClass "Commandbutton", however the declaration spelled either. It matters because a
    /// comparison against a literal is case-sensitive.
    pub fn spell_class(word: &str) -> String {
        let name = word.trim();
        match name.chars().next() {
            None => String::new(),
            Some(first) => first.to_uppercase().collect::<String>() + &name[first.len_utf8()..].to_lowercase(),
        }
    }

    /// The name Visual FoxPro gives an object made of a base class with no name of its own:
    /// `CREATEOBJECT("TextBox").Name` is "Text". It is in the table with everything else.
    pub fn base_class_name(class: &str) -> String {
        crate::base_classes::find(class)
            .and_then(|b| b.property("Name"))
            .and_then(|v| v.as_str().ok().map(|s| s.to_string()))
            .unwrap_or_else(|| class.to_string())
    }

    /// `ADDPROPERTY()`, in either of its two spellings.
    ///
    /// The name may carry subscripts - `AddProperty("aPoly[1,1]")` - and then the property is an
    /// array of that shape holding the value in every element. Measured in Visual FoxPro: the
    /// answer is .T. whether or not the object already had the property, and a zero subscript
    /// raises 31.
    pub fn add_property(&mut self, obj: Handle, name: &str, value: Value) -> Value {
        if let Some((bare, rows, cols)) = array_subscripts(name) {
            if rows == 0 || (name.contains(',') && cols == 0) {
                self.refused = Some(RtError::invalid_subscript());
                return Value::Null;
            }
            let mut array = crate::value::FoxArray::new(rows, cols);
            array.items.fill(value);
            self.set_prop(obj, &bare.to_ascii_uppercase(), Value::Array(std::rc::Rc::new(std::cell::RefCell::new(array))));
            if let Some(o) = self.objects.get_mut(&obj.0) {
                o.added.insert(bare.to_ascii_uppercase());
            }
            return Value::Logical(true);
        }
        self.set_prop(obj, &name.to_ascii_uppercase(), value);
        if let Some(o) = self.objects.get_mut(&obj.0) {
            o.added.insert(name.to_ascii_uppercase());
        }
        Value::Logical(true)
    }

    /// The error the object's class raises when a program writes that property, if it refuses.
    pub fn read_only(&self, h: Handle, name: &str) -> Option<u32> {
        let upper = name.to_ascii_uppercase();
        self.objects
            .get(&h.0)
            .and_then(|o| crate::base_classes::find(&o.class))
            .and_then(|b| b.read_only.iter().find(|(p, _)| *p == upper).map(|(_, code)| *code))
    }

    /// The copy of the product the program is running in: `_VFP`, and what every object's
    /// `Application` property answers with. They are all the same object.
    fn application(&mut self) -> Handle {
        match self.application {
            Some(h) => h,
            None => {
                let h = self.add_object("Application", "Microsoft Visual FoxPro", None);
                self.application = Some(h);
                h
            }
        }
    }

    pub fn add_object(&mut self, class: &str, name: &str, parent: Option<Handle>) -> Handle {
        let h = Handle(self.next_handle);
        self.next_handle += 1;
        // an object of a base class starts out holding what that class holds; a class the table
        // does not know - one a program defined for itself - starts out with only its identity
        let shape = crate::base_classes::find(class);
        let mut props: HashMap<String, Value> = match shape {
            Some(b) => b.properties.iter().map(|(n, v)| (n.clone(), v.to_value())).collect(),
            None => HashMap::new(),
        };
        // A class the table knows answers to exactly what the table says, so an Empty stays
        // empty; anything else - a class a program defined, `_SCREEN` - at least says what it is.
        // A class whose Name has no default of its own still has a Name: the measurement calls
        // it varying because two OLE containers were made under different names, which is what
        // an object made under a name answers with.
        let has_name = shape.is_some_and(|b| b.computed.iter().any(|n| n == "NAME"));
        if shape.is_none() || props.contains_key("NAME") || has_name {
            props.insert("NAME".to_string(), Value::str(name));
        }
        if shape.is_none() {
            props.insert("CLASS".to_string(), Value::str(&Self::spell_class(class)));
            props.insert("BASECLASS".to_string(), Value::str(&Self::spell_class(class)));
        }
        let methods = match shape {
            Some(b) => b.events.iter().chain(b.methods.iter()).cloned().collect(),
            None => DEFAULT_METHODS.iter().map(|s| s.to_string()).collect(),
        };
        self.objects.insert(
            h.0,
            MockObject {
                class: class.to_string(),
                name: name.to_string(),
                parent,
                props,
                children: Vec::new(),
                methods,
                computed: shape.map(|b| b.computed.clone()).unwrap_or_default(),
                written: Default::default(),
                added: Default::default(),
            },
        );
        if let Some(p) = parent
            && let Some(po) = self.objects.get_mut(&p.0)
        {
            po.children.push(h);
        }
        h
    }

    /// Lays a base class's properties and members under an object that already exists.
    ///
    /// `DEFINE CLASS mine AS CommandButton` is a button before it is anything of its own, so the
    /// button's Caption and Height are there unless the declaration says otherwise. What the
    /// object says its own name and class are is not touched.
    pub fn seed_base_class(&mut self, h: Handle, base: &str) {
        let Some(shape) = crate::base_classes::find(base) else { return };
        let identity: Vec<(String, Value)> = ["NAME", "CLASS", "BASECLASS"]
            .iter()
            .filter_map(|k| self.prop(h, k).map(|v| ((*k).to_string(), v)))
            .collect();
        let Some(o) = self.objects.get_mut(&h.0) else { return };
        for (name, value) in &shape.properties {
            o.props.insert(name.clone(), value.to_value());
        }
        o.computed.extend(shape.computed.iter().cloned());
        o.methods.extend(shape.events.iter().chain(shape.methods.iter()).cloned());
        for (name, value) in identity {
            o.props.insert(name, value);
        }
    }

    /// The bytes of an open file, as they are now: what a test asserts a write landed in.
    pub fn open_file(&self, handle: u32) -> Option<&[u8]> {
        self.open.get(handle as usize - 1).map(Vec::as_slice)
    }

    pub fn object(&self, h: Handle) -> Option<&MockObject> {
        self.objects.get(&h.0)
    }

    pub fn set_prop(&mut self, h: Handle, name: &str, v: Value) {
        if let Some(o) = self.objects.get_mut(&h.0) {
            o.props.insert(name.to_ascii_uppercase(), v.deref());
        }
    }

    pub fn prop(&self, h: Handle, name: &str) -> Option<Value> {
        self.objects.get(&h.0)?.props.get(&name.to_ascii_uppercase()).cloned()
    }

    pub fn add_method(&mut self, h: Handle, name: &str) {
        if let Some(o) = self.objects.get_mut(&h.0) {
            o.methods.push(name.to_ascii_uppercase());
        }
    }

    /// Removes the object and its children (later `object_class` answers `None`).
    pub fn release(&mut self, h: Handle) {
        if let Some(o) = self.objects.remove(&h.0) {
            self.released.push(h.0);
            for c in o.children {
                self.release(c);
            }
            if let Some(p) = o.parent
                && let Some(po) = self.objects.get_mut(&p.0)
            {
                po.children.retain(|c| *c != h);
            }
        }
    }

    /// Builds the object tree of `resources/samples/HelloWorld.fxf`; returns the form handle.
    pub fn add_hello_world_form(&mut self) -> Handle {
        let form = self.add_object("Form", "frmHello", None);
        self.set_prop(form, "Caption", Value::str("Hello, World"));
        let lbl = self.add_object("Label", "lblName", Some(form));
        self.set_prop(lbl, "Caption", Value::str("Your name:"));
        let txt = self.add_object("TextBox", "txtName", Some(form));
        self.set_prop(txt, "Value", Value::str(""));
        let chk = self.add_object("CheckBox", "chkLoud", Some(form));
        self.set_prop(chk, "Value", Value::Logical(false));
        let pgf = self.add_object("PageFrame", "pgfMain", Some(form));
        let page1 = self.add_object("Page", "Page1", Some(pgf));
        self.set_prop(page1, "Caption", Value::str("Greeting"));
        let greet = self.add_object("Label", "lblGreeting", Some(page1));
        self.set_prop(greet, "Caption", Value::str("(nothing yet)"));
        let page2 = self.add_object("Page", "Page2", Some(pgf));
        self.set_prop(page2, "Caption", Value::str("About"));
        let hi = self.add_object("CommandButton", "cmdSayHi", Some(form));
        self.set_prop(hi, "Caption", Value::str("Say Hi"));
        let close = self.add_object("CommandButton", "cmdClose", Some(form));
        self.set_prop(close, "Caption", Value::str("Close"));
        form
    }

    /// Finds a child by dotted path relative to `root` (`"pgfMain.Page1.lblGreeting"`).
    pub fn find(&self, root: Handle, path: &str) -> Option<Handle> {
        let mut h = root;
        for part in path.split('.').filter(|p| !p.is_empty()) {
            let o = self.objects.get(&h.0)?;
            h = *o
                .children
                .iter()
                .find(|c| self.objects.get(&c.0).is_some_and(|co| co.name.eq_ignore_ascii_case(part)))?;
        }
        Some(h)
    }

    /// Builds what a `DEFINE CLASS` declares onto a fresh object: its property values, one child
    /// per `ADD OBJECT` member and the names of the methods it declares. Inheritance is the real
    /// host's job, so only this class's own declaration is applied.
    pub fn apply_class_def(&mut self, obj: Handle, def: &ClassDefOut) {
        self.set_prop(obj, "Class", Value::str(&Self::spell_class(&def.name)));
        self.set_prop(obj, "BaseClass", Value::str(&Self::spell_class(&def.base_class)));
        for p in &def.properties {
            self.set_prop(obj, &p.name, p.value.to_value());
        }
        for m in &def.members {
            let child = self.add_object(&m.class, &m.name, Some(obj));
            for p in &m.properties {
                self.set_prop(child, &p.name, p.value.to_value());
            }
        }
        for method in &def.methods {
            match method.name.split_once('.') {
                Some((member, event)) => {
                    if let Some(child) = self.find(obj, member) {
                        self.add_method(child, event);
                    }
                }
                None => self.add_method(obj, &method.name),
            }
        }
    }

    /// Remembers the case a file or folder was actually named with, by its upper-cased key.
    fn note_case(&mut self, key: &str, given: &str) {
        self.case.insert(key.to_string(), given.to_string());
    }

    /// The name a `dir` entry displays: its original case for `nFlag` 1, upper-cased otherwise -
    /// what Visual FoxPro answers by default, and the only other form this runtime tells apart.
    fn display_leaf(&self, key: &str, original_case: bool) -> String {
        let leaf = split_folder(key).1.to_string();
        if !original_case {
            return leaf;
        }
        match self.case.get(key) {
            Some(given) => split_folder(given).1.to_string(),
            None => leaf,
        }
    }

    /// Whether the folder a `dir` path names is one this host has: made by MKDIR, holding a
    /// file, or the folder the program is in, which is where a bare name lives.
    fn holds_folder(&self, path: &str) -> bool {
        let (folder, _) = split_folder(path);
        if folder.is_empty() {
            return true;
        }
        let inside = format!("{folder}\\");
        self.dirs.contains(folder) || self.files.keys().any(|k| k.replace('/', "\\").starts_with(&inside))
    }

    /// The name a file is really under: `path` when there is something there, and otherwise the
    /// first of the folders `SET PATH TO` named that has it. Everything is compared upper-cased,
    /// as a Windows file system compares it, and a name nothing answers to comes back as it was
    /// so that what is reported missing is the place the program asked for.
    fn on_path(&self, path: &str, search: &[String]) -> String {
        let key = path.to_ascii_uppercase();
        if self.files.contains_key(&key) || self.tables.contains_key(&key) {
            return key;
        }
        for candidate in search {
            let candidate = candidate.to_ascii_uppercase();
            if self.files.contains_key(&candidate) || self.tables.contains_key(&candidate) {
                return candidate;
            }
        }
        key
    }

    /// A low-level file operation against the files this host keeps in memory.
    ///
    /// The answer is what the real host sends: the value, and the error number FERROR() then
    /// reports. The paths are compared upper-cased, as a Windows file system compares them.
    fn file_op(&mut self, request: &HostRequest) -> Value {
        let HostRequest::FileOp { op, handle, path, target, text, count, offset, whence, search } = request else {
            return Value::Null;
        };
        let key = self.on_path(path, search);
        let reply = |value: Value, errno: f64| array_of(vec![value, Value::number(errno)]);
        match op.as_str() {
            "open" => match self.files.get(&key) {
                Some(_) => {
                    let h = self.next_file_handle;
                    self.next_file_handle += 1;
                    self.file_handles.insert(h, (key, 0));
                    reply(Value::number(f64::from(h)), 0.0)
                }
                None => reply(Value::number(-1.0), 2.0),
            },
            "create" => {
                self.note_case(&key, path);
                self.files.insert(key.clone(), Vec::new());
                if *count != 0.0 {
                    self.readonly.insert(key.clone());
                } else {
                    self.readonly.remove(&key);
                }
                let h = self.next_file_handle;
                self.next_file_handle += 1;
                self.file_handles.insert(h, (key, 0));
                reply(Value::number(f64::from(h)), 0.0)
            }
            "close" => match self.file_handles.remove(handle) {
                Some(_) => reply(Value::Logical(true), 0.0),
                None => reply(Value::Logical(false), 6.0),
            },
            "read" | "gets" => {
                let Some((file, pos)) = self.file_handles.get(handle).cloned() else {
                    return reply(Value::str(""), 6.0);
                };
                let bytes = self.files.get(&file).cloned().unwrap_or_default();
                let rest = &bytes[pos.min(bytes.len())..];
                let take = (*count as usize).min(rest.len());
                let (chunk, advance) = if op == "gets" {
                    // a line ends at the line feed, which is consumed and not returned
                    let line = &rest[..take];
                    match line.iter().position(|&b| b == b'\n') {
                        Some(i) => (line[..i].strip_suffix(b"\r").unwrap_or(&line[..i]).to_vec(), i + 1),
                        None => (line.to_vec(), take),
                    }
                } else {
                    (rest[..take].to_vec(), take)
                };
                self.file_handles.insert(*handle, (file, pos + advance));
                reply(Value::str(chunk.iter().map(|&b| b as char).collect::<String>()), 0.0)
            }
            "write" => {
                let Some((file, pos)) = self.file_handles.get(handle).cloned() else {
                    return reply(Value::number(0.0), 6.0);
                };
                if self.readonly.contains(&file) {
                    return reply(Value::number(0.0), 5.0);
                }
                let data: Vec<u8> = text.chars().map(|c| (c as u32 & 0xff) as u8).collect();
                let bytes = self.files.entry(file.clone()).or_default();
                if bytes.len() < pos {
                    bytes.resize(pos, 0);
                }
                bytes.truncate(pos);
                bytes.extend_from_slice(&data);
                self.file_handles.insert(*handle, (file, pos + data.len()));
                reply(Value::number(data.len() as f64), 0.0)
            }
            "seek" => {
                let Some((file, pos)) = self.file_handles.get(handle).cloned() else {
                    return reply(Value::number(-1.0), 6.0);
                };
                let len = self.files.get(&file).map_or(0, Vec::len) as i64;
                let base = match whence {
                    1 => pos as i64,
                    2 => len,
                    _ => 0,
                };
                let at = (base + *offset as i64).max(0) as usize;
                self.file_handles.insert(*handle, (file, at));
                reply(Value::number(at as f64), 0.0)
            }
            "eof" => {
                let Some((file, pos)) = self.file_handles.get(handle).cloned() else {
                    return reply(Value::Logical(true), 6.0);
                };
                let len = self.files.get(&file).map_or(0, Vec::len);
                reply(Value::Logical(pos >= len), 0.0)
            }
            "flush" => reply(Value::Logical(self.file_handles.contains_key(handle)), 0.0),
            "chsize" => {
                let Some((file, _)) = self.file_handles.get(handle).cloned() else {
                    return reply(Value::number(-1.0), 6.0);
                };
                let bytes = self.files.entry(file).or_default();
                bytes.resize(*count as usize, 0);
                reply(Value::number(*count), 0.0)
            }
            "exists" => reply(Value::Logical(self.files.contains_key(&key)), 0.0),
            "copy" => match self.files.get(&key).cloned() {
                Some(bytes) => {
                    self.note_case(&target.to_ascii_uppercase(), target);
                    self.files.insert(target.to_ascii_uppercase(), bytes);
                    reply(Value::Null, 0.0)
                }
                None => reply(Value::Null, 2.0),
            },
            "rename" => match self.files.remove(&key) {
                Some(bytes) => {
                    self.note_case(&target.to_ascii_uppercase(), target);
                    self.files.insert(target.to_ascii_uppercase(), bytes);
                    reply(Value::Null, 0.0)
                }
                None => reply(Value::Null, 2.0),
            },
            "mkdir" => {
                self.note_case(&key.replace('/', "\\"), &path.replace('/', "\\"));
                self.dirs.insert(key.replace('/', "\\"));
                reply(Value::Null, 0.0)
            }
            "rmdir" => {
                self.dirs.remove(&key.replace('/', "\\"));
                reply(Value::Null, 0.0)
            }
            "dir" if !self.holds_folder(&key) => reply(Value::Null, 2.0),
            "dir" => {
                // the mask is a glob over the files held in the folder the path names; cAttribute
                // carries the letters ADIR()'s third argument was given, and `D` among them
                // brings the folder's own subdirectories into the same listing - `.` and `..`
                // besides - mixed in with the files rather than listed after them. An empty mask
                // matches no file at all, which is how "just the subdirectories" is asked for.
                // `nFlag` rides in `whence`, which a listing has no other use for: 1 asks for the
                // name as it was actually given rather than upper-cased, as `ADIR()`'s fourth
                // argument does.
                let (folder, mask) = split_folder(&key);
                let attrs = target.to_ascii_uppercase();
                let original_case = *whence == 1;
                // sorted by the name every entry is held under here, which is always upper-case,
                // so the display case `nFlag` asks for never moves an entry out of order
                let mut entries: Vec<(String, Value)> = Vec::new();
                if !mask.is_empty() {
                    for name in self.files.keys() {
                        let (where_, leaf) = split_folder(name);
                        if where_ == folder && glob(mask.as_bytes(), leaf.as_bytes()) {
                            let size = self.files[name].len() as f64;
                            entries.push((
                                leaf.to_string(),
                                array_of(vec![
                                    Value::str(self.display_leaf(name, original_case)),
                                    Value::number(size),
                                    Value::Date(Some(self.now.0)),
                                    Value::str("12:30:00"),
                                    Value::str(".A..."),
                                ]),
                            ));
                        }
                    }
                }
                if attrs.contains('D') {
                    for dot in [".", ".."] {
                        entries.push((
                            dot.to_string(),
                            array_of(vec![
                                Value::str(dot.to_string()),
                                Value::number(0.0),
                                Value::Date(Some(self.now.0)),
                                Value::str("12:30:00"),
                                Value::str("....D"),
                            ]),
                        ));
                    }
                    for dir in self.dirs.clone() {
                        let (where_, leaf) = split_folder(&dir);
                        if where_ == folder {
                            entries.push((
                                leaf.to_string(),
                                array_of(vec![
                                    Value::str(self.display_leaf(&dir, original_case)),
                                    Value::number(0.0),
                                    Value::Date(Some(self.now.0)),
                                    Value::str("12:30:00"),
                                    Value::str("....D"),
                                ]),
                            ));
                        }
                    }
                }
                entries.sort_by(|a, b| a.0.cmp(&b.0));
                let rows = entries.into_iter().map(|(_, row)| row).collect();
                reply(array_of(rows), 0.0)
            }
            // FULLPATH answers in upper case, as the product does; this host's default
            // directory is C:\WORK, which is as good a one as any for a test
            "fullpath" => reply(
                Value::str(
                    if path.contains(':') { path.clone() } else { format!("C:\\WORK\\{path}") }.to_ascii_uppercase(),
                ),
                0.0,
            ),
            "diskspace" => reply(Value::number(1_000_000_000.0), 0.0),
            "drivetype" => reply(Value::number(3.0), 0.0),
            "locfile" => match self.files.contains_key(&key) {
                true => reply(Value::str(path), 0.0),
                false => reply(Value::str(""), 2.0),
            },
            _ => reply(Value::Null, 31.0),
        }
    }

    /// Default answer for a request (also applies its effect to the object tree).
    pub fn default_answer(&mut self, req: &HostRequest) -> Value {
        match req {
            HostRequest::SetProp { obj, name, value } => {
                // a form's Class, a page frame's PageWidth, a text box's Text: the product works
                // these out and refuses a program's write rather than quietly taking it
                if let Some(code) = self.read_only(Handle(*obj), name) {
                    self.refused =
                        Some(RtError::new(code, format!("{} is a read-only property", name.to_ascii_uppercase())));
                    return Value::Null;
                }
                self.set_prop(Handle(*obj), name, value.to_value());
                // the program wrote it, so it now holds something other than what its class
                // started it at: AMEMBERS()' "changed"
                if let Some(o) = self.objects.get_mut(obj) {
                    o.written.insert(name.to_ascii_uppercase());
                }
                Value::Null
            }
            // `obj.aProp[2] = v`: the object owns the array, so the element is written where the
            // property holds it rather than in a copy
            HostRequest::SetPropIndex { obj, name, index, value } => {
                if let Some(Value::Array(a)) = self.prop(Handle(*obj), name).map(|v| v.deref()) {
                    let subs: Vec<usize> = index.iter().map(|i| *i as usize).collect();
                    if let Err(e) = a.borrow_mut().set(&subs, value.to_value()) {
                        self.refused = Some(e);
                    }
                }
                Value::Null
            }
            // `DIMENSION obj.aProp[2, 3]`: what the property already holds is regrown, so the
            // elements that still fit stay where they are, as the product keeps them
            HostRequest::DimProp { obj, name, rows, cols } => {
                let upper = name.to_ascii_uppercase();
                let grown = match self.prop(Handle(*obj), name).map(|v| v.deref()) {
                    Some(Value::Array(a)) => {
                        let mut copy = a.borrow().clone();
                        copy.redim(*rows as usize, *cols as usize);
                        copy
                    }
                    // only a property that was declared with subscripts can be sized; the
                    // product refuses the rest in these words rather than making an array
                    Some(_) => {
                        self.refused = Some(RtError::new(232, format!("'{upper}' is not an array.")));
                        return Value::Null;
                    }
                    None => {
                        self.refused = Some(RtError::new(1734, format!("Property {upper} is not found.")));
                        return Value::Null;
                    }
                };
                self.set_prop(Handle(*obj), name, Value::Array(std::rc::Rc::new(std::cell::RefCell::new(grown))));
                Value::Null
            }
            HostRequest::ReleaseObject { obj } => {
                self.release(Handle(*obj));
                Value::Null
            }
            // the report listener answers a few of its own; the rest of a method call is Null
            HostRequest::CallMethod { obj, name, args } => match name.to_ascii_uppercase().as_str() {
                // every Visual FoxPro object answers to these, so a golden may write
                // `o.AddProperty("Caption", "x")` the way real source does rather than the
                // function form. The real host does the same in `objectModel.ts`.
                "ADDPROPERTY" => {
                    let Some(name) = args.first().map(JsonValue::to_value).and_then(|v| v.as_str().ok().map(|s| s.to_string())) else {
                        return Value::Logical(false);
                    };
                    let value = args.get(1).map_or(Value::Logical(false), JsonValue::to_value);
                    self.add_property(Handle(*obj), &name, value)
                }
                // the product refuses this one at run time, in these words and with this
                // number: CloneObject is the designer copying what it is designing, and a
                // program that is already running is not that. The real host says the same.
                "CLONEOBJECT" => {
                    self.refused =
                        Some(RtError::new(1953, "Feature is only available if the object is in design mode."));
                    Value::Null
                }
                "GETPAGEHEIGHT" => self.prop(Handle(*obj), "PAGEHEIGHT").unwrap_or(Value::number(0.0)),
                "GETPAGEWIDTH" => self.prop(Handle(*obj), "PAGEWIDTH").unwrap_or(Value::number(0.0)),
                "SUPPORTSLISTENERTYPE" => {
                    let kind = args.first().map(JsonValue::to_value).and_then(|v| v.as_number().ok()).unwrap_or(-1.0);
                    Value::Logical((0.0..=3.0).contains(&kind))
                }
                "INCLUDEPAGEINOUTPUT" => Value::Logical(true),
                "CANCELREPORT" => {
                    self.set_prop(Handle(*obj), "OUTPUTPAGECOUNT", Value::number(0.0));
                    Value::Null
                }
                // a container really does grow a child, so that ControlCount and reaching the
                // child by name answer the way they do in the product
                "ADDOBJECT" | "NEWOBJECT" => {
                    let text = |i: usize| args.get(i).map(JsonValue::to_value).and_then(|v| v.as_str().ok().map(|s| s.to_string()));
                    match (text(0), text(1)) {
                        // the product upper-cases the name a control is added under, which is
                        // what the new control then answers for its own Name
                        (Some(name), Some(class)) => {
                            let added = self.add_object(&class, &name.to_ascii_uppercase(), Some(Handle(*obj)));
                            // AddObject adds a control hidden: the product's own answer for one
                            // it has just added is Visible .F., whatever the class says, and a
                            // program shows it when it has finished setting it up. A control a
                            // form file is built from is not added this way and keeps its own.
                            if let Some(o) = self.objects.get_mut(&added.0)
                                && (o.props.contains_key("VISIBLE") || o.computed.iter().any(|n| n == "VISIBLE"))
                            {
                                o.props.insert("VISIBLE".to_string(), Value::Logical(false));
                            }
                            Value::Null
                        }
                        _ => Value::Null,
                    }
                }
                "REMOVEOBJECT" => {
                    let name = args.first().map(JsonValue::to_value).and_then(|v| v.as_str().ok().map(|s| s.to_string()));
                    if let Some(child) = name.and_then(|n| self.find(Handle(*obj), &n)) {
                        self.release(child);
                    }
                    Value::Null
                }
                _ => Value::Null,
            },
            HostRequest::MessageBox { .. } => Value::number(1.0),
            HostRequest::InputBox { .. } | HostRequest::GetFile { .. } | HostRequest::PutFile { .. } => Value::str(""),
            // the plain file requests share the same in-memory files as the low-level ones
            HostRequest::FileRead { path, search } => match self.files.get(&self.on_path(path, search)) {
                Some(bytes) => Value::str(bytes.iter().map(|&b| b as char).collect::<String>()),
                None => Value::str(""),
            },
            HostRequest::FileExists { path, search } => {
                let name = self.on_path(path, search);
                Value::Logical(self.files.contains_key(&name) || self.tables.contains_key(&name))
            }
            HostRequest::FileWrite { path, text, append } => {
                let data: Vec<u8> = text.chars().map(|c| (c as u32 & 0xff) as u8).collect();
                self.note_case(&path.to_ascii_uppercase(), path);
                let entry = self.files.entry(path.to_ascii_uppercase()).or_default();
                if !*append {
                    entry.clear();
                }
                entry.extend_from_slice(&data);
                Value::number(data.len() as f64)
            }
            HostRequest::FileDelete { path } => {
                let name = path.to_ascii_uppercase();
                let gone = self.files.remove(&name).is_some() || self.tables.remove(&name).is_some();
                Value::Logical(gone)
            }
            HostRequest::FileOp { .. } => self.file_op(req),
            // SET CLASSLIB TO: nothing here reads a `.vcx`, so what the mock host keeps is the
            // list the product reports - a file that is not on hand is refused the way the
            // product refuses one, and the answer is what SET("CLASSLIB") is to say.
            HostRequest::LoadClassLib { files, alias, additive } => {
                if !*additive {
                    self.class_libraries.clear();
                }
                for file in files {
                    let key = file.to_ascii_uppercase();
                    if !self.files.contains_key(&key) {
                        self.refused = Some(RtError::file_not_found(file));
                        return Value::Null;
                    }
                    let name = match alias.is_empty() {
                        true => key.rsplit(['/', '\\']).next().unwrap_or(&key).rsplit_once('.').map_or(key.clone(), |(s, _)| s.to_string()),
                        false => alias.to_ascii_uppercase(),
                    };
                    if !self.class_libraries.iter().any(|(f, _)| *f == key) {
                        self.class_libraries.push((key, name));
                    }
                }
                Value::str(
                    self.class_libraries
                        .iter()
                        .map(|(file, name)| format!("\"{file}\" ALIAS {name}"))
                        .collect::<Vec<_>>()
                        .join(", "),
                )
            }
            HostRequest::CreateObject { class, definition, module, .. } => {
                // NEWOBJECT() naming a file it cannot find is refused before any class is looked
                // for, which is the half of it this host can answer without reading a `.vcx`.
                if !module.is_empty() && !self.files.contains_key(&module.to_ascii_uppercase()) {
                    self.refused = Some(RtError::file_not_found(module));
                    return Value::Null;
                }
                let h = self.add_object(class, &Self::base_class_name(class), None);
                if let Some(def) = definition {
                    // a class of the program's own stands on a base class, and starts out with
                    // what that base class holds before its own declaration goes over the top
                    self.seed_base_class(h, &def.base_class);
                    self.apply_class_def(h, def);
                }
                Value::Object(h)
            }
            HostRequest::DataOpen { path, search, .. } => match self.tables.get(&self.on_path(path, search)) {
                Some(bytes) => {
                    let handle = self.open.len() as u32 + 1;
                    let bytes = bytes.clone();
                    self.open.push(bytes.clone());
                    self.open_paths.push(self.on_path(path, search));
                    let bytes = &bytes;
                    array_of(vec![Value::number(f64::from(handle)), byte_array(&bytes[..header_len(bytes)])])
                }
                None => array_of(vec![Value::number(-1.0), Value::Null]),
            },
            HostRequest::DataCreate { path, header, memo } => {
                if *memo {
                    self.memo_files.insert(path.to_ascii_uppercase(), empty_memo_file());
                }
                let mut bytes = header.clone();
                bytes.push(0x1A);
                let name = path.to_ascii_uppercase();
                // a handle already open on that name reads the new file, as a file handle does
                for (i, open) in self.open_paths.iter().enumerate() {
                    if *open == name {
                        self.open[i] = bytes.clone();
                    }
                }
                self.tables.insert(name, bytes);
                Value::Null
            }
            HostRequest::DataRead { handle, first, count } => {
                let Some(bytes) = self.open.get(*handle as usize - 1) else {
                    return Value::Null;
                };
                let header = header_len(bytes);
                let record_len = u16::from_le_bytes([bytes[10], bytes[11]]) as usize;
                let start = header + (*first as usize - 1) * record_len;
                let end = (start + *count as usize * record_len).min(bytes.len());
                byte_array(bytes.get(start..end).unwrap_or_default())
            }
            HostRequest::DataWrite { handle, recno, bytes, count } => {
                // the write lands in the file held here, so a test can read back what it wrote,
                // and in the table it was opened from, so opening it again sees it too
                let path = self.open_paths.get(*handle as usize - 1).cloned();
                if let Some(file) = self.open.get_mut(*handle as usize - 1) {
                    write_record(file, *recno as usize, bytes, *count);
                }
                if let Some(file) = path.and_then(|p| self.tables.get_mut(&p)) {
                    write_record(file, *recno as usize, bytes, *count);
                }
                Value::Null
            }
            HostRequest::FileWriteBytes { path, bytes } => {
                self.note_case(&path.to_ascii_uppercase(), path);
                self.files.insert(path.to_ascii_uppercase(), bytes.clone());
                Value::Null
            }
            HostRequest::FileReadBytes { path } => {
                let name = path.to_ascii_uppercase();
                match self.files.get(&name).or_else(|| self.tables.get(&name)) {
                    Some(bytes) => byte_array(bytes),
                    None => Value::str(""),
                }
            }
            HostRequest::DataReadMemo { handle, block } => {
                let file = self.open_paths.get(*handle as usize - 1).and_then(|p| self.memo_files.get(p));
                let (Some(file), block) = (file, *block as usize) else { return Value::str("") };
                let size = block_size(file);
                let start = block * size;
                if block == 0 || start + 8 > file.len() {
                    return Value::str("");
                }
                let len = u32::from_be_bytes([file[start + 4], file[start + 5], file[start + 6], file[start + 7]]);
                let end = (start + 8 + len as usize).min(file.len());
                byte_array(&file[start..end])
            }
            HostRequest::DataWriteMemo { handle, bytes } => {
                let Some(path) = self.open_paths.get(*handle as usize - 1).cloned() else {
                    return Value::number(0.0);
                };
                let file = self.memo_files.entry(path).or_insert_with(empty_memo_file);
                let size = block_size(file);
                let block = file.len() / size;
                // a block opens with its type and the length of what follows, both big-endian
                let mut written = Vec::with_capacity(8 + bytes.len());
                written.extend_from_slice(&1u32.to_be_bytes());
                written.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
                written.extend_from_slice(bytes);
                while written.len() % size != 0 {
                    written.push(0);
                }
                file.extend_from_slice(&written);
                let next = (file.len() / size) as u32;
                file[0..4].copy_from_slice(&next.to_be_bytes());
                Value::number(block as f64)
            }
            HostRequest::DataIndex { handle } => match self.open_paths.get(*handle as usize - 1).and_then(|p| self.indexes.get(p)) {
                Some(bytes) => byte_array(bytes),
                None => Value::str(""),
            },
            HostRequest::DataWriteIndex { handle, bytes } => {
                if let Some(path) = self.open_paths.get(*handle as usize - 1).cloned() {
                    if bytes.is_empty() {
                        self.indexes.remove(&path);
                    } else {
                        self.indexes.insert(path.clone(), bytes.clone());
                    }
                    // byte 28 of the header says a structural index sits beside the table
                    let flag = u8::from(!bytes.is_empty());
                    if let Some(file) = self.open.get_mut(*handle as usize - 1) {
                        file[28] = flag;
                    }
                    if let Some(file) = self.tables.get_mut(&path) {
                        file[28] = flag;
                    }
                }
                Value::Null
            }
            HostRequest::SetScreen { screen } => {
                self.screen = Some(screen.clone());
                Value::Null
            }
            HostRequest::ReadGets { fields } => match self.reads.pop_front() {
                // nothing scripted: the user left every field as it was
                None => array_of(fields.iter().map(|f| f.value.to_value()).collect()),
                Some(values) => array_of(values),
            },
            HostRequest::ChooseFrom { .. } => Value::number(self.choices.pop_front().unwrap_or(0.0)),
            HostRequest::Printers { choose } => {
                if *choose {
                    Value::str(self.printers.first().cloned().unwrap_or_default())
                } else {
                    array_of(self.printers.iter().map(|p| Value::str(p.clone())).collect())
                }
            }
            HostRequest::DataWriteHeader { handle, bytes } => {
                if let Some(file) = self.open.get_mut(*handle as usize - 1) {
                    let end = bytes.len().min(file.len());
                    file[..end].copy_from_slice(&bytes[..end]);
                    let path = self.open_paths[*handle as usize - 1].clone();
                    let copy = file.clone();
                    self.tables.insert(path, copy);
                }
                Value::Null
            }
            // a test host runs nothing and has nothing to catch up on
            HostRequest::MousePress { .. } => Value::Null,
            HostRequest::EditMemo { text, .. } => Value::str(text.clone()),
            HostRequest::Environment { name } => Value::str(std::env::var(name).unwrap_or_default()),
            HostRequest::Enumerate { .. } => Value::Null,
            // Where the runtime was installed, and the folders beside it. A directory always
            // ends in a separator, which is the part a program leans on; 4 is Visual FoxPro's
            // samples tree, which nothing here has.
            HostRequest::HomeDir { which } => Value::str(match which {
                4 => String::new(),
                _ => "C:\\WORK\\".to_string(),
            }),
            // a test host builds nothing and compiles nothing; both are recorded by the
            // request log the mock keeps, which is what the golden programs look at
            HostRequest::Build { .. } | HostRequest::Compile { .. } => Value::Null,
            HostRequest::GetObject { .. } => Value::Null,
            HostRequest::RunLine { .. } => Value::Null,
            HostRequest::NewDocument { .. } => Value::Null,
            HostRequest::CallParentMethod { .. } => Value::Logical(true),
            // no data source to reach, which is what a connection that cannot be made answers
            HostRequest::Sql { .. } => Value::number(-1.0),
            HostRequest::CloseMemo { .. } => Value::Null,
            HostRequest::RunProgram { .. } => Value::number(0.0),
            HostRequest::Settle { .. } => Value::Null,
            HostRequest::DataClose { .. } => Value::Null,
            // LOADPICTURE()'s answer is a bag of four, with the picture itself left in the VM
            HostRequest::MakePicture { picture, of_kind, width, height } => {
                let h = self.add_object("Empty", "Picture", None);
                for (name, value) in
                    [("HANDLE", picture), ("TYPE", of_kind), ("WIDTH", width), ("HEIGHT", height)]
                {
                    self.set_prop(h, name, Value::number(*value));
                }
                Value::Object(h)
            }
            HostRequest::AddProperty { obj, name, value } => self.add_property(Handle(*obj), name, value.to_value()),
            HostRequest::RemoveProperty { obj, name } => {
                let gone = self
                    .objects
                    .get_mut(obj)
                    .is_some_and(|o| o.props.remove(&name.to_ascii_uppercase()).is_some());
                Value::Logical(gone)
            }
            // The binding table is the host's, so the mock keeps one too, and answers the
            // numbers Visual FoxPro answers: how many delegates the event now has, and how many
            // bindings an unbind took away.
            HostRequest::BindEvent { source, event, handler, delegate, .. } => {
                let one = (*source, event.to_ascii_uppercase(), *handler, delegate.to_ascii_uppercase());
                if !self.bindings.contains(&one) {
                    self.bindings.push(one.clone());
                }
                let bound = self.bindings.iter().filter(|(s, e, _, _)| *s == one.0 && *e == one.1).count();
                Value::number(bound as f64)
            }
            HostRequest::UnbindEvent { source, event, handler, delegate } => {
                let before = self.bindings.len();
                let matches = |s: &u32, e: &String, h: &u32, d: &String| {
                    source.is_none_or(|want| want == *s)
                        && event.as_ref().is_none_or(|want| want.eq_ignore_ascii_case(e))
                        && handler.is_none_or(|want| want == *h)
                        && delegate.as_ref().is_none_or(|want| want.eq_ignore_ascii_case(d))
                };
                self.bindings.retain(|(s, e, h, d)| !matches(s, e, h, d));
                Value::number((before - self.bindings.len()) as f64)
            }
            // nothing here runs a method, so the event reaches no delegate; what it answers is
            // still the logical the product answers
            HostRequest::RaiseEvent { .. } => Value::Logical(true),
            HostRequest::CreateException { code, message, program, line, user_value } => {
                let h = self.add_object("Exception", "Exception", None);
                self.set_prop(h, "ERRORNO", Value::number(f64::from(*code)));
                self.set_prop(h, "MESSAGE", Value::str(message.clone()));
                self.set_prop(h, "PROCEDURE", Value::str(program.clone()));
                self.set_prop(h, "LINENO", Value::number(f64::from(*line)));
                self.set_prop(h, "LINECONTENTS", Value::str(""));
                self.set_prop(h, "DETAILS", Value::str(""));
                self.set_prop(h, "STACKLEVEL", Value::number(1.0));
                self.set_prop(h, "USERVALUE", user_value.to_value());
                Value::Object(h)
            }
            _ => Value::Null,
        }
    }
}

impl Host for MockHost {
    fn mouse(&mut self) -> (f64, f64, bool) {
        self.pointer
    }
    fn get_prop(&mut self, obj: Handle, name: &str) -> Result<Value, RtError> {
        let o = self.objects.get(&obj.0).ok_or_else(RtError::object_not_valid)?;
        let upper = name.to_ascii_uppercase();
        if upper == "PARENT" {
            return Ok(o.parent.map(Value::Object).unwrap_or(Value::Null));
        }
        if upper == "CONTROLCOUNT" {
            return Ok(Value::number(o.children.len() as f64));
        }
        if upper == "APPLICATION" {
            return Ok(Value::Object(self.application()));
        }
        o.props.get(&upper).cloned().ok_or_else(|| RtError::property_not_found(name))
    }

    fn get_member(&mut self, obj: Handle, name: &str) -> Result<Member, RtError> {
        let o = self.objects.get(&obj.0).ok_or_else(RtError::object_not_valid)?;
        let upper = name.to_ascii_uppercase();
        if let Some(c) =
            o.children.iter().find(|c| self.objects.get(&c.0).is_some_and(|co| co.name.eq_ignore_ascii_case(name)))
        {
            return Ok(Member::Child(*c));
        }
        if upper == "PARENT"
            || upper == "CONTROLCOUNT"
            || upper == "APPLICATION"
            || o.props.contains_key(&upper)
            || o.computed.contains(&upper)
        {
            return Ok(Member::Property);
        }
        if o.methods.contains(&upper) {
            return Ok(Member::Method);
        }
        Ok(Member::None)
    }

    fn members(&mut self, obj: Handle) -> Option<Vec<MemberInfo>> {
        let o = self.objects.get(&obj.0)?;
        // "native" means the Visual FoxPro base class the object stands on declares it; a class
        // of the program's own is what everything else came from
        let base = o
            .props
            .get("BASECLASS")
            .and_then(|v| v.as_str().ok().map(|s| s.to_string()))
            .unwrap_or_else(|| o.class.clone());
        let shape = crate::base_classes::find(&base);
        let declares = |name: &str| shape.is_some_and(|b| b.properties.iter().any(|(n, _)| n == name));
        let is_event = |name: &str| shape.is_some_and(|b| b.events.iter().any(|e| e == name));
        let read_only = |name: &str| shape.is_some_and(|b| b.read_only.iter().any(|(n, _)| n == name));

        let mut out: Vec<MemberInfo> = Vec::new();
        for (name, value) in &o.props {
            out.push(MemberInfo {
                name: name.clone(),
                kind: crate::host::MemberKind::Property,
                native: declares(name),
                added: o.added.contains(name),
                read_only: read_only(name),
                changed: o.written.contains(name),
                value: Some(value.clone()),
            });
        }
        // the members whose value is worked out rather than stored answer just the same
        let computed = o.computed.clone();
        for name in &computed {
            let value = self.get_prop(obj, name).ok();
            out.push(MemberInfo {
                name: name.clone(),
                kind: crate::host::MemberKind::Property,
                native: true,
                added: false,
                read_only: read_only(name),
                changed: false,
                value,
            });
        }
        let o = self.objects.get(&obj.0)?;
        for name in &o.methods {
            out.push(MemberInfo {
                name: name.clone(),
                kind: if is_event(name) { crate::host::MemberKind::Event } else { crate::host::MemberKind::Method },
                native: shape.is_some_and(|b| b.events.iter().chain(b.methods.iter()).any(|m| m == name)),
                added: false,
                read_only: false,
                changed: false,
                value: None,
            });
        }
        for child in &o.children {
            let Some(c) = self.objects.get(&child.0) else { continue };
            out.push(MemberInfo {
                name: c.name.to_ascii_uppercase(),
                kind: crate::host::MemberKind::Object,
                native: false,
                added: false,
                read_only: false,
                changed: false,
                value: Some(Value::Object(*child)),
            });
        }
        Some(out)
    }

    fn object_class(&mut self, obj: Handle) -> Option<String> {
        self.objects.get(&obj.0).map(|o| o.class.clone())
    }

    fn output(&mut self, text: &str, newline: bool) {
        if newline || self.output.is_empty() {
            self.output.push(text.to_string());
        } else {
            self.output.last_mut().expect("line").push_str(text);
        }
    }

    fn now(&mut self) -> (i32, f64) {
        self.now
    }

    fn random(&mut self) -> f64 {
        self.rng.next_number()
    }

    fn seed_random(&mut self, seed: f64) {
        self.rng = crate::host::SeededRandom::from_seed(seed);
    }

    fn resolve_program(&mut self, name: &str) -> Option<u32> {
        self.programs.get(&name.to_ascii_uppercase()).copied()
    }
}

/// Drives `fiber` to completion. Requests are recorded in `host.requests` and answered from
/// `answers` (front first) or, when empty, by `MockHost::default_answer`.
pub fn run_with_requests(
    vm: &mut Vm,
    host: &mut MockHost,
    fiber: FiberId,
    answers: &mut VecDeque<Value>,
) -> Result<Value, RtError> {
    loop {
        match vm.step(host, fiber) {
            Step::Done { value, .. } => return Ok(value),
            Step::Error(e) => return Err(e),
            Step::Suspend(req) => {
                // a program the host holds the source of is compiled and loaded on request
                if let HostRequest::LoadProgram { name } = &req
                    && let Some(src) = host.program_sources.get(&name.to_ascii_uppercase()).cloned()
                {
                    let upper = name.to_ascii_uppercase();
                    let module = compiler::compile_program(&src, &upper.to_ascii_lowercase())
                        .module
                        .ok_or_else(|| RtError::syntax(format!("{upper} does not compile")))?;
                    let id = vm.load_module(module);
                    host.programs.insert(upper, id);
                    host.requests.push(req);
                    vm.resume(fiber, Value::number(id as f64));
                    continue;
                }
                let answer = match answers.pop_front() {
                    Some(v) => {
                        // Scripted answers still apply property writes so later reads see them.
                        if let HostRequest::SetProp { obj, name, value } = &req {
                            host.set_prop(Handle(*obj), name, value.to_value());
                        }
                        v
                    }
                    None => host.default_answer(&req),
                };
                host.requests.push(req);
                match host.refused.take() {
                    Some(e) => vm.resume_error(fiber, e),
                    None => vm.resume(fiber, answer),
                }
            }
        }
    }
}

/// Compiles and runs a program to completion with default request answers; returns the VM and
/// the captured `?` output. Compile errors become `RtError::syntax` with the diagnostic's line.
pub fn run_program(src: &str, host: &mut MockHost) -> Result<(Vm, Vec<String>), RtError> {
    let result = compiler::compile_program(src, "main");
    let Some(module) = result.module else {
        let d = result.diagnostics.iter().find(|d| d.is_error()).expect("error diagnostic");
        let mut e = RtError::syntax(d.message.clone());
        e.line = d.line;
        e.program = "main".into();
        return Err(e);
    };
    let mut vm = Vm::new();
    let id = vm.load_module(module);
    let fiber = vm.start(id, 0, None, Vec::new());
    let mut answers = VecDeque::new();
    run_with_requests(&mut vm, host, fiber, &mut answers)?;
    Ok((vm, host.output.clone()))
}

impl JsonValue {
    /// Convenience for tests: a request argument list as values.
    pub fn values(list: &[JsonValue]) -> Vec<Value> {
        list.iter().map(JsonValue::to_value).collect()
    }
}

/// Bytes as the host sends them: an array of numbers.
fn byte_array(bytes: &[u8]) -> Value {
    array_of(bytes.iter().map(|b| Value::number(f64::from(*b))).collect())
}

fn array_of(items: Vec<Value>) -> Value {
    Value::Array(std::rc::Rc::new(std::cell::RefCell::new(crate::value::FoxArray::of(items))))
}

/// One record back into a file held in memory, and the count in its header with it.
fn write_record(file: &mut Vec<u8>, recno: usize, bytes: &[u8], count: Option<f64>) {
    let header = header_len(file);
    let record_len = u16::from_le_bytes([file[10], file[11]]) as usize;
    let start = header + (recno - 1) * record_len;
    if start + bytes.len() > file.len() {
        file.resize(start + bytes.len(), 0x1A);
    }
    file[start..start + bytes.len()].copy_from_slice(bytes);
    if let Some(n) = count {
        file[4..8].copy_from_slice(&(n as u32).to_le_bytes());
    }
}

/// A memo file with nothing in it: a 512-byte header saying the next block is the one after.
fn empty_memo_file() -> Vec<u8> {
    let mut head = vec![0u8; 512];
    head[0..4].copy_from_slice(&8u32.to_be_bytes());
    head[6..8].copy_from_slice(&64u16.to_be_bytes());
    head
}

/// The block size a memo file records at offset 6.
fn block_size(file: &[u8]) -> usize {
    match u16::from_be_bytes([file[6], file[7]]) {
        0 => 512,
        n => n as usize,
    }
}

/// The header length a DBF records at offset 8.
fn header_len(bytes: &[u8]) -> usize {
    bytes.get(8..10).map_or(0, |b| u16::from_le_bytes([b[0], b[1]]) as usize)
}

/// A file-name glob: a star is any run of characters, a question mark one; case does not count.
/// A path split into the folder it is in and the name inside it, either separator accepted.
fn split_folder(path: &str) -> (&str, &str) {
    match path.rfind(['\\', '/']) {
        Some(at) => (&path[..at], &path[at + 1..]),
        None => ("", path),
    }
}

fn glob(mask: &[u8], name: &[u8]) -> bool {
    let (mask, name): (Vec<u8>, Vec<u8>) = (mask.to_ascii_uppercase(), name.to_ascii_uppercase());
    let (mut m, mut n) = (0usize, 0usize);
    let (mut star, mut mark) = (usize::MAX, 0usize);
    while n < name.len() {
        if m < mask.len() && (mask[m] == b'?' || mask[m] == name[n]) {
            m += 1;
            n += 1;
        } else if m < mask.len() && mask[m] == b'*' {
            star = m;
            mark = n;
            m += 1;
        } else if star != usize::MAX {
            m = star + 1;
            mark += 1;
            n = mark;
        } else {
            return false;
        }
    }
    mask[m..].iter().all(|&c| c == b'*')
}

/// The shape in a property name that carries subscripts: `aPoly[1,1]`, `aRows(3)`.
///
/// `ADDPROPERTY()` takes the name and the size in one string, and both bracket styles index
/// alike in FoxPro. The second number is 0 for a list of one dimension, which is the convention
/// `FoxArray` uses for its column count.
fn array_subscripts(name: &str) -> Option<(String, usize, usize)> {
    let trimmed = name.trim();
    let open = trimmed.find(['[', '('])?;
    let close = trimmed.rfind([']', ')'])?;
    if close + 1 != trimmed.len() || close <= open {
        return None;
    }
    let mut dims = trimmed[open + 1..close].split(',').map(|d| d.trim().parse::<usize>());
    let rows = dims.next()?.ok()?;
    let cols = match dims.next() {
        None => 0,
        Some(Ok(c)) => c,
        Some(Err(_)) => return None,
    };
    if dims.next().is_some() {
        return None;
    }
    Some((trimmed[..open].trim().to_string(), rows, cols))
}
