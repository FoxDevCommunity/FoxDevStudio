//! Runtime errors. Codes follow Visual FoxPro's error numbers where one exists so `ERROR()`
//! and `MESSAGE()` behave familiarly; the program and line are filled in by the VM.

use serde::{Deserialize, Serialize};

use crate::host::JsonValue;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RtError {
    pub code: u32,
    pub message: String,
    /// Name of the program or method (e.g. `main`, `cmdSayHi.Click`); empty until the VM sets it.
    pub program: String,
    /// 1-based line inside `program`; 0 until the VM sets it.
    pub line: u32,
    /// What `THROW` was given, which the exception object hands back as `UserValue`. The message
    /// says nothing about it - VFP's wording for a thrown error is the same sentence whatever
    /// was thrown - so the value has to travel with the error to reach the CATCH. `.F.` is what
    /// an error the program did not throw leaves there.
    #[serde(default = "user_value_of_an_error_no_one_threw")]
    pub user_value: JsonValue,
}

fn user_value_of_an_error_no_one_threw() -> JsonValue {
    JsonValue::Bool(false)
}

impl RtError {
    /// `ERROR "text"`: a program's own error, as VFP numbers it.
    pub const USER_DEFINED: u32 = 1098;

    /// The sentence Visual FoxPro gives an error number, with `{}` where the name or value the
    /// error is about is written in, and - where the product words the two differently - the
    /// sentence it gives when there is nothing to put there.
    ///
    /// Every one of these was read off the product with `ERROR n` inside a TRY and again by
    /// provoking the error for real. Two things they all have in common: they are sentences, so
    /// they end in a full stop, and the gap stays open when nothing fills it, which is why
    /// `ERROR 1734` says "Property  is not found." with two spaces showing. Three of them close
    /// the gap up instead - `ERROR 12` says "Variable is not found." rather than "Variable ''
    /// is not found." - so those carry both wordings.
    fn words(code: u32) -> Option<(&'static str, Option<&'static str>)> {
        Some(match code {
            Self::FILE_NOT_FOUND => ("File '{}' does not exist.", Some("File does not exist.")),
            Self::DATA_TYPE_MISMATCH => ("Data type mismatch.", None),
            Self::SYNTAX_ERROR => ("Syntax error.", None),
            Self::FUNCTION_ARG_INVALID => ("Function argument value, type, or count is invalid.", None),
            Self::VARIABLE_NOT_FOUND => ("Variable '{}' is not found.", Some("Variable is not found.")),
            Self::ALIAS_NOT_FOUND => ("Alias '{}' is not found.", Some("Alias is not found.")),
            Self::UNRECOGNIZED_VERB => ("Unrecognized command verb.", None),
            Self::NO_ORDER_SET => ("Table has no index order set.", None),
            Self::INVALID_SUBSCRIPT => ("Invalid subscript reference.", None),
            Self::UNRECOGNIZED_PHRASE => ("Command contains unrecognized phrase/keyword.", None),
            Self::ILLEGAL_VALUE => ("Expression evaluated to an illegal value.", None),
            Self::NO_FIELDS => ("No fields found to process.", None),
            Self::NO_TABLE_OPEN => ("No table is open in the current work area.", None),
            Self::TYPE_MISMATCH => ("Operator/operand type mismatch.", None),
            Self::FEATURE_NOT_AVAILABLE => ("Feature is not available.", None),
            Self::USER_DEFINED => ("API function _UserError() was called.", None),
            Self::CANNOT_CREATE_FILE => ("Cannot create file {}.", None),
            Self::PROCEDURE_NOT_FOUND => ("Procedure '{}' is not found.", None),
            Self::API_LIBRARY_NOT_FOUND => ("API library is not found.", None),
            Self::TOO_MANY_ARGS => ("Too many arguments.", None),
            Self::TOO_FEW_ARGS => ("Too few arguments.", None),
            Self::CONNECTION_HANDLE_INVALID => ("Connection handle is invalid.", None),
            Self::SUBSCRIPT_OUT_OF_RANGE => ("Subscript is outside defined range.", None),
            Self::DIVISION_BY_ZERO => ("Cannot divide by 0.", None),
            Self::BUFFERED_TABLE => ("Command cannot be issued on a table with cursors in table buffering mode.", None),
            Self::INSERT_NOT_ALLOWED => (
                "INSERT cannot be issued when row or table buffering is enabled or when integrity constraints are in effect.",
                None,
            ),
            Self::TAG_NOT_FOUND => ("Index tag is not found.", None),
            Self::FIELD_NAME_INVALID => ("Field name is a duplicate or invalid.", None),
            Self::FIELD_WIDTH_INVALID => ("Field width or number of decimal places is invalid.", None),
            Self::PROPERTY_NOT_FOUND => ("Property {} is not found.", None),
            Self::STRING_TOO_LONG => ("String is too long to fit.", None),
            Self::NOT_AN_OBJECT => ("{} is not an object.", None),
            Self::UNKNOWN_MEMBER => ("Unknown member {}.", None),
            Self::OBJECT_NOT_VALID => ("Member {} does not evaluate to an object.", None),
            Self::NOT_OFFLINE_VIEW => ("Object is not an offline view.", None),
            Self::TABLE_IN_DATABASE => ("Cannot add this table: it belongs to database {}.", None),
            Self::USER_THROWN => ("User Thrown Error {}.", None),
            Self::HAVING_INVALID => ("SQL: HAVING clause is invalid.", None),
            Self::GROUP_BY_MISSING => ("SQL: GROUP BY clause is missing or invalid.", None),
            Self::SUBQUERY_INVALID => ("SQL: Invalid use of subquery.", None),
            Self::TOP_INVALID => ("SQL: Invalid TOP specification.", None),
            Self::TOP_NEEDS_ORDER => ("SQL: TOP requires an ORDER BY.", None),
            Self::MISSING_PARAMETER => ("Must specify additional parameters.", None),
            Self::POPUP_NOT_DEFINED => ("Menu has not been defined with DEFINE POPUP.", None),
            _ => return None,
        })
    }

    /// The sentence for an error number with the name or value it is about written into it.
    pub fn message_about(code: u32, about: &str) -> Option<String> {
        Self::words(code).map(|(sentence, _)| sentence.replace("{}", about))
    }

    /// The text VFP gives an error number when there is nothing to say about it, which is what
    /// `ERROR n` raises and `MESSAGE()` reads back. Only the numbers this runtime raises itself
    /// are known by text; any other reads as "Error n".
    pub fn standard_message(code: u32) -> Option<String> {
        Self::words(code).map(|(sentence, alone)| match alone {
            Some(text) => text.to_string(),
            None => sentence.replace("{}", ""),
        })
    }

    /// An error of this number about `about`, worded as the product words it.
    pub fn about(code: u32, about: &str) -> Self {
        let message = Self::message_about(code, about).unwrap_or_else(|| format!("Error {code}"));
        Self::new(code, message)
    }

    pub fn new(code: u32, message: impl Into<String>) -> Self {
        RtError {
            code,
            message: message.into(),
            program: String::new(),
            line: 0,
            user_value: user_value_of_an_error_no_one_threw(),
        }
    }

    // ---- VFP error numbers ----
    pub const FILE_NOT_FOUND: u32 = 1;
    pub const DATA_TYPE_MISMATCH: u32 = 9;
    pub const FUNCTION_ARG_INVALID: u32 = 11;
    pub const VARIABLE_NOT_FOUND: u32 = 12;
    pub const UNRECOGNIZED_VERB: u32 = 16;
    pub const INVALID_SUBSCRIPT: u32 = 31;
    /// "Command contains unrecognized phrase/keyword": a line that will not read as a command,
    /// which is what is left when a macro stood for a name that is not a character variable.
    pub const UNRECOGNIZED_PHRASE: u32 = 36;
    pub const TYPE_MISMATCH: u32 = 107;
    pub const FEATURE_NOT_AVAILABLE: u32 = 1001;
    pub const CANNOT_CREATE_FILE: u32 = 1102;
    pub const PROCEDURE_NOT_FOUND: u32 = 1162;
    /// "Invalid operation for the cursor.": asking a cursor something only another kind answers.
    pub const INVALID_CURSOR_OPERATION: u32 = 1115;
    /// "Cannot add this table: it belongs to database ...": a table is in one database at a time.
    pub const TABLE_IN_DATABASE: u32 = 1537;
    pub const TOO_MANY_ARGS: u32 = 1230;
    /// "Too few arguments.": a function whose reference page shows a later argument as required
    /// even though an earlier one of the same function is optional, so the compiler's own arity
    /// check - which only knows the widest range a name ever takes - lets the call through and
    /// the function itself has to notice. `DISPLAYPATH(cFileName)` and `CREATEOBJECTEX()` are
    /// two this runtime has measured: the reference gives a later argument with no brackets
    /// around it at all. QUARTER() and ALANGUAGE() are two more, registered with a lenient
    /// minimum so the call still compiles and refusing at run time instead.
    pub const TOO_FEW_ARGS: u32 = 1229;
    /// "Subscript is outside defined range": an array read or written past the size it was
    /// dimensioned to, which is a different complaint from a subscript that makes no sense.
    pub const SUBSCRIPT_OUT_OF_RANGE: u32 = 1234;
    pub const DIVISION_BY_ZERO: u32 = 1307;
    /// "File access is denied.": what a database event that refuses to let a table be opened,
    /// a container be opened or a container be packed raises. Measured in Visual FoxPro 9: the
    /// message names the file, and the other refusals a dbc event can make raise nothing.
    pub const FILE_ACCESS_DENIED: u32 = 1705;
    /// "API library is not found.": what `SET LIBRARY TO` says about a .fll it cannot load.
    /// Measured in Visual FoxPro 9: `SET LIBRARY TO nosuchthing.fll` raises 1726, not error 1.
    pub const API_LIBRARY_NOT_FOUND: u32 = 1726;
    pub const PROPERTY_NOT_FOUND: u32 = 1734;
    /// "SQL: HAVING clause is invalid.": a HAVING that names a column which is neither one of
    /// the GROUP BY keys nor inside an aggregate. Measured in Visual FoxPro 9: the select list
    /// holding that column makes no difference, and a name given by AS loses to a field of the
    /// same name.
    pub const HAVING_INVALID: u32 = 1803;
    /// "SQL: GROUP BY clause is missing or invalid.": measured, what an aggregate in a HAVING
    /// clause raises when the query has neither a GROUP BY nor an aggregate of its own.
    pub const GROUP_BY_MISSING: u32 = 1807;
    /// "SQL: Invalid use of subquery.": measured in Visual FoxPro 9, what a subquery written in
    /// a HAVING clause raises. The same subquery in that query's WHERE clause is accepted.
    pub const SUBQUERY_INVALID: u32 = 1810;
    /// "SQL: Invalid TOP specification.": measured in Visual FoxPro 9, `TOP 0`, and any
    /// percentage that is not more than nothing and less than the whole - `TOP 0 PERCENT`,
    /// `TOP 100 PERCENT` and `TOP 150 PERCENT` are all this error, while `TOP 0.05 PERCENT`
    /// and `TOP 99.9 PERCENT` are fine.
    pub const TOP_INVALID: u32 = 1866;
    /// "SQL: TOP requires an ORDER BY.": TOP says which rows of an order, so there has to be one.
    pub const TOP_NEEDS_ORDER: u32 = 1867;
    pub const NOT_AN_OBJECT: u32 = 1924;
    /// "Expression is not valid outside of WITH/ENDWITH.": a `.member` with nothing open.
    pub const OUTSIDE_WITH: u32 = 1940;
    /// "Cannot redefine THISFORM.": something assigned to the word naming what it runs in.
    pub const CANNOT_REDEFINE: u32 = 1930;
    pub const UNKNOWN_MEMBER: u32 = 1925;
    pub const OBJECT_NOT_VALID: u32 = 1943;
    pub const USER_THROWN: u32 = 2071;
    /// "Alias 'x' is not found": SELECT or an alias-qualified field naming a closed work area.
    /// "Tag ... does not exist": SET ORDER or SEEK named a tag the index has not got.
    pub const TAG_NOT_FOUND: u32 = 1683;
    /// "Table has no index order set": SEEK with nothing to seek down.
    pub const NO_ORDER_SET: u32 = 26;
    pub const ALIAS_NOT_FOUND: u32 = 13;
    /// "Alias name is already in use.": a result asked to land under the name of an open table,
    /// which is what `SELECT ... FROM orders INTO CURSOR orders` asks for - the query's own
    /// source is still open when the result arrives.
    pub const ALIAS_IN_USE: u32 = 24;
    /// "Field <name> does not accept null values.": `.NULL.` into a column not declared NULL.
    pub const NULL_REFUSED: u32 = 1581;
    /// "Table is not open": a command that needs a table where the work area is empty.
    pub const NO_TABLE_OPEN: u32 = 52;
    pub const SYNTAX_ERROR: u32 = 10;
    /// "Expression evaluated to an illegal value": a setting given a number or a character
    /// outside what it takes, such as SET FDOW TO 8.
    pub const ILLEGAL_VALUE: u32 = 46;
    /// "String is too long to fit": SET NULLDISPLAY past the fifteen characters it keeps.
    pub const STRING_TOO_LONG: u32 = 1903;
    /// "No fields found to process": `CREATE CURSOR ... FROM ARRAY` given an array that is not
    /// wide enough to describe a field - fewer than the four columns AFIELDS() starts with.
    pub const NO_FIELDS: u32 = 47;
    /// "Field name is a duplicate or invalid".
    pub const FIELD_NAME_INVALID: u32 = 1712;
    /// "Field width or number of decimal places is invalid".
    pub const FIELD_WIDTH_INVALID: u32 = 1713;
    /// "INSERT cannot be issued ...": what VFP answers `INSERT [BEFORE] [BLANK]` with when the
    /// table has an index open. Moving every record down one would leave every entry of every
    /// tag pointing at the wrong record, so the product refuses rather than rebuild them.
    pub const INSERT_NOT_ALLOWED: u32 = 1588;
    /// "Command cannot be issued on a table with cursors in table buffering mode".
    pub const BUFFERED_TABLE: u32 = 1579;
    /// "Object is not an offline view": `USE ... ONLINE` or `ADMIN` on anything else.
    pub const NOT_OFFLINE_VIEW: u32 = 2009;
    /// "OLE error code ...": what the product answers when something it handed to Windows came
    /// back a failure. Bytes that are not a picture are the case here.
    pub const OLE_ERROR: u32 = 1426;
    /// "Must specify additional parameters.": an argument that only means anything alongside
    /// another was given on its own - `TXTWIDTH(text, cFont)` with no size to go with the font.
    pub const MISSING_PARAMETER: u32 = 94;
    /// "Menu has not been defined with DEFINE POPUP.": `BARPROMPT()` asked about a bar of a
    /// popup that is not there - named and wrong, or left out with none active either.
    pub const POPUP_NOT_DEFINED: u32 = 165;
    /// "Connection handle is invalid": ASQLHANDLES() asked about a specific SQL connection
    /// statement handle that is not one of the handles currently open - measured, every value
    /// errors this way when nothing has ever called SQLCONNECT().
    pub const CONNECTION_HANDLE_INVALID: u32 = 1466;

    /// An error of this number with nothing to say about anything in particular.
    fn plain(code: u32) -> Self {
        Self::new(code, Self::standard_message(code).unwrap_or_else(|| format!("Error {code}")))
    }

    pub fn type_mismatch() -> Self {
        Self::plain(Self::TYPE_MISMATCH)
    }
    pub fn data_type_mismatch() -> Self {
        Self::plain(Self::DATA_TYPE_MISMATCH)
    }
    pub fn function_arg_invalid() -> Self {
        Self::plain(Self::FUNCTION_ARG_INVALID)
    }
    pub fn missing_parameter() -> Self {
        Self::plain(Self::MISSING_PARAMETER)
    }
    pub fn popup_not_defined() -> Self {
        Self::plain(Self::POPUP_NOT_DEFINED)
    }
    pub fn variable_not_found(name: &str) -> Self {
        Self::about(Self::VARIABLE_NOT_FOUND, &name.to_ascii_uppercase())
    }
    pub fn invalid_subscript() -> Self {
        Self::plain(Self::INVALID_SUBSCRIPT)
    }
    pub fn division_by_zero() -> Self {
        Self::plain(Self::DIVISION_BY_ZERO)
    }
    pub fn property_not_found(name: &str) -> Self {
        Self::about(Self::PROPERTY_NOT_FOUND, &name.to_ascii_uppercase())
    }
    pub fn unknown_member(name: &str) -> Self {
        Self::about(Self::UNKNOWN_MEMBER, &name.to_ascii_uppercase())
    }
    pub fn not_an_object(name: &str) -> Self {
        Self::about(Self::NOT_AN_OBJECT, &name.to_ascii_uppercase())
    }
    pub fn object_not_valid() -> Self {
        Self::plain(Self::OBJECT_NOT_VALID)
    }
    pub fn procedure_not_found(name: &str) -> Self {
        Self::about(Self::PROCEDURE_NOT_FOUND, &name.to_ascii_uppercase())
    }
    pub fn too_many_args() -> Self {
        Self::plain(Self::TOO_MANY_ARGS)
    }
    pub fn too_few_args() -> Self {
        Self::plain(Self::TOO_FEW_ARGS)
    }
    pub fn connection_handle_invalid() -> Self {
        Self::plain(Self::CONNECTION_HANDLE_INVALID)
    }
    /// The product has every feature, so its own wording has nothing to name; what this runtime
    /// has not written yet says which one after the sentence, because a person reading the
    /// message needs to know that much.
    pub fn feature_not_available(what: &str) -> Self {
        Self::new(Self::FEATURE_NOT_AVAILABLE, format!("Feature is not available: {what}"))
    }
    pub fn file_not_found(path: &str) -> Self {
        Self::about(Self::FILE_NOT_FOUND, path)
    }
    pub fn cannot_create_file(path: &str) -> Self {
        Self::about(Self::CANNOT_CREATE_FILE, path)
    }
    pub fn illegal_value() -> Self {
        Self::plain(Self::ILLEGAL_VALUE)
    }
    pub fn no_table_open() -> Self {
        Self::plain(Self::NO_TABLE_OPEN)
    }
    pub fn no_order_set() -> Self {
        Self::plain(Self::NO_ORDER_SET)
    }
    pub fn tag_not_found() -> Self {
        Self::plain(Self::TAG_NOT_FOUND)
    }
    pub fn alias_not_found(name: &str) -> Self {
        Self::about(Self::ALIAS_NOT_FOUND, &name.to_ascii_uppercase())
    }
    pub fn subscript_out_of_range() -> Self {
        Self::plain(Self::SUBSCRIPT_OUT_OF_RANGE)
    }
    /// A line that will not read as a command, which is what a macro that stood for nothing
    /// leaves behind.
    pub fn unrecognized_phrase() -> Self {
        Self::plain(Self::UNRECOGNIZED_PHRASE)
    }
    pub fn string_too_long() -> Self {
        Self::plain(Self::STRING_TOO_LONG)
    }
    pub fn syntax(message: impl Into<String>) -> Self {
        Self::new(Self::SYNTAX_ERROR, message)
    }
    /// `THROW <value>`: the sentence is always the same one, and the value the program threw is
    /// carried alongside it for the exception object's `UserValue`.
    pub fn user_thrown(user_value: JsonValue) -> Self {
        let mut e = Self::plain(Self::USER_THROWN);
        e.user_value = user_value;
        e
    }
}

impl std::fmt::Display for RtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error {}: {}", self.code, self.message)
    }
}

impl std::error::Error for RtError {}
