$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$docSearch = Split-Path -Parent $here

Push-Location $docSearch
try {
    python .\tests\test_generator.py
    if ($LASTEXITCODE -ne 0) { throw '共通テストデータの検証に失敗しました。' }

    python .\tauri\tests\run-backend-cases.py
    if ($LASTEXITCODE -ne 0) { throw 'バックエンドの共通ケースに失敗しました。' }

    cargo test --manifest-path .\tauri\core\Cargo.toml
    if ($LASTEXITCODE -ne 0) { throw '検索コアのテストに失敗しました。' }

    cargo check --manifest-path .\tauri\frontend\Cargo.toml --target wasm32-unknown-unknown
    if ($LASTEXITCODE -ne 0) { throw 'WASM のチェックに失敗しました。' }

    & (Join-Path $here 'Build.ps1')
    if (-not (Test-Path -LiteralPath (Join-Path $here 'docs-search-desktop.exe'))) {
        throw 'ビルド成果物が見つかりません。'
    }
} finally {
    Pop-Location
}

Write-Host 'ローカル CI 完了。'
