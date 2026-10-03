# Cargo 集約・WASM 撤去の検証方針

2026-10-04方針更新: 試験はバックエンド試験・フロントエンド試験・アプリケーション結合試験の3区分とする。バックエンド試験・[フロントエンド試験](playwright-ui-test-plan.md)でカバーできないもののみ[アプリケーション結合試験](playwright-exe-test-plan.md)へ回す。G01/G02の構造・配色はフロントエンド試験で担当する。追加ランナー・拡張は未実装。Rust製OS操作補助は導入せず、OS固有操作・表示は保証対象外とする。M1〜M5の必須手動票は廃止。以下は移行時の手順・実施状況として保持する。

2026-10-03。この方針の作成時点では実装と静的確認のみだった。同日の後続依頼で手順1〜4の準備・全自動試験・旧生成器比較・Windows Release ビルドが成功した。[v3.0.1 検証記録](../releases/validation-v3.0.1.md)を参照する。手順5の旧/新 cold・warm 比較と実アプリ起動/GUIは未実施。次の検証担当はこの方針と[実装報告](../reports/implementation-report-cargo-native-ui.md)を読んで、依頼された範囲を実行する。

## 自動検証の手順と期待結果

| 順序 | 手順 | 期待結果 |
|---|---|---|
| 1 | Rust/MSVC 1.91.1、C++ Build Tools、Node 20以上を確認し `cargo xtask setup` | 固定 npm lock で Playwright 1.63.0 と Chromium を準備。Python/PowerShell/WASM target/wasm-bindgen-cli は通常工程に不要 |
| 2 | `cargo xtask test` | core 単体/API/proptest、生成器、実 CLI の全6群、ヘッドレス UI が成功。全群を実行し、失敗/前提不足も工程別に記録 |
| 3 | 下記の旧生成器比較を一度実施 | 非ZIPの全バイト、ZIP部品の順序/展開内容/固定日時/圧縮方式が一致。圧縮ストリーム/ヘッダー差は別記録。差異を期待値変更で隠さない |
| 4 | `cargo xtask build` または検証依頼範囲の `cargo xtask ci` | 実frontend8資材が取り込まれ、現在ビルドが成功した EXE だけを新規成果物先へコピー。SHA256/サイズを記録。ciの必須テスト失敗時はビルドしない |
| 5 | 下記の性能計測を実施 | 同一環境の旧/新 cold と warm中央値を記録。工程別所要時間と外側の総時間を分けて説明 |

CLI の確認項目は `core/tests/cli_{backend,context,conditions,office,issues,index}.rs`。共通27、周辺5、条件31、順位20組を維持する。終了/sequence/searchId/件数、Unicode範囲、要求オプション、注記/数式の既定off、部分エラーと索引保存拒否、一括の256上限・重複語・識別子境界・未確定0件、索引offの無書込・mtime/サイズ・削除・破損・旧版・利用不能を確認する。周辺・編集・出力の実API/単体試験も残す。全child CLIは自身の一時 LOCALAPPDATAを使う。

UI は既存 U01〜U39 の51件に、U40資材/WASM要求なし、U41起動資材失敗、U42CRLF要求を追加した54件。モックはTauri/OSのみ。通常起動の例外・HTTP失敗・未解決応答は不合格とする。U41だけは意図した app.js の読み込み失敗を発生させ、起動エラー表示と検索不可を判定する。画像/動画/トレースは使わない。[既存 UI 方針](playwright-ui-test-plan.md)の全期待値を引き継ぐ。

## 旧生成器との一度の比較

基準ソースは実装前 commit `750c4674df79c3a0d7f9bd014462ace525e8dcbc`。検証担当が独立した一時 checkout に取り出し、元のワークツリーや保存データを変更しない。旧 Python はこの移行比較だけで使用する。

旧 `tests/fixtures/generate-fixtures.py` を acceptance/load でそれぞれ新規出力先へ実行し、新版は次で対応するデータを作る。

```text
cargo xtask fixtures --profile acceptance --output outputs/migration-validation/new-acceptance
cargo xtask fixtures --profile load --output outputs/migration-validation/new-load
cargo xtask compare-fixtures --old <旧acceptance> --new outputs/migration-validation/new-acceptance
cargo xtask compare-fixtures --old <旧load> --new outputs/migration-validation/new-load
```

context/conditions/office/issues も旧生成器と `cargo xtask fixtures --kind <種類> --output <新規先>` で比較する。旧contextはファイルを指定するので、比較用の親フォルダーに `row-window.xlsx` を生成する。再生成は別の新規先で行い、新版同士は全ファイルのSHA256が一致することを確認する。共通 acceptance の2回生成は Rust 自動試験にも含む。

`fixture-comparison.json` は同じ文書在庫を判定し、非ZIPは全バイト、ZIPは順番ごとの名前・展開内容・日時・圧縮方式を照合する。ZIP全体のSHA256差は `zipSerializationDifferences` に記録する。旧PythonのZIP/ZlibとRust ZIP/flate2は圧縮バイトやヘッダーが異なり得る。展開内容と意味の一致が必須で、バイト差の存在も記録に残す。Office100件の450〜550KB、seed20260927、acceptance33/load148、原本27ケースも確認する。

アイコンは `test-support/tests/icons.rs` が承認済み ICO の全バイトと PNG の展開したRGBA行を比較する。通常の検証では tracked icon を書き換えず、一時先で生成する。

## 失敗・前提不足と成果物

不足したNode/Playwright/Chromiumを通常コマンドで自動導入しない。UIは理由付き skipped、全体は非ゼロ。テスト1群の失敗後も独立した残りの群を実行する。ciのビルドは全必須成功でのみ進む。必要なら不足/意図した失敗を隔離環境で作り、工程記録とビルド未実施を確かめる。

各 `outputs/runs/<ID>/` の report.json/report.md、stdout/stderr、ranking-top5.json、ui/results.json/テキスト失敗添付を読む。commit/dirty・実行版・開始/終了・工程時間・終了コード・前提不足理由を照合する。成功EXEとSHA256が同じ現在実行に属することを確認する。既存EXEや過去の成功ログを今回の証拠にしない。

## 性能計測

旧版と新版で同じWindows端末・電源設定・Rust/MSVC・文書・オプションを使う。setup/ダウンロードは先に済ませ、coldは別の新規Cargo targetに固定する。既存targetを削除しない。旧版の生成物掃除も元ワークツリーでは行わない。

外側のStopwatch等でコマンド開始から終了までを測り、xtask自身の初回コンパイルを含む総時間を記録する。レポートの工程時間はxtask起動後の値なので、総時間の代わりにしない。coldテスト/coldビルド各1回、同じキャッシュでwarmテスト/warmビルド各3回を実行し、warm中央値を比較する。Git dirty、版、環境、コマンド、ターゲット先、全件の成否を同時に保存する。

移行の目的はWASM生成/チェックと複数ランナーのビルド重複・キャッシュ削除を減らすこと。検索コアのアルゴリズムは変えていない。検索の実行速度やEXEサイズが改善したと結論するには別に同じデータ/条件で測定する。

移行時には実アプリ起動・GUIを実施していない。今後は [アプリケーション結合試験設計](playwright-exe-test-plan.md)の有効なEケース、[フロントエンド試験設計](playwright-ui-test-plan.md)のU/Gケース、保証対象外を区別し、実ダイアログなどをユーザーの必須確認票へ移さない。
