$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$docSearch = Split-Path -Parent $here

Push-Location $docSearch
try {
    & (Join-Path $here 'Run-Automated-Tests.ps1')

    & (Join-Path $here 'Build.ps1')
    if (-not (Test-Path -LiteralPath (Join-Path $docSearch 'outputs\build\docs-search-desktop.exe'))) {
        throw 'ビルド成果物が見つかりません。'
    }
} finally {
    Pop-Location
}

Write-Host 'ローカル CI 完了。'
