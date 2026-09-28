param([switch]$Run)

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$frontendBuild = Join-Path $here '.frontend-build'
$pkg = Join-Path $frontendBuild 'pkg'
$app = Join-Path $here 'docs-search-desktop.exe'
$releaseDirectories = @(
    (Join-Path $here 'frontend\target\release'),
    (Join-Path $here 'frontend\target\wasm32-unknown-unknown\release'),
    (Join-Path $here 'src-tauri\target\release')
)

function Remove-BuildDirectory([string]$path) {
    $fullPath = [System.IO.Path]::GetFullPath($path)
    $root = [System.IO.Path]::GetFullPath($here).TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar
    if (-not $fullPath.StartsWith($root, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "作業ディレクトリ外は削除できません: $fullPath"
    }
    if (Test-Path -LiteralPath $fullPath) {
        if ((Get-Item -LiteralPath $fullPath).LinkType) {
            throw "リンク先を削除しないため処理を中止します: $fullPath"
        }
        try {
            Remove-Item -LiteralPath $fullPath -Recurse -Force -ErrorAction Stop
        } catch {
            # Windows PowerShell can report a missing child after deleting the directory.
            if (Test-Path -LiteralPath $fullPath) { throw }
        }
    }
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw 'Rust / Cargo が必要です。' }
if (-not (Get-Command wasm-bindgen -ErrorAction SilentlyContinue)) { throw 'wasm-bindgen-cli 0.2.129 が必要です。README を参照してください。' }
if (-not ((rustup target list --installed) -contains 'wasm32-unknown-unknown')) { throw 'rustup target add wasm32-unknown-unknown を実行してください。' }
$runningApp = Get-Process -Name 'docs-search-desktop' -ErrorAction SilentlyContinue |
    Where-Object { $_.Path -and $_.Path.StartsWith($here + '\', [System.StringComparison]::OrdinalIgnoreCase) }
if ($runningApp) { throw '起動中の docs-search-desktop.exe を閉じてからビルドしてください。' }

try {
    Remove-BuildDirectory $frontendBuild

    cargo build --release --manifest-path (Join-Path $here 'frontend\Cargo.toml') --target wasm32-unknown-unknown
    if ($LASTEXITCODE -ne 0) { throw 'WASM のビルドに失敗しました。' }

    New-Item -ItemType Directory -Force -Path $pkg | Out-Null
    $wasm = Join-Path $here 'frontend\target\wasm32-unknown-unknown\release\docs_search_ui.wasm'
    wasm-bindgen --target web --out-dir $pkg $wasm
    if ($LASTEXITCODE -ne 0) { throw 'WASM の JavaScript 接続コードを作れませんでした。' }
    Copy-Item -LiteralPath (Join-Path $here 'frontend\index.html') -Destination $frontendBuild
    Copy-Item -LiteralPath (Join-Path $here 'frontend\style.css') -Destination $frontendBuild
    Copy-Item -LiteralPath (Join-Path $here 'frontend\boot.js') -Destination $frontendBuild

    cargo build --release --manifest-path (Join-Path $here 'src-tauri\Cargo.toml')
    if ($LASTEXITCODE -ne 0) { throw 'デスクトップアプリのビルドに失敗しました。' }

    Copy-Item -LiteralPath (Join-Path $here 'src-tauri\target\release\docs-search-desktop.exe') -Destination $app -Force
} finally {
    Remove-BuildDirectory $frontendBuild
    foreach ($directory in $releaseDirectories) { Remove-BuildDirectory $directory }
}

Write-Host "ビルド完了: $app"
if ($Run) { & $app }
