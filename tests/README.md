# Rust 版のテストデータ

このディレクトリは Rust/Tauri 版の検索テスト入力を管理します。PowerShell 版のリポジトリに移した生成用ファイルとは独立して管理します。同じデータを使う変更を行う場合は、両リポジトリの変更を個別に確認します。

実WASM画面のヘッドレス試験は [tauri/tests/ui](../tauri/tests/ui/package.json)、自動/手動の境界は [Playwright UI試験方針](../docs/playwright-ui-test-plan.md)を参照してください。Tauri境界の固定応答を使うため、この共通文書の実検索試験は置き換えません。導入時点でPlaywright実行は未実施です。

| ファイル | 役割 |
| --- | --- |
| `generate-fixtures.py` | Python 3 標準ライブラリだけで検索用文書を生成 |
| `Generate-Test-Data.ps1` | ダブルクリック用ランチャーから呼ばれ、保存先を作成して生成後に開く |
| `backend-cases.json` | バックエンドの操作、検索入力、期待結果を記述した共通契約 |
| `test_generator.py` | 同一シードのバイト一致、Office ZIP/XML、ケースの参照先を確認 |
| `generate-context-fixture.py` | P2 周辺セルの独立した疎な Excel 文書を指定先へ生成。既存ファイルは上書きしない |
| `generate-conditions-fixture.py` | P3 条件検索の Excel・テキスト文書を指定先へ生成。既存ディレクトリは上書きしない |

P2の検索位置と索引経路の自動確認は[`tauri/tests/run-context-cases.py`](../tauri/tests/run-context-cases.py)を使います。疎なExcelデータを一時生成し、セル・図形の場所と索引初回/再利用の一致をCLIで検証します。周辺表の取得規則はRust単体テスト、ブラウザー内の操作・表示状態はヘッドレスPlaywrightで確認し、実アプリのOS連携と見た目は別に扱います。

P3の自動確認は[`tauri/tests/run-condition-cases.py`](../tauri/tests/run-condition-cases.py)を使います。専用データを一時生成し、条件範囲、根拠、索引・あいまい検索の4通り、入力拒否、読み取りエラーをCLIで検証します。

## 生成と実行

Rust の自動確認には [自動テストの入口](../tauri/Run-Automated-Tests.ps1) を使用します。Rust/Tauri 版のルートから実行します。

```powershell
powershell.exe -NoProfile -ExecutionPolicy RemoteSigned -File .\tauri\Run-Automated-Tests.ps1
```

各 Rust ランナーは試験用データを一時フォルダーへ生成し、終了時に片付けます。実 WASM 画面を扱うヘッドレス Playwright とその準備用 WASM 生成も、自動テストの一部です。[UI 試験方針](../docs/playwright-ui-test-plan.md)を参照してください。生成物は Git で管理しません。

データを保存して確認するときは、リポジトリ直下の [`Generate-Test-Data.cmd`](../Generate-Test-Data.cmd) をダブルクリックします。既定の `acceptance` データを `outputs/test-data/acceptance-日時-識別子/` に作り、完了後にそのフォルダーを開きます。繰り返し実行しても毎回別のフォルダーを作るため、既存データを上書きしません。`outputs/` は Git の除外対象です。Python 3 の `python` コマンドが必要で、失敗時は画面にエラーを残します。PowerShell 版には `Generate-Test-Data.ps1` の独立コピーがあり、その出力は PowerShell 側のリポジトリを使います。

負荷確認用の `load` データを保存する場合は、コマンドプロンプトまたは PowerShell から次を実行します。

```powershell
.\Generate-Test-Data.cmd -Profile load
```

ジェネレータを直接実行するときは次のコマンドを使います。ジェネレータは出力先が空のときだけ動作し、既存データを上書きしません。既存の出力を残して新しい内容を生成する場合は、別の空ディレクトリを `--output` に指定します。

```powershell
python .\tests\generate-fixtures.py --output .\outputs\test-data\acceptance-manual-new
python .\tests\generate-fixtures.py --output .\outputs\test-data\load-manual-new --profile load
```

## 生成仕様

| プロファイル | 出力先 | 文書数 | 用途 |
| --- | --- | ---: | --- |
| `acceptance`（既定） | `search/` | 33 | 検索結果とエラーの期待値を検証 |
| `load` | `search/` と `load/` | 148（33 + 115） | 受け入れ用データに負荷確認用データを追加 |

両プロファイルとも、文書のほかに `manifest.json` と、原本をそのままコピーした `backend-cases.json` を出力します。`manifest.json` の `files` は `search/` と `load/` に作った文書の相対パスを昇順で列挙し、JSON ファイル2件自体は含めません。`profile` と固定シードも記録します。

`search/` には既存10拡張子のデータと、追加形式 `.vue`、`.mjs`、`.jsx`、`.tsx`、`.json`、`.md`、`.csv`、`.py`、`.yaml`、`.svg`、`.ipynb` の代表ファイルを作ります。これは追加対応形式の代表例であり、対応する全93拡張子を網羅するデータセットではありません。入れ子のフォルダー、非表示シート・スライド、Excel の共有文字列・セル・保存済み数式結果・図形、PowerPoint の図形・表、Word の本文・入れ子表、UTF-8 と BOM、Shift_JIS の `.txt` と `.html`、破損・不正な文字コード・対象外形式・Office 一時ファイルも含みます。`.jsp`、`.xhtml`、`.js`、`.java` はそれぞれ1ファイル、`.html` は2ファイルです。Excel は3シート各295行 × 12列、PowerPoint は165スライド各8図形、Word は3400本文段落と表、テキストは各150行を生成します。

エラー確認用には、既存の `spreadsheets/broken.xlsx` と `notes/invalid-utf8.txt` に加え、`errors/` に壊れた `.pptx` と `.docx`、UTF-8 BOM の後に不正なバイトがある `.txt` を作ります。`search/` を既定の5拡張子で検索すると計5件の読み取りエラーを確認でき、共通ケース `multiple-read-errors` は `errors/` の3件を一度に照合します。これらは意図的に破損した入力であり、正常な Office 文書の件数には含めません。

`load/` には Excel 32個、PowerPoint 32個、Word 31個、テキスト20個を追加します。正常な Office 文書は両ディレクトリを合わせて100ファイルで、各ファイルはおおむね0.5 MB（45万～55万バイト）です。破損した Office 文書、Office 一時ファイル、対象外の `.xls` はこの100ファイルに含めません。性能の合格時間はここでは定めません。

固定シードは `20260927` です。乱数は各ファイル名から独立に初期化し、ZIP 内の時刻・格納順も固定します。同じプロファイル、ジェネレータ、検証ケース原本からは同じバイト列を出力します。シードや内容を変更する場合は、期待結果と一緒に更新します。

`fuzzy/variants.md` は識別子表記、幅、空白差、タイプミス、短語の確認用です。既存の `extended/sample.md` と合わせて `.md` は2ファイルになります。

## バックエンド検証データ

proptestによる生成テストはRustコアのテストに含まれます。[確認方針](../docs/property-testing-test-plan.md)を参照してください。生成した文書は独立した一時フォルダー、索引の性質テストはメモリ内SQLiteを使用し、利用者の索引は変更しません。失敗seedの `tauri/core/proptest-regressions/` は回帰データとしてGit管理対象とします。

v3.0.0は [Rust要求仕様とテスト対応](../docs/rust-requirements-v3.0.0.md)を基準にします。`tauri/Run-Automated-Tests.ps1` が既存CLIケースと [索引試験](../tauri/tests/run-index-cases.py)、[公開API要求試験](../tauri/core/tests/requirements_v3.rs)を実行し、WASM型検査、試験用WASM生成、ヘッドレスPlaywrightまで確認します。実アプリ起動・対話的GUI・Windows配布ビルドは含みません。実行結果は [検証記録](../docs/validation-v3.0.0.md)へ記録します。Playwright導入前の成功記録と、導入後の試験実行が未実施であることを区別してください。

P4a〜P6の専用データは `generate-office-search-fixture.py --output <新規フォルダー>` で作成し、`tauri/tests/run-office-search-cases.py` から確認します。注記・数式、部品の欠落/破損/外部参照/上限、索引範囲、一括検索の境界と件数、順位の20固定クエリを含みます。既存の共通27ケースと別の生成フォルダーを使い、既存件数を変えません。[確認項目・手順・期待結果](../docs/office-ranking-batch-test-plan.md)を読む担当が、依頼されたテスト範囲で実行してください。初回実装報告では未実施でしたが、その後のRust/CLI自動確認は [検証記録](../docs/validation-v3.0.0.md)に記載しています。

複数フォルダーのケースは `additionalDirectories`、対象外のケースは `excludedDirectories` を `SearchRequest` に加えます。Tauri のテストアダプターは CLI の `--add-directory` と `--exclude-directory` に変換し、重なる検索フォルダーの重複排除、対象外フォルダーの優先、存在しない対象外フォルダーの入力拒否を確認します。生成する文書数と内容は変えません。

`backend-cases.json` が管理対象の原本で、生成時に同一内容を出力先へコピーします。JSON は言語や実装の型を指定しません。Rust 側のテストアダプターは `${fixtureRoot}` を生成先の絶対パスに置換し、OS のパス区切りに変換して `SearchRequest` と操作を実行します。`expected.results[].file` と `expected.issues[].file` は `search/` からの相対パスです。結果の `sourceKind`、構造化 `location` の掲載フィールド、`textContains` を照合し、余分な結果・Issue がないことを件数と併せて確認します。`resultId`、`searchId`、通知の到着順、プレビューの全文、利用者向けエラー文言には依存しません。

`expected.counts` は `Finished` の最終集計です。入力拒否ケースはイベントを発行しません。`cancel-load-search` は `load` プロファイル専用で、`Started` 受信後、`Finished` 前に中断操作を配送できるテスト環境で実行します。中断の結果件数は固定せず、イベント件数と最終集計の整合を確認します。Rust 側の [テストアダプター](../tauri/tests/run-backend-cases.py) がこの JSON を実行します。PowerShell 版には同じ API がなく、同一ケースの全件照合を行いません。

通常検索の1ケースとあいまい検索の3ケースでは `matchType`、`matchCategory`、`score` の代表値も期待値に含めます。検索オプションは `useIndex` と `fuzzySearch` で指定し、省略時はどちらも無効です。確認項目と手順は [あいまい検索の確認方針](../docs/fuzzy-search-test-plan.md) と [結果ラベル・エラー表示の確認方針](../docs/result-match-error-test-plan.md) にまとめています。

`manifest.json` は生成した相対パス一覧とシードを記録します。`search/` と `load/` の文書はテスト専用で、実際の利用者文書や正規の Office ファイルの代用品として配布しません。

## 既存イシューの一括対応

2026-10-03に #1〜#9 の折りたたみ・本文フィルター・一致箇所編集・更新日時・フォルダー復元・位置順・Excel書式/可視文字の対応を追加しました。[実装報告](../docs/implementation-report-issues.md)は実装と静的確認までの記録です。その後のRust/CLI自動確認は [検証記録](../docs/validation-v3.0.0.md)、次の担当向けの手順は [確認方針](../docs/issues-test-plan.md)を参照してください。Playwright導入後の実行・実アプリGUI・配布ビルド・公開は未実施です。
