# 共通テストデータ

このディレクトリは PowerShell 実装と Tauri 実装が共用するテスト入力を管理します。検索エンジン、GUI、通信方式が変わっても、文書データと期待結果の意味を同じに保ちます。

| ファイル | 役割 |
| --- | --- |
| `generate-fixtures.py` | Python 3 標準ライブラリだけで検索用文書を生成 |
| `Generate-Test-Data.ps1` | ダブルクリック用ランチャーから呼ばれ、保存先を作成して生成後に開く |
| `backend-cases.json` | バックエンドの操作、検索入力、期待結果を記述した共通契約 |
| `test_generator.py` | 同一シードのバイト一致、Office ZIP/XML、ケースの参照先を確認 |
| `run-powershell-tests.ps1` | 共通データを一時生成して PowerShell 実装の Excel 検索を確認し、終了時に削除 |

## 生成と実行

Windows PowerShell 5.1 と Python 3 を使用します。`docs-search` を作業ディレクトリとして実行します。

```powershell
powershell.exe -NoProfile -ExecutionPolicy RemoteSigned -File .\tests\run-powershell-tests.ps1
```

`run-powershell-tests.ps1` は OS の一時ディレクトリ内に `search/`、`manifest.json`、`backend-cases.json` を作り、成功・失敗のどちらでも `finally` で削除します。バックエンド再構築版のテストランナーも、各テストの前に同じジェネレータを実行し、終了時に一時ディレクトリを削除してください。生成物は Git で管理しません。

データを保存して確認するときは、リポジトリ直下の [`Generate-Test-Data.cmd`](../Generate-Test-Data.cmd) をダブルクリックします。既定の `acceptance` データを `outputs/test-data/acceptance-日時-識別子/` に作り、完了後にそのフォルダーを開きます。繰り返し実行しても毎回別のフォルダーを作るため、既存データを上書きしません。`outputs/` は Git の除外対象です。Python 3 の `python` コマンドが必要で、失敗時は画面にエラーを残します。

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

複数フォルダーのケースは `additionalDirectories`、対象外のケースは `excludedDirectories` を `SearchRequest` に加えます。Tauri のテストアダプターは CLI の `--add-directory` と `--exclude-directory` に変換し、重なる検索フォルダーの重複排除、対象外フォルダーの優先、存在しない対象外フォルダーの入力拒否を確認します。生成する文書数と内容は変えません。

`backend-cases.json` が管理対象の原本で、生成時に同一内容を出力先へコピーします。JSON は言語や実装の型を指定しません。テストアダプターは `${fixtureRoot}` を生成先の絶対パスに置換し、OS のパス区切りに変換して `SearchRequest` と操作を実行します。`expected.results[].file` と `expected.issues[].file` は `search/` からの相対パスです。結果の `sourceKind`、構造化 `location` の掲載フィールド、`textContains` を照合し、余分な結果・Issue がないことを件数と併せて確認します。`resultId`、`searchId`、通知の到着順、プレビューの全文、利用者向けエラー文言には依存しません。

`expected.counts` は `Finished` の最終集計です。入力拒否ケースはイベントを発行しません。`cancel-load-search` は `load` プロファイル専用で、`Started` 受信後、`Finished` 前に中断操作を配送できるテスト環境で実行します。中断の結果件数は固定せず、イベント件数と最終集計の整合を確認します。Tauri 実装は [テストアダプター](../tauri/tests/run-backend-cases.py) でこの JSON を実行します。PowerShell 実装は仕様と API が異なるため、同じ文書を使って既存動作だけを検証します。

あいまい検索の3ケースでは `matchType` と `score` も期待値に含めます。確認項目と手順は [あいまい検索の確認方針](../docs/fuzzy-search-test-plan.md) にまとめています。

`manifest.json` は生成した相対パス一覧とシードを記録します。`search/` と `load/` の文書はテスト専用で、実際の利用者文書や正規の Office ファイルの代用品として配布しません。
