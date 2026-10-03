# PowerShell 版の docs-search-ps への移行計画

作成日: 2026-10-03。M0〜M3のローカル移行と静的確認後、PowerShell版の自動テスト・配布ZIP作成・[v1.0.0公開](https://github.com/kz-oshiro/docs-search-ps/releases/tag/v1.0.0)、Rust版の自動テスト・Windowsビルド・[v3.0.0公開](https://github.com/kz-oshiro/docs-search/releases/tag/v3.0.0)を完了した。両リポジトリのmain反映により移行の公開も完了した。実アプリGUIは未確認。以下の初回記録と手順は履歴として保持し、最新の結果は移行先の [移行記録](https://github.com/kz-oshiro/docs-search-ps/blob/main/docs/migration-record.md)とRust側の [検証記録](validation-v3.0.0.md)を参照する。

2026-10-03 の文書レビューで、再開手順、独立したケース原本の説明、自動テストと配布ビルドの入口、過去の検証結果と現在の未実施範囲を修正した。再開時は移行記録を先に読み、依頼された M4 / M5 の未完了手順から進む。完了済みの M0〜M3 を繰り返さない。

2026-10-03 の追加修正依頼により、PowerShell側の本体2ファイルと専用ランナーを変更し、ジョブ受信関数と回帰試験を追加した。これは移行後の独立した保守変更である。M0 / M1の不変9コピーのハッシュは初回移行の出典として保持し、現在のPowerShellソースは移行先 `docs/migration-record.md` の最新SHA-256と `docs/powershell-source-fixes.md` を照合する。この修正時はRust側の実行ソース・共通生成器・ケース原本を変更せず、実行テストも未実施だった。その後の検証・公開は上記の最新記録を参照する。

目的は、PowerShell 版のソース・使用説明・開発手順を `docs-search-ps` に独立させ、`docs-search` を Rust コア / Tauri / Rust→WASM 版のリポジトリとして運用すること。Luna が作業するときは、以下の段階を順番に実施し、各段階の完了条件を記録する。

## 1. 移行開始前に確認した前提

| 項目 | 2026-10-03、M0 開始前の確認結果 |
| --- | --- |
| 移行元 | `C:\Users\kazum\codex-projects\docs-search` |
| 移行元の origin | `git@github.com:kz-oshiro/docs-search.git` |
| 移行先 | [kz-oshiro/docs-search-ps](https://github.com/kz-oshiro/docs-search-ps) |
| 移行先の origin | `git@github.com:kz-oshiro/docs-search-ps.git` |
| 移行先のローカル配置 | `C:\Users\kazum\codex-projects\docs-search-ps` を使用。確認時点では存在しない |
| 移行先のリモート状態 | 空。ブランチはまだない |
| 移行元の HEAD | `e875fbbaf4dbe09c833df878a289ae4f894d6342` |
| 作業ツリー | v3.0.0 関連の変更・未追跡ファイルが多数ある。HEAD と現在のファイルは一致しない |
| PowerShell 本体 | `powershell/Search-Excel.ps1`、`powershell/docs-search.ps1`、`powershell/Start-docs-search.vbs`（移行前の配置。移行先は [docs-search-ps](https://github.com/kz-oshiro/docs-search-ps)） |
| データ生成 | [生成器](../tests/generate-fixtures.py)・[ケース原本](../tests/backend-cases.json)・[生成器検証](../tests/test_generator.py) を Rust 側も使用 |
| 現行データの静的な期待値 | acceptance 33 文書、load 148 文書、ケース原本 27 件、固定 seed `20260927` |
| PowerShell の確認内容 | `tests/run-powershell-tests.ps1`（移行前の配置）は文書を共用するが、27 ケースを PowerShell API で実行するものではない |
| 自動テストの現行入口 | Rust は [Run-Automated-Tests.ps1](../tauri/Run-Automated-Tests.ps1)。配布ビルド込みは [Run-Local-CI.ps1](../tauri/Run-Local-CI.ps1) |

この表は移行前の記録であり、移行先のローカル配置は M1 で作成済みである。後続作業の開始時には移行記録と現在のリモート状態・HEAD・件数を再確認する。文書に記載済みの以前の検証結果は、移行後の検証結果として流用しない。

## 2. 今回の設計を固定する

1. `docs-search-ps` にも `powershell/` を置く。ファイル名・相対配置・画面の製品名は維持する。ルートへの平坦化は今回に含めない。
2. `docs-search` の `tauri/` はそのまま維持する。Rust ソース、Cargo/lock、Tauri 設定、ビルドスクリプト、WASM、Playwright、アイコン、回帰 seed は移動・改変しない。
3. PowerShell 本体と既存 PowerShell テストランナーはバイト単位でコピーする。検索仕様・出力形式・テストの期待値は変えない。
4. 共用している小さなテスト用ソースは両リポジトリに独立配置する。移行時点では同一内容、その後は各リポジトリで管理する。相互参照、submodule、symlink、共有パッケージ、自動同期を導入しない。
5. `docs/requirements.md` 等の現行仕様は Rust の機能を含むため、PowerShell の仕様として丸ごとコピーしない。PowerShell 用仕様は現行の PowerShell コード・README・既存テストから新規作成する。
6. 元の Git 履歴・タグ・Releases は `docs-search` に残す。新リポジトリはコピー内容から履歴を開始し、移行元 URL・HEAD・作業ツリーからのコピーであること・ハッシュを記録する。履歴書き換え、タグの移植、リリース作成は今回に含めない。
7. `outputs/`、生成文書、`target/`、`node_modules/`、配布 EXE、`.git/` はコピーしない。既存のログ・保存用文書も元の場所に残す。
8. 実装のみの依頼では M0〜M3 まで。テスト依頼があれば M4、リモート反映の依頼があれば M5 に進む。段階の完了と移行全体の完了を区別する。

PowerShell に Rust v3.0.0 の版番号や機能を付与しない。移行に伴うアプリ名変更、新機能追加、生成器の Excel 専用化は別の変更として扱う。

## 3. ファイルの配置を固定する

### 3.1 docs-search-ps へコピーするファイル

以下は許可リスト。ディレクトリ全体を再帰コピーしない。

| 移行元の相対パス | 移行先の相対パス | コピー後の扱い | docs-search 側 |
| --- | --- | --- | --- |
| `powershell/Search-Excel.ps1` | 同じ | バイト不変 | M2 で削除 |
| `powershell/docs-search.ps1` | 同じ | バイト不変 | M2 で削除 |
| `powershell/Start-docs-search.vbs` | 同じ | バイト不変 | M2 で削除 |
| `powershell/README.md` | 同じ | 移行先の使用説明に修正 | M2 で削除 |
| `tests/run-powershell-tests.ps1` | 同じ | バイト不変 | M2 で削除 |
| `tests/generate-fixtures.py` | 同じ | バイト不変 | 残す・変更しない |
| `tests/backend-cases.json` | 同じ | バイト不変 | 残す・変更しない |
| `tests/test_generator.py` | 同じ | バイト不変 | 残す・変更しない |
| `tests/Generate-Test-Data.ps1` | 同じ | バイト不変 | 残す・変更しない |
| `Generate-Test-Data.cmd` | 同じ | バイト不変 | 残す・変更しない |

**不変照合の対象は上表のうち README を除いた 9 ファイル。** PowerShell の `-Root` / `-Query`、テストランナーの `powershell/Search-Excel.ps1` 参照、VBS の同じフォルダーにある画面スクリプト参照を維持できる配置である。

`WindowsPowerShell\v1.0\powershell.exe` は Windows の標準パス。旧版名と誤認して置換しない。日本語を含む既存 `.ps1` の文字コード・BOM・改行もコピー時に変更しない。

### 3.2 docs-search-ps で新規作成する文書・設定

| パス | 必須内容 |
| --- | --- |
| `README.md` | 独立した PowerShell 版の概要、対応形式、必要環境、`powershell/Start-docs-search.vbs` からの起動、CLI、テスト入口、文書へのローカルリンク、Rust 版への外部リンク |
| `AGENTS.md` | タイトルを docs-search-ps とし、移行元の「設計・実装・テスト方針」「実装のみは静的確認まで」「テスト時は最新方針を読む」「依頼済み範囲を完了」「GUI の事前確認」の 5 項目を引き継ぐ。WASM/Playwright の 2 項目は Rust 専用のため引き継がず、PowerShell の CLI 自動試験と対話的 Windows Forms 確認の境界を記す |
| `.gitignore` | `/outputs/`、`/tests/generated/`、`**/__pycache__/`、`*.pyc`。Rust/Tauri/Node 向け項目は追加しない |
| `docs/requirements.md` | 下記の PowerShell 仕様。現在の実装に根拠を置く |
| `tests/README.md` | 5 個のテスト用ファイルの役割、必要環境、生成仕様と件数、実際の PowerShell 確認範囲、保存用生成と一時生成、テスト方針 |
| `docs/migration-record.md` | 作業日、元 URL/HEAD、移行元の dirty 状態、コピー一覧と SHA-256、実施段階、静的確認・実行確認・公開の結果、未実施項目、旧履歴へのリンク |

移行後の構成は次のとおり。

```text
docs-search-ps/
  .gitignore
  AGENTS.md
  README.md
  Generate-Test-Data.cmd
  powershell/
    README.md
    Search-Excel.ps1
    docs-search.ps1
    Start-docs-search.vbs
  tests/
    README.md
    Generate-Test-Data.ps1
    backend-cases.json
    generate-fixtures.py
    run-powershell-tests.ps1
    test_generator.py
  docs/
    requirements.md
    migration-record.md
```

PowerShell 仕様に最低限記載する内容:

- Windows PowerShell 5.1 / Windows Forms。実行に Python・Rust・Node・Excel 本体は不要。開発時のデータ生成・自動試験には Python 3 を使用する。
- CLI は `Search-Excel.ps1 -Root <フォルダー> -Query <語句>`。検索語が空白のみ、またはルートが存在しない場合は入力エラー。
- サブフォルダーの `.xlsx` / `.xlsm` を検索し、`~$` で始まる Office 一時ファイルは除外する。
- ファイル名、セルの保存値・保存済み数式結果、実装が読み取る DrawingML/VML の図形テキストを検索。非表示シートも検索する。
- 検索は `OrdinalIgnoreCase` の部分一致。Rust の NFC 正規化・case folding・あいまい検索・索引・条件検索・一括検索を PowerShell の機能として記載しない。
- CLI の `Result` は `Kind` / `Path` / `Sheet` / `Location` / `Value`、`Error` は `Path` / `Message`、`Progress` は `Current` / `Total` / `Path`、`Done` は `Total` を持つ。Rust の SearchRequest / Issue / Finished と同一 API であるとは記載しない。
- 読み取り・列挙エラーを記録し、読めるファイルの検索を継続する。GUI のフォルダー指定・検索・中断・結果から開く・場所コピー・エラー表示は既存コードに従う。
- `.xls`、グラフ、SmartArt、Excel の表示書式を反映した文字列は対象外。Excel/マクロの起動や再計算、元文書・索引・検索履歴への書き込みは行わない。
- 移行後にもファイル名は `docs-search.ps1`、画面の名前は `docs-search` のまま使用する。リポジトリ名は `docs-search-ps`。

コピーした生成器は Word・PowerPoint・テキスト等も作る。これは既存テストデータの仕様であり、PowerShell の対応形式が増えたことを意味しない。

## 4. docs-search に残すもの・文書の修正箇所

`tauri/` 全体、`tests/` の PowerShell ランナー以外、ルートのデータ生成ランチャーを残す。とくに以下の専用生成器を `docs-search-ps` へコピーしない:

- `tests/generate-context-fixture.py`
- `tests/generate-conditions-fixture.py`
- `tests/generate-office-search-fixture.py`（現時点で未追跡）
- `tests/generate-issue-fixture.py`（現時点で未追跡）

`tauri/tests/ui/`、`tauri/core/tests/`、`tauri/core/proptest-regressions/` などの未追跡ソースも元のまま保持する。「未追跡だから不要」と判断しない。移行時点では GitHub Actions 設定・LICENSE・submodule の管理ファイルは確認できないため、移植や新規作成を行わない。実施時に増えていたら個別に用途を確認する。

### 4.1 現行運用文書を修正する

| 文書 | 必要な変更 |
| --- | --- |
| `README.md` | 冒頭を Rust/Tauri/WASM 版として説明。PowerShell 起動・PowerShell テストのローカル手順を削除し、外部リポジトリへの短い案内にする。「共通テストデータ」はこのリポジトリのテストデータとして説明。生成ランチャーは残す |
| `tests/README.md` | PowerShell ランナーの表・実行手順・両実装の共用運用を除去。生成器・ケース原本のローカル配置と Rust 側のテスト入口を説明。移行時に別リポジトリへ同じデータ用ソースをコピーし、以後は独立管理すると記す |
| `docs/requirements.md` | `../powershell/README.md` と「別ディレクトリに保存」を外部リポジトリへの案内に修正。Rust の要求・受け入れ条件は維持 |
| `docs/rust-requirements-v3.0.0.md` | 「旧 PowerShell 実装は保存対象」を「PowerShell 版は docs-search-ps で管理」に修正。要求 ID・版番号・テスト対応は維持 |
| `docs/search-foundation-test-plan.md` | PowerShell ランナーの実行を Rust の回帰手順から除去 |
| `docs/result-export-test-plan.md` | 同上 |
| `docs/excel-context-test-plan.md` | 同上。「PowerShell 実装に差分がない」を移行前の Rust の件数・場所・一致分類・索引挙動を維持する条件に修正 |
| `docs/condition-search-test-plan.md` | PowerShell ランナーの実行を Rust の回帰手順から除去 |
| `docs/fuzzy-search-test-plan.md` | 同上 |
| `docs/result-match-error-test-plan.md` | 回帰確認の PowerShell 実行・PowerShell 成功条件を除去 |
| `docs/issues-test-plan.md` | PowerShell ランナーの実行を Rust の回帰手順から除去 |
| `docs/office-ranking-batch-test-plan.md` | 同上 |
| `docs/theme-settings-test-plan.md` | 同上 |

上表のテスト方針では「自動テストのみ」の入口を `tauri/Run-Automated-Tests.ps1`、配布ビルド込みの入口を `tauri/Run-Local-CI.ps1` とする。既存の固有確認項目・期待結果・GUI 確認の境界は維持する。ヘッドレス Playwright と準備用 WASM 生成を自動試験から外さない。

`tauri/Run-Automated-Tests.ps1` と `tauri/Run-Local-CI.ps1` は確認時点で PowerShell 版のランナーを呼んでいない。拡張子が `.ps1` であることを理由に削除・移動・書き換えない。

### 4.2 履歴と仕様の維持

`docs/backend.md`、`docs/gui.md`、`docs/boundary.md`、Rust の実装報告・要求対応・Playwright/proptest 方針は Rust 側に残す。移行のために検索仕様を改訂しない。

`docs/implementation-plan.md` は元の計画、`docs/implementation-status.md` は進捗と公開履歴、`docs/release-notes-v2.0.6.md` / `v2.0.7.md` と `docs/validation-v3.0.0.md` は当時の記録として残す。過去に実施した PowerShell テストの記述を、未実施だったことに書き換えない。現在の配置に誤解を生む場合は冒頭に移行日と外部 URL を追記する。

この計画も過去の配置を記述する文書として残す。移行元のファイルパスは履歴として記載し、`docs-search-ps` 内にあるかのようなローカルリンクにはしない。移行後に確認用リンクを設ける場合は、移行先が push され、対象ファイルを含むブランチを確認した後に GitHub の実際のパスを使う。

現行の入口・手順から削除されたローカルファイルへのリンクがなくなることが完了条件。外部リンク、過去の検証記録、この計画の旧パス一覧は参照残りとして許容する。

## 5. Luna の実施手順

### M0: 作業範囲とバックアップを確定する

読む順序は移行元 `AGENTS.md` → この計画 → `powershell/README.md` → `tests/run-powershell-tests.ps1` → `tests/README.md` → Rust の最新テスト方針。全ソースをまとめて読み込まず、疑問がある箇所だけ追加確認する。

次の操作は読み取りとバックアップだけ。実装を依頼されたときに実行する例であり、計画作成時には実行しない。

```powershell
$ErrorActionPreference = 'Stop'
$sourceRepo = 'C:\Users\kazum\codex-projects\docs-search'
$targetRepo = 'C:\Users\kazum\codex-projects\docs-search-ps'
$runId = (Get-Date -Format 'yyyyMMdd-HHmmss') + '-' + [guid]::NewGuid().ToString('N')
$checkpointDirectory = Join-Path $sourceRepo ('outputs\migration-ps\' + $runId)
$backupRoot = Join-Path $checkpointDirectory 'source-files'
New-Item -ItemType Directory -Path $backupRoot | Out-Null

$trackedPaths = @(git -C $sourceRepo ls-files)
if ($LASTEXITCODE -ne 0) { throw '移行元の管理ファイルを取得できません。' }
$untrackedPaths = @(git -C $sourceRepo ls-files --others --exclude-standard)
if ($LASTEXITCODE -ne 0) { throw '移行元の未追跡ファイルを取得できません。' }
$sourceStatus = @(git -C $sourceRepo status --porcelain=v1 --untracked-files=all)
if ($LASTEXITCODE -ne 0) { throw '移行元の状態を取得できません。' }
$sourceHead = git -C $sourceRepo rev-parse HEAD
if ($LASTEXITCODE -ne 0) { throw '移行元の HEAD を取得できません。' }
$inventory = foreach ($relative in @($trackedPaths + $untrackedPaths | Sort-Object -Unique)) {
    $original = Join-Path $sourceRepo $relative
    if (-not (Test-Path -LiteralPath $original -PathType Leaf)) {
        throw "管理一覧にあるファイルが見つかりません: $relative"
    }
    $saved = Join-Path $backupRoot $relative
    New-Item -ItemType Directory -Path (Split-Path -Parent $saved) -Force | Out-Null
    Copy-Item -LiteralPath $original -Destination $saved
    $originalHash = (Get-FileHash -LiteralPath $original -Algorithm SHA256).Hash
    if ($originalHash -ne (Get-FileHash -LiteralPath $saved -Algorithm SHA256).Hash) {
        throw "バックアップが一致しません: $relative"
    }
    [pscustomobject]@{ Path = $relative; SHA256 = $originalHash; Tracked = ($trackedPaths -contains $relative) }
}
$inventory | Export-Csv -LiteralPath (Join-Path $checkpointDirectory 'source-inventory.csv') -NoTypeInformation -Encoding UTF8
$sourceStatus | Set-Content -LiteralPath (Join-Path $checkpointDirectory 'source-status.txt') -Encoding UTF8
$sourceHead | Set-Content -LiteralPath (Join-Path $checkpointDirectory 'source-head.txt') -Encoding UTF8
```

`git ls-files` は削除済みの管理ファイルも返す。その場合は上例で停止し、欠損が既存作業によるものかを記録してからバックアップ対象を調整する。空ファイルを作って帳尻を合わせない。既存のステージ状態も確認・記録する。バックアップは無視された生成物を含まないため、`outputs/` の既存成果物はそのまま保持する。

続けて `git remote -v`、移行先の `gh repo view kz-oshiro/docs-search-ps --json nameWithOwner,url,isEmpty,defaultBranchRef`、移行先ローカルパスの有無を確認する。移行先が引き続き空でローカルパスも存在しなければ、次で取得する。

```powershell
git clone git@github.com:kz-oshiro/docs-search-ps.git $targetRepo
if ($LASTEXITCODE -ne 0) { throw '移行先の取得に失敗しました。' }
git -C $targetRepo symbolic-ref HEAD refs/heads/main
if ($LASTEXITCODE -ne 0) { throw '空の移行先の main を設定できません。' }
```

移行先が存在するときは origin・状態・既存内容を読む。空でないリポジトリに上のブランチ設定を実行しない。既存ファイルを強制上書きせず、今回の許可リストとの重複を確認し、差分を個別に扱う。

完了条件: バックアップと一覧が作成でき、元の dirty/未追跡/ステージ状態、両 origin、コピー元、コピー先が確定している。移行元には変更を加えていない。

### M1: 移行先を単独で使える状態にする

3.1 の 10 ファイルだけを同じ相対パスへコピーする。コピー前に全ファイルの存在と移行先との重複を確認し、コピー後に README 以外の 9 ファイルを照合する。コピーの例:

```powershell
$copyPaths = @(
    'powershell/Search-Excel.ps1',
    'powershell/docs-search.ps1',
    'powershell/Start-docs-search.vbs',
    'powershell/README.md',
    'tests/run-powershell-tests.ps1',
    'tests/generate-fixtures.py',
    'tests/backend-cases.json',
    'tests/test_generator.py',
    'tests/Generate-Test-Data.ps1',
    'Generate-Test-Data.cmd'
)
foreach ($relative in $copyPaths) {
    if (-not (Test-Path -LiteralPath (Join-Path $sourceRepo $relative) -PathType Leaf)) {
        throw "コピー元が見つかりません: $relative"
    }
    if (Test-Path -LiteralPath (Join-Path $targetRepo $relative)) {
        throw "コピー先に既存内容があります: $relative"
    }
}
foreach ($relative in $copyPaths) {
    $destination = Join-Path $targetRepo $relative
    New-Item -ItemType Directory -Path (Split-Path -Parent $destination) -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $sourceRepo $relative) -Destination $destination
}
```

途中でコピーに失敗した場合は、一括削除してやり直さず、実在する各ファイルのハッシュと完了記録から再開する。不変 9 ファイルは M0 の保存内容とも照合する。元ファイルが途中で変わっていたら、その変更を含む新しい基準を記録し、当該ファイルを再コピー・再照合してから進む。

続けて 3.2 の文書・設定を作成し、`powershell/README.md` の冒頭を独立した実装の説明へ修正する。上位 README へのリンクは `docs-search-ps` の説明を指すようになり、`../tests/README.md` は同じ移行先のファイルを指す。Rust 仕様やテスト入口へは外部の参考リンクとして案内する場合だけリンクする。

`tests/README.md` には次を明記する:

- acceptance 33 / load 148 / seed `20260927` は移行時点の既存仕様を維持する。
- `backend-cases.json` の 27 件はデータ生成と生成器検証のために保存する。PowerShell は異なる API のため、この 27 件を全件実行しない。
- 既存 PowerShell ランナーは生成器の 2 テスト、33 文書/27 原本ケースの整合、Excel の needle 7 一致・破損 Excel 1 エラー、共有/inline 文字列、保存済み式結果、図形/VML、数値、非表示シート、xlsm、バックグラウンドジョブを確認する。
- 一時文書は既存ランナーで生成・削除する。保存用文書は `Generate-Test-Data.cmd` または `tests/Generate-Test-Data.ps1 -NoOpen` でリポジトリ内 `outputs/test-data/` に毎回新規生成する。
- Rust のイベント/索引/条件/あいまい/順位/編集/出力/Playwright は、このリポジトリの自動試験に含まれない。

完了条件: 新規配置が構成図と一致し、文書の相対リンクが存在し、不変 9 ファイルが M0 の基準と一致する。移行先は元リポジトリのファイルを読み込む相対パスや隣接フォルダーを必要としない。まだ元ファイルを削除しない。

### M2: 元リポジトリから PowerShell 版を分離する（2026-10-03 完了）

M1 の完了条件を満たした後に、元から削除する対象を次の **5 ファイルだけ**に固定する。

```text
powershell/README.md
powershell/Search-Excel.ps1
powershell/docs-search.ps1
powershell/Start-docs-search.vbs
tests/run-powershell-tests.ps1
```

削除直前に元の 5 ファイルが M0 から変わっていないことを確認する。変更がある場合はバックアップ・移行先を更新してから再照合する。削除は各絶対パスを移行元配下として検証し、`Remove-Item -LiteralPath <対象ファイル>` で個別に行う。`-Recurse`、`-Force`、ワイルドカード、一括フォルダー削除を使わない。`powershell/` が空になったら、空であることを確認して空ディレクトリだけを削除できる。

4.1 の文書を順に修正する。現行の検索機能・テスト項目・記録済みの検証結果を落とさない。元の `AGENTS.md` は Rust/WASM/Playwright の規則を含めて維持する。原則として Rust ソース・設定・スクリプトには差分を作らない。

`docs/implementation-status.md` に移行日・移行先・実施段階を短く追記し、現在の管理場所を示す。未検証・未公開なら、その状態を明記する。移行先の `docs/migration-record.md` も更新する。

完了条件: 元リポジトリの PowerShell 本体と専用ランナーが除去され、Rust の入口から PowerShell 版に依存する説明が消えている。共用だった生成器・原本・検証・生成ランチャー、Rust 全体、既存成果物は保持されている。

### M3: 静的確認と実装報告（2026-10-03 完了）

以下は初回移行の静的確認手順と実施結果である。その後のユーザー依頼による文書レビューでは `docs/backend.md` と `tauri/README.md` も修正したため、M3 完了時の文書ハッシュとの差分を移行の破損とは扱わない。実行ソース・設定・生成器・ケース原本のハッシュは引き続き M0 と照合し、文書の追加修正は移行記録で区別する。

移行後に依頼されたPowerShellの動作修正は最新の移行記録で区別し、本体2ファイル・専用ランナーの現在のハッシュを初回コピー9ファイルの一致条件へ戻さない。VBS、共通生成器・原本・生成ランチャー、Rust側のソースはM0との不変照合を継続する。

M0 のハッシュ一覧と現在の元リポジトリを比較する。差分の許可範囲は M2 の削除 5 件、4.1 の文書、`docs/implementation-status.md` の移行追記、この計画のリンク更新に限定する。M0 の dirty 差分を基準にするため、HEAD との `git diff` だけで「移行が Rust を変更した」と判断しない。

| 確認項目 | 手順 | 期待結果 |
| --- | --- | --- |
| コピーの完全性 | M0 と移行先の不変 9 ファイルの SHA-256 を比較 | 全件一致。文字コード・BOM・改行も維持 |
| Rust の保持 | M0 の `tauri/` 全ファイルと現在の存在・SHA-256 を比較 | 変更・欠落なし。元からの未追跡ファイルも保持 |
| 共用ソースの保持 | 元の生成器・原本・生成器検証・生成ランチャーのハッシュを比較 | 元は不変、移行先のコピーも一致 |
| 削除の限定 | M0 一覧と現在の元の一覧を比較 | 削除は許可した 5 ファイルだけ |
| 相対参照 | 下記 rg と Markdown の相対リンクを静的に確認 | 現行手順に欠損パスなし。外部案内・履歴・移行記録を区別 |
| ケース原本 | JSON をデータとして読み、件数・seed を確認。生成器・ランナーの定数と比較 | 27 件・seed `20260927`。期待値変更なし |
| Git 除外 | 両リポジトリで `git check-ignore --no-index` に出力/生成物の候補パスを渡す | 出力・生成文書・Python キャッシュが除外対象。必要ソースは除外されない |
| 差分形式 | 両方で `git diff --check`。新規文書は直接読んで末尾空白・リンクも確認 | 移行が追加した空白エラーなし |
| 独立性 | 移行先のソース・文書でローカルの docs-search 絶対パス、`../docs-search/`、`tauri/` を検索 | 実行時依存なし。移行記録中の出典・外部参考は可 |

参照検索の例。PowerShell のコードフェンス名や `powershell.exe` 自体は削除対象ではない。

```powershell
rg -n -i 'run-powershell-tests|Search-Excel|Start-docs-search|powershell[/\\]|両実装|共用|別ディレクトリ' README.md AGENTS.md docs tests tauri
```

実装報告では、変更ファイル、コピーの照合結果、既存作業の保持、各段階、未実施のテスト・ビルド・GUI・公開、M4 の確認手順を記す。実装のみの依頼ならここで報告し、テストをユーザーへ実施依頼せず次のモデルへ引き継ぐ。ローカルコミットもリモート反映の依頼がない段階では行わない。

完了条件: 静的な分離が完成し、次の担当が両リポジトリを単独で検証できる。動作確認済み・公開済みとは報告しない。初回 M3 完了時は移行元の `git diff --check` 成功、移行先16ファイルの末尾空白なし、両リポジトリの Markdown ローカルリンク259件のファイル存在、9コピーの M0 SHA-256一致、`tauri/` 58ファイルの SHA-256不変、JSON27ケース/seed `20260927`、両 `.gitignore` の生成物除外とソース保持を確認した。テスト・ビルド・GUI・リモート反映は未実施である。

### M4: 依頼された自動テストとビルド

検証担当は実装報告、移行先の `tests/README.md`、元の [Playwright UI 試験方針](playwright-ui-test-plan.md)、最新の Rust テスト方針を先に読む。PowerShell 版は Windows PowerShell 5.1 と Python 3、Rust 版は [Tauri README](../tauri/README.md)・UI 試験方針の開発ツールが必要。依存不足は手順に従って準備する。

| 対象・確認項目 | 手順 | 期待結果 |
| --- | --- | --- |
| PowerShell 版の自動回帰 | docs-search-ps 直下で下の PowerShell コマンドを実行 | 終了コード 0。生成器・既存 Excel 検索・ジョブが成功。元リポジトリの本体を使わない |
| Rust 版の自動回帰 | docs-search 直下で下の Run-Automated-Tests を実行 | 終了コード 0。共通ケース、P2/P3、Office/順位/一括、イシュー、索引、Rust 単体/公開 API/proptest、WASM 型検査、実 WASM のヘッドレス Playwright が成功 |
| Rust の配布ビルド（依頼がある場合） | 自動試験のみではなく下の Run-Local-CI を使用 | 自動試験と Windows EXE 生成に成功。成果物は元リポジトリの `tauri/docs-search-desktop.exe` |
| 保存用データの入口 | 各リポジトリで下の Generate-Test-Data を `-NoOpen` 付きで実行 | 各自の `outputs/test-data/` に新しい acceptance 33 文書＋manifest＋原本を生成。既存文書を上書きしない |
| 出力の管理 | 試験後に両方の Git status を確認 | テスト成果物が管理対象に混入しない。回帰 seed に新差分があれば失敗原因と合わせて扱う |

PowerShell 版のリポジトリ直下:

```powershell
powershell.exe -NoProfile -ExecutionPolicy RemoteSigned -File .\tests\run-powershell-tests.ps1
powershell.exe -NoProfile -ExecutionPolicy RemoteSigned -File .\tests\Generate-Test-Data.ps1 -Profile acceptance -NoOpen
```

Rust 版のリポジトリ直下（自動テストのみ）:

```powershell
powershell.exe -NoProfile -ExecutionPolicy RemoteSigned -File .\tauri\Run-Automated-Tests.ps1
powershell.exe -NoProfile -ExecutionPolicy RemoteSigned -File .\tests\Generate-Test-Data.ps1 -Profile acceptance -NoOpen
```

Rust 版の配布ビルドまで依頼された場合は、上の自動試験を別に繰り返さず、次の入口で一式を実施する:

```powershell
powershell.exe -NoProfile -ExecutionPolicy RemoteSigned -File .\tauri\Run-Local-CI.ps1
```

PowerShell 版に配布用コンパイルはない。移行確認のために新たなテストフレームワークや追加試験実装を作らない。生成データの件数・原本の件数・検索の期待値を失敗に合わせて下げない。

Playwright はスクリーンショット・画像比較・動画・トレースなしで、DOM・状態・要求引数・テキスト失敗記録により判定する。試験用 WASM 生成とヘッドレス Chromium は自動テストに含まれる。実アプリを対話的に起動する GUI 確認とは区別する。

GUI が依頼された場合に限り、PowerShell は新リポジトリの VBS 起動、検索、中断、ファイルを開く、場所コピー、エラー表示を確認する。Rust は既存方針の M1〜M5 に従う。明示依頼がない場合に GUI を必要と判断したときだけ事前確認を求め、依頼済みの自動試験は継続する。

結果は両リポジトリで別々に記録し、終了コード・実施項目・ログの保存先・未実施範囲を示す。過去の検証記録は書き換えず、移行後の記録を新しく追加する。

完了条件: 依頼された非 GUI の確認範囲が成功し、移行先が単独で動作することと Rust の回帰を確認できている。自動試験成功から GUI 成功を断定しない。

### M5: 依頼されたリモート反映

計画・実装・テストの依頼だけでは実行しない。リモート反映まで依頼された場合は次の順を守る。

1. 両方の origin とブランチを確認する。移行先を先に commit/push し、`docs-search-ps` のリモートから必要ファイルが取得できる状態にする。
2. その反映先で README、起動ファイル、テスト用ソース、仕様・移行記録が存在することと SHA を確認する。Git の改行変換で raw のハッシュが変わる場合は、Git blob/取得した作業ツリーの比較方法・改行差を区別して記録する。
3. 次に元の分離差分を commit/push する。各 commit は `git diff --cached --check` とステージ内容を確認し、`git add .` / `git add -A` は使わない。
4. それぞれ `git rev-parse HEAD` と `git ls-remote origin refs/heads/main` を比較し、ahead/behind と状態を確認する。両リポジトリの commit SHA と URL を報告する。
5. 移行先を先に公開できても元の反映が失敗した場合は、移行を「公開完了」としない。移行先の commit は維持し、元側の失敗を修復して続ける。

**既存の dirty 差分の公開は移行の依頼から推定しない。** 同じ README・仕様文書に元からの変更があるため、ファイルを限定して `git add` するだけでは混入を防げない。公開時の扱いを次で固定する。

- 元からの変更が別作業として既に commit 済みで、公開対象も確定している場合は、その基点に移行差分を載せる。
- 元からの変更が未確定のまま「移行だけを反映」と依頼された場合は、公開用の別チェックアウトを元の公開対象コミットから作り、そこで本計画の移行を再適用する。元の dirty 作業ツリーは保持する。元のローカル HEAD と remote main が異なるときは、公開対象の基点を記録してから進む。
- 公開用の基点が古い場合、その基点の README・仕様・テスト入口を使う。未公開の v3.0.0 文書や Run-Automated-Tests を参照させない。旧 Run-Local-CI にヘッドレス試験がない場合、移行だけの差分では足りないため、既存の UI 自動試験導入を別の明示された公開範囲として扱う。新しい基点の導入が公開対象に含まれるかを確認できるまで、Rust 側の公開だけを完了と扱わない。
- 公開する内容が M4 で試験した作業ツリーと異なる場合は、その内容について適切な自動確認を行う。別の内容での成功を公開の検証結果にしない。

強制 push、既存ブランチ/タグの削除、元の変更をまとめた無断 commit、`reset --hard`、`clean`、`stash` は使わない。バージョン変更・タグ・GitHub Release は別途依頼がある場合だけ実施する。

完了条件: 新リポジトリの取得・実行・開発手順が公開され、元の現行ブランチから PowerShell 版が分離され、各リポジトリでローカル HEAD とその origin/main の SHA が一致する。ローカルの旧履歴・既存作業・成果物は保持されている。

## 6. 段階ごとの記録と復旧

各段階の終了時に `docs-search-ps/docs/migration-record.md` に短く追記する。Luna の次のターンは記録を読み、完了済みのコピーや試験をやり直さず、未完了の段階から再開する。

```text
実施段階: M0 / M1 / M2 / M3 / M4 / M5
元 URL・HEAD・公開基点:
移行先 URL・HEAD:
基準バックアップ:
コピー・SHA-256 の照合:
元の既存変更の保持:
静的確認:
自動試験（対象・終了コード・ログ）:
配布ビルド:
GUI:
リモート反映:
未完了の項目・次に行う手順:
```

復旧は対象ファイル単位で行う。M2 の削除を戻す必要がある場合は、M0 の保存内容から削除 5 ファイルだけを元のパスへコピーする。変更した文書も M0 以降の追加編集がないことを照合してから戻す。Rust 全体やリポジトリ全体を巻き戻さない。移行先のフォルダーを消して復旧する必要はない。

## 7. 実装担当への依頼文

```text
docs/powershell-repository-migration-plan.md を読み、M0〜M3 の実装と静的確認を実施してください。
PowerShell 版を C:\Users\kazum\codex-projects\docs-search-ps に独立させ、元の docs-search は Rust/Tauri/WASM 版として整理してください。
許可リストどおりにコピーし、不変 9 ファイルをハッシュで照合してから、元の指定 5 ファイルを削除してください。
共用だったテスト用ソースは両方へ独立配置し、Rust 全体・既存の dirty/未追跡変更・outputs は保持してください。
テスト、ビルド、アプリ起動、GUI、commit/push、タグ・Release は実施せず、移行記録と次の担当向けの確認手順を報告してください。
```

M4 の依頼例:

```text
移行の実装報告と最新のテスト方針を読み、両リポジトリの M4 自動テストを実施してください。
docs-search のヘッドレス Playwright と必要な WASM 生成を含めてください。
保存用データ生成は -NoOpen を使用してください。
実アプリ起動・対話的 GUI・Windows 配布ビルド・リモート反映は対象外です。
各リポジトリの結果、ログ、未実施範囲を別々に記録してください。
```
