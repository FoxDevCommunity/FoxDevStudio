* The three classes that read and write XML: XMLAdapter, XMLTable and XMLField.
LOCAL oXml, oTable, oField
oXml = CREATEOBJECT("XMLAdapter")
? oXml.BaseClass, oXml.Class, oXml.Name, "[" + oXml.ParentClass + "]"
? "[" + oXml.XMLName + "]", "[" + oXml.XMLNamespace + "]", "[" + oXml.XMLPrefix + "]", oXml.XMLNameIsXPath
? TRANSFORM(oXml.CodePage), oXml.UseCodePage, oXml.Unicode, oXml.UTF8Encoded, oXml.NoCpTrans
? oXml.IsLoaded, oXml.IsDiffGram, oXml.FormattedOutput, oXml.PreserveWhiteSpace
? oXml.RespectNesting, oXml.RespectCursorCP, oXml.DeclareXMLPrefix, oXml.DisableEncode
? oXml.ForceCloseTag, oXml.WrapCharInCDATA, oXml.WrapMemoInCDATA, oXml.MapBinary, oXml.MapVarchar
? oXml.MapN19_4ToCurrency, TRANSFORM(oXml.XMLSchemaLocation), "[" + oXml.SelectionNamespaces + "]"
? VARTYPE(oXml.SOM), VARTYPE(oXml.XMLConstraints), VARTYPE(oXml.IXMLDOMElement)
* XMLName is left alone: the product refuses a write to it, and the measurement cannot see that,
* because it asks the question by writing back the value the property already holds
oXml.FormattedOutput = .F.
oXml.RespectNesting = .T.
? oXml.FormattedOutput, oXml.RespectNesting
? PEMSTATUS(oXml, "LoadXML", 5), PEMSTATUS(oXml, "ToXML", 5), PEMSTATUS(oXml, "ReleaseXML", 5)
? PEMSTATUS(oXml, "AddTableSchema", 5), PEMSTATUS(oXml, "Attach", 5), PEMSTATUS(oXml, "Tables", 5)
? PEMSTATUS(oXml, "ApplyDiffgram", 5), PEMSTATUS(oXml, "ToCursor", 5), PEMSTATUS(oXml, "Nest", 5)

* one table inside a document, and how it sits inside another
oTable = CREATEOBJECT("XMLTable")
? oTable.BaseClass, oTable.Class, oTable.Name, "[" + oTable.Alias + "]"
? "[" + oTable.XMLName + "]", VARTYPE(oTable.NestedInto), "[" + oTable.XMLNamespace + "]"
? VARTYPE(oTable.ParentTable), VARTYPE(oTable.ChildTable), VARTYPE(oTable.FirstNestedTable)
? VARTYPE(oTable.NextSiblingTable), VARTYPE(oTable.XMLAdapter), TRANSFORM(oTable.CodePage)
? PEMSTATUS(oTable, "ApplyDiffgram", 5), PEMSTATUS(oTable, "ChangesToCursor", 5), PEMSTATUS(oTable, "ToCursor", 5)
? PEMSTATUS(oTable, "Nest", 5), PEMSTATUS(oTable, "Unnest", 5), PEMSTATUS(oTable, "Fields", 5)
? PEMSTATUS(oTable, "LoadXML", 5), PEMSTATUS(oTable, "ToXML", 5)

* one column of one of them
oField = CREATEOBJECT("XMLField")
? oField.BaseClass, oField.Class, oField.Name, "[" + oField.Alias + "]", "[" + oField.DataType + "]"
? oField.IsAttribute, oField.IsBase64, oField.IsBinary, oField.IsNull, oField.Keyfield
? TRANSFORM(oField.MaxLength), TRANSFORM(oField.FractionDigits), oField.WrapInCDATA, oField.Unicode
? VARTYPE(oField.XMLType), "[" + oField.XSDtype + "]", "[" + oField.XSDmaxLength + "]"
? "[" + oField.XSDfractionDigits + "]", "[" + oField.XSDtotalDigits + "]", VARTYPE(oField.XMLTable)
? oField.Unicode, oField.NoCpTrans, oField.DisableEncode, "[" + oField.ClassLibrary + "]"
* COVERS: XMLAdapter, XMLTable, XMLField, XMLName, XMLNamespace, XMLPrefix, XMLNameIsXPath,
* COVERS: UseCodePage, UTF8Encoded, IsLoaded, IsDiffGram, FormattedOutput, PreserveWhiteSpace,
* COVERS: RespectNesting, RespectCursorCP, DeclareXMLPrefix, DisableEncode, ForceCloseTag,
* COVERS: WrapCharInCDATA, WrapMemoInCDATA, MapN19_4ToCurrency, XMLSchemaLocation,
* COVERS: SelectionNamespaces, SOM, XMLConstraints, IXMLDOMElement, LoadXML, ToXML, ReleaseXML,
* COVERS: AddTableSchema, Attach, ApplyDiffgram, ToCursor, Nest, Unnest, ChangesToCursor,
* COVERS: NestedInto, ParentTable, ChildTable, FirstNestedTable, NextSiblingTable, DataType,
* COVERS: IsAttribute, IsBase64, IsBinary, IsNull, Keyfield, FractionDigits, WrapInCDATA,
* COVERS: XMLType, XSDtype, XSDmaxLength, XSDfractionDigits, XSDtotalDigits, CodePage, NoCpTrans,
* COVERS: Unicode, ClassLibrary
