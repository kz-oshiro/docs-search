param(
    [ValidateSet('acceptance', 'load')]
    [string]$Profile = 'acceptance',
    [switch]$NoOpen
)

Set-StrictMode -Version 2.0
$ErrorActionPreference = 'Stop'

$python = Get-Command python -ErrorAction SilentlyContinue
if (-not $python) {
    throw 'Python 3 was not found. Install Python 3 and add python to PATH.'
}

$generator = Join-Path $PSScriptRoot 'generate-fixtures.py'
$repoRoot = Split-Path -Parent $PSScriptRoot
$outputRoot = Join-Path (Join-Path $repoRoot 'outputs') 'test-data'
[void][System.IO.Directory]::CreateDirectory($outputRoot)
$runName = '{0}-{1}-{2}' -f $Profile, (Get-Date -Format 'yyyyMMdd-HHmmss'), ([guid]::NewGuid().ToString('N').Substring(0, 8))
$output = Join-Path $outputRoot $runName

& $python.Source $generator --output $output --profile $Profile
if ($LASTEXITCODE -ne 0) {
    throw "Test data generation failed with exit code $LASTEXITCODE."
}
if (-not (Test-Path -LiteralPath (Join-Path $output 'manifest.json') -PathType Leaf)) {
    throw "The generator did not create manifest.json in $output."
}

Write-Host "Output: $output"
if (-not $NoOpen) {
    Invoke-Item -LiteralPath $output
}
