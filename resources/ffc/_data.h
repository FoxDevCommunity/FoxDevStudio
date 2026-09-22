* _DATA.H

***********************************
* Constants for classes
#DEFINE MB_ICONEXCLAMATION		48
#DEFINE MB_QUESTIONYESNO	36
#DEFINE MB_ISYES			6

***********************************
* strings for _cbodistinctvalues classes

#DEFINE NO_TABLE_LOC "The table alias must be specified in the controlSource property, in the format 'alias.column'."

***********************************
* strings for _cbolookup classes

#DEFINE NO_LOOKUP_TABLE_LOC "A lookup table must be specified in the 'lookup_table' property (combo box " + THIS.name + ")."
#DEFINE NO_DISPLAY_COLUMN_LOC "A display column must be specified in the 'display_column' property (combo box " + THIS.name + ")."
#DEFINE NO_RETURN_COLUMN_LOC "A return column must be specified in the 'return_column' property (combo box " + THIS.name + ")."

***********************************
* strings for _conflicts classes

#DEFINE CS_NOALIAS_LOC					"No cursor specified."
#DEFINE CS_ROWCONFLICT_LOC				"Conflicts on row:  "
#DEFINE CS_NOCONFLICTS1_LOC				"No conflicts found."
#DEFINE CS_NOCONFLICTS_LOC				"No more conflicts found."
#DEFINE CS_NOBUFFER_LOC					"Data buffering must be on to check conflicts."
#DEFINE WAIT_REVERTFAIL_LOC		"Revert failed"
#DEFINE WAIT_REVERT1_LOC		"1 row reverted"
#DEFINE	WAIT_UPDATEOK_LOC		"Update succeded"
#DEFINE	WAIT_UPDATEFAIL1_LOC	"Update failed, the primary key has changed"

***********************************
* strings for _execsp classes

#DEFINE BADPARMS_LOC	"You must pass at least 3 parameters to this class."

***********************************
* strings for _offline classes
#DEFINE C_NODATABASE_LOC	"No database or view is specified."
#DEFINE C_NOVIEWS_LOC		"No views specified to take offline."
#DEFINE C_OKONLINE_LOC		"View were successfully taken online."
#DEFINE C_RECORDCONLICT_LOC "A record conflict occured during the update with view: "
#DEFINE C_FAILONLINE_LOC 	"Failed to take online view: "
#DEFINE C_NOOPEN_LOC 		"Could not open following view: "
#DEFINE C_FAILOFFLINE_LOC	"Failed to take following view offline: "
#DEFINE C_OKOFFLINE_LOC		"View were successfully taken offline."

***********************************
* strings for _datachecker class

#define NOBUFF1_LOC 'Data buffering is not enabled.'
	#define WINDMSG_LOC "Invalid value passed to conflictmanager.handlerecord"
* Declare constants & variables
#define CR_LOC CHR(13)
#define SAVE_LOC "Do you want to overwrite the current value with your change?" + CR_LOC + "(Choose 'Cancel' to restore the original value.)"
#define CONFLICT_LOC "Data Conflict"
#define VERIFY_LOC "Verify Changes"
#define ORG_LOC "Original Value: "
#define CUR_LOC "Current Value: " 
#define CHG_LOC "Your change: "
#define MEMO_LOC " is a Memo field."
#define FIELD_LOC "Field: "
#define RECORD_LOC "Record Number: "
#define VALCHG1_LOC "A value has been changed by another user."
#define VALCHG2_LOC "A value has been changed."
#define SAVECHG_LOC 'Do you want to save your changes?'
#define SAVECHG2_LOC 'Save Changes'

***********************************
* strings for _datanavbtns class
#define NUM_LOC "Error Number: "
#define PROG_LOC "Procedure: "
#define MSG_LOC "Error Message: "
#define SELTABLE_LOC "Select Table:"
#define OPEN_LOC "Open"
#define SAVE1_LOC "Do you want to save your changes anyway?"
#define CONFLICT_LOC "Unable to resolve data conflict."
