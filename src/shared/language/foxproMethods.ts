/** Methods available on runtime objects (upper-case). Not functions: they need an object. */
export const FOXPRO_METHODS = [
  // every object
  'SETFOCUS',
  'REFRESH',
  'RELEASE',
  'SHOW',
  'HIDE',
  'MOVE',
  'ZORDER',
  'ADDOBJECT',
  'REMOVEOBJECT',
  'ADDPROPERTY',
  'RESETTODEFAULT',
  'READMETHOD',
  'WRITEMETHOD',
  'READEXPRESSION',
  'WRITEEXPRESSION',
  'SETALL',
  'SAVEASCLASS',
  // it copies an object for the designer that is designing it, and refuses a running program
  // in the product's own words, which is what this runtime does with it too
  'CLONEOBJECT',
  'SHOWWHATSTHIS',
  'WHATSTHISMODE',
  // what a form draws on itself
  'CLS',
  'DRAW',
  'BOX',
  'CIRCLE',
  'LINE',
  'POINT',
  'PSET',
  'TEXTHEIGHT',
  'TEXTWIDTH',
  'DOSCROLL',
  'SETVIEWPORT',
  // the list controls
  'ADDITEM',
  'ADDLISTITEM',
  'REMOVEITEM',
  'REMOVELISTITEM',
  'CLEAR',
  'REQUERY',
  'MOVEITEM',
  'INDEXTOITEMID',
  'ITEMIDTOINDEX',
  // the grid
  'ACTIVATECELL',
  'ADDCOLUMN',
  'DELETECOLUMN',
  'AUTOFIT',
  'DOSTATUS',
  // the timer, the page frame and the toolbar
  'RESET',
  'GETPAGEHEIGHT',
  'GETPAGEWIDTH',
  'DOCK',
  'GETDOCKSTATE',
  // what a drag carries, and what starts one
  'DRAG',
  'OLEDRAG',
  'SETDATA',
  'GETDATA',
  'CLEARDATA',
  'SETFORMAT',
  'GETFORMAT',
  // what a collection is asked about an item
  'GETKEY',
  // what a form writes on itself
  'PRINT',
  // the application object
  'QUIT',
  'HELP',
  'DOCMD',
  'EVAL',
  'SETVAR',
  'REQUESTDATA',
  'DATATOCLIP',
  'DOMESSAGE',
  'CLEARSTATUS',
  'UPDATESTATUS',
];

/**
 * The events a command raises rather than an object.
 *
 * READ names five procedures in its own clauses and runs each at its moment: the reference
 * calls them events because that is what they are to the program that writes them.
 */
export const COMMAND_EVENTS = ['READWHEN', 'READSHOW', 'READACTIVATE', 'READDEACTIVATE', 'READVALID'];

/**
 * What the Project object and the File objects in it answer to: a builder reads a project and
 * acts on it, which is what these do.
 */
export const PROJECT_METHODS = [
  'BUILD',
  'CLEANUP',
  'CLOSE',
  'MODIFY',
  'RUN',
  'SAVEAS',
  'SETMAIN',
  'REMOVE',
  // source control: the provider behind them is git, and a project outside a working tree
  // answers .F. the way VFP's do when there is no provider
  'ADDTOSCC',
  'CHECKIN',
  'CHECKOUT',
  'GETLATESTVERSION',
  'REMOVEFROMSCC',
  'UNDOCHECKOUT',
];

/**
 * Methods this runtime knows by name and answers with a reason rather than doing.
 *
 * Each of them acts on something that is not here - a source-control provider, an embedded OLE
 * document, a design surface - so the honest answer is to say which, and the coverage map
 * counts them apart from the ones that work.
 */
export const REPORTED_METHODS = [
  // an embedded OLE document has verbs; an automation object, which is what the COM bridge
  // reaches, has none
  'DOVERB',
];

/**
 * What an XMLAdapter and the XMLTable and XMLField objects under it answer to.
 */
export const XML_METHODS = [
  'LOADXML',
  'TOXML',
  'TOCURSOR',
  // the rows a diffgram says have changed, as a cursor of their own
  'CHANGESTOCURSOR',
  'ADDTABLESCHEMA',
  'APPLYDIFFGRAM',
  'RELEASEXML',
  'ATTACH',
  'NEST',
  'UNNEST',
  'GETFORMAT',
  'SETFORMAT',
];

/**
 * What a ReportListener answers to. The report engine calls these as it works through a report,
 * so a listener a program subclasses inherits them whether it overrides them or not.
 */
export const REPORT_METHODS = [
  'CANCELREPORT',
  'GETPAGEHEIGHT',
  'GETPAGEWIDTH',
  'INCLUDEPAGEINOUTPUT',
  'OUTPUTPAGE',
  'RENDER',
  'SUPPORTSLISTENERTYPE',
  'ONPREVIEWCLOSE',
];

/**
 * What a data environment and the cursors in it answer to. They are objects of their own, so
 * their methods are listed apart from the ones every control has.
 */
export const DATA_METHODS = [
  'OPENTABLES',
  'CLOSETABLES',
  'CURSORFILL',
  'CURSORREFRESH',
  'CURSORATTACH',
  'CURSORDETACH',
  'RECORDREFRESH',
  'REQUERY',
  // the data environment opens a CursorAdapter's cursor by calling this
  'AUTOOPEN',
  // a memo the cursor did not fetch with the record: this engine fetches every one as it is
  // read, so what a delayed fetch asks for is already there
  'DELAYEDMEMOFETCH',
];
