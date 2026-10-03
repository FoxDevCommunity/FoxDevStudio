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

This is a read-only preview, visibly labelled as such. Row navigation, editing,
Before/AfterRowColChange dispatch, Dynamic* expressions, per-column formatting,
record markers and full Grid parity remain unimplemented. Selecting a record via
the grid is not supported; consuming form actions still see the VM's current
record, which the preview never moves.

Host-backed tables, buffering, SET FILTER, controlling indexes, key ranges and
relations report a limitation instead of showing an unfiltered or misleading
snapshot. SET DELETED ON omits deleted records for supported cursors. Preview
limits are 200 visible rows, 128 fields and 512 characters per cell; shortening is
reported. There is no pagination in this first slice. Private data sessions and
native Windows VFP comparisons are not validated.
