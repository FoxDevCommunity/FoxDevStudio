*
*	GDI+ Class library for Visual Foxpro
*
#ifndef _GDIPLUS_H_INCLUDED

* Localisation
#include "gdiplus_locs.h"

* Modify_GDIPLUS.VCX behavior (recompile _GDIPLUS.VCX to take effect)
* Set these constants to .F. to bypass most parameter checking
* (Code will run faster, but more dangerously)
#define GDIPLUS_CHECK_PARAMS	.T.	&& Check parameter types
#define GDIPLUS_CHECK_OBJECT	.T.	&& Check GDI+ object handle
#define GDIPLUS_CHECK_GDIPLUSNOTINIT	.T.	&& Throw error if GDI+ not initialised

* Classes instantiated from gdiplus.vcx
* If you subclass anything in _gdiplus.vcx, you MUST change at least
* GDIPLUS_CLASS_LIBRARY
#define GDIPLUS_CLASS_LIBRARY	This.ClassLibrary
* #define GDIPLUS_CLASS_LIBRARY	'_gdiplus.vcx'
#define GDIPLUS_CLASS_RECT		'GpRectangle'
#define GDIPLUS_CLASS_POINT		'GpPoint'
#define GDIPLUS_CLASS_SIZE		'GpSize'
#define GDIPLUS_CLASS_FONTFAMILY	'GpFontFamily'
#define GDIPLUS_CLASS_IMAGE		'GpImage'
#define GDIPLUS_CLASS_BITMAP	'GpBitmap'
#define GDIPLUS_CLASS_GRAPHICS	'GpGraphics'

* Control error handler behavior (default for all objects: you
* can also change this per-object)
* If you want to change these modes in the GpBase.Init() method
* then uncomment and adjust the following
*#define GDIPLUS_ERRHANDLER_ALLOWMODAL	(inlist(_VFP.StartMode,0,4))
*#define GDIPLUS_ERRHANDLER_QUIET		(not inlist(_VFP.StartMode,0,4))
*#define GDIPLUS_ERRHANDLER_IGNOREERRORS	.F.
*#define GDIPLUS_ERRHANDLER_APPNAME		"GDI+ FFC Library"

* Set to .T. to rethrow errors inside error handler (eg when debugging)
#define GDIPLUS_ERRHANDLER_RETHROW	.F.


* Status enumeration
#define GDIPLUS_STATUS_OK	0
#define GDIPLUS_STATUS_GenericError  1
#define GDIPLUS_STATUS_InvalidParameter  2
#define GDIPLUS_STATUS_OutOfMemory  3
#define GDIPLUS_STATUS_ObjectBusy  4
#define GDIPLUS_STATUS_InsufficientBuffer  5
#define GDIPLUS_STATUS_NotImplemented  6
#define GDIPLUS_STATUS_Win32Error  7
#define GDIPLUS_STATUS_WrongState  8
#define GDIPLUS_STATUS_Aborted  9
#define GDIPLUS_STATUS_FileNotFound  10
#define GDIPLUS_STATUS_ValueOverflow  11
#define GDIPLUS_STATUS_AccessDenied  12
#define GDIPLUS_STATUS_UnknownImageFormat  13
#define GDIPLUS_STATUS_FontFamilyNotFound  14
#define GDIPLUS_STATUS_FontStyleNotFound  15
#define GDIPLUS_STATUS_NotTrueTypeFont  16
#define GDIPLUS_STATUS_UnsupportedGdiplusVersion  17
#define GDIPLUS_STATUS_GdiplusNotInitialized  18
#define GDIPLUS_STATUS_PropertyNotFound  19
#define GDIPLUS_STATUS_PropertyNotSupported	20



* Fill mode (how a closed path is filled)
#define GDIPLUS_FillMode_Alternate	0
#define GDIPLUS_FillMode_Winding		1


* Quality mode constants
#define GDIPLUS_QualityMode_Invalid   -1
#define GDIPLUS_QualityMode_Default   0
#define GDIPLUS_QualityMode_Low       1	&& Best performance
#define GDIPLUS_QualityMode_High      2  && Best rendering quality

* Alpha Compositing mode constants
#define GDIPLUS_CompositingMode_SourceOver	0
#define GDIPLUS_CompositingMode_SourceCopy	1

* Alpha Compositing quality constants
#define	GDIPLUS_CompositingQuality_Invalid          GDIPLUS_QualityMode_Invalid
#define	GDIPLUS_CompositingQuality_Default          GDIPLUS_QualityMode_Default
#define	GDIPLUS_CompositingQuality_HighSpeed        GDIPLUS_QualityMode_Low
#define	GDIPLUS_CompositingQuality_HighQuality      GDIPLUS_QualityMode_High
#define	GDIPLUS_CompositingQuality_GammaCorrected	3
#define	GDIPLUS_CompositingQuality_AssumeLinear		4

* Units
#define	GDIPLUS_Unit_World      0 && World coordinate (non-physical unit)
#define	GDIPLUS_Unit_Display    1 && Variable -- for PageTransform only
#define	GDIPLUS_Unit_Pixel      2 && one device pixel.
#define	GDIPLUS_Unit_Point      3 && 1/72 inch.
#define	GDIPLUS_Unit_Inch       4 && 1 inch.
#define	GDIPLUS_Unit_Document   5 && 1/300 inch.
#define	GDIPLUS_Unit_Millimeter 6 && 1 millimeter.

#define	GDIPLUS_MetafileFrameUnit_Pixel      GDIPLUS_Unit_Pixel
#define	GDIPLUS_MetafileFrameUnit_Point      GDIPLUS_Unit_Point
#define	GDIPLUS_MetafileFrameUnit_Inch       GDIPLUS_Unit_Inch
#define	GDIPLUS_MetafileFrameUnit_Document   GDIPLUS_Unit_Document
#define	GDIPLUS_MetafileFrameUnit_Millimeter GDIPLUS_Unit_Millimeter
#define	GDIPLUS_MetafileFrameUnit_Gdi        7	&& GDI compatible .01 MM units


* Coordinate Space
#define	GDIPLUS_CoordinateSpace_World      0
#define	GDIPLUS_CoordinateSpace_Page       1
#define	GDIPLUS_CoordinateSpace_Device     2

* Wrap mode for brushes
#define	GDIPLUS_WrapMode_Tile		0
#define	GDIPLUS_WrapMode_TileFlipX	1
#define	GDIPLUS_WrapMode_TileFlipY	2
#define	GDIPLUS_WrapMode_TileFlipXY	3
#define	GDIPLUS_WrapMode_Clamp		4


* HatchBrush styles
#define	GDIPLUS_HatchStyle_Horizontal	0
#define	GDIPLUS_HatchStyle_Vertical	1
#define	GDIPLUS_HatchStyle_ForwardDiagonal	2
#define	GDIPLUS_HatchStyle_BackwardDiagonal	3
#define	GDIPLUS_HatchStyle_Cross	4
#define	GDIPLUS_HatchStyle_DiagonalCross	5
#define	GDIPLUS_HatchStyle_05Percent	6
#define	GDIPLUS_HatchStyle_10Percent	7
#define	GDIPLUS_HatchStyle_20Percent	8
#define	GDIPLUS_HatchStyle_25Percent	9
#define	GDIPLUS_HatchStyle_30Percent	10
#define	GDIPLUS_HatchStyle_40Percent	11
#define	GDIPLUS_HatchStyle_50Percent	12
#define GDIPLUS_HatchStyle_60Percent	13
#define GDIPLUS_HatchStyle_70Percent	14
#define GDIPLUS_HatchStyle_75Percent	15
#define GDIPLUS_HatchStyle_80Percent	16
#define GDIPLUS_HatchStyle_90Percent	17
#define GDIPLUS_HatchStyle_LightDownwardDiagonal	18
#define GDIPLUS_HatchStyle_LightUpwardDiagonal	19
#define GDIPLUS_HatchStyle_DarkDownwardDiagonal	20
#define GDIPLUS_HatchStyle_DarkUpwardDiagonal	21
#define GDIPLUS_HatchStyle_WideDownwardDiagonal	22
#define GDIPLUS_HatchStyle_WideUpwardDiagonal	23
#define GDIPLUS_HatchStyle_LightVertical	24
#define GDIPLUS_HatchStyle_LightHorizontal	25
#define GDIPLUS_HatchStyle_NarrowVertical	26
#define GDIPLUS_HatchStyle_NarrowHorizontal	27
#define GDIPLUS_HatchStyle_DarkVertical	28
#define GDIPLUS_HatchStyle_DarkHorizontal	29
#define GDIPLUS_HatchStyle_DashedDownwardDiagonal	30
#define GDIPLUS_HatchStyle_DashedUpwardDiagonal	31
#define GDIPLUS_HatchStyle_DashedHorizontal	32
#define GDIPLUS_HatchStyle_DashedVertical	33
#define GDIPLUS_HatchStyle_SmallConfetti	34
#define GDIPLUS_HatchStyle_LargeConfetti	35
#define GDIPLUS_HatchStyle_ZigZag	36
#define GDIPLUS_HatchStyle_Wave	37
#define GDIPLUS_HatchStyle_DiagonalBrick	38
#define GDIPLUS_HatchStyle_HorizontalBrick	39
#define GDIPLUS_HatchStyle_Weave	40
#define GDIPLUS_HatchStyle_Plaid	41
#define GDIPLUS_HatchStyle_Divot	42
#define GDIPLUS_HatchStyle_DottedGrid	43
#define GDIPLUS_HatchStyle_DottedDiamond	44
#define GDIPLUS_HatchStyle_Shingle	45
#define GDIPLUS_HatchStyle_Trellis	46
#define GDIPLUS_HatchStyle_Sphere	47
#define GDIPLUS_HatchStyle_SmallGrid	48
#define GDIPLUS_HatchStyle_SmallCheckerBoard	49
#define GDIPLUS_HatchStyle_LargeCheckerBoard	50
#define GDIPLUS_HatchStyle_OutlinedDiamond	51
#define GDIPLUS_HatchStyle_SolidDiamond	52


* Dash style constants

#define GDIPLUS_DashStyle_Solid	0
#define GDIPLUS_DashStyle_Dash	1
#define GDIPLUS_DashStyle_Dot	2
#define GDIPLUS_DashStyle_DashDot	3
#define GDIPLUS_DashStyle_DashDotDot	4
#define GDIPLUS_DashStyle_Custom          	5

* Dash cap constants
#define GDIPLUS_DashCap_Flat             	0
#define GDIPLUS_DashCap_Round            	2
#define GDIPLUS_DashCap_Triangle         	3

* LineCap
#define GDIPLUS_LineCap_Flat             0
#define GDIPLUS_LineCap_Square           1
#define GDIPLUS_LineCap_Round            2
#define GDIPLUS_LineCap_Triangle         3
#define GDIPLUS_LineCap_NoAnchor         0x10 && corresponds to flat cap
#define GDIPLUS_LineCap_SquareAnchor     0x11 && corresponds to square cap
#define GDIPLUS_LineCap_RoundAnchor      0x12 && corresponds to round cap
#define GDIPLUS_LineCap_DiamondAnchor    0x13 && corresponds to triangle cap
#define GDIPLUS_LineCap_ArrowAnchor      0x14 && no correspondence
#define GDIPLUS_LineCap_Custom           0xff && custom cap
#define GDIPLUS_LineCap_AnchorMask       0xf0 && mask to check for anchor or not.

* Custom Line cap type constants
#define GDIPLUS_CustomLineCapType_Default         	0
#define GDIPLUS_CustomLineCapType_AdjustableArrow 	1

* Line join constants
#define GDIPLUS_LineJoin_Miter        	0
#define GDIPLUS_LineJoin_Bevel        	1
#define GDIPLUS_LineJoin_Round        	2
#define GDIPLUS_LineJoin_MiterClipped 	3

* Path point types (only the lowest 8 bits are used.)
*  The lowest 3 bits are interpreted as point type
*  The higher 5 bits are reserved for flags.
#define GDIPLUS_PathPointType_Start           0    && move
#define GDIPLUS_PathPointType_Line            1    && line
#define GDIPLUS_PathPointType_Bezier          3    && default Bezier (= cubic Bezier)
#define GDIPLUS_PathPointType_PathTypeMask    0x07 && type mask (lowest 3 bits).
#define GDIPLUS_PathPointType_DashMode        0x10 && currently in dash mode.
#define GDIPLUS_PathPointType_PathMarker      0x20 && a marker for the path.
#define GDIPLUS_PathPointType_CloseSubpath    0x80 && closed flag
#define GDIPLUS_PathPointType_Bezier3    3         && cubic Bezier


* WarpMode constants
#define GDIPLUS_WarpMode_Perspective	0
#define GDIPLUS_WarpMode_Bilinear    1


* LinearGradient Mode
#define GDIPLUS_LinearGradientMode_Horizontal	0
#define GDIPLUS_LinearGradientMode_Vertical	1
#define GDIPLUS_LinearGradientMode_ForwardDiagonal	2
#define GDIPLUS_LinearGradientMode_BackwardDiagonal    	3

* CombineMode (for regions)
#define GDIPLUS_CombineMode_Replace	0
#define GDIPLUS_CombineMode_Intersect	1
#define GDIPLUS_CombineMode_Union	2
#define GDIPLUS_CombineMode_Xor	3
#define GDIPLUS_CombineMode_Exclude	4
#define GDIPLUS_CombineMode_Complement   	5

* Image types
#define GDIPLUS_ImageType_Unknown	0
#define GDIPLUS_ImageType_Bitmap	1
#define GDIPLUS_ImageType_Metafile   	2


* StringAlignment enumeration
* Applies to GpStringFormat::Alignment, GpStringFormat::LineAlignment
#define GDIPLUS_STRINGALIGNMENT_Near		0	&& in Left-To-Right locale, this is Left
#define GDIPLUS_STRINGALIGNMENT_Center	1
#define GDIPLUS_STRINGALIGNMENT_Far		2	&& in Left-To-Right locale, this is Right


* StringFormatFlags enumeration
* applies to GpStringFormat::FormatFlags
#define GDIPLUS_STRINGFORMATFLAGS_DirectionRightToLeft	1 
#define GDIPLUS_STRINGFORMATFLAGS_DirectionVertical 2 
#define GDIPLUS_STRINGFORMATFLAGS_NoFitBlackBox 4 
#define GDIPLUS_STRINGFORMATFLAGS_DisplayFormatControl 32 
#define GDIPLUS_STRINGFORMATFLAGS_NoFontFallback 1024 
#define GDIPLUS_STRINGFORMATFLAGS_MeasureTrailingSpaces 2048 
#define GDIPLUS_STRINGFORMATFLAGS_NoWrap 4096 
#define GDIPLUS_STRINGFORMATFLAGS_LineLimit 8192 
#define GDIPLUS_STRINGFORMATFLAGS_NoClip 16384 

* StringTrimming enumeration
#define GDIPLUS_STRINGTRIMMING_None 		0	&& no trimming. 
#define GDIPLUS_STRINGTRIMMING_Character 1	&& nearest character. 
#define GDIPLUS_STRINGTRIMMING_Word		2	&& nearest wor 
#define GDIPLUS_STRINGTRIMMING_EllipsisCharacter 3	&& nearest character, ellipsis at end
#define GDIPLUS_STRINGTRIMMING_EllipsisWord 	4	&& nearest word, ellipsis at end
#define GDIPLUS_STRINGTRIMMING_EllipsisPath 	5	&& ellipsis in center, favouring last slash-delimited segment

* StringDigitSubstitute
#define GDIPLUS_STRINGDIGITSUBSTITUTE_User 	0
#define GDIPLUS_STRINGDIGITSUBSTITUTE_None 	1
#define GDIPLUS_STRINGDIGITSUBSTITUTE_National	2
#define GDIPLUS_STRINGDIGITSUBSTITUTE_Traditional 	3

* HotkeyPrefix enumeration
#define GDIPLUS_HOTKEYPREFIX_None 0	&& No hot-key prefix. 
#define GDIPLUS_HOTKEYPREFIX_Show 1	&& display hot-key prefix
#define GDIPLUS_HOTKEYPREFIX_Hide 2	&& Do not display the hot-key prefix. 

* FontStyle: face types and common styles
#define GDIPLUS_FontStyle_Regular     0
#define GDIPLUS_FontStyle_Bold        1
#define GDIPLUS_FontStyle_Italic      2
#define GDIPLUS_FontStyle_BoldItalic  3
#define GDIPLUS_FontStyle_Underline   4
#define GDIPLUS_FontStyle_Strikeout   8

#define GDIPLUS_InterpolationMode_Invalid          GDIPLUS_QualityMode_Invalid
#define GDIPLUS_InterpolationMode_Default          GDIPLUS_QualityMode_Default
#define GDIPLUS_InterpolationMode_LowQuality       GDIPLUS_QualityMode_Low
#define GDIPLUS_InterpolationMode_HighQuality      GDIPLUS_QualityMode_High
#define GDIPLUS_InterpolationMode_Bilinear			3
#define GDIPLUS_InterpolationMode_Bicubic			4
#define GDIPLUS_InterpolationMode_NearestNeighbor	5
#define GDIPLUS_InterpolationMode_HighQualityBilinear	6
#define GDIPLUS_InterpolationMode_HighQualityBicubic	7

#define GDIPLUS_PenAlignment_Center       	0
#define GDIPLUS_PenAlignment_Inset        	1

* Brush types
#define GDIPLUS_BrushType_SolidColor       	0
#define GDIPLUS_BrushType_HatchFill        	1
#define GDIPLUS_BrushType_TextureFill      	2
#define GDIPLUS_BrushType_PathGradient     	3
#define GDIPLUS_BrushType_LinearGradient   	4

* Pen's Fill types
#define GDIPLUS_PenType_SolidColor       GDIPLUS_BrushType_SolidColor
#define GDIPLUS_PenType_HatchFill        GDIPLUS_BrushType_HatchFill
#define GDIPLUS_PenType_TextureFill      GDIPLUS_BrushType_TextureFill
#define GDIPLUS_PenType_PathGradient     GDIPLUS_BrushType_PathGradient
#define GDIPLUS_PenType_LinearGradient   GDIPLUS_BrushType_LinearGradient
#define GDIPLUS_PenType_Unknown          -1

* Matrix Order
#define GDIPLUS_MatrixOrder_Prepend    0
#define GDIPLUS_MatrixOrder_Append     1

* SmoothingMode
#define GDIPLUS_SmoothingMode_Invalid     GDIPLUS_QualityMode_Invalid
#define GDIPLUS_SmoothingMode_Default     GDIPLUS_QualityMode_Default
#define GDIPLUS_SmoothingMode_HighSpeed   GDIPLUS_QualityMode_Low,
#define GDIPLUS_SmoothingMode_HighQuality GDIPLUS_QualityMode_High
#define GDIPLUS_SmoothingMode_None		3
#define GDIPLUS_SmoothingMode_AntiAlias	4

* PixelOffsetMode
#define GDIPLUS_PixelOffsetMode_Invalid		GDIPLUS_QualityMode_Invalid
#define GDIPLUS_PixelOffsetMode_Default		GDIPLUS_QualityMode_Default
#define GDIPLUS_PixelOffsetMode_HighSpeed	GDIPLUS_QualityMode_Low
#define GDIPLUS_PixelOffsetMode_HighQuality	GDIPLUS_QualityMode_High
#define GDIPLUS_PixelOffsetMode_None			3
#define GDIPLUS_PixelOffsetMode_Half			4


* GpGraphics::Flush() modes
#define GDIPLUS_FlushIntention_Flush	0
#define GDIPLUS_FlushIntention_Sync	1




*---------------------------------------------------------------------------
* Image file format identifiers (GUIDs)
#define GDIPLUS_IMAGEFORMAT_Undefined	0hA93C6BB92807D3119D7B0000F81EF32E
#define GDIPLUS_IMAGEFORMAT_MemoryBMP	0hAA3C6BB92807D3119D7B0000F81EF32E
#define GDIPLUS_IMAGEFORMAT_BMP	0hAB3C6BB92807D3119D7B0000F81EF32E
#define GDIPLUS_IMAGEFORMAT_EMF	0hAC3C6BB92807D3119D7B0000F81EF32E
#define GDIPLUS_IMAGEFORMAT_WMF	0hAD3C6BB92807D3119D7B0000F81EF32E
#define GDIPLUS_IMAGEFORMAT_JPEG	0hAE3C6BB92807D3119D7B0000F81EF32E
#define GDIPLUS_IMAGEFORMAT_PNG	0hAF3C6BB92807D3119D7B0000F81EF32E
#define GDIPLUS_IMAGEFORMAT_GIF	0hB03C6BB92807D3119D7B0000F81EF32E
#define GDIPLUS_IMAGEFORMAT_TIFF	0hB13C6BB92807D3119D7B0000F81EF32E
#define GDIPLUS_IMAGEFORMAT_EXIF	0hB23C6BB92807D3119D7B0000F81EF32E
#define GDIPLUS_IMAGEFORMAT_Icon	0hB53C6BB92807D3119D7B0000F81EF32E

* Pixel formats
#define	GDIPLUS_PIXELFORMAT_Indexed      0x00010000 && Indexes into a palette
#define	GDIPLUS_PIXELFORMAT_GDI          0x00020000 && Is a GDI-supported format
#define	GDIPLUS_PIXELFORMAT_Alpha        0x00040000 && Has an alpha component
#define	GDIPLUS_PIXELFORMAT_PAlpha       0x00080000 && Pre-multiplied alpha
#define	GDIPLUS_PIXELFORMAT_Extended     0x00100000 && Extended color 16 bits/channel
#define	GDIPLUS_PIXELFORMAT_Canonical    0x00200000 
#define	GDIPLUS_PIXELFORMAT_Undefined       0
#define	GDIPLUS_PIXELFORMAT_DontCare        0

#define	GDIPLUS_PIXELFORMAT_1bppIndexed     	0x00030101
#define	GDIPLUS_PIXELFORMAT_4bppIndexed     	0x00030402
#define	GDIPLUS_PIXELFORMAT_8bppIndexed     	0x00030803
#define	GDIPLUS_PIXELFORMAT_16bppGrayScale  	0x00101004
#define	GDIPLUS_PIXELFORMAT_16bppRGB555     	0x00021005
#define	GDIPLUS_PIXELFORMAT_16bppRGB565     	0x00021006
#define	GDIPLUS_PIXELFORMAT_16bppARGB1555   	0x00061007
#define	GDIPLUS_PIXELFORMAT_24bppRGB        	0x00021808
#define	GDIPLUS_PIXELFORMAT_32bppRGB        	0x00022009
#define	GDIPLUS_PIXELFORMAT_32bppARGB       	0x0026200A
#define	GDIPLUS_PIXELFORMAT_32bppPARGB      	0x000E200B
#define	GDIPLUS_PIXELFORMAT_48bppRGB        	0x0010300C
#define	GDIPLUS_PIXELFORMAT_64bppPARGB      	0x001C400E

* --------------
* Image flags (see GpImage::Flags property)
 #define GDIPLUS_ImageFlags_None			0
 #define GDIPLUS_ImageFlags_Scalable		0x0001
 #define GDIPLUS_ImageFlags_HasAlpha		0x0002
 #define GDIPLUS_ImageFlags_HasTranslucent	0x0004
 #define GDIPLUS_ImageFlags_PartiallyScalable	0x0008
 #define GDIPLUS_ImageFlags_ColorSpaceRGB	0x0010
 #define GDIPLUS_ImageFlags_ColorSpaceCMYK	0x0020
 #define GDIPLUS_ImageFlags_ColorSpaceGRAY	0x0040
 #define GDIPLUS_ImageFlags_ColorSpaceYCBCR	0x0080
 #define GDIPLUS_ImageFlags_ColorSpaceYCCK	0x0100
 #define GDIPLUS_ImageFlags_HasRealDPI		0x1000
 #define GDIPLUS_ImageFlags_HasRealPixelSize	0x2000
 #define GDIPLUS_ImageFlags_ReadOnly		0x00010000
 #define GDIPLUS_ImageFlags_Caching			0x00020000



* -------------
* Encoder parameter type
#define GDIPLUS_ValueDataType_Byte			1	&& 8-bit unsigned
#define GDIPLUS_ValueDataType_ASCII			2	&& character string
#define GDIPLUS_ValueDataType_Short			3	&& 16-bit unsigned
#define GDIPLUS_ValueDataType_Long			4	&& 32-bit unsigned
#define GDIPLUS_ValueDataType_Rational		5	&& fraction ulong/ulong
#define GDIPLUS_ValueDataType_LongRange		6	&& Two ulongs (min,max)
#define GDIPLUS_ValueDataType_Undefined		7	&& array of bytes
#define GDIPLUS_ValueDataType_RationalRange	8	&& four ulongs
#define GDIPLUS_ValueDataType_Pointer		9	&& pointer

#define GDIPLUS_ENCODER_Compression	0h9D739DE0D4CCEE448EBA3FBF8BE4FC58
#define GDIPLUS_ENCODER_ColorDepth	0h5570086666AD7C4C9A1838A2310B8337
#define GDIPLUS_ENCODER_ScanMethod	0h61264E3A0931564E853642C156E7DCFA
#define GDIPLUS_ENCODER_Version	0h768CD1244A81A441BF531C219CCCF797
#define GDIPLUS_ENCODER_RenderMethod	0h3AC5426D9A2225488BB75C99E2B9A8B8
#define GDIPLUS_ENCODER_Quality	0hB5E45B1D4AFA2D459CDD5DB35105E7EB
#define GDIPLUS_ENCODER_Transformation	0hD1B20E8D8EA5A84EAA14108074B7B6F9
#define GDIPLUS_ENCODER_LuminanceTable	0hCE3BB3ED6602774AB90427216099E717
#define GDIPLUS_ENCODER_ChrominanceTable	0hDC55E4F2B30916438260676ADA32481C
#define GDIPLUS_ENCODER_SaveFlag	0hFC66222940ACBF478CFCA85B89A655DE

* GpImage::RotateFlip() parameter
#define GDIPLUS_ROTATEFLIPTYPE_RotateNoneFlipNone 0
#define GDIPLUS_ROTATEFLIPTYPE_Rotate90FlipNone   1
#define GDIPLUS_ROTATEFLIPTYPE_Rotate180FlipNone  2
#define GDIPLUS_ROTATEFLIPTYPE_Rotate270FlipNone  3

#define GDIPLUS_ROTATEFLIPTYPE_RotateNoneFlipX    4
#define GDIPLUS_ROTATEFLIPTYPE_Rotate90FlipX      5
#define GDIPLUS_ROTATEFLIPTYPE_Rotate180FlipX     6
#define GDIPLUS_ROTATEFLIPTYPE_Rotate270FlipX     7

#define GDIPLUS_ROTATEFLIPTYPE_RotateNoneFlipY    GDIPLUS_ROTATEFLIPTYPE_Rotate180FlipX
#define GDIPLUS_ROTATEFLIPTYPE_Rotate90FlipY      GDIPLUS_ROTATEFLIPTYPE_Rotate270FlipX
#define GDIPLUS_ROTATEFLIPTYPE_Rotate180FlipY     GDIPLUS_ROTATEFLIPTYPE_RotateNoneFlipX
#define GDIPLUS_ROTATEFLIPTYPE_Rotate270FlipY     GDIPLUS_ROTATEFLIPTYPE_Rotate90FlipX

#define GDIPLUS_ROTATEFLIPTYPE_RotateNoneFlipXY   GDIPLUS_ROTATEFLIPTYPE_Rotate180FlipNone
#define GDIPLUS_ROTATEFLIPTYPE_Rotate90FlipXY     GDIPLUS_ROTATEFLIPTYPE_Rotate270FlipNone
#define GDIPLUS_ROTATEFLIPTYPE_Rotate180FlipXY    GDIPLUS_ROTATEFLIPTYPE_RotateNoneFlipNone
#define GDIPLUS_ROTATEFLIPTYPE_Rotate270FlipXY    GDIPLUS_ROTATEFLIPTYPE_Rotate90FlipNone

#endif && _GDIPLUS_H_INCLUDED