param(
    [Parameter(Mandatory = $true)][string]$OutputDirectory,
    [ValidateSet('debug', 'release')][string]$Profile = 'release',
    [string]$TargetDirectory
)

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$repository = Split-Path -Parent $here
$destination = [System.IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $destination) { throw "画面生成先は新しいディレクトリを指定してください: $destination" }
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw 'Rust / Cargo が必要です。' }
if (-not (Get-Command wasm-bindgen -ErrorAction SilentlyContinue)) { throw 'wasm-bindgen-cli 0.2.129 が必要です。' }
$bindgenVersion = & wasm-bindgen --version
if ($LASTEXITCODE -ne 0 -or $bindgenVersion -ne 'wasm-bindgen 0.2.129') { throw 'wasm-bindgen-cli 0.2.129 が必要です。' }
if (-not ((rustup target list --installed) -contains 'wasm32-unknown-unknown')) { throw 'rustup target add wasm32-unknown-unknown を実行してください。' }

$arguments = @('build', '--locked', '--manifest-path', (Join-Path $repository 'frontend\Cargo.toml'), '--target', 'wasm32-unknown-unknown')
if ($Profile -eq 'release') { $arguments += '--release' }
$target = Join-Path $repository 'frontend\target'
if ($TargetDirectory) {
    $target = [System.IO.Path]::GetFullPath($TargetDirectory)
}
$arguments += @('--target-dir', $target)
& cargo @arguments
if ($LASTEXITCODE -ne 0) { throw 'WASM のビルドに失敗しました。' }

$pkg = Join-Path $destination 'pkg'
New-Item -ItemType Directory -Path $pkg -Force | Out-Null
$wasm = Join-Path $target "wasm32-unknown-unknown\$Profile\docs_search_ui.wasm"
& wasm-bindgen --target web --out-dir $pkg $wasm
if ($LASTEXITCODE -ne 0) { throw 'WASM の JavaScript 接続コードを作れませんでした。' }
foreach ($asset in @('index.html', 'style.css', 'theme.css', 'theme.js', 'boot.js')) {
    Copy-Item -LiteralPath (Join-Path $repository "frontend\$asset") -Destination $destination
}
