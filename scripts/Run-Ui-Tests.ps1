param([string]$Case)

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$repository = Split-Path -Parent $here
$ui = Join-Path $repository 'tests\ui'
if (-not (Get-Command node -ErrorAction SilentlyContinue)) { throw 'Node.js 20 以上が必要です。' }
$nodeVersion = & node --version
if ($LASTEXITCODE -ne 0 -or [int]($nodeVersion.TrimStart('v').Split('.')[0]) -lt 20) { throw 'Node.js 20 以上が必要です。' }
if (-not (Get-Command npm.cmd -ErrorAction SilentlyContinue)) { throw 'npm.cmd が必要です。' }

$previousSite = $env:DOCS_SEARCH_UI_SITE
$previousArtifacts = $env:DOCS_SEARCH_UI_ARTIFACTS
Push-Location $ui
try {
    # This checks files only; it does not install dependencies or launch a browser.
    & node (Join-Path $ui 'preflight.mjs')
    if ($LASTEXITCODE -ne 0) { throw 'tests/ui で npm.cmd ci と npm.cmd exec -- playwright install chromium を実行してください。' }
    $runName = (Get-Date -Format 'yyyyMMdd-HHmmss') + '-' + [Guid]::NewGuid().ToString('N')
    $artifacts = Join-Path $repository "outputs\ui-tests\$runName"
    New-Item -ItemType Directory -Path $artifacts | Out-Null
    $env:DOCS_SEARCH_UI_SITE = Join-Path $artifacts 'site'
    $env:DOCS_SEARCH_UI_ARTIFACTS = $artifacts
    & (Join-Path $here 'Build-Frontend.ps1') -OutputDirectory $env:DOCS_SEARCH_UI_SITE -Profile debug -TargetDirectory (Join-Path $repository 'outputs\ui-tests-cache\target')
    $playwrightArguments = @('exec', '--', 'playwright', 'test')
    if ($Case) { $playwrightArguments += @('--grep', $Case) }
    & npm.cmd @playwrightArguments
    if ($LASTEXITCODE -ne 0) { throw "Playwright UI 試験に失敗しました。テキスト記録: $artifacts" }
    Write-Host "ヘッドレス UI 試験完了。テキスト記録: $artifacts"
} finally {
    $env:DOCS_SEARCH_UI_SITE = $previousSite
    $env:DOCS_SEARCH_UI_ARTIFACTS = $previousArtifacts
    Pop-Location
}
