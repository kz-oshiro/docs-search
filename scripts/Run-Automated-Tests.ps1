param(
    [string]$RankingReport
)

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$docSearch = Split-Path -Parent $here

Push-Location $docSearch
try {
    python .\tests\fixtures\test_generator.py
    if ($LASTEXITCODE -ne 0) { throw '共通テストデータの検証に失敗しました。' }

    python .\tests\cli\run-backend-cases.py
    if ($LASTEXITCODE -ne 0) { throw 'バックエンドの共通ケースに失敗しました。' }

    python .\tests\cli\run-context-cases.py
    if ($LASTEXITCODE -ne 0) { throw 'Excel 周辺検索の受け入れケースに失敗しました。' }

    python .\tests\cli\run-condition-cases.py
    if ($LASTEXITCODE -ne 0) { throw '高度な検索の受け入れケースに失敗しました。' }

    $officeArguments = @('.\tests\cli\run-office-search-cases.py')
    if ($RankingReport) { $officeArguments += @('--report', $RankingReport) }
    python @officeArguments
    if ($LASTEXITCODE -ne 0) { throw 'Office 検索範囲・順位・一括検索の受け入れケースに失敗しました。' }

    python .\tests\cli\run-issue-cases.py
    if ($LASTEXITCODE -ne 0) { throw 'Excel 抽出・位置順・更新日時の確認に失敗しました。' }

    python .\tests\cli\run-index-cases.py
    if ($LASTEXITCODE -ne 0) { throw '索引の鮮度・無効化・復旧の確認に失敗しました。' }

    cargo test --locked --manifest-path .\core\Cargo.toml
    if ($LASTEXITCODE -ne 0) { throw '検索コアのテストに失敗しました。' }

    cargo check --locked --manifest-path .\frontend\Cargo.toml --target wasm32-unknown-unknown
    if ($LASTEXITCODE -ne 0) { throw 'WASM の型検査に失敗しました。' }

    & (Join-Path $here 'Run-Ui-Tests.ps1')
} finally {
    Pop-Location
}

Write-Host 'v3.0.0 自動テスト完了（ヘッドレス UI を含む。実アプリ GUI・Windows 配布ビルドは対象外）。'
