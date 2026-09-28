Set-StrictMode -Version 2.0
$ErrorActionPreference = 'Stop'
$env:PYTHONDONTWRITEBYTECODE = '1'

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

$root = Join-Path ([System.IO.Path]::GetTempPath()) ('docs-search-test-' + [guid]::NewGuid().ToString('N'))
[void][System.IO.Directory]::CreateDirectory($root)
try {
    $generator = Join-Path $PSScriptRoot 'generate-fixtures.py'
    & python -m unittest discover -s $PSScriptRoot -p 'test_*.py'
    Assert-True ($LASTEXITCODE -eq 0) 'Fixture generator verification failed'
    & python $generator --output $root
    Assert-True ($LASTEXITCODE -eq 0) 'Fixture generator failed'

    $corpus = Join-Path $root 'search'
    $manifest = Get-Content -LiteralPath (Join-Path $root 'manifest.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    $cases = Get-Content -LiteralPath (Join-Path $root 'backend-cases.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    Assert-True ($manifest.seed -eq $cases.seed) 'Generator seed and backend cases disagree'
    Assert-True ($manifest.files.Count -eq 32) "Expected 32 corpus files; got $($manifest.files.Count)"
    Assert-True ($cases.cases.Count -eq 18) "Expected 18 backend cases; got $($cases.cases.Count)"
    foreach ($name in @('spreadsheets/needle-book.xlsx', 'nested/quarterly-ledger.xlsm', 'nested/team-briefing.pptx', 'documents/operations-guide.docx', 'notes/research-log.txt')) {
        Assert-True (Test-Path -LiteralPath (Join-Path $corpus $name)) "Missing generated file: $name"
    }

    $engine = Join-Path (Split-Path $PSScriptRoot -Parent) 'powershell/Search-Excel.ps1'
    $items = @(& $engine -Root $corpus -Query 'needle')
    $hits = @($items | Where-Object Type -eq 'Result')
    $issues = @($items | Where-Object Type -eq 'Error')
    Assert-True ($hits.Count -eq 7) "Expected 7 Excel hits; got $($hits.Count)"
    Assert-True (@($hits | Where-Object { $_.Sheet -eq 'Main' -and $_.Location -eq 'A1' }).Count -eq 1) 'Shared string not found'
    Assert-True (@($hits | Where-Object { $_.Sheet -eq 'Main' -and $_.Location -eq 'B1' }).Count -eq 1) 'Inline string not found'
    Assert-True (@($hits | Where-Object { $_.Sheet -eq 'Main' -and $_.Location -eq 'D1' }).Count -eq 1) 'Cached formula value not found'
    Assert-True (@($hits | Where-Object { $_.Location -eq 'Callout (C4)' }).Count -eq 1) 'Grouped shape not found'
    Assert-True (@($hits | Where-Object { $_.Location -eq 'LegacyBox (B5)' }).Count -eq 1) 'VML box not found'
    Assert-True ($issues.Count -eq 1) "Expected one broken Excel error; got $($issues.Count)"

    $numbers = @(& $engine -Root $corpus -Query '42' | Where-Object Type -eq 'Result')
    Assert-True (@($numbers | Where-Object { $_.Location -eq 'C1' }).Count -eq 1) 'Numeric cell not found'
    $macro = @(& $engine -Root $corpus -Query 'BEACON' | Where-Object Type -eq 'Result')
    Assert-True (@($macro | Where-Object { $_.Sheet -eq 'Hidden' -and $_.Location -eq 'A1' }).Count -eq 1) 'Hidden Excel sheet not found'
    Assert-True (@($macro | Where-Object { $_.Path -like '*.xlsm' -and $_.Location -eq 'A1' }).Count -eq 1) 'XLSM content not found'

    $job = Start-Job -ScriptBlock {
        param($scriptPath, $folder)
        & $scriptPath -Root $folder -Query 'needle'
    } -ArgumentList $engine, $corpus
    try {
        [void](Wait-Job -Job $job -Timeout 30)
        Assert-True ($job.State -eq 'Completed') "Background job failed: $($job.State)"
        $jobItems = @(Receive-Job -Job $job -ErrorAction Stop)
        Assert-True (@($jobItems | Where-Object Type -eq 'Result').Count -eq 7) 'Background results missing'
    }
    finally { Remove-Job -Job $job -Force }
    Write-Output 'All docs-search PowerShell engine tests passed.'
}
finally {
    $temp = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath()).TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar
    $resolved = [System.IO.Path]::GetFullPath($root)
    if ($resolved.StartsWith($temp, [System.StringComparison]::OrdinalIgnoreCase) -and (Test-Path -LiteralPath $resolved)) {
        Remove-Item -LiteralPath $resolved -Recurse -Force
    }
}
