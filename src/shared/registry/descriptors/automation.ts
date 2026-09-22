import type { ObjectDescriptor } from '../types';
import * as c from '../common';

/**
 * The objects a Visual FoxPro program reaches that are not on a form: the application itself,
 * the project it was built from, the error it caught, and the adapter that turns a cursor into
 * XML and back.
 *
 * None of them is drawn, so each is described only by what a program can read and write on it.
 * They are listed here together because they share a shape: a program gets one, reads what it
 * holds, and calls a handful of methods on it.
 */

/** `_VFP` and `Application`: the copy of Visual FoxPro the program is running in. */
export const application: ObjectDescriptor = {
  displayName: 'Application',
  baseClass: 'application',
  defaultEvent: 'Init',
  properties: c.props(
    c.prop('Name', 'text', 'Other', 'VFP', { readOnly: true }),
    c.prop('Caption', 'text', 'Appearance', 'FoxDev Studio'),
    c.prop('Version', 'text', 'Other', '', { readOnly: true }),
    c.prop('Visible', 'boolean', 'Appearance', true),
    c.prop('ActiveForm', 'text', 'Other', '', { readOnly: true, description: 'The form with the focus, or .NULL. when none has it.' }),
    c.prop('ActiveProject', 'text', 'Other', '', { readOnly: true, description: 'The project that is open in the development environment.' }),
    c.prop('Forms', 'text', 'Other', '', { readOnly: true, description: 'Every form that is up, by number.' }),
    c.prop('FormCount', 'number', 'Other', 0, { readOnly: true }),
    c.prop('Projects', 'text', 'Other', '', { readOnly: true, description: 'The projects that are open.' }),
    c.prop('AutoYield', 'boolean', 'Behavior', true, { description: 'Whether the application handles events between statements.' }),
    c.prop('DefaultFilePath', 'text', 'Data', '', { description: 'Where a file with no path of its own is looked for.' }),
    c.prop('StartMode', 'number', 'Other', 0, { readOnly: true, description: 'How it was started: 0 the development environment, 2 a program, 3 an in-process server, 4 out of process, 5 a runtime application.' }),
    c.prop('ProcessId', 'number', 'Other', 0, { readOnly: true, description: 'The number Windows knows the process by.' }),
    c.prop('ThreadId', 'number', 'Other', 0, { readOnly: true }),
    c.prop('LanguageOptions', 'number', 'Behavior', 0, { description: 'How strictly the language is read: 0 as VFP has always read it, 1 stricter.' }),
    c.prop('HomeDir', 'text', 'Data', '', { readOnly: true, description: 'Where the runtime was installed.' }),
    c.prop('HostName', 'text', 'Other', '', { description: 'What the application calls itself when another program asks.' }),
    c.prop('StatusBar', 'text', 'Appearance', '', { description: 'What the status bar says.' }),
    c.prop('VisualEffect', 'number', 'Appearance', 0, { description: 'Which visual style the application draws with.' }),
    c.prop('EditorOptions', 'text', 'Behavior', '', { description: 'The letters that say how the editor behaves.' }),
    c.prop('BreakOnError', 'boolean', 'Behavior', false, { description: 'Whether an error stops the program in the debugger.' }),
    c.prop('DoCmd', 'text', 'Other', '', { hidden: true, description: 'A command for the application to run, written rather than called.' }),
    c.prop('WindowList', 'text', 'Other', '', { readOnly: true, description: 'Every window the application has, by number.' }),
    c.prop('MacDesktop', 'boolean', 'Appearance', false, { description: 'Whether a window belongs to the desktop rather than to the main window.' }),
    c.prop('SOM', 'boolean', 'Other', false, { hidden: true, description: 'Kept for the Macintosh version, which had a system object model.' }),
    c.prop('ServerName', 'text', 'Other', '', { readOnly: true, description: 'The file an automation server was started from.' }),
  ),
  events: c.events([
    c.ev('Init', '', 'The application has started.'),
    c.ev('Destroy', '', 'It is going away.'),
    c.ev('Error', 'nError, cMethod, nLine', 'An error nothing else handled.'),
    c.ev('QueryUnload', '', 'Something has asked it to close.'),
  ]),
};

/** The error a `TRY` caught, or the one an `Error` method was told about. */
export const exception: ObjectDescriptor = {
  displayName: 'Exception',
  baseClass: 'exception',
  defaultEvent: 'Init',
  properties: c.props(
    c.prop('ErrorNo', 'number', 'Data', 0, { readOnly: true, description: 'The Visual FoxPro error number.' }),
    c.prop('Message', 'text', 'Data', '', { readOnly: true, description: 'What went wrong, in words.' }),
    c.prop('Details', 'text', 'Data', '', { readOnly: true, description: 'What it went wrong on: the file, the field, the class.' }),
    c.prop('LineNo', 'number', 'Data', 0, { readOnly: true, description: 'The line it happened on.' }),
    c.prop('LineContents', 'text', 'Data', '', { readOnly: true, description: 'That line, as it was written.' }),
    c.prop('Procedure', 'text', 'Data', '', { readOnly: true, description: 'The program or method it happened in.' }),
    c.prop('StackLevel', 'number', 'Data', 0, { readOnly: true, description: 'How deep in the call stack that was.' }),
    c.prop('UserValue', 'text', 'Data', '', { description: 'Whatever THROW was given, kept as it was.' }),
  ),
  events: c.events([c.ev('Init', ''), c.ev('Destroy', '')]),
};

/** A project as an object: what it holds and what building it produces. */
export const project: ObjectDescriptor = {
  displayName: 'Project',
  baseClass: 'project',
  defaultEvent: 'Init',
  properties: c.props(
    c.prop('Name', 'text', 'Data', '', { readOnly: true, description: 'The project file, with its path.' }),
    c.prop('Files', 'text', 'Data', '', { readOnly: true, description: 'Everything in the project, by number.' }),
    c.prop('Servers', 'text', 'Data', '', { readOnly: true, description: 'The automation servers it defines.' }),
    c.prop('MainFile', 'text', 'Data', '', { description: 'The file a built application starts at.' }),
    c.prop('HomeDir', 'text', 'Data', '', { description: 'The folder every relative path in it is relative to.' }),
    c.prop('ProjectHook', 'text', 'Behavior', '', { readOnly: true, description: 'The hook object watching what happens to the project.' }),
    c.prop('ProjectHookClass', 'text', 'Behavior', '', { description: 'What class that hook is.' }),
    c.prop('ProjectHookLibrary', 'text', 'Behavior', '', { description: 'And which class library it comes from.' }),
    c.prop('AutoIncrement', 'boolean', 'Behavior', false, { description: 'Whether the version number goes up with every build.' }),
    c.prop('BuildDateTime', 'text', 'Other', '', { readOnly: true, description: 'When it was last built.' }),
    c.prop('Debug', 'boolean', 'Behavior', true, { description: 'Whether what is built keeps the information a debugger needs.' }),
    c.prop('Encrypted', 'boolean', 'Behavior', false, { description: 'Whether the code in it is encrypted.' }),
    c.prop('SCCProvider', 'text', 'Other', '', { readOnly: true, description: 'The source control the project is under.' }),
    c.prop('SCCStatus', 'number', 'Other', 0, { readOnly: true }),
    c.prop('ServerHelpFile', 'text', 'Other', '', { description: 'The help file a server built from it points at.' }),
    c.prop('TypeLibCLSID', 'text', 'Other', '', { readOnly: true }),
    c.prop('TypeLibDesc', 'text', 'Other', '', { description: 'What the type library calls itself.' }),
    c.prop('TypeLibName', 'text', 'Other', '', { readOnly: true }),
    c.prop('VersionComments', 'multiline', 'Other', ''),
    c.prop('VersionCompany', 'text', 'Other', ''),
    c.prop('VersionCopyright', 'text', 'Other', ''),
    c.prop('VersionDescription', 'text', 'Other', ''),
    c.prop('VersionNumber', 'text', 'Other', ''),
    c.prop('VersionProduct', 'text', 'Other', ''),
    c.prop('VersionTrademarks', 'text', 'Other', ''),
    c.prop('VisualEffect', 'number', 'Appearance', 0),
  ),
  events: c.events([c.ev('Init', ''), c.ev('Destroy', '')]),
};

/** The object a project calls before and after everything that happens to it. */
export const projecthook: ObjectDescriptor = {
  displayName: 'ProjectHook',
  baseClass: 'projecthook',
  defaultEvent: 'QueryAddFile',
  properties: c.props(
    c.prop('Name', 'text', 'Other', 'projecthook'),
    c.prop('OleObject', 'text', 'Other', '', { hidden: true }),
  ),
  events: c.events([
    c.ev('Init', 'oProject', 'The project has been opened and the hook is being attached.'),
    c.ev('Destroy', '', 'The project is closing.'),
    c.ev('QueryAddFile', 'cFileName', 'A file is about to be added; .F. stops it.'),
    c.ev('QueryModifyFile', 'cFileName', 'A file is about to be opened for editing.'),
    c.ev('QueryNewFile', 'cFileName, lNoDefault', 'A file is about to be made.'),
    c.ev('QueryRemoveFile', 'cFileName', 'A file is about to be taken out.'),
    c.ev('QueryRunFile', 'cFileName', 'A file is about to be run.'),
    c.ev('BeforeBuild', 'nBuildAction, cOutputName, lRebuildAll, lShowErrors, lBuildNewGuids', 'A build is about to start.'),
    c.ev('AfterBuild', 'nBuildAction, cOutputName, lRebuildAll, lShowErrors, lBuildNewGuids', 'It has finished.'),
    c.ev('SCCInit', '', 'Source control has been turned on for the project.'),
    c.ev('SCCDestroy', '', 'And turned off again.'),
  ]),
};

/** One file of a project. */
export const projectfile: ObjectDescriptor = {
  displayName: 'File',
  baseClass: 'file',
  defaultEvent: 'Init',
  properties: c.props(
    c.prop('Name', 'text', 'Data', '', { readOnly: true, description: 'The file, with its path.' }),
    c.prop('Type', 'text', 'Data', '', { readOnly: true, description: 'One letter for what it is: P a program, V a class library, K a form.' }),
    c.prop('Description', 'multiline', 'Other', ''),
    c.prop('Exclude', 'boolean', 'Behavior', false, { description: 'Whether it is left out of what is built.' }),
    c.prop('FileClass', 'text', 'Data', '', { readOnly: true, description: 'The class it holds, when it holds one.' }),
    c.prop('FileClassLibrary', 'text', 'Data', '', { readOnly: true }),
    c.prop('LastModified', 'text', 'Other', '', { readOnly: true }),
    c.prop('ReadOnly', 'boolean', 'Behavior', false, { readOnly: true }),
    c.prop('SCCStatus', 'number', 'Other', 0, { readOnly: true }),
    c.prop('CodePage', 'number', 'Data', 0, { description: 'The code page its text is in.' }),
    c.prop('FullName', 'text', 'Data', '', { readOnly: true, description: 'The file with its whole path, whatever Name was given as.' }),
  ),
  events: c.events([c.ev('Init', ''), c.ev('Destroy', '')]),
};

/** An automation server a project builds. */
export const server: ObjectDescriptor = {
  displayName: 'Server',
  baseClass: 'server',
  defaultEvent: 'Init',
  properties: c.props(
    c.prop('ServerName', 'text', 'Data', '', { readOnly: true, description: 'What another program asks for it by.' }),
    c.prop('ServerClass', 'text', 'Data', '', { readOnly: true, description: 'The class it stands for.' }),
    c.prop('ServerClassLibrary', 'text', 'Data', '', { readOnly: true }),
    c.prop('ServerProject', 'text', 'Data', '', { readOnly: true, description: 'The project it was built from.' }),
    c.prop('ServerHelpFile', 'text', 'Other', ''),
    c.prop('Description', 'multiline', 'Other', ''),
    c.prop('Instancing', 'number', 'Behavior', 5, { description: 'How another program may make one: 1 not at all, 3 one for each, 5 one shared.' }),
    c.prop('CLSID', 'text', 'Other', '', { readOnly: true }),
    c.prop('ProgID', 'text', 'Other', '', { readOnly: true }),
  ),
  events: c.events([c.ev('Init', ''), c.ev('Destroy', '')]),
};

/** A named connection to a server, as a database container keeps one. */
export const connection: ObjectDescriptor = {
  displayName: 'Connection',
  baseClass: 'connection',
  defaultEvent: 'Init',
  properties: c.props(
    c.prop('Name', 'text', 'Data', '', { readOnly: true }),
    c.prop('DataSource', 'text', 'Data', '', { description: 'The data source name it connects through.' }),
    c.prop('ConnectString', 'text', 'Data', '', { description: 'Or the whole connection string, when it has one.' }),
    c.prop('UserId', 'text', 'Data', ''),
    c.prop('Password', 'text', 'Data', ''),
    c.prop('Asynchronous', 'boolean', 'Behavior', false),
    c.prop('BatchMode', 'boolean', 'Behavior', true),
    c.prop('ConnectTimeOut', 'number', 'Behavior', 15),
    c.prop('IdleTimeOut', 'number', 'Behavior', 0),
    c.prop('QueryTimeOut', 'number', 'Behavior', 0),
    c.prop('WaitTime', 'number', 'Behavior', 100),
    c.prop('Transactions', 'number', 'Behavior', 1, { description: '1 the server commits each statement, 2 the program says when.' }),
    c.prop('PacketSize', 'number', 'Behavior', 4096),
    c.prop('DispLogin', 'number', 'Behavior', 2, { description: 'When the login dialog is shown: 1 always, 2 only if it is needed, 3 never.' }),
    c.prop('DispWarnings', 'boolean', 'Behavior', true),
  ),
  events: c.events([c.ev('Init', ''), c.ev('Destroy', '')]),
};

/** The properties every part of an XMLAdapter names itself by. */
const xmlNaming = c.props(
  c.prop('XMLName', 'text', 'Data', '', { description: 'What it is called in the XML, which need not be what it is called here.' }),
  c.prop('XMLNameIsXPath', 'boolean', 'Data', false, { description: 'Whether that name is a path through the document rather than a plain name.' }),
  c.prop('XMLNamespace', 'text', 'Data', ''),
  c.prop('XMLPrefix', 'text', 'Data', ''),
);

/** The XMLAdapter: a cursor as XML, and XML back into a cursor. */
export const xmladapter: ObjectDescriptor = {
  displayName: 'XMLAdapter',
  baseClass: 'xmladapter',
  defaultEvent: 'Init',
  properties: c.props(
    xmlNaming,
    c.prop('Tables', 'text', 'Data', '', { readOnly: true, description: 'The tables the document holds, as XMLTable objects.' }),
    c.prop('FileName', 'text', 'Data', '', { readOnly: true, description: 'The file the XML was read from.' }),
    c.prop('IsDiffGram', 'boolean', 'Data', false, { readOnly: true, description: 'Whether what was read is a diffgram: the changes rather than the whole thing.' }),
    c.prop('IsLoaded', 'boolean', 'Data', false, { readOnly: true, description: 'Whether anything has been read yet.' }),
    c.prop('UpdateGram', 'boolean', 'Data', false, { description: 'Whether what is written out is the changes rather than the whole thing.' }),
    c.prop('UpdateGramSchemaLocation', 'text', 'Data', ''),
    c.prop('XMLSchemaLocation', 'text', 'Data', ''),
    c.prop('XMLConstraints', 'number', 'Data', 0, { description: 'How much of the table structure the schema carries.' }),
    c.prop('RespectNesting', 'boolean', 'Data', true, { description: 'Whether a nested table is written inside its parent.' }),
    c.prop('PreserveWhiteSpace', 'boolean', 'Data', false),
    c.prop('AddLineFeeds', 'boolean', 'Data', false, { description: 'Whether the XML is written one element to a line.' }),
    c.prop('DeclareXMLPrefix', 'boolean', 'Data', true),
    c.prop('DisableEncode', 'boolean', 'Data', false, { description: 'Whether the characters XML reserves are left as they are.' }),
    c.prop('ForceCloseTag', 'boolean', 'Data', false, { description: 'Whether an empty element is written with a closing tag of its own.' }),
    c.prop('WrapInCDATA', 'boolean', 'Data', false),
    c.prop('WrapMemoInCDATA', 'boolean', 'Data', true),
    c.prop('WrapCharInCDATA', 'boolean', 'Data', false),
    c.prop('MapBinary', 'boolean', 'Data', false, { description: 'Whether a binary field is written as base 64.' }),
    c.prop('MapN19_4ToCurrency', 'boolean', 'Data', false),
    c.prop('MapVarchar', 'boolean', 'Data', true),
    c.prop('Encoding', 'text', 'Data', '', { description: 'What the XML declaration says it is written in.' }),
    c.prop('CodePage', 'number', 'Data', 0),
    c.prop('ADOCodePage', 'number', 'Data', 0),
    c.prop('UseCodePage', 'boolean', 'Data', false),
    c.prop('UTF8Encoded', 'boolean', 'Data', false),
    c.prop('Unicode', 'boolean', 'Data', false),
    c.prop('NoCpTrans', 'boolean', 'Data', false, { description: 'Whether the text is left in the code page it arrived in.' }),
    c.prop('FormattedOutput', 'boolean', 'Data', false, { description: 'Whether the XML is written with indentation.' }),
    c.prop('SelectionNamespaces', 'text', 'Data', '', { description: 'The namespaces an XPath in this adapter may use.' }),
    c.prop('IXMLDOMElement', 'text', 'Data', '', { readOnly: true, description: 'The document itself, where the host has one to give.' }),
    c.prop('SOM', 'text', 'Data', '', { readOnly: true, description: 'The schema the document was read against.' }),
    c.prop('VFPXMLProgID', 'text', 'Other', '', { description: 'The program identifier the XML says it came from.' }),
  ),
  events: c.events([c.ev('Init', ''), c.ev('Destroy', '')]),
};

/** One table of that document. */
export const xmltable: ObjectDescriptor = {
  displayName: 'XMLTable',
  baseClass: 'xmltable',
  defaultEvent: 'Init',
  properties: c.props(
    xmlNaming,
    c.prop('Alias', 'text', 'Data', '', { description: 'What the cursor made from it is called.' }),
    c.prop('Fields', 'text', 'Data', '', { readOnly: true, description: 'Its fields, as XMLField objects.' }),
    c.prop('ChildTables', 'text', 'Data', '', { readOnly: true }),
    c.prop('FirstNestedTable', 'text', 'Data', '', { readOnly: true, description: 'The first table written inside this one.' }),
    c.prop('NextSiblingTable', 'text', 'Data', '', { readOnly: true, description: 'The next table written beside it.' }),
    c.prop('NestedInto', 'text', 'Data', '', { readOnly: true, description: 'The table this one is written inside.' }),
    c.prop('Keyfield', 'text', 'Data', '', { description: 'The field that says which record is which.' }),
    c.prop('AttributesOnly', 'boolean', 'Data', false, { description: 'Whether its fields are written as attributes rather than as elements.' }),
    c.prop('MaxRecords', 'number', 'Data', 0),
    c.prop('XMLType', 'text', 'Data', '', { readOnly: true }),
    c.prop('XMLAdapter', 'text', 'Data', '', { readOnly: true, description: 'The adapter this table was read by.' }),
  ),
  events: c.events([c.ev('Init', ''), c.ev('Destroy', '')]),
};

/** One field of one of those tables. */
export const xmlfield: ObjectDescriptor = {
  displayName: 'XMLField',
  baseClass: 'xmlfield',
  defaultEvent: 'Init',
  properties: c.props(
    xmlNaming,
    c.prop('Name', 'text', 'Data', '', { description: 'What the field is called in the cursor.' }),
    c.prop('DataType', 'text', 'Data', '', { description: 'The Visual FoxPro type the XML type became.' }),
    c.prop('Type', 'text', 'Data', '', { readOnly: true, description: 'The one letter that type is written as.' }),
    c.prop('MaxLength', 'number', 'Data', 0),
    c.prop('TotalDigits', 'number', 'Data', 0),
    c.prop('FractionDigits', 'number', 'Data', 0),
    c.prop('IsAttribute', 'boolean', 'Data', false, { description: 'Whether it is written as an attribute rather than as an element.' }),
    c.prop('IsBinary', 'boolean', 'Data', false),
    c.prop('IsBase64', 'boolean', 'Data', false),
    c.prop('IsNull', 'boolean', 'Data', false, { description: 'Whether it may hold nothing at all.' }),
    c.prop('Keyfield', 'boolean', 'Data', false, { description: 'Whether it is part of what says which record is which.' }),
    c.prop('NoCpTrans', 'boolean', 'Data', false),
    c.prop('XSDtype', 'text', 'Data', '', { description: 'The schema type it came from.' }),
    c.prop('XSDmaxLength', 'number', 'Data', 0),
    c.prop('XSDtotalDigits', 'number', 'Data', 0),
    c.prop('XSDfractionDigits', 'number', 'Data', 0),
    c.prop('XMLTable', 'text', 'Data', '', { readOnly: true, description: 'The table this field belongs to.' }),
  ),
  events: c.events([c.ev('Init', ''), c.ev('Destroy', '')]),
};
