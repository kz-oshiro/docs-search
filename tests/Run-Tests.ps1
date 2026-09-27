Set-StrictMode -Version 2.0
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

$root = Join-Path ([System.IO.Path]::GetTempPath()) ('doc-search-test-' + [guid]::NewGuid().ToString('N'))
[void][System.IO.Directory]::CreateDirectory($root)

function Write-TestWorkbook([string]$Path) {
    $entries = @{
        'xl/workbook.xml' = '<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Main" sheetId="1" r:id="rId1"/></sheets></workbook>'
        'xl/_rels/workbook.xml.rels' = '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/></Relationships>'
        'xl/sharedStrings.xml' = '<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><si><t>Alpha needle</t></si><si><r><t>Rich </t></r><r><t>needle</t></r></si></sst>'
        'xl/worksheets/sheet1.xml' = '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheetData><row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1" t="inlineStr"><is><t>Inline needle</t></is></c><c r="C1"><v>42</v></c><c r="D1" t="str"><f>"needle"</f><v>needle</v></c></row><row r="2"><c r="A2" t="s"><v>1</v></c></row></sheetData><drawing r:id="rId3"/><legacyDrawing r:id="rId4"/></worksheet>'
        'xl/worksheets/_rels/sheet1.xml.rels' = '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing1.xml"/><Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/vmlDrawing" Target="../drawings/vmlDrawing1.vml"/></Relationships>'
        'xl/drawings/drawing1.xml' = '<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><xdr:twoCellAnchor><xdr:from><xdr:col>2</xdr:col><xdr:row>3</xdr:row></xdr:from><xdr:grpSp><xdr:sp><xdr:nvSpPr><xdr:cNvPr id="1" name="Callout"/></xdr:nvSpPr><xdr:txBody><a:p><a:r><a:t>Shape </a:t></a:r><a:r><a:t>needle</a:t></a:r></a:p></xdr:txBody></xdr:sp></xdr:grpSp></xdr:twoCellAnchor></xdr:wsDr>'
        'xl/drawings/vmlDrawing1.vml' = '<xml xmlns:v="urn:schemas-microsoft-com:vml" xmlns:x="urn:schemas-microsoft-com:office:excel"><v:shape id="LegacyBox"><v:textbox><div>VML needle</div></v:textbox><x:ClientData><x:Row>4</x:Row><x:Column>1</x:Column></x:ClientData></v:shape></xml>'
    }
    $stream = [System.IO.File]::Open($Path, [System.IO.FileMode]::CreateNew)
    try {
        $archive = New-Object System.IO.Compression.ZipArchive($stream, [System.IO.Compression.ZipArchiveMode]::Create, $true)
        try {
            foreach ($name in $entries.Keys) {
                $entry = $archive.CreateEntry($name)
                $writer = New-Object System.IO.StreamWriter($entry.Open(), (New-Object System.Text.UTF8Encoding($false)))
                try { $writer.Write($entries[$name]) }
                finally { $writer.Dispose() }
            }
        }
        finally { $archive.Dispose() }
    }
    finally { $stream.Dispose() }
}

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

try {
    $book = Join-Path $root 'needle-book.xlsx'
    Write-TestWorkbook $book
    [System.IO.File]::WriteAllText((Join-Path $root 'broken.xlsx'), 'not a zip')
    $engine = Join-Path (Split-Path $PSScriptRoot -Parent) 'Search-Excel.ps1'
    $items = @(& $engine -Root $root -Query 'needle')
    $hits = @($items | Where-Object Type -eq 'Result')
    $issues = @($items | Where-Object Type -eq 'Error')
    Assert-True ($hits.Count -eq 7) "Expected 7 hits; got $($hits.Count)"
    Assert-True (@($hits | Where-Object { $_.Kind -eq 'セル' -and $_.Location -eq 'A1' }).Count -eq 1) 'Shared string not found'
    Assert-True (@($hits | Where-Object { $_.Kind -eq 'セル' -and $_.Location -eq 'B1' }).Count -eq 1) 'Inline string not found'
    Assert-True (@($hits | Where-Object { $_.Kind -eq 'セル' -and $_.Location -eq 'D1' }).Count -eq 1) 'Cached formula value not found'
    Assert-True (@($hits | Where-Object { $_.Kind -eq '図形' -and $_.Location -eq 'Callout (C4)' }).Count -eq 1) 'Grouped shape not found'
    Assert-True (@($hits | Where-Object { $_.Kind -eq '図形' -and $_.Location -eq 'LegacyBox (B5)' }).Count -eq 1) 'VML box not found'
    Assert-True ($issues.Count -eq 1) "Expected one broken-file error; got $($issues.Count)"
    $numbers = @(& $engine -Root $root -Query '42' | Where-Object Type -eq 'Result')
    Assert-True (@($numbers | Where-Object { $_.Location -eq 'C1' }).Count -eq 1) 'Numeric cell not found'
    $job = Start-Job -ScriptBlock {
        param($scriptPath, $folder)
        & $scriptPath -Root $folder -Query 'needle'
    } -ArgumentList $engine, $root
    try {
        [void](Wait-Job -Job $job -Timeout 15)
        Assert-True ($job.State -eq 'Completed') "Background job failed: $($job.State)"
        $jobItems = @(Receive-Job -Job $job -ErrorAction Stop)
        Assert-True (@($jobItems | Where-Object Type -eq 'Result').Count -eq 7) 'Background results missing'
    }
    finally { Remove-Job -Job $job -Force }
    Write-Output 'All doc-search engine tests passed.'
}
finally {
    if ($root.StartsWith([System.IO.Path]::GetTempPath(), [System.StringComparison]::OrdinalIgnoreCase)) {
        Remove-Item -LiteralPath $root -Recurse -Force
    }
}
