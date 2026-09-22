//! Work areas and cursors: FoxPro's table state, on the VM side of the host/VM split.
//!
//! A cursor never holds a file. It holds the table's header, a window of raw record bytes it last
//! asked the host for, and the record it is sitting on, decoded. That shape is what keeps field
//! reads cheap: moving the record pointer may cost a round trip to the host, but reading `custno`
//! afterwards is a lookup in memory, so an expression that touches ten fields costs nothing extra.
//!
//! Records are numbered from 1 and counted in `u64`. Visual FoxPro stops at 2 GB per table
//! because it does this arithmetic in signed 32-bit; the point of the split is not to.

use std::collections::HashMap;

use crate::cdx::Tag;
use crate::dbf::{DbfHeader, DbfRecord, DbfValue, OwnedMemoBlock, Padding, decode_record};
use crate::error::RtError;
use crate::value::Value;

/// How many records one `DataRead` asks for. A page is one copy across the bridge, so a SCAN over
/// a table costs a request per page rather than per record; 64 records of a 300-byte table is a
/// 19 KB block, which is small enough not to matter when only one record is wanted.
pub const PAGE_RECORDS: u32 = 64;

/// Work areas, numbered from 1 as VFP numbers them, plus which one is selected.
#[derive(Debug, Default)]
pub struct DataSession {
    areas: Vec<Option<Cursor>>,
    current: usize,
}

impl DataSession {
    pub fn new() -> DataSession {
        DataSession { areas: Vec::new(), current: 0 }
    }

    /// The selected work area, 1-based, as `SELECT()` reports it.
    pub fn current_area(&self) -> usize {
        self.current + 1
    }

    pub fn cursor(&self) -> Option<&Cursor> {
        self.areas.get(self.current).and_then(Option::as_ref)
    }

    pub fn cursor_mut(&mut self) -> Option<&mut Cursor> {
        self.areas.get_mut(self.current).and_then(Option::as_mut)
    }

    /// The cursor in a work area given by number (1-based) or alias.
    pub fn find(&self, what: &AreaRef) -> Option<&Cursor> {
        match what {
            AreaRef::Number(n) => self.areas.get(n.checked_sub(1)?).and_then(Option::as_ref),
            AreaRef::Alias(name) => self.areas.iter().flatten().find(|c| c.alias.eq_ignore_ascii_case(name)),
        }
    }

    /// The 1-based work area a table alias is open in, for `SELECT(cTableAlias)`. 0 when
    /// nothing answers to that alias, which is what Visual FoxPro answers too.
    pub fn area_of_alias(&self, name: &str) -> usize {
        self.areas
            .iter()
            .position(|a| a.as_ref().is_some_and(|c| c.alias.eq_ignore_ascii_case(name)))
            .map_or(0, |i| i + 1)
    }

    pub fn find_mut(&mut self, what: &AreaRef) -> Option<&mut Cursor> {
        match what {
            AreaRef::Number(n) => self.areas.get_mut(n.checked_sub(1)?).and_then(Option::as_mut),
            AreaRef::Alias(name) => self.areas.iter_mut().flatten().find(|c| c.alias.eq_ignore_ascii_case(name)),
        }
    }

    /// Selects a work area, creating it if it has never been used. `SELECT 0` picks the lowest
    /// unused one, which is how a program opens a table without caring where it lands.
    pub fn select(&mut self, what: &AreaRef) -> Result<usize, RtError> {
        let index = match what {
            AreaRef::Number(0) => self.lowest_free(),
            AreaRef::Number(n) => n - 1,
            AreaRef::Alias(name) => {
                let named = |wanted: &str| {
                    self.areas.iter().position(|a| a.as_ref().is_some_and(|c| c.alias.eq_ignore_ascii_case(wanted)))
                };
                // A table's alias is the stem of the file it was opened from, so the file's own
                // name finds it: `CREATE TABLE (lcName)` opens it as its stem and the
                // `INSERT INTO (lcName)` that follows hands over the whole path again.
                named(name)
                    .or_else(|| named(&crate::vm::stem_of(name)))
                    .ok_or_else(|| RtError::alias_not_found(name))?
            }
        };
        self.ensure(index);
        self.current = index;
        Ok(index + 1)
    }

    /// Puts a freshly opened cursor in the selected area, replacing whatever was there.
    /// Returns the handle of the table that was closed, if any, so the caller can release it.
    pub fn install(&mut self, cursor: Cursor) -> Option<u32> {
        let index = self.current;
        self.ensure(index);
        self.areas[index].replace(cursor).and_then(|old| old.handle())
    }

    /// `USE` with no name: empties the selected area.
    pub fn close_current(&mut self) -> Option<u32> {
        let index = self.current;
        self.areas.get_mut(index).and_then(Option::take).and_then(|c| c.handle())
    }

    /// Empties every area and reports the handles to release.
    pub fn close_all(&mut self) -> Vec<u32> {
        let handles = self.areas.iter_mut().filter_map(Option::take).filter_map(|c| c.handle()).collect();
        self.current = 0;
        handles
    }

    /// True when a work area, given by number or alias, holds a table.
    pub fn used(&self, what: &AreaRef) -> bool {
        self.find(what).is_some()
    }

    fn lowest_free(&self) -> usize {
        self.areas.iter().position(Option::is_none).unwrap_or(self.areas.len())
    }

    /// `SELECT(1)`: the highest work area not in use, which is where VFP puts a scratch cursor.
    pub fn highest_free(&self) -> usize {
        self.areas.iter().rposition(Option::is_some).map_or(1, |i| i + 2)
    }

    fn ensure(&mut self, index: usize) {
        if index >= self.areas.len() {
            self.areas.resize_with(index + 1, || None);
        }
    }
}

/// How a statement names a work area: `SELECT 2`, `SELECT customer`, `customer.custno`.
#[derive(Debug, Clone, PartialEq)]
pub enum AreaRef {
    Number(usize),
    Alias(String),
}

/// Where a cursor's records come from.
#[derive(Debug)]
pub enum Source {
    /// A file the host holds open; records arrive a page at a time and `handle` names it.
    Host { handle: u32 },
    /// Rows the VM built - a query result - which are simply there. Nothing is read from disk,
    /// and closing one throws the rows away rather than telling the host anything.
    Memory { rows: Vec<DbfRecord> },
}

/// One open table.
#[derive(Debug)]
pub struct Cursor {
    source: Source,
    /// The name this table answers to: the file stem, or whatever `USE ... ALIAS` said.
    pub alias: String,
    /// The table as `USE` named it, for `DBF()`.
    pub path: String,
    pub header: DbfHeader,
    /// Record the pointer sits on. 0 before the first (BOF), `count + 1` after the last (EOF).
    recno: u64,
    /// Records the table held when it was opened.
    count: u64,
    /// The decoded record at `recno`, or `None` at BOF/EOF where there is no record.
    record: Option<DbfRecord>,
    /// Raw bytes of the page last read, and the record number it starts at.
    page: Vec<u8>,
    page_first: u64,
    /// Memo blocks fetched for the page in hand, by block number. Cleared with the page, so
    /// this never grows past what one page of records points at.
    memos: HashMap<u32, OwnedMemoBlock>,
    /// The block a `DataReadMemo` is out for, so the reply knows what it is.
    pending_memo: Option<u32>,
    /// The first record of the page a `DataRead` is out for.
    pending_page: Option<u64>,
    /// Where a movement command had got to when it had to stop and ask for a page.
    walk: Option<Walk>,
    /// What FOUND() reports: did the last LOCATE or CONTINUE on this table find anything.
    found: bool,
    /// What `SET KEY TO` narrowed the table to: only the records whose controlling index key
    /// matches this value are reached by walking. Measured against vfp9.exe, which uses the
    /// first expression the command names and nothing else - see `set_key_limit`.
    key_limit: Option<Value>,
    /// An outer join has parked this table with no matching record, so every field of it reads
    /// as .NULL. until the pointer moves again. Reading past the last record is otherwise the
    /// empty value of each field's type, which is what the rest of the language sees.
    outer_miss: bool,
    /// The record has been changed and not yet written back.
    dirty: bool,
    /// The change added a record, so the header's count has to go back with it.
    grew: bool,
    /// The tags of the compound index beside the table, once the host has sent it, and before
    /// them the tag of every single-entry index open beside it. A table whose index has not
    /// been asked for yet has none, which is not the same as having none.
    tags: Vec<Tag>,
    /// The files of the single-entry indexes `USE ... INDEX` and `SET INDEX TO` opened, in the
    /// order they were named. Visual FoxPro numbers them before the tags of the compound
    /// index, so the first `idx.len()` of `tags` are theirs, one each.
    idx: Vec<String>,
    /// True once the index has been asked for, so it is read at most once per open table.
    index_asked: bool,
    /// The controlling order: which tag decides what "the next record" means. None is record
    /// order, which is what a table opens in.
    order: Option<usize>,
    /// Where each record sits in that order, so SKIP is a step rather than a search.
    positions: HashMap<u32, usize>,
    /// `SET ORDER TO tag DESCENDING` walks a tag the other way without changing the index.
    order_desc: Option<bool>,
    /// A tag has changed since the index was read, so it has to be written back.
    index_dirty: bool,
    /// `SET FILTER TO`: the condition a record has to pass to be seen, as it was written.
    filter: String,
    /// `SET RELATION TO`: an expression and the work area it is looked up in, one per relation.
    relations: Vec<Relation>,
    /// CURSORSETPROP("Buffering"): 1 writes go straight through, 3 the record in hand is
    /// held until the pointer moves or TABLEUPDATE, 5 every changed record is held until then.
    /// 2 and 4 are the pessimistic pair, which lock as well as hold.
    buffering: u8,
    /// The records changed but not written yet, by record number.
    held: std::collections::BTreeMap<u64, HeldRecord>,
    /// What each changed record was before it was changed, for OLDVAL() and an updategram.
    originals: std::collections::BTreeMap<u64, Vec<u8>>,
    /// An autoincrementing field has moved on, so the header has to be written again.
    header_changed: bool,
    /// The fields written to since the record was last sent on or held.
    touched: Vec<usize>,
    /// Memo text written to the record but not yet given a block by the host, by field.
    memo_writes: std::collections::VecDeque<(usize, Vec<u8>)>,
    /// The field a `DataWriteMemo` is out for, so the block that comes back goes in the right
    /// place.
    memo_writing: Option<usize>,
    /// FLOCK(): the whole table is locked.
    file_locked: bool,
    /// RLOCK(): the records that are locked.
    locked: std::collections::HashSet<u32>,
}

impl Cursor {
    pub fn new(handle: u32, alias: String, path: String, header: DbfHeader) -> Cursor {
        Cursor::over(Source::Host { handle }, alias, path, header)
    }

    /// A cursor over rows already in hand: the result of a query, and what INTO CURSOR makes.
    pub fn in_memory(alias: String, fields: Vec<crate::dbf::DbfField>, rows: Vec<DbfRecord>) -> Cursor {
        // a real layout, not a placeholder: RECSIZE() reads it, and so does encoding a row to
        // bytes for `current_bytes()` - which buffering, GETFLDSTATE() and an updategram all
        // need even though a query result never otherwise touches its rows as bytes
        let mut pos = 1usize;
        let layout: Vec<(usize, usize)> = fields
            .iter()
            .map(|f| {
                let start = pos;
                pos += f.length as usize;
                (start, f.length as usize)
            })
            .collect();
        let mut header = crate::dbf::DbfHeader {
            version: 0x30,
            layout,
            fields,
            record_count: rows.len() as u64,
            header_len: 0,
            record_len: pos,
            codepage: None,
            has_index: false,
            last_update: None,
            // a cursor holds its rows rather than bytes, so there is nowhere to keep a flag bit
            null_flags: None,
        };
        header.record_count = rows.len() as u64;
        let mut cursor = Cursor::over(Source::Memory { rows }, alias, String::new(), header);
        // the rows are already here, so the pointer can rest on the first one straight away
        cursor.seek(1);
        cursor
    }

    fn over(source: Source, alias: String, path: String, header: DbfHeader) -> Cursor {
        let count = header.record_count;
        // VFP opens a table at record 1, which for an empty one is already end of file
        Cursor {
            source,
            alias,
            path,
            header,
            recno: 1,
            count,
            record: None,
            page: Vec::new(),
            page_first: 0,
            memos: HashMap::new(),
            pending_memo: None,
            pending_page: None,
            walk: None,
            found: false,
            key_limit: None,
            outer_miss: false,
            dirty: false,
            grew: false,
            tags: Vec::new(),
            idx: Vec::new(),
            index_asked: false,
            order: None,
            positions: HashMap::new(),
            order_desc: None,
            index_dirty: false,
            filter: String::new(),
            relations: Vec::new(),
            buffering: 1,
            held: std::collections::BTreeMap::new(),
            originals: std::collections::BTreeMap::new(),
            header_changed: false,
            touched: Vec::new(),
            memo_writes: std::collections::VecDeque::new(),
            memo_writing: None,
            file_locked: false,
            locked: std::collections::HashSet::new(),
        }
    }

    // ----- indexes ---------------------------------------------------------------------------
    //
    // A tag is a list of keys with the records they belong to, in the order the index sorts them.
    // Making it the controlling order means the table is walked down that list instead of down
    // the file, which is all "ordered" means here: SKIP steps to the next entry, GO TOP is the
    // first one, and RECNO() still answers with the record the entry points at.

    /// The tags of the compound index beside this table.
    pub fn tags(&self) -> &[Tag] {
        &self.tags
    }

    /// The tags of the compound index alone, which is what is written back to the `.cdx`.
    pub fn cdx_tags(&self) -> &[Tag] {
        &self.tags[self.idx.len()..]
    }

    /// The files of the single-entry indexes open beside the table, in the order they were
    /// opened, which is how NDX() numbers them.
    pub fn idx_files(&self) -> &[String] {
        &self.idx
    }

    /// Opens one beside the table, as `USE ... INDEX` and `SET INDEX TO` do: its tag goes in
    /// before the compound index's, which is the order Visual FoxPro numbers them in.
    pub fn open_idx(&mut self, file: String, tag: Tag) -> usize {
        // the same file opened again, or built again by INDEX ON ... TO, is the one that is
        // already there rather than a second index of the same name
        if let Some(at) = self.idx.iter().position(|f| f.eq_ignore_ascii_case(&file)) {
            self.idx[at] = file;
            self.tags[at] = tag;
            let order = self.order;
            self.set_order(order);
            return at;
        }
        let at = self.idx.len();
        self.idx.push(file);
        self.tags.insert(at, tag);
        // the tags of the compound index have all moved along one
        let order = self.order.map(|i| if i >= at { i + 1 } else { i });
        self.set_order(order);
        at
    }

    /// Closes every one of them, which is what `SET INDEX TO` with nothing after it does. The
    /// structural compound index stays open, as it does there.
    pub fn close_idx(&mut self) {
        let gone = self.idx.len();
        self.tags.drain(..gone);
        self.idx.clear();
        let order = self.order.and_then(|i| i.checked_sub(gone));
        self.set_order(order);
    }

    /// Files the index the host sent. The controlling order is dropped: the tag it named may
    /// not be there any more.
    pub fn set_tags(&mut self, tags: Vec<Tag>) {
        self.tags.truncate(self.idx.len());
        self.tags.extend(tags);
        self.index_asked = true;
        self.set_order(None);
    }

    /// True once the index beside the table has been asked for, whether or not there was one.
    pub fn index_asked(&self) -> bool {
        self.index_asked
    }

    pub fn mark_index_asked(&mut self) {
        self.index_asked = true;
    }

    /// The tag ordering the table, if any.
    pub fn order(&self) -> Option<&Tag> {
        self.order.and_then(|i| self.tags.get(i))
    }

    /// Which tag that is, numbered from 1 as TAGNO() and SET ORDER TO n do.
    pub fn order_no(&self) -> Option<usize> {
        self.order.map(|i| i + 1)
    }

    /// Puts the table in the order of a tag, given by its position in the index, or back in
    /// record order.
    pub fn set_order(&mut self, which: Option<usize>) {
        self.set_order_way(which, None);
    }

    /// The same, with the direction the command asked for: SET ORDER can walk a tag backwards
    /// without the index itself changing.
    pub fn set_order_way(&mut self, which: Option<usize>, descending: Option<bool>) {
        self.order = which.filter(|i| *i < self.tags.len());
        self.order_desc = descending;
        self.positions.clear();
        // a descending tag is stored in ascending order and walked backwards, so the position
        // of a record is counted from the other end
        let down = self.descending();
        let places: Vec<(u32, usize)> = match self.order() {
            Some(tag) => {
                let n = tag.entries.len();
                tag.entries.iter().enumerate().map(|(i, e)| (e.recno, if down { n - i } else { i + 1 })).collect()
            }
            None => Vec::new(),
        };
        self.positions.extend(places);
    }

    /// The tag of that name, by position, as SET ORDER and SEEK name one.
    pub fn tag_index(&self, name: &str) -> Option<usize> {
        self.tags.iter().position(|t| t.name.eq_ignore_ascii_case(name))
    }

    /// True when a tag decides the order of the records.
    pub fn ordered(&self) -> bool {
        self.order().is_some()
    }

    /// How many entries the controlling order holds. Positions run from 1 to this.
    pub fn order_len(&self) -> usize {
        self.order().map_or(0, |t| t.entries.len())
    }

    /// Which way the controlling order runs: the tag's own direction unless SET ORDER said.
    pub fn descending(&self) -> bool {
        self.order_desc.unwrap_or_else(|| self.order().is_some_and(|t| t.descending))
    }

    /// A tag of this index, replacing one of the same name. Answers with where it went.
    pub fn put_tag(&mut self, tag: Tag) -> usize {
        self.index_dirty = true;
        // a single-entry index of the same name is a different file and is left where it is
        match self.tags.iter().skip(self.idx.len()).position(|t| t.name.eq_ignore_ascii_case(&tag.name)) {
            Some(i) => {
                self.tags[self.idx.len() + i] = tag;
                self.idx.len() + i
            }
            None => {
                self.tags.push(tag);
                self.tags.len() - 1
            }
        }
    }

    /// Drops a tag of the compound index, or every one of them, and lets go of the order when
    /// it was the one dropped. A single-entry index is closed rather than deleted.
    pub fn drop_tags(&mut self, name: Option<&str>) {
        let open = self.idx.len();
        match name {
            Some(name) => {
                let mut at = 0usize;
                self.tags.retain(|t| {
                    let keep = at < open || !t.name.eq_ignore_ascii_case(name);
                    at += 1;
                    keep
                });
            }
            None => self.tags.truncate(open),
        }
        self.index_dirty = true;
        let order = self.order;
        self.set_order(order.filter(|i| *i < self.tags.len()));
    }

    /// REINDEX: the index is written back from the entries in hand.
    pub fn put_tag_dirty(&mut self) {
        self.index_dirty = !self.cdx_tags().is_empty();
    }

    // ----- buffering -------------------------------------------------------------------------
    //
    // A buffered table holds what was written to it rather than sending it on: the record is
    // changed where the program can see it, and the bytes wait here until TABLEUPDATE sends them
    // or TABLEREVERT throws them away. That is the whole of what buffering is - the rest is
    // which records are held, which is what the buffering mode says.

    pub fn buffering(&self) -> u8 {
        self.buffering
    }

    /// Sets the buffering mode. Turning it off leaves what is already held to be written or
    /// reverted, as VFP does: a mode change is not a write.
    pub fn set_buffering(&mut self, mode: u8) {
        self.buffering = mode.clamp(1, 5);
    }

    /// True when a write is to be held rather than sent on.
    pub fn buffered(&self) -> bool {
        self.buffering >= 2
    }

    /// Keeps what a record was before it is changed, so `OLDVAL()` and an updategram can say.
    ///
    /// A change goes straight into the page - that is what the program reads back - so the only
    /// moment the record as the file has it can be kept is before the first change to it. It is
    /// kept for every record a change touches; what is not held when the write goes through is
    /// dropped with it.
    pub fn remember_original(&mut self) {
        let recno = self.recno;
        if self.originals.contains_key(&recno) || self.held.contains_key(&recno) {
            return;
        }
        // `current_bytes()` rather than `stored()`: nothing is held for this record yet, so for
        // a table on disk the two answer the same page bytes, and a cursor with rows in memory -
        // which `stored()` has no page to answer from at all - gets an original to remember too
        if let Some(bytes) = self.current_bytes() {
            self.originals.insert(recno, bytes);
        }
    }

    /// What that record was, or nothing when it was never changed.
    pub fn original_bytes(&self, recno: u64) -> Option<&[u8]> {
        self.originals.get(&recno).map(Vec::as_slice)
    }

    /// Holds the record in hand: its bytes as they are now, and which fields were changed.
    pub fn hold(&mut self, fields: &[usize], appended: bool) {
        let recno = self.recno;
        let Some(bytes) = self.current_bytes() else { return };
        let entry = self.held.entry(recno).or_insert_with(|| HeldRecord {
            bytes: bytes.clone(),
            fields: Vec::new(),
            appended,
            was: self.count,
        });
        entry.bytes = bytes;
        entry.appended = entry.appended || appended;
        for f in fields {
            if !entry.fields.contains(f) {
                entry.fields.push(*f);
            }
        }
        self.dirty = false;
        self.grew = false;
    }

    // ----- memo fields -----------------------------------------------------------------------
    //
    // A record holds the block a memo's text lives at, so writing a memo is two writes: the text
    // goes to the file beside the table and the block it landed at goes in the record. The text
    // waits here in between.

    /// The next memo the record is waiting to write, and the field it belongs to.
    pub fn next_memo_write(&mut self) -> Option<(usize, Vec<u8>)> {
        let next = self.memo_writes.pop_front()?;
        self.memo_writing = Some(next.0);
        Some(next.clone())
    }

    /// The block the host wrote the text at, into the record, and into what the program reads.
    pub fn memo_written(&mut self, block: u32) -> Result<(), RtError> {
        let Some(index) = self.memo_writing.take() else { return Ok(()) };
        let Some(field) = self.header.fields.get(index).cloned() else { return Ok(()) };
        let (start, width) = self.header.layout[index];
        let bytes = crate::dbf::write::encode_field(&field, width, &Value::number(f64::from(block)), None)?;
        if let Some(record) = self.record_bytes_mut() {
            let end = start + bytes.len();
            if end <= record.len() {
                record[start..end].copy_from_slice(&bytes);
            }
        }
        let recno = self.recno;
        self.seek(recno);
        Ok(())
    }

    /// True while a memo's text is with the host and its block has not come back.
    pub fn awaiting_memo(&self) -> bool {
        self.memo_writing.is_some()
    }

    /// The text a memo field holds while it waits for a block, so a read gives it back.
    pub fn pending_memo(&self, index: usize) -> Option<&[u8]> {
        self.memo_writes.iter().find(|(i, _)| *i == index).map(|(_, b)| b.as_slice())
    }

    /// The fields written to since the record was last sent on, and a clean slate after.
    pub fn take_touched(&mut self) -> Vec<usize> {
        std::mem::take(&mut self.touched)
    }

    /// Lets go of the page of records in hand, so the next read fetches it again. This is
    /// what REFRESH() does: a record is fresh when it has been read since.
    pub fn forget_page(&mut self) {
        self.page.clear();
        self.page_first = 0;
        self.memos.clear();
        self.record = None;
    }

    /// The records held, oldest first, with their record numbers.
    pub fn held(&self) -> impl Iterator<Item = (u64, &HeldRecord)> {
        self.held.iter().map(|(n, r)| (*n, r))
    }

    /// GETNEXTMODIFIED(): the first held record after `recno`, or 0 when there is none.
    pub fn next_modified(&self, recno: u64) -> u64 {
        self.held.range(recno + 1..).map(|(n, _)| *n).next().unwrap_or(0)
    }

    /// GETFLDSTATE(): 1 unchanged, 2 changed, 3 the deletion flag changed, 4 a new record's
    /// field. `index` is the field's position from 1; 0 asks about the record itself.
    pub fn field_state(&self, recno: u64, index: usize) -> u8 {
        let Some(held) = self.held.get(&recno) else { return 1 };
        if held.appended {
            return 4;
        }
        if index == 0 {
            return if held.fields.is_empty() { 3 } else { 2 };
        }
        if held.fields.contains(&(index - 1)) { 2 } else { 1 }
    }

    /// SETFLDSTATE(): says a field was changed, or was not, without changing what is in it.
    pub fn set_field_state(&mut self, recno: u64, index: usize, state: u8) {
        let Some(held) = self.held.get_mut(&recno) else { return };
        if index == 0 {
            return;
        }
        let field = index - 1;
        match state {
            1 => held.fields.retain(|f| *f != field),
            _ => {
                if !held.fields.contains(&field) {
                    held.fields.push(field);
                }
            }
        }
    }

    /// The first record still being held, taken out of the buffer: one write is one round
    /// trip, so the caller asks for them one at a time.
    pub fn take_one_held(&mut self) -> Option<(u64, HeldRecord)> {
        let first = *self.held.keys().next()?;
        self.held.remove_entry(&first)
    }

    /// TABLEUPDATE: the records that were held are on their way; this is what is left to send.
    pub fn take_held(&mut self, recno: Option<u64>) -> Vec<(u64, HeldRecord)> {
        match recno {
            Some(n) => self.held.remove(&n).map(|r| vec![(n, r)]).unwrap_or_default(),
            None => std::mem::take(&mut self.held).into_iter().collect(),
        }
    }

    /// TABLEREVERT: what was held goes, and the records go back to what the table holds. The
    /// records that were appended go with it, which is what shortens the table again.
    pub fn revert(&mut self, recno: Option<u64>) -> u64 {
        let held = self.take_held(recno);
        match recno {
            Some(n) => self.originals.remove(&n),
            None => {
                self.originals.clear();
                None
            }
        };
        let appended = held.iter().filter(|(_, r)| r.appended).count() as u64;
        self.count -= appended.min(self.count);
        self.page = Vec::new();
        self.page_first = 0;
        self.memos.clear();
        self.record = None;
        self.dirty = false;
        self.grew = false;
        let here = self.recno.min(self.count.max(1));
        self.seek(here);
        held.len() as u64
    }

    /// The record a buffered table holds for that record number, when it holds one.
    pub fn held_bytes(&self, recno: u64) -> Option<&[u8]> {
        self.held.get(&recno).map(|r| r.bytes.as_slice())
    }

    // ----- locks -----------------------------------------------------------------------------
    //
    // One program has a table to itself here, so a lock is always granted; what the functions
    // report is what this program has asked for, which is what a program that locks a record,
    // writes it and unlocks it needs to see.

    /// FLOCK(): locks the whole table.
    pub fn lock_file(&mut self) -> bool {
        self.file_locked = true;
        true
    }

    /// RLOCK(): locks a record, or the one the pointer is on.
    pub fn lock_record(&mut self, recno: u64) -> bool {
        if recno == 0 || recno > self.count {
            return false;
        }
        self.locked.insert(recno as u32);
        true
    }

    /// UNLOCK: lets go of one record, or of everything this table has locked.
    pub fn unlock(&mut self, recno: Option<u64>) {
        match recno {
            Some(n) => {
                self.locked.remove(&(n as u32));
            }
            None => {
                self.locked.clear();
                self.file_locked = false;
            }
        }
    }

    /// ISFLOCKED(): whether the whole table is locked.
    pub fn file_locked(&self) -> bool {
        self.file_locked
    }

    /// ISRLOCKED(): whether a record is locked, the pointer's own when none is named.
    pub fn record_locked(&self, recno: u64) -> bool {
        self.file_locked || self.locked.contains(&(recno as u32))
    }

    /// What a field held when the record was read, for `OLDVAL()` and `CURVAL()` alike. A record
    /// being held has the changed value in `self.record`; what the file itself holds - and what
    /// it was read with, since one program has it to itself here - is what this answers.
    pub fn original(&self, name: &str) -> Option<Value> {
        let index = self.header.field_index(name)?;
        let recno = self.recno;
        if !self.held.contains_key(&recno) {
            return self.field(name);
        }
        // what the record was before the change, kept when the change was made
        let raw = self.originals.get(&recno).map(Vec::as_slice).or_else(|| self.stored(recno))?;
        let record = decode_record(&self.header, raw, Padding::Keep, |_| None);
        record.values.get(index).map(value_of)
    }

    /// What the file itself holds for a field, for `CURVAL()` - the page in hand, never the
    /// change a buffered record is holding over it. `field()` answers what the program sees,
    /// which is the held change once there is one; this is what disk still says.

    /// The condition SET FILTER put on the table, empty when there is none.
    pub fn filter(&self) -> &str {
        &self.filter
    }

    pub fn set_filter(&mut self, filter: String) {
        self.filter = filter;
    }

    /// The relations that follow this table's record pointer into other work areas.
    pub fn relations(&self) -> &[Relation] {
        &self.relations
    }

    /// `SET RELATION TO`: one more relation, or - without ADDITIVE - the only one.
    pub fn relate(&mut self, relation: Option<Relation>, additive: bool) {
        if !additive {
            self.relations.clear();
        }
        if let Some(r) = relation {
            self.relations.push(r);
        }
    }

    /// True when a tag has changed since the index was read.
    pub fn index_dirty(&self) -> bool {
        self.index_dirty
    }

    pub fn index_written(&mut self) {
        self.index_dirty = false;
    }

    /// Puts a record back where it belongs in one tag, under the key it now has. `None` takes
    /// it out, which is what a FOR condition that has stopped holding means.
    pub fn reindex_record(&mut self, tag: usize, recno: u32, key: Option<Vec<u8>>) {
        let Some(t) = self.tags.get_mut(tag) else { return };
        let was = t.entries.iter().position(|e| e.recno == recno);
        if let Some(i) = was {
            if t.entries[i].key.as_slice() == key.as_deref().unwrap_or_default() && key.is_some() {
                return;
            }
            t.entries.remove(i);
        }
        if let Some(key) = key {
            let entry = crate::cdx::Entry { key, recno };
            let at = t.entries.partition_point(|e| (&e.key, e.recno) < (&entry.key, entry.recno));
            t.entries.insert(at, entry);
        } else if was.is_none() {
            return;
        }
        self.index_dirty = true;
        let order = self.order;
        let way = self.order_desc;
        self.set_order_way(order, way);
    }

    /// The record at a position in the controlling order, counting from 1.
    pub fn record_at(&self, pos: usize) -> Option<u64> {
        let tag = self.order()?;
        let n = tag.entries.len();
        if pos < 1 || pos > n {
            return None;
        }
        let i = if self.descending() { n - pos } else { pos - 1 };
        Some(u64::from(tag.entries[i].recno))
    }

    /// Where a record sits in the controlling order, or None when the tag does not hold it -
    /// which a FOR condition, or a record added since, can both mean.
    pub fn position_of(&self, recno: u64) -> Option<usize> {
        let recno = u32::try_from(recno).ok()?;
        self.positions.get(&recno).copied()
    }

    /// The key the controlling order holds a record under, for SEEK to compare against.
    pub fn key_at(&self, pos: usize) -> Option<&[u8]> {
        let tag = self.order()?;
        let n = tag.entries.len();
        if pos < 1 || pos > n {
            return None;
        }
        let i = if self.descending() { n - pos } else { pos - 1 };
        Some(&tag.entries[i].key)
    }

    /// The host's name for the file, when there is a file. A query result has none.
    pub fn handle(&self) -> Option<u32> {
        match self.source {
            Source::Host { handle } => Some(handle),
            Source::Memory { .. } => None,
        }
    }

    /// The rows a query result holds, for anything outside this file that reads them whole.
    /// `None` for a table the host has open, whose records arrive a page at a time.
    pub fn memory_rows(&self) -> Option<&[DbfRecord]> {
        self.rows()
    }

    /// Rows a query result holds, for the field and movement paths that read them directly.
    fn rows(&self) -> Option<&[DbfRecord]> {
        match &self.source {
            Source::Memory { rows } => Some(rows),
            Source::Host { .. } => None,
        }
    }

    pub fn recno(&self) -> u64 {
        self.recno
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    /// VFP is at end of file when the pointer is past the last record, and an empty table is
    /// both at the beginning and at the end.
    pub fn eof(&self) -> bool {
        self.recno > self.count
    }

    pub fn bof(&self) -> bool {
        self.recno == 0 || self.count == 0
    }

    pub fn deleted(&self) -> bool {
        self.record.as_ref().is_some_and(|r| r.deleted)
    }

    /// The value of a field of the current record. `None` when the table has no such field;
    /// at BOF or EOF every field reads as empty, which is what VFP does.
    pub fn field(&self, name: &str) -> Option<Value> {
        let index = self.header.field_index(name)?;
        // a memo written a moment ago is here until the host says which block it went to
        if let Some(text) = self.pending_memo(index) {
            return Some(Value::str(crate::dbf::encoding::decode(text, self.header.codepage)));
        }
        let field = &self.header.fields[index];
        let Some(record) = &self.record else {
            return Some(match self.outer_miss {
                true => Value::Null,
                false => shaped_by(field, &empty_for(field.kind)),
            });
        };
        Some(record.values.get(index).map(|v| shaped_by(field, &value_of(v))).unwrap_or(Value::Null))
    }

    pub fn has_field(&self, name: &str) -> bool {
        self.header.field_index(name).is_some()
    }

    /// Where the pointer would land, given a movement. Nothing is read here: the caller compares
    /// this with the loaded page to decide whether it has to ask the host for bytes.
    pub fn target(&self, to: Move) -> u64 {
        match to {
            Move::Top => 1,
            Move::Bottom => self.count.max(1),
            Move::Record(n) => n,
            Move::Skip(delta) => {
                let next = self.recno as i64 + delta;
                next.clamp(0, self.count as i64 + 1) as u64
            }
        }
    }

    /// True when `recno` is inside the page already in hand.
    pub fn page_holds(&self, recno: u64) -> bool {
        if self.rows().is_some() || self.held.contains_key(&recno) {
            return true;
        }
        recno >= self.page_first
            && self.page_first > 0
            && ((recno - self.page_first + 1) as usize) * self.header.record_len <= self.page.len()
    }

    /// The first record of the page that would hold `recno`, so reads align and a SKIP through a
    /// table asks for each page exactly once.
    pub fn page_start(&self, recno: u64) -> u64 {
        let page = (recno.saturating_sub(1)) / PAGE_RECORDS as u64;
        page * PAGE_RECORDS as u64 + 1
    }

    pub fn set_page(&mut self, first: u64, bytes: Vec<u8>) {
        self.page_first = first;
        self.page = bytes;
        self.memos.clear();
    }

    /// Records that a `DataRead` for the page starting at `first` is on its way.
    pub fn expect_page(&mut self, first: u64) {
        self.pending_page = Some(first);
    }

    /// Files the page the host sent for the request `expect_page` announced.
    pub fn accept_page(&mut self, bytes: Vec<u8>) {
        let first = self.pending_page.take().unwrap_or_else(|| self.page_start(self.recno));
        self.set_page(first, bytes);
    }

    /// Whether the record at `recno` is marked deleted, when the page holding it is in hand.
    /// Only the deletion flag is read, so walking past deleted records costs no decoding.
    pub fn deleted_at(&self, recno: u64) -> Option<bool> {
        if let Some(rows) = self.rows() {
            return rows.get(recno.checked_sub(1)? as usize).map(|r| r.deleted);
        }
        self.raw(recno).map(|r| r.first() == Some(&crate::dbf::layout::FLAG_DELETED))
    }

    pub fn set_walk(&mut self, walk: Walk) {
        self.walk = Some(walk);
    }

    pub fn take_walk(&mut self) -> Option<Walk> {
        self.walk.take()
    }

    pub fn found(&self) -> bool {
        self.found
    }

    /// `SET KEY TO eKey`: only the records whose controlling index key matches `eKey` are
    /// reached by walking the table. `None` puts the whole of it back.
    ///
    /// The reference calls the second expression the top of a range. Visual FoxPro was asked
    /// six ways - a range wider than one key, a range whose top is another record's key, and
    /// the same under SET EXACT ON - and every answer was the records matching the first
    /// expression alone, so that is what this is. The second is read and does nothing.
    pub fn set_key_limit(&mut self, key: Option<Value>) {
        self.key_limit = key;
    }

    pub fn key_limit(&self) -> Option<&Value> {
        self.key_limit.as_ref()
    }

    /// Parks the table past its last record with every field reading as .NULL., which is the
    /// side of an outer join that had no match.
    pub fn set_outer_miss(&mut self) {
        self.outer_miss = true;
    }

    pub fn clear_outer_miss(&mut self) {
        self.outer_miss = false;
    }

    pub fn set_found(&mut self, found: bool) {
        self.found = found;
    }

    /// The first memo block the record at `recno` points at that has not been fetched yet.
    pub fn missing_memo(&self, recno: u64) -> Option<u32> {
        self.memo_blocks(recno).into_iter().find(|b| !self.memos.contains_key(b))
    }

    // ----- writing ---------------------------------------------------------------------------
    //
    // A record is a fixed run of bytes, so a change never moves anything: the new field bytes go
    // where the old ones were, in the page already in hand, and the whole record goes back to the
    // host when the statement that changed it finishes.

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    /// The record count to send with the write, when the write added a record.
    pub fn grew(&self) -> Option<u64> {
        self.grew.then_some(self.count)
    }

    /// The bytes of the record the pointer is on: for writing it back when it has a host to
    /// write to, and for `hold()` to keep regardless - a cursor with rows in memory has nowhere
    /// to send them (`flush_record` already leaves it there when the handle is `None`), but it
    /// still needs its own held copy for buffering, `GETFLDSTATE()` and an updategram to work
    /// from, the same as a table on disk does.
    pub fn current_bytes(&self) -> Option<Vec<u8>> {
        match &self.source {
            Source::Memory { rows } => {
                let row = rows.get(self.recno.checked_sub(1)? as usize)?;
                let mut out = vec![if row.deleted { b'*' } else { b' ' }];
                for (field, value) in self.header.fields.iter().zip(&row.values) {
                    let v = value_of(value);
                    let bytes = crate::dbf::write::encode_field(field, field.length as usize, &v, self.header.codepage).ok()?;
                    out.extend_from_slice(&bytes);
                }
                Some(out)
            }
            Source::Host { .. } => self.raw(self.recno).map(<[u8]>::to_vec),
        }
    }

    /// Whether what the header says has changed since it was last written, and clearing it.
    pub fn take_header_changed(&mut self) -> bool {
        std::mem::take(&mut self.header_changed)
    }

    pub fn written(&mut self) {
        self.touched.clear();
        self.dirty = false;
        self.grew = false;
        // the write went through, so what the record was before it is no longer of interest
        if self.held.is_empty() {
            self.originals.clear();
        }
    }

    /// Changes one field of the record the pointer is on.
    pub fn set_field(&mut self, name: &str, value: &Value) -> Result<(), RtError> {
        self.remember_original();
        let index = self
            .header
            .field_index(name)
            .ok_or_else(|| RtError::variable_not_found(name))?;
        if self.recno == 0 || self.recno > self.count {
            return Err(RtError::no_table_open());
        }
        let field = self.header.fields[index].clone();
        let (start, width) = self.header.layout[index];
        if matches!(field.kind, 'M' | 'G' | 'P') && matches!(self.source, Source::Host { .. }) {
            let text: Vec<u8> = match value.deref() {
                Value::Str(s) => crate::dbf::encoding::encode(&s, self.header.codepage),
                Value::Null => Vec::new(),
                _ => return Err(RtError::data_type_mismatch()),
            };
            self.memo_writes.retain(|(i, _)| *i != index);
            self.memo_writes.push_back((index, text));
            if !self.touched.contains(&index) {
                self.touched.push(index);
            }
            self.dirty = true;
            return Ok(());
        }

        match &mut self.source {
            Source::Memory { rows } => {
                let row = rows
                    .get_mut(self.recno as usize - 1)
                    .ok_or_else(|| RtError::no_table_open())?;
                row.values[index] = as_dbf_value(&field, value)?;
            }
            Source::Host { .. } => {
                let bytes = crate::dbf::write::encode_field(&field, width, value, self.header.codepage)?;
                let null = matches!(value.deref(), Value::Null);
                let slot = self.header.null_slot(index);
                let record = self
                    .record_bytes_mut()
                    .ok_or_else(|| RtError::no_table_open())?;
                let end = start + bytes.len();
                if end > record.len() {
                    return Err(RtError::no_table_open());
                }
                record[start..end].copy_from_slice(&bytes);
                // the bytes of a null field are blank, so what says it is null is the flag bit
                crate::dbf::layout::set_null_slot(record, slot, null);
            }
        }
        let recno = self.recno;
        self.seek(recno);
        self.dirty = true;
        if !self.touched.contains(&index) {
            self.touched.push(index);
        }
        Ok(())
    }

    /// DELETE and RECALL: the flag at the front of the record.
    pub fn set_deleted(&mut self, deleted: bool) -> Result<(), RtError> {
        self.remember_original();
        if self.recno == 0 || self.recno > self.count {
            return Err(RtError::no_table_open());
        }
        match &mut self.source {
            Source::Memory { rows } => {
                if let Some(row) = rows.get_mut(self.recno as usize - 1) {
                    row.deleted = deleted;
                }
            }
            Source::Host { .. } => {
                let record = self
                    .record_bytes_mut()
                    .ok_or_else(|| RtError::no_table_open())?;
                record[0] = if deleted { crate::dbf::layout::FLAG_DELETED } else { b' ' };
            }
        }
        let recno = self.recno;
        self.seek(recno);
        self.dirty = true;
        Ok(())
    }

    /// ZAP: every record goes.
    ///
    /// The header's count is what says how many records a table has, so setting it to zero is
    /// what empties one. Visual FoxPro also shortens the file; the bytes left behind here are
    /// past the end of the table and are written over by whatever is appended next.
    pub fn zap(&mut self) {
        self.count = 0;
        self.recno = 1;
        self.page.clear();
        self.page_first = 0;
        self.memos.clear();
        self.walk = None;
        self.found = false;
        if let Source::Memory { rows } = &mut self.source {
            rows.clear();
        }
        self.grew = true;
    }

    /// The bytes of the record APPEND BLANK and INSERT BLANK add: empty, but with the fields
    /// the table fills in itself already filled in.
    fn blank_row(&mut self) -> Vec<u8> {
        let mut blank = crate::dbf::write::blank_record(&self.header);
        // a field the table fills in itself takes the next number, and the header counts on
        for (index, field) in self.header.fields.clone().iter().enumerate() {
            if !field.autoincrements() {
                continue;
            }
            let value = Value::number(f64::from(field.autoinc_next));
            let (start, width) = self.header.layout[index];
            if let Ok(bytes) = crate::dbf::write::encode_field(field, width, &value, self.header.codepage) {
                let end = (start + bytes.len()).min(blank.len());
                blank[start..end].copy_from_slice(&bytes[..end - start]);
            }
            self.header.fields[index].autoinc_next = field.autoinc_next.saturating_add(u32::from(field.autoinc_step));
            self.header_changed = true;
        }
        blank
    }

    /// Where `INSERT [BEFORE] [BLANK]` puts the record it adds.
    ///
    /// The new record goes below the one the pointer is on, or above it for BEFORE. Past the
    /// last record there is nothing to go below, so both land at the end and the command is an
    /// APPEND BLANK - which is what Visual FoxPro does at end of file and on an empty table.
    pub fn insert_at(&self, before: bool) -> u64 {
        let step = u64::from(!before);
        self.recno.saturating_add(step).clamp(1, self.count + 1)
    }

    /// Whether an index stands over this table. A table whose index has not been read yet still
    /// has one, which is what the header's flag says, so this never needs to ask the host.
    pub fn indexed(&self) -> bool {
        self.header.has_index || !self.tags.is_empty()
    }

    /// The same record for a cursor whose rows are here.
    ///
    /// A table in a file lays its blank out from the header; a cursor has no such layout, so
    /// the values are made straight from the field types. A blank field reads as the empty
    /// value of its type - `""`, 0, `.F.`, an empty date - not as `.NULL.`, which is what lets
    /// `IF EMPTY(t.name)` work on a record that has just been added.
    fn blank_values(&mut self) -> DbfRecord {
        let mut values = Vec::with_capacity(self.header.fields.len());
        for index in 0..self.header.fields.len() {
            let field = self.header.fields[index].clone();
            let value = if field.autoincrements() {
                self.header.fields[index].autoinc_next =
                    field.autoinc_next.saturating_add(u32::from(field.autoinc_step));
                self.header_changed = true;
                Value::number(f64::from(field.autoinc_next))
            } else {
                empty_for(field.kind)
            };
            values.push(as_dbf_value(&field, &value).unwrap_or(DbfValue::Null));
        }
        DbfRecord { deleted: false, values }
    }

    /// The record `INSERT BLANK` is about to put into a table in a file, for the caller that
    /// writes it there itself. Taking it moves an autoincrementing field on, so it is taken once.
    pub fn blank_for_insert(&mut self) -> Vec<u8> {
        self.blank_row()
    }

    /// `INSERT BLANK` on a cursor whose rows are here: the blank goes in at `at` and everything
    /// from there down moves one.
    pub fn insert_row(&mut self, at: u64) {
        let record = self.blank_values();
        if let Source::Memory { rows } = &mut self.source {
            let index = (at as usize).saturating_sub(1).min(rows.len());
            rows.insert(index, record);
        }
        self.count += 1;
        self.header.record_count = self.count;
        self.seek(at);
        self.dirty = true;
        self.grew = true;
    }

    /// `INSERT BLANK` on a table in a file, once every record from `at` down has been written
    /// one lower and the blank has been written in their place. The table is a record longer,
    /// and the pointer sits on the new one.
    pub fn inserted(&mut self, at: u64, blank: Vec<u8>) {
        self.count += 1;
        self.header.record_count = self.count;
        self.memos.clear();
        self.set_page(at, blank);
        self.seek(at);
        // the records went to the file as they were written, so nothing is left over to send
        self.written();
    }

    /// APPEND BLANK: one empty record at the end, which the pointer then sits on.
    pub fn append_blank(&mut self) {
        if matches!(self.source, Source::Memory { .. }) {
            let record = self.blank_values();
            self.count += 1;
            self.recno = self.count;
            if let Source::Memory { rows } = &mut self.source {
                rows.push(record);
            }
        } else {
            let blank = self.blank_row();
            self.count += 1;
            self.recno = self.count;
            // the page becomes the one record just added; the rest is re-read when asked for
            self.page = blank;
            self.page_first = self.recno;
            self.memos.clear();
        }
        let recno = self.recno;
        self.seek(recno);
        self.dirty = true;
        self.grew = true;
    }

    /// `ALTER TABLE`: the file under this work area has different columns now, so the header
    /// and everything read from it start again.
    pub fn restructure(&mut self, header: DbfHeader) {
        self.count = header.record_count;
        self.header = header;
        self.page = Vec::new();
        self.page_first = 0;
        self.memos.clear();
        self.record = None;
        self.dirty = false;
        self.grew = false;
        self.seek(1);
    }

    /// `PACK`: the table is now this long, the records moved as `moved` says, and the page in
    /// hand is no longer what is on disk.
    pub fn repack(&mut self, count: u64, moved: &[(u32, u32)]) {
        self.count = count;
        self.header.record_count = count;
        self.page = Vec::new();
        self.page_first = 0;
        self.memos.clear();
        self.record = None;
        // an index entry follows its record to where it went, and goes when the record went
        let places: std::collections::HashMap<u32, u32> = moved.iter().copied().collect();
        if !self.tags.is_empty() {
            for tag in &mut self.tags {
                tag.entries.retain_mut(|e| match places.get(&e.recno) {
                    Some(now) => {
                        e.recno = *now;
                        true
                    }
                    None => false,
                });
                tag.entries.sort_by(|a, b| (&a.key, a.recno).cmp(&(&b.key, b.recno)));
            }
            self.index_dirty = true;
        }
        let order = self.order;
        let way = self.order_desc;
        self.set_order_way(order, way);
        self.seek(1);
    }

    /// Takes back the record `append_blank` just added, for a record that turned out not to
    /// belong: nothing has been written yet, so the table never had it.
    pub fn drop_appended(&mut self) {
        if self.count == 0 {
            return;
        }
        self.count -= 1;
        if let Source::Memory { rows } = &mut self.source {
            rows.pop();
        }
        self.dirty = false;
        self.grew = false;
        let past = self.count + 1;
        self.seek(past);
    }

    /// Everything a query result holds, for a cursor built from another one.
    pub fn all_rows(&self) -> &[DbfRecord] {
        self.rows().unwrap_or_default()
    }

    /// Records that a `DataReadMemo` for `block` is on its way, so the reply can be placed.
    pub fn expect_memo(&mut self, block: u32) {
        self.pending_memo = Some(block);
    }

    pub fn take_expected_memo(&mut self) -> Option<u32> {
        self.pending_memo.take()
    }

    /// Files a memo block the host sent: the 8-byte block header (type and payload length, both
    /// big-endian) followed by the payload. Anything shorter is filed as an empty block, which
    /// is what a missing memo file amounts to and stops the fetch being asked for again.
    pub fn store_memo(&mut self, block: u32, bytes: &[u8]) {
        let value = match bytes.get(..8) {
            Some(head) => {
                let kind = u32::from_be_bytes([head[0], head[1], head[2], head[3]]);
                let len = u32::from_be_bytes([head[4], head[5], head[6], head[7]]) as usize;
                let payload = bytes.get(8..8 + len).unwrap_or(&bytes[8..]).to_vec();
                if kind == 1 { OwnedMemoBlock::Text(payload) } else { OwnedMemoBlock::Binary(payload) }
            }
            None => OwnedMemoBlock::Text(Vec::new()),
        };
        self.memos.insert(block, value);
    }

    /// Moves the pointer, decoding with the memo blocks already in hand.
    pub fn seek(&mut self, recno: u64) {
        if let Some(rows) = self.rows() {
            self.record = rows.get(recno.wrapping_sub(1) as usize).cloned();
            self.recno = recno;
            return;
        }
        let memos = std::mem::take(&mut self.memos);
        self.seek_to(recno, |block| memos.get(&block).cloned());
        self.memos = memos;
    }

    /// Moves the pointer, decoding from the page in hand. `memo` resolves memo blocks the record
    /// points at; the caller fetches them from the host first.
    pub fn seek_to(&mut self, recno: u64, memo: impl FnMut(u32) -> Option<OwnedMemoBlock>) {
        self.recno = recno;
        self.record = self.raw(recno).map(|raw| decode_record(&self.header, raw, Padding::Keep, memo));
    }

    /// The raw bytes of a record, when the page in hand covers it.
    pub fn raw(&self, recno: u64) -> Option<&[u8]> {
        if recno == 0 || recno > self.count {
            return None;
        }
        // what a buffered table is holding for a record is what the program sees of it
        if let Some(held) = self.held.get(&recno) {
            return Some(&held.bytes);
        }
        if !self.page_holds(recno) {
            return None;
        }
        let start = (recno - self.page_first) as usize * self.header.record_len;
        self.page.get(start..start + self.header.record_len)
    }

    /// The bytes of a record as the file has them, whatever is being held for it. This is what
    /// the record was before a buffered change, which is what OLDVAL() and an updategram want.
    pub fn stored(&self, recno: u64) -> Option<&[u8]> {
        if recno == 0 || recno > self.count || !self.page_holds(recno) {
            return None;
        }
        let start = (recno - self.page_first) as usize * self.header.record_len;
        self.page.get(start..start + self.header.record_len)
    }

    /// The bytes of the record in hand, to be changed: the ones being held for it when the
    /// table is buffered, and its place in the page otherwise.
    fn record_bytes_mut(&mut self) -> Option<&mut [u8]> {
        let recno = self.recno;
        if self.held.contains_key(&recno) {
            return self.held.get_mut(&recno).map(|h| h.bytes.as_mut_slice());
        }
        let len = self.header.record_len;
        let start = (recno.checked_sub(self.page_first)?) as usize * len;
        self.page.get_mut(start..start + len)
    }

    /// Memo block numbers the record at `recno` points at, so they can be fetched in one go.
    pub fn memo_blocks(&self, recno: u64) -> Vec<u32> {
        if self.rows().is_some() {
            return Vec::new();
        }
        let Some(raw) = self.raw(recno) else {
            return Vec::new();
        };
        self.header
            .fields
            .iter()
            .zip(&self.header.layout)
            .filter(|(f, _)| matches!(f.kind, 'M' | 'G' | 'P'))
            .filter_map(|(_, &(start, width))| crate::dbf::memo_pointer(raw.get(start..start + width)?))
            .collect()
    }
}

/// Bytes the host sent.
///
/// A page of records is tens of kilobytes, and marshalling that as an array of numbers costs more
/// than reading it did, so the host sends one character per byte and this reads them back. Code
/// points 0..255 survive that exactly, which is all a byte needs.
pub fn bytes_of(v: &Value) -> Vec<u8> {
    match v.deref() {
        Value::Str(s) => s.chars().map(|c| c as u32 as u8).collect(),
        Value::Array(a) => a.borrow().items.iter().map(|b| b.as_number().unwrap_or(0.0) as u8).collect(),
        _ => Vec::new(),
    }
}

/// A value as an index key, in as many bytes as it takes. Text is not padded out, so a SEEK
/// for "SM" compares against the first two bytes of each key, which is what makes a partial
/// SEEK find the first company beginning with those letters.
pub fn index_key(value: &Value, numeric: bool, key_len: usize) -> Result<Vec<u8>, RtError> {
    let key = match value.deref() {
        Value::Number(n, ..) if numeric => crate::cdx::encode_number(n).to_vec(),
        Value::Date(d) if numeric => crate::cdx::encode_date(d.unwrap_or(0)).to_vec(),
        Value::DateTime(t) if numeric => crate::cdx::encode_datetime(t.unwrap_or(0.0)).to_vec(),
        Value::Logical(b) => vec![if b { b'T' } else { b'F' }],
        other if numeric => crate::cdx::encode_number(other.as_number()?).to_vec(),
        Value::Str(s) => s.chars().map(|c| (c as u32 & 0xff) as u8).take(key_len).collect(),
        other => other.as_str()?.chars().map(|c| (c as u32 & 0xff) as u8).take(key_len).collect(),
    };
    Ok(key)
}

/// The same key, padded out to the width the tag holds, which is what is stored.
pub fn index_key_padded(value: &Value, numeric: bool, key_len: usize) -> Result<Vec<u8>, RtError> {
    let mut key = index_key(value, numeric, key_len)?;
    key.resize(key_len, if numeric { 0 } else { b' ' });
    Ok(key)
}

/// Moves the pointer down the controlling order to the first record held under a key, and says
/// whether it was there. `near` is SET NEAR: where the pointer lands when it was not.
pub fn seek_in_order(cursor: &mut Cursor, key: &Value, near: bool) -> Result<bool, RtError> {
    let down = cursor.descending();
    let count = cursor.count();
    let Some(tag) = cursor.order() else {
        return Err(RtError::no_order_set());
    };
    let probe = index_key(key, tag.numeric, tag.key_len)?;
    // the entries are in ascending key order in the file whichever way the order runs, so the
    // search is the same either way and only the end it answers from changes
    let width = probe.len();
    let side = |e: &crate::cdx::Entry| e.key.get(..width.min(e.key.len())).unwrap_or(&e.key).cmp(probe.as_slice());
    let lo = tag.entries.partition_point(|e| side(e) == std::cmp::Ordering::Less);
    let hi = tag.entries.partition_point(|e| side(e) != std::cmp::Ordering::Greater);
    let found = hi > lo;
    let landing = if found {
        Some(if down { hi - 1 } else { lo })
    } else if near {
        // the record on the side of the missing key the order goes on to
        if down { lo.checked_sub(1) } else { Some(lo).filter(|i| *i < tag.entries.len()) }
    } else {
        None
    };
    let recno = landing.and_then(|i| tag.entries.get(i)).map(|e| u64::from(e.recno));
    cursor.seek(recno.unwrap_or(count + 1));
    Ok(found)
}

/// A record changed but not written: what it now holds, and which of its fields were touched.
#[derive(Debug, Clone)]
pub struct HeldRecord {
    pub bytes: Vec<u8>,
    /// The positions of the fields that were changed, from 0.
    pub fields: Vec<usize>,
    /// The record was appended while the table was buffered, so reverting takes it away.
    pub appended: bool,
    /// How long the table was when the record was held, for a revert that has to undo it.
    pub was: u64,
}

/// One relation: what to look the child up by, and where the child is.
#[derive(Debug, Clone)]
pub struct Relation {
    /// The expression, as it was written, evaluated against the parent's record.
    pub expr: String,
    /// The alias of the work area it moves.
    pub into: String,
}

/// A movement in progress: where it has got to, how many records are left to cross, and which
/// way. It lives on the cursor because a movement that has to ask the host for a page runs the
/// instruction again when the answer comes back, and has to carry on rather than start over.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Walk {
    /// 0 is before the first record and `count + 1` is past the last, so both ends are sayable.
    pub at: i64,
    pub remaining: i64,
    pub step: i64,
}

impl Walk {
    /// Where a movement starts, expressed as "step this many records from here".
    ///
    /// GO TOP is one step forward from before the first record and GO BOTTOM one step back from
    /// past the last, which is what makes them land on the first and last *visible* record when
    /// SET DELETED is on without needing a case of their own.
    pub fn new(cursor: &Cursor, to: Move) -> Walk {
        match to {
            Move::Top => Walk { at: 0, remaining: 1, step: 1 },
            Move::Bottom => Walk { at: cursor.count() as i64 + 1, remaining: -1, step: -1 },
            Move::Record(n) => Walk { at: n as i64, remaining: 0, step: 0 },
            Move::Skip(delta) => Walk { at: cursor.recno() as i64, remaining: delta, step: delta.signum() },
        }
    }
}

/// Where a movement command puts the record pointer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Move {
    Top,
    Bottom,
    Record(u64),
    Skip(i64),
}

/// A number read out of a field prints in the width the field was declared with.
///
/// This is why `? price` on a field declared `N(8,2)` holding 3200 is "   3200.00" and not
/// "3200": the field's own width and places travel with the value.
pub fn shaped_by(field: &crate::dbf::DbfField, value: &Value) -> Value {
    match value {
        Value::Number(n, _) => {
            Value::Number(*n, crate::value::Width::field(field.kind, field.length, field.decimals, *n))
        }
        other => other.clone(),
    }
}

/// A field of a record as the language sees it.
/// A stored field value as the language sees it.
pub fn value_of(v: &DbfValue) -> Value {
    match v {
        DbfValue::Null => Value::Null,
        DbfValue::Text(s) | DbfValue::Memo(s) => Value::str(s),
        DbfValue::Number(n) => Value::number(*n),
        DbfValue::Currency(c) => Value::Currency(*c),
        DbfValue::Logical(b) => Value::Logical(*b),
        DbfValue::Date(d) => Value::Date(*d),
        DbfValue::DateTime(t) => Value::DateTime(*t),
        DbfValue::Bytes(b) => Value::str(String::from_utf8_lossy(b).into_owned()),
    }
}

/// A value on its way into a field of a cursor whose rows are in memory. There are no bytes to
/// go through, but the field still has a type, and a value that does not fit it is still an error.
pub fn as_dbf_value(field: &crate::dbf::DbfField, value: &Value) -> Result<DbfValue, RtError> {
    // `.NULL.` is a value of its own: a column declared NULL keeps it, and one that was not
    // refuses it whatever its type - measured, error 1581 naming the column.
    if matches!(value.deref(), Value::Null) {
        return match field.nullable {
            true => Ok(DbfValue::Null),
            false => Err(RtError::new(
                RtError::NULL_REFUSED,
                format!("Field {} does not accept null values.", field.name.to_uppercase()),
            )),
        };
    }
    // A field that cannot hold the value is error 9, "Data type mismatch." - measured in Visual
    // FoxPro 9 for every field type a REPLACE can write - rather than the 107 an operator raises.
    Ok(match field.kind {
        'C' | 'M' | 'G' | 'P' => {
            let text = match value.deref() {
                Value::Str(s) => s.to_string(),
                _ => return Err(RtError::data_type_mismatch()),
            };
            // A character field is as wide as it was declared however it is stored, so a value
            // put in one is padded with blanks to that width and anything past it is cut off -
            // measured in Visual FoxPro 9, where `C(6)` holding "abc" reads back "abc   " and
            // "abcdefghij" reads back "abcdef". A memo has no width to fit.
            DbfValue::Text(if field.kind == 'C' { fit_to_width(&text, field.length as usize) } else { text })
        }
        'N' | 'F' | 'I' | '+' | 'B' | 'O' | 'Y' => {
            DbfValue::Number(value.as_number().map_err(|_| RtError::data_type_mismatch())?)
        }
        'L' => DbfValue::Logical(value.truthy().map_err(|_| RtError::data_type_mismatch())?),
        'D' => match value.deref() {
            Value::Date(d) => DbfValue::Date(d),
            _ => return Err(RtError::data_type_mismatch()),
        },
        'T' | '@' => match value.deref() {
            Value::DateTime(t) => DbfValue::DateTime(t),
            _ => return Err(RtError::data_type_mismatch()),
        },
        _ => return Err(RtError::data_type_mismatch()),
    })
}

/// Text cut or blank-padded to the width a character field was declared with.
fn fit_to_width(text: &str, width: usize) -> String {
    let mut out: String = text.chars().take(width).collect();
    let have = out.chars().count();
    out.extend(std::iter::repeat_n(' ', width.saturating_sub(have)));
    out
}

/// The empty value of a field's type, which is what SCATTER BLANK hands out and what a field
/// reads as at end of file.
pub fn empty_of(kind: char) -> Value {
    empty_for(kind)
}

/// The columns an array describes, for `CREATE CURSOR ... FROM ARRAY` and `CREATE TABLE ...
/// FROM ARRAY`.
///
/// The shape is the one `AFIELDS()` hands back: a row per field, whose first four columns are
/// the name, the one-letter type, the width and the decimal places. Visual FoxPro 9 fills
/// eighteen columns and ignores everything past the ones it needs, so anything from four
/// columns up is taken. An array of one dimension is a single field written flat.
pub fn fields_from_array(value: &Value) -> Result<Vec<crate::dbf::DbfField>, RtError> {
    let Value::Array(array) = value.deref() else { return Err(RtError::type_mismatch()) };
    let array = array.borrow();
    let cols = if array.cols == 0 { array.items.len() } else { array.cols };
    let rows = if array.cols == 0 { 1 } else { array.rows };
    if cols < 4 || rows == 0 {
        return Err(RtError::new(RtError::NO_FIELDS, "No fields found to process"));
    }
    let mut fields = Vec::with_capacity(rows);
    for row in 0..rows {
        let cell = |col: usize| array.items.get(row * cols + col).cloned().unwrap_or(Value::Logical(false));
        fields.push(field_from_row(&cell(0), &cell(1), &cell(2), &cell(3))?);
    }
    Ok(fields)
}

/// One row of that array as a column of a table.
fn field_from_row(
    name: &Value,
    kind: &Value,
    width: &Value,
    decimals: &Value,
) -> Result<crate::dbf::DbfField, RtError> {
    let invalid_name = || RtError::new(RtError::FIELD_NAME_INVALID, "Field name is a duplicate or invalid");
    let bad_width = || RtError::new(RtError::FIELD_WIDTH_INVALID, "Field width or number of decimal places is invalid");

    let name = name.deref().as_str().map_err(|_| invalid_name())?.trim().to_ascii_uppercase();
    let is_a_name = !name.is_empty()
        && name.len() <= 128
        && name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !is_a_name {
        return Err(invalid_name());
    }

    // the type is one letter, and only the letters a record can actually hold
    let letter = kind.deref().as_str().map_err(|_| RtError::function_arg_invalid())?;
    let letter = letter.trim().chars().next().unwrap_or(' ').to_ascii_uppercase();
    if !matches!(letter, 'C' | 'N' | 'F' | 'I' | '+' | 'B' | 'O' | 'Y' | 'L' | 'D' | 'T' | '@' | 'M' | 'G' | 'P') {
        return Err(RtError::function_arg_invalid());
    }

    // the width and the decimals may be written as numbers or as digits in a string; VFP
    // takes either, so a structure read out of a table of its own still describes one
    let number = |v: &Value| -> Result<f64, RtError> {
        match v.deref() {
            Value::Str(s) => Ok(s.trim().parse::<f64>().unwrap_or(0.0)),
            other => other.as_number().map_err(|_| bad_width()),
        }
    };
    let (length, decimals) = match fixed_width(letter) {
        // a type whose width is part of what it is takes its own, whatever the array says
        Some(w) => (w, 0u8),
        None => {
            let w = number(width)?;
            let d = number(decimals)?;
            if !(1.0..=255.0).contains(&w) || d < 0.0 || d >= w {
                return Err(bad_width());
            }
            (w as u8, d as u8)
        }
    };
    Ok(crate::dbf::DbfField::new(name, letter, length, decimals))
}

/// The width a field type carries of its own; `None` for the ones whose width is written
/// down, which are character, numeric and float.
fn fixed_width(kind: char) -> Option<u8> {
    match kind {
        'L' => Some(1),
        'D' | 'T' | '@' | 'B' | 'O' | 'Y' => Some(8),
        'I' | '+' | 'M' | 'G' | 'P' => Some(4),
        _ => None,
    }
}

/// What a field reads as when the pointer is at end of file: the empty value of its type, not
/// `.NULL.`, because `IF EOF() OR customer.balance = 0` has to keep working.
fn empty_for(kind: char) -> Value {
    match kind {
        'N' | 'F' | 'I' | '+' | 'B' | 'O' | 'Y' => Value::number(0.0),
        'L' => Value::Logical(false),
        'D' => Value::Date(None),
        'T' | '@' => Value::DateTime(None),
        _ => Value::str(""),
    }
}
