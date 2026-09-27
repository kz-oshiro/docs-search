param(
    [Parameter(Mandatory = $true)][string]$Root,
    [Parameter(Mandatory = $true)][string]$Query
)

Set-StrictMode -Version 2.0
$ErrorActionPreference = 'Stop'

$script:SpreadsheetNs = 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'
$script:DocumentRelNs = 'http://schemas.openxmlformats.org/officeDocument/2006/relationships'
$script:PackageRelNs = 'http://schemas.openxmlformats.org/package/2006/relationships'
$script:DrawingNs = 'http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing'
$script:DrawingTextNs = 'http://schemas.openxmlformats.org/drawingml/2006/main'
$script:VmlNs = 'urn:schemas-microsoft-com:vml'
$script:ExcelVmlNs = 'urn:schemas-microsoft-com:office:excel'

function New-XmlSettings {
    $settings = New-Object System.Xml.XmlReaderSettings
    $settings.DtdProcessing = [System.Xml.DtdProcessing]::Prohibit
    $settings.XmlResolver = $null
    $settings.MaxCharactersFromEntities = 0
    return $settings
}

function Get-EntryXml([System.IO.Compression.ZipArchive]$Archive, [string]$Path) {
    $entry = $Archive.GetEntry($Path)
    if ($null -eq $entry) { return $null }
    $stream = $entry.Open()
    try {
        $reader = [System.Xml.XmlReader]::Create($stream, (New-XmlSettings))
        try { return [System.Xml.Linq.XDocument]::Load($reader) }
        finally { $reader.Dispose() }
    }
    finally { $stream.Dispose() }
}

function Get-AttributeValue($Element, [string]$Name) {
    if ($null -eq $Element) { return '' }
    $attribute = $Element.Attribute([System.Xml.Linq.XName]::Get($Name))
    if ($null -eq $attribute) { return '' }
    return [string]$attribute.Value
}

function Get-RelatedPath([string]$SourcePath, [string]$Target) {
    if ([string]::IsNullOrWhiteSpace($Target)) { return $null }
    $base = [System.Uri]::new(('https://package.invalid/' + $SourcePath))
    $resolved = [System.Uri]::new($base, $Target)
    if ($resolved.Host -ne 'package.invalid') { return $null }
    return [System.Uri]::UnescapeDataString($resolved.AbsolutePath.TrimStart('/'))
}

function Get-RelationshipMap([System.IO.Compression.ZipArchive]$Archive, [string]$SourcePath) {
    $directory = [System.IO.Path]::GetDirectoryName($SourcePath).Replace('\', '/')
    $filename = [System.IO.Path]::GetFileName($SourcePath)
    $relsPath = if ($directory) { "$directory/_rels/$filename.rels" } else { "_rels/$filename.rels" }
    $document = Get-EntryXml $Archive $relsPath
    $map = @{}
    if ($null -eq $document) { return $map }
    $name = [System.Xml.Linq.XName]::Get('Relationship', $script:PackageRelNs)
    foreach ($rel in $document.Descendants($name)) {
        if ((Get-AttributeValue $rel 'TargetMode') -eq 'External') { continue }
        $id = Get-AttributeValue $rel 'Id'
        $target = Get-RelatedPath $SourcePath (Get-AttributeValue $rel 'Target')
        if ($id -and $target) { $map[$id] = $target }
    }
    return $map
}

function Get-JoinedText($Container, [System.Xml.Linq.XName]$ParagraphName, [System.Xml.Linq.XName]$TextName) {
    $paragraphs = New-Object 'System.Collections.Generic.List[string]'
    foreach ($paragraph in $Container.Descendants($ParagraphName)) {
        $builder = New-Object System.Text.StringBuilder
        foreach ($run in $paragraph.Descendants($TextName)) { [void]$builder.Append($run.Value) }
        $paragraphs.Add($builder.ToString())
    }
    return [string]::Join("`n", $paragraphs.ToArray())
}

function Get-SharedStrings([System.IO.Compression.ZipArchive]$Archive, [string]$Path) {
    $strings = New-Object 'System.Collections.Generic.List[string]'
    $document = Get-EntryXml $Archive $Path
    if ($null -eq $document) { return ,$strings }
    $itemName = [System.Xml.Linq.XName]::Get('si', $script:SpreadsheetNs)
    $textName = [System.Xml.Linq.XName]::Get('t', $script:SpreadsheetNs)
    foreach ($item in $document.Descendants($itemName)) {
        $builder = New-Object System.Text.StringBuilder
        foreach ($textNode in $item.Descendants($textName)) { [void]$builder.Append($textNode.Value) }
        $strings.Add($builder.ToString())
    }
    return ,$strings
}

function Test-Match([string]$Value) {
    if ([string]::IsNullOrEmpty($Value)) { return $false }
    return $Value.IndexOf($Query, [System.StringComparison]::OrdinalIgnoreCase) -ge 0
}

function New-Hit([string]$Path, [string]$Kind, [string]$Sheet, [string]$Location, [string]$Value) {
    return [pscustomobject]@{
        Type = 'Result'; Kind = $Kind; Path = $Path; Sheet = $Sheet
        Location = $Location; Value = $Value
    }
}

function Get-CellValue($Cell, $SharedStrings) {
    $type = Get-AttributeValue $Cell 't'
    $valueName = [System.Xml.Linq.XName]::Get('v', $script:SpreadsheetNs)
    $valueNode = $Cell.Element($valueName)
    if ($type -eq 'inlineStr') {
        $inline = $Cell.Element([System.Xml.Linq.XName]::Get('is', $script:SpreadsheetNs))
        if ($null -eq $inline) { return '' }
        $builder = New-Object System.Text.StringBuilder
        foreach ($textNode in $inline.Descendants([System.Xml.Linq.XName]::Get('t', $script:SpreadsheetNs))) {
            [void]$builder.Append($textNode.Value)
        }
        return $builder.ToString()
    }
    if ($null -eq $valueNode) { return '' }
    if ($type -eq 's') {
        $index = 0
        if (-not [int]::TryParse($valueNode.Value, [ref]$index) -or $index -lt 0 -or $index -ge $SharedStrings.Count) {
            throw "共有文字列の参照が不正です: $($valueNode.Value)"
        }
        return $SharedStrings[$index]
    }
    if ($type -eq 'b') { if ($valueNode.Value -eq '1') { return 'TRUE' } else { return 'FALSE' } }
    return [string]$valueNode.Value
}

function Convert-ColumnToLetters([int]$Column) {
    $result = ''
    do {
        $result = [char](65 + ($Column % 26)) + $result
        $Column = [math]::Floor($Column / 26) - 1
    } while ($Column -ge 0)
    return $result
}

function Get-Anchor($Shape) {
    $anchors = @('twoCellAnchor', 'oneCellAnchor', 'absoluteAnchor')
    $parent = $Shape.Parent
    while ($null -ne $parent) {
        if ($parent.Name.NamespaceName -eq $script:DrawingNs -and $anchors -contains $parent.Name.LocalName) {
            $from = $parent.Element([System.Xml.Linq.XName]::Get('from', $script:DrawingNs))
            if ($null -eq $from) { return '' }
            $col = $from.Element([System.Xml.Linq.XName]::Get('col', $script:DrawingNs))
            $row = $from.Element([System.Xml.Linq.XName]::Get('row', $script:DrawingNs))
            if ($null -ne $col -and $null -ne $row) {
                return (Convert-ColumnToLetters ([int]$col.Value)) + ([int]$row.Value + 1)
            }
            return ''
        }
        $parent = $parent.Parent
    }
    return ''
}

function Search-Drawing([System.IO.Compression.ZipArchive]$Archive, [string]$DrawingPath, [string]$FilePath, [string]$Sheet) {
    $document = Get-EntryXml $Archive $DrawingPath
    if ($null -eq $document) { return }
    $shapeName = [System.Xml.Linq.XName]::Get('sp', $script:DrawingNs)
    $bodyName = [System.Xml.Linq.XName]::Get('txBody', $script:DrawingNs)
    $propertyName = [System.Xml.Linq.XName]::Get('cNvPr', $script:DrawingNs)
    $paragraphName = [System.Xml.Linq.XName]::Get('p', $script:DrawingTextNs)
    $textName = [System.Xml.Linq.XName]::Get('t', $script:DrawingTextNs)
    foreach ($shape in $document.Descendants($shapeName)) {
        $body = $shape.Element($bodyName)
        if ($null -eq $body) { continue }
        $value = Get-JoinedText $body $paragraphName $textName
        if (-not (Test-Match $value)) { continue }
        $property = $shape.Descendants($propertyName) | Select-Object -First 1
        $name = Get-AttributeValue $property 'name'
        $anchor = Get-Anchor $shape
        $location = if ($anchor -and $name) { "$name ($anchor)" } elseif ($name) { $name } else { $anchor }
        New-Hit $FilePath '図形' $Sheet $location $value
    }
}

function Search-Vml([System.IO.Compression.ZipArchive]$Archive, [string]$DrawingPath, [string]$FilePath, [string]$Sheet) {
    $document = Get-EntryXml $Archive $DrawingPath
    if ($null -eq $document) { return }
    $boxName = [System.Xml.Linq.XName]::Get('textbox', $script:VmlNs)
    foreach ($box in $document.Descendants($boxName)) {
        $value = $box.Value.Trim()
        if (-not (Test-Match $value)) { continue }
        $shape = $box.Parent
        $name = Get-AttributeValue $shape 'id'
        $clientData = $shape.Element([System.Xml.Linq.XName]::Get('ClientData', $script:ExcelVmlNs))
        $row = if ($null -ne $clientData) { $clientData.Element([System.Xml.Linq.XName]::Get('Row', $script:ExcelVmlNs)) } else { $null }
        $col = if ($null -ne $clientData) { $clientData.Element([System.Xml.Linq.XName]::Get('Column', $script:ExcelVmlNs)) } else { $null }
        $anchor = if ($null -ne $row -and $null -ne $col) { (Convert-ColumnToLetters ([int]$col.Value)) + ([int]$row.Value + 1) } else { '' }
        $location = if ($name -and $anchor) { "$name ($anchor)" } elseif ($name) { $name } else { $anchor }
        New-Hit $FilePath '図形' $Sheet $location $value
    }
}

function Search-Worksheet([System.IO.Compression.ZipArchive]$Archive, [string]$SheetPath, [string]$FilePath, [string]$Sheet, $SharedStrings) {
    $document = Get-EntryXml $Archive $SheetPath
    if ($null -eq $document) { throw "シートが見つかりません: $SheetPath" }
    $cellName = [System.Xml.Linq.XName]::Get('c', $script:SpreadsheetNs)
    foreach ($cell in $document.Descendants($cellName)) {
        $value = Get-CellValue $cell $SharedStrings
        if (Test-Match $value) { New-Hit $FilePath 'セル' $Sheet (Get-AttributeValue $cell 'r') $value }
    }
    $relationships = Get-RelationshipMap $Archive $SheetPath
    foreach ($elementName in @('drawing', 'legacyDrawing')) {
        $qualifiedName = [System.Xml.Linq.XName]::Get($elementName, $script:SpreadsheetNs)
        foreach ($drawing in $document.Descendants($qualifiedName)) {
            $idAttribute = $drawing.Attribute([System.Xml.Linq.XName]::Get('id', $script:DocumentRelNs))
            if ($null -eq $idAttribute -or -not $relationships.ContainsKey($idAttribute.Value)) { continue }
            $drawingPath = $relationships[$idAttribute.Value]
            if ($elementName -eq 'drawing') { Search-Drawing $Archive $drawingPath $FilePath $Sheet }
            else { Search-Vml $Archive $drawingPath $FilePath $Sheet }
        }
    }
}

function Search-Workbook([string]$Path) {
    $stream = New-Object System.IO.FileStream($Path, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, [System.IO.FileShare]::ReadWrite)
    try {
        $archive = New-Object System.IO.Compression.ZipArchive($stream, [System.IO.Compression.ZipArchiveMode]::Read, $true)
        try {
            $workbookPath = 'xl/workbook.xml'
            $workbook = Get-EntryXml $archive $workbookPath
            if ($null -eq $workbook) { throw 'Excel ブックの構成が見つかりません' }
            $relationships = Get-RelationshipMap $archive $workbookPath
            $sharedPath = 'xl/sharedStrings.xml'
            foreach ($rel in (Get-EntryXml $archive 'xl/_rels/workbook.xml.rels').Descendants([System.Xml.Linq.XName]::Get('Relationship', $script:PackageRelNs))) {
                if ((Get-AttributeValue $rel 'Type').EndsWith('/sharedStrings')) {
                    $id = Get-AttributeValue $rel 'Id'
                    if ($relationships.ContainsKey($id)) { $sharedPath = $relationships[$id] }
                }
            }
            $sharedStrings = Get-SharedStrings $archive $sharedPath
            $sheetName = [System.Xml.Linq.XName]::Get('sheet', $script:SpreadsheetNs)
            foreach ($sheet in $workbook.Descendants($sheetName)) {
                $name = Get-AttributeValue $sheet 'name'
                $relId = $sheet.Attribute([System.Xml.Linq.XName]::Get('id', $script:DocumentRelNs))
                if ($null -eq $relId -or -not $relationships.ContainsKey($relId.Value)) { continue }
                Search-Worksheet $archive $relationships[$relId.Value] $Path $name $sharedStrings
            }
        }
        finally { $archive.Dispose() }
    }
    finally { $stream.Dispose() }
}

if (-not (Test-Path -LiteralPath $Root -PathType Container)) { throw "フォルダーが見つかりません: $Root" }
if ([string]::IsNullOrWhiteSpace($Query)) { throw '検索語を入力してください' }
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
Add-Type -AssemblyName System.Xml.Linq

$traversalErrors = @()
$files = @(Get-ChildItem -LiteralPath $Root -File -Recurse -ErrorAction SilentlyContinue -ErrorVariable traversalErrors |
    Where-Object { $_.Extension -in @('.xlsx', '.xlsm') -and -not $_.Name.StartsWith('~$') } |
    Sort-Object FullName)
foreach ($issue in $traversalErrors) {
    [pscustomobject]@{ Type = 'Error'; Path = [string]$issue.TargetObject; Message = $issue.Exception.Message }
}
$count = 0
foreach ($file in $files) {
    $count++
    [pscustomobject]@{ Type = 'Progress'; Current = $count; Total = $files.Count; Path = $file.FullName }
    if (Test-Match $file.Name) { New-Hit $file.FullName 'ファイル名' '' '' $file.Name }
    try { Search-Workbook $file.FullName }
    catch { [pscustomobject]@{ Type = 'Error'; Path = $file.FullName; Message = $_.Exception.Message } }
}
[pscustomobject]@{ Type = 'Done'; Total = $files.Count }
