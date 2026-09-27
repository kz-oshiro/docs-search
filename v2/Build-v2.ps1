param([switch]$Run)

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$dist = Join-Path $here 'dist'
$pkg = Join-Path $dist 'pkg'

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw 'Rust / Cargo が必要です。' }
if (-not (Get-Command wasm-bindgen -ErrorAction SilentlyContinue)) { throw 'wasm-bindgen-cli 0.2.129 が必要です。README を参照してください。' }
if (-not ((rustup target list --installed) -contains 'wasm32-unknown-unknown')) { throw 'rustup target add wasm32-unknown-unknown を実行してください。' }

cargo build --release --manifest-path (Join-Path $here 'frontend\Cargo.toml') --target wasm32-unknown-unknown
if ($LASTEXITCODE -ne 0) { throw 'WASM のビルドに失敗しました。' }

New-Item -ItemType Directory -Force -Path $pkg | Out-Null
$wasm = Join-Path $here 'frontend\target\wasm32-unknown-unknown\release\doc_search_ui.wasm'
wasm-bindgen --target web --out-dir $pkg $wasm
if ($LASTEXITCODE -ne 0) { throw 'WASM の JavaScript 接続コードを作れませんでした。' }
Copy-Item -LiteralPath (Join-Path $here 'frontend\index.html') -Destination $dist
Copy-Item -LiteralPath (Join-Path $here 'frontend\style.css') -Destination $dist
Copy-Item -LiteralPath (Join-Path $here 'frontend\boot.js') -Destination $dist

cargo build --release --manifest-path (Join-Path $here 'src-tauri\Cargo.toml')
if ($LASTEXITCODE -ne 0) { throw 'デスクトップアプリのビルドに失敗しました。' }

$app = Join-Path $here 'src-tauri\target\release\doc-search-desktop.exe'
Write-Host "ビルド完了: $app"
if ($Run) { & $app }
