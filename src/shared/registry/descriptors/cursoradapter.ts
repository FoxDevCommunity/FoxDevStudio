import type { ObjectDescriptor } from '../types';
import * as c from '../common';

/**
 * The CursorAdapter: a cursor filled from somewhere that is not a Visual FoxPro table.
 *
 * Visual FoxPro 9 added it so one object could stand in front of a native table, an ODBC
 * connection, an ADO recordset or an XML document, and the form above it need not care which.
 * The properties say where the data comes from, what is fetched, and how a change goes back.
 */
export const cursoradapter: ObjectDescriptor = {
  displayName: 'CursorAdapter',
  baseClass: 'cursoradapter',
  defaultEvent: 'Init',
  properties: c.props(
    c.prop('Alias', 'text', 'Data', '', { description: 'What the cursor it fills is called.' }),
    c.prop('DataSource', 'text', 'Data', '', { description: 'The connection, recordset or document it reads from.' }),
    c.prop('DataSourceType', 'text', 'Data', '', { description: 'Which of those it is: Native, ODBC, ADO or XML.' }),
    c.prop('CursorSchema', 'multiline', 'Data', '', { description: 'The columns the cursor has, written as a CREATE CURSOR field list.' }),
    c.prop('UseCursorSchema', 'boolean', 'Data', true, { description: 'Whether the cursor is built from that rather than from what came back.' }),
    c.prop('SelectCmd', 'multiline', 'Data', '', { description: 'What is sent to fetch the data.' }),
    c.prop('Prepared', 'boolean', 'Data', false, { description: 'Whether that command is prepared once and run again after.' }),
    c.prop('FetchSize', 'number', 'Data', 100, { description: 'How many records are fetched at a time; -1 fetches them all.' }),
    c.prop('MaxRecords', 'number', 'Data', -1, { description: 'How many to fetch at most; -1 is all of them.' }),
    c.prop('RecordsFetched', 'number', 'Data', 0, { readOnly: true }),
    c.prop('FetchAsNeeded', 'boolean', 'Data', false, { description: 'Whether the rest is fetched as the program reaches it.' }),
    c.prop('FetchMemo', 'boolean', 'Data', true),
    c.prop('NoData', 'boolean', 'Data', false, { description: 'Whether the cursor is made empty rather than filled.' }),
    c.prop('NoDataOnLoad', 'boolean', 'Data', false),
    c.prop('BufferModeOverride', 'number', 'Data', 5, { description: 'The buffering the cursor is given, whatever the form asks for.' }),
    c.prop('BatchUpdateCount', 'number', 'Data', 1, { description: 'How many changed records go back in one go.' }),
    c.prop('SendUpdates', 'boolean', 'Data', true, { description: 'Whether a change to the cursor is sent back at all.' }),
    c.prop('KeyFieldList', 'text', 'Data', '', { description: 'The fields that say which record is which.' }),
    c.prop('UpdatableFieldList', 'text', 'Data', '', { description: 'The fields a change may be sent back for.' }),
    c.prop('UpdateNameList', 'multiline', 'Data', '', { description: 'What each field is called where the data came from.' }),
    c.prop('UpdateType', 'number', 'Data', 1, { description: '1 sends the fields that changed, 2 deletes the record and inserts it again.' }),
    c.prop('WhereType', 'number', 'Data', 3, { description: 'What a sent change matches on: the key, the key and what changed, or every field.' }),
    c.prop('CompareMemo', 'boolean', 'Data', true, { description: 'Whether a memo counts as changed for that comparison.' }),
    c.prop('ConversionFunc', 'text', 'Data', '', { description: 'A function each field goes through on its way back.' }),
    c.prop('Flags', 'number', 'Data', 0, { description: 'What the adapter is told about the data source, one bit each.' }),
    c.prop('ShareConnection', 'boolean', 'Data', false, { description: 'Whether it uses the connection the data environment already has.' }),
    c.prop('AllowInsert', 'boolean', 'Data', true),
    c.prop('AllowUpdate', 'boolean', 'Data', true),
    c.prop('AllowDelete', 'boolean', 'Data', true),
    c.prop('AllowSimultaneousFetch', 'boolean', 'Data', false),
    c.prop('BreakOnError', 'boolean', 'Behavior', false),
    c.prop('MapBinary', 'boolean', 'Data', false),
    c.prop('MapN19_4ToCurrency', 'boolean', 'Data', false),
    c.prop('UseDeDataSource', 'boolean', 'Data', false, { description: "Whether it takes the data environment's source rather than its own." }),
    c.prop('UseMemoSize', 'number', 'Data', 255, { description: 'How long a text field has to be before it becomes a memo.' }),
    c.prop('Tables', 'text', 'Data', '', { description: 'The tables a change is sent back to.' }),
  ),
  events: c.events([
    c.ev('Init', ''),
    c.ev('Destroy', ''),
    c.ev('BeforeCursorFill', 'lUseCursorSchema, lNoDataOnLoad, cSelectCmd', 'The cursor is about to be filled; .F. stops it.'),
    c.ev('AfterCursorFill', 'lUseCursorSchema, lNoDataOnLoad, cSelectCmd, lResult', 'It has been.'),
    c.ev('BeforeCursorRefresh', 'cSelectCmd'),
    c.ev('AfterCursorRefresh', 'cSelectCmd, lResult'),
    c.ev('BeforeCursorAttach', 'cAlias'),
    c.ev('AfterCursorAttach', 'cAlias'),
    c.ev('BeforeCursorDetach', 'cAlias'),
    c.ev('AfterCursorDetach', 'cAlias'),
    c.ev('BeforeCursorClose', 'cAlias'),
    c.ev('AfterCursorClose', 'cAlias'),
    c.ev('BeforeCursorUpdate', 'cFldState, lForce, nDataSessionID', 'A change is about to be sent back.'),
    c.ev('AfterCursorUpdate', 'cFldState, lForce, nDataSessionID, lResult'),
    c.ev('BeforeInsert', 'cFldState, lForce, nDataSessionID, cInsertCmd'),
    c.ev('AfterInsert', 'cFldState, lForce, nDataSessionID, cInsertCmd, lResult'),
    c.ev('BeforeUpdate', 'cFldState, lForce, nDataSessionID, cUpdateCmd, cUpdateWhereCmd'),
    c.ev('AfterUpdate', 'cFldState, lForce, nDataSessionID, cUpdateCmd, cUpdateWhereCmd, lResult'),
    c.ev('BeforeDelete', 'cFldState, lForce, nDataSessionID, cDeleteCmd'),
    c.ev('AfterDelete', 'cFldState, lForce, nDataSessionID, cDeleteCmd, lResult'),
    c.ev('BeforeRecordRefresh', 'nRecords'),
    c.ev('AfterRecordRefresh', 'nRecords, lResult'),
    c.ev('Error', 'nError, cMethod, nLine'),
  ]),
};

/**
 * The DataObject a drag carries: what is being dragged, in whatever formats it can be had in.
 *
 * A program never makes one; it is handed one by every OLE drag event, and asks it what it
 * holds and in what format.
 */
export const dataobject: ObjectDescriptor = {
  displayName: 'DataObject',
  baseClass: 'dataobject',
  defaultEvent: 'Init',
  properties: c.props(
    c.prop('Files', 'text', 'Data', '', { readOnly: true, description: 'The files being dragged, when files are what is being dragged.' }),
  ),
  events: c.events([]),
};

/**
 * `Control`: what every control on a form is, before it is any control in particular.
 *
 * Visual FoxPro lets a class be built on it directly, so a program that subclasses Control
 * gets what every control has and nothing more.
 */
export const control: ObjectDescriptor = {
  displayName: 'Control',
  baseClass: 'control',
  defaultEvent: 'Init',
  properties: c.props(c.behaviorProps, c.fontProps),
  events: c.events(c.baseEvents, c.mouseEvents, c.focusEvents, c.dragEvents),
};
