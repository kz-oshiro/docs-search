# 共通テストデータ

このディレクトリは PowerShell 実装と Tauri 実装が共用するテスト入力を管理します。検索エンジン、GUI、通信方式が変わっても、文書データと期待結果の意味を同じに保ちます。

| ファイル | 役割 |
| --- | --- |
| `generate-fixtures.py` | Python 3 標準ライブラリだけで検索用文書を生成 |
| `backend-cases.json` | バックエンドの操作、検索入力、期待結果を記述した共通契約 |
| `test_generator.py` | 同一シードのバイト一致、Office ZIP/XML、ケースの参照先を確認 |
| `run-powershell-tests.ps1` | 共通データを一時生成して PowerShell 実装の Excel 検索を確認し、終了時に削除 |

## 生成と実行

Windows PowerShell 5.1 と Python 3 を使用します。`docs-search` を作業ディレクトリとして実行します。

```powershell
powershell.exe -NoProfile -ExecutionPolicy RemoteSigned -File .\tests\run-powershell-tests.ps1
```

`run-powershell-tests.ps1` は OS の一時ディレクトリ内に `search/`、`manifest.json`、`backend-cases.json` を作り、成功・失敗のどちらでも `finally` で削除します。バックエンド再構築版のテストランナーも、各テストの前に同じジェネレータを実行し、終了時に一時ディレクトリを削除してください。生成物は Git で管理しません。

データを目視確認するときは次を実行します。既定の `tests/generated/` は Git の除外対象です。ジェネレータは出力先が空のときだけ動作し、既存データを上書きしません。既存の出力を残して新しい内容を生成する場合は、別の空ディレクトリを `--output` に指定します。

```powershell
python .\tests\generate-fixtures.py
python .\tests\generate-fixtures.py --output C:\Temp\docs-search-corpus --profile load
```

`acceptance` プロファイルは `search/` に 18 ファイルを作ります。対象10拡張子、入れ子のフォルダー、非表示シート・スライド、Excel の共有文字列・セル・保存済み数式結果・図形、PowerPoint の図形・表、Word の本文・入れ子表、UTF-8 と BOM、Shift_JIS の `.txt` と `.html`、破損・不正な文字コード・対象外形式・Office 一時ファイルを含みます。`.jsp`、`.xhtml`、`.js`、`.java` はそれぞれ1ファイル、`.html` は2ファイルです。Excel は 3 シート各 295 行 × 12 列、PowerPoint は 165 スライド各 8 図形、Word は 3400 本文段落と表、テキストは各 150 行を生成します。`load` は `search/` の内容に加えて、独立した `load/` に Excel 32 個、PowerPoint 32 個、Word 31 個、テキスト 20 個を生成し、合計133ファイルになります。正常な Office 文書は両ディレクトリを合わせて100ファイルで、各ファイルはおおむね 0.5 MB（45万～55万バイト）です。破損した `.xlsx`、Office 一時ファイル、対象外の `.xls` はこの100ファイルに含めません。性能の合格時間はここでは定めません。

固定シードは `20260927` です。乱数は各ファイル名から独立に初期化し、ZIP 内の時刻・格納順も固定します。同じプロファイル、ジェネレータ、検証ケース原本からは同じバイト列を出力します。シードや内容を変更する場合は、期待結果と一緒に更新します。

## バックエンド検証データ

`backend-cases.json` が管理対象の原本で、生成時に同一内容を出力先へコピーします。JSON は言語や実装の型を指定しません。テストアダプターは `${fixtureRoot}` を生成先の絶対パスに置換し、OS のパス区切りに変換して `SearchRequest` と操作を実行します。`expected.results[].file` と `expected.issues[].file` は `search/` からの相対パスです。結果の `sourceKind`、構造化 `location` の掲載フィールド、`textContains` を照合し、余分な結果・Issue がないことを件数と併せて確認します。`resultId`、`searchId`、通知の到着順、プレビューの全文、利用者向けエラー文言には依存しません。

`expected.counts` は `Finished` の最終集計です。入力拒否ケースはイベントを発行しません。`cancel-load-search` は `load` プロファイル専用で、`Started` 受信後、`Finished` 前に中断操作を配送できるテスト環境で実行します。中断の結果件数は固定せず、イベント件数と最終集計の整合を確認します。Tauri 実装は [テストアダプター](../tauri/tests/run-backend-cases.py) でこの JSON を実行します。PowerShell 実装は仕様と API が異なるため、同じ文書を使って既存動作だけを検証します。

`manifest.json` は生成した相対パス一覧とシードを記録します。`search/` と `load/` の文書はテスト専用で、実際の利用者文書や正規の Office ファイルの代用品として配布しません。
