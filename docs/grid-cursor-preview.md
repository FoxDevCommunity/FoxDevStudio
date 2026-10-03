# Runtime grid cursor preview

The runtime grid previously drew headers and blank cells without reading
RecordSource. RecordSourceType 1 now reads a bounded snapshot of an open,
unbuffered in-memory cursor and displays its fields. No selected work area,
record pointer or database file is changed by a preview.

An empty Column.ControlSource uses the field at that column's ordinal; a simple
field name or matching alias.field maps explicitly, case-insensitively. Other
expressions show an unsupported-field indicator. Existing columns and captions
are preserved; automatic ColumnCount=-1 expansion is not implemented here.

Snapshots refresh after runtime state changes and desktop revisions. Closing an
alias removes its old values and reports that the alias is not open. Rendering
uses text nodes, not HTML, and does not execute ControlSource expressions.

Clicking a displayed cell or pressing Up/Down/Home/End selects its physical
record in the VM and selects the source work area. Thus form actions read the
chosen record. Highlighting follows the VM pointer, including programmatic
movement. Selection is limited to the displayed snapshot; cells remain read-only.

Navigation dispatches BeforeRowColChange with the old column, then the current
control's Valid, selects the target record, dispatches the target control's When,
and finally AfterRowColChange with the new column. NODEFAULT in Before cancels;
Valid returning false cancels. When returning false restores the previous visible
record: this is a conservative fallback, not VFP's full alternate-focus search.
RowColChange uses 1=row, 2=column, 3=both. Keyboard KeyPress can suppress default
navigation. Mouse selection also dispatches Grid.Click; keyboard navigation does
not synthesize a click. Full child-control focus/click/double-click behavior,
cell editing and Dynamic* expressions remain separate work.

Every cursor installation gets a generation token. Stale requests after closing
and recreating an alias are rejected, as are disabled/rebound grids and concurrent
navigation on the same grid. Events that move the cursor themselves cancel the
pending default movement. No host file writes are performed by navigation.

Host-backed tables, buffering, SET FILTER, controlling indexes, key ranges and
relations report a limitation instead of showing an unfiltered or misleading
snapshot. SET DELETED ON omits deleted records for supported cursors. Preview
limits are 200 visible rows, 128 fields and 512 characters per cell; shortening is
reported. There is no pagination in this first slice. Private data sessions and
native Windows VFP comparisons are not validated.
