# 実装・設計の履歴

過去の変更内容と設計判断を記録する。現在の状態と次の作業は[実装状況](implementation-status.md)、実行手順は[開発手順](../development.md)を参照する。案件の詳細は開発レポート、公開版の案内は[GitHub Releases](https://github.com/kz-oshiro/docs-search/releases)、技術的な検証は[公開検証MD](validation/v3.0.4.md)へ分ける。整理前の履歴全文は[保存版](https://github.com/kz-oshiro/docs-search/blob/8894cf81e653d8917895004f323925355b95fec1/docs/reports/implementation-history.md)で参照できる。

## 開発レポート一覧

| 日付 | 案件 | 記録の扱い |
| --- | --- | --- |
| 2026-10-03 | [Office検索範囲・順位・一括検索](development/2026-10-03-office-ranking-batch.md) | 旧報告から移行。当時の判断と後続検証を分ける |
| 2026-10-03 | [イシュー9件への対応](development/2026-10-03-issues.md) | 同上。イシュー開閉状態は再確認していない |
| 2026-10-03 | [画面の自動テスト導入](development/2026-10-03-ui-tests.md) | 当時のWASM構成と現行の実行手順を区別する |
| 2026-10-04 | [検索処理の重複削減](development/2026-10-04-backend-optimization.md) | v3.0.4の実装・検証参照と残る性能比較 |

今後は完了した案件の概要とリンクをこの一覧へ追加し、工程別の詳細を下へ転記しない。進行中の案件は[実装状況](implementation-status.md#開発レポート)へ置く。

## 2026-10-01〜2026-10-03: 検索・出力・Office・一括検索

[機能実装計画](../plans/implementation-plan.md)に沿って、P0の検索単位・Excel行・索引基盤、P1の一括コピー/CSV/TSV/JSON出力、P2の周辺セルと変更検出、P3のAND/OR/除外と判定範囲を追加した。新旧設計書の意味的差分表示は対象外とした。

| 段階 | 要求と契約 | 主な実装 | 確認方針 |
| --- | --- | --- | --- |
| P0 | [A-13](../specifications/requirements.md)、[境界契約](../specifications/boundary.md)、[抽出と索引](../specifications/backend.md) | [`extract.rs`](../../core/src/extract.rs)、[`index.rs`](../../core/src/index.rs) | [P0](../test-plans/search-foundation-test-plan.md) |
| P1 | [A-12](../specifications/requirements.md)、[GUI仕様](../specifications/gui.md)、[出力契約](../specifications/boundary.md) | [`report.rs`](../../core/src/report.rs)、[`view.js`](../../frontend/view.js)、[`src-tauri/src/main.rs`](../../src-tauri/src/main.rs) | [P1](../test-plans/result-export-test-plan.md) |
| P2 | [A-14](../specifications/requirements.md)、[GUI仕様](../specifications/gui.md)、[周辺取得契約](../specifications/boundary.md) | [`context.rs`](../../core/src/context.rs)、[`extract.rs`](../../core/src/extract.rs)、[`view.js`](../../frontend/view.js) | [P2](../test-plans/excel-context-test-plan.md)、[専用データ生成器](../../test-support/src/specialized.rs) |
| P3 | [A-15](../specifications/requirements.md)、[検索規則](../specifications/backend.md)、[条件の境界契約](../specifications/boundary.md) | [`query.rs`](../../core/src/query.rs)、[`core/src/lib.rs`](../../core/src/lib.rs)、[`app.js`](../../frontend/app.js) | [P3](../test-plans/condition-search-test-plan.md)、[専用データ生成器](../../test-support/src/specialized.rs) |

計画の配置案にあった `group.rs` は作らず、P3の行単位集約は `query.rs` に実装した。当初の索引経路は検索漏れを避けるため、対象ファイルの保存済み検索単位を全件読み込む方式を選んだ。公開版の記録は[P0・P1のv2.0.6](https://github.com/kz-oshiro/docs-search/releases/tag/v2.0.6)、[P2・P3のv2.0.7](https://github.com/kz-oshiro/docs-search/releases/tag/v2.0.7)を参照する。

P4a〜P4bはOfficeの数式・注記・追加部品を `extract.rs` / `index.rs` / `context.rs`、P5は順位と根拠を `ranking.rs` と通知経路、P6は最大256語の一括検索を `batch.rs` / `report.rs` と画面に追加した。注記・数式は初期オフとし、部分障害で条件成立や索引確定をしない。詳細は[開発レポート](development/2026-10-03-office-ranking-batch.md)、確認項目は[P4〜P6の試験設計](../test-plans/office-ranking-batch-test-plan.md)を参照する。

## 2026-10-03: イシュー #1〜#9と性質テスト

折りたたみ・本文フィルター・編集・Excel書式・更新日時・フォルダー復元・位置順・ふりがな除外を追加した。Excel書式は罫線・塗りつぶし・文字書式の近似表示とし、抽出仕様版を4へ更新した。P5のファイル評価を維持し、ファイル内の箇所を文書位置順へ変更した。詳細は[開発レポート](development/2026-10-03-issues.md)、確認項目は[イシュー試験設計](../test-plans/issues-test-plan.md)を参照する。

proptestをテスト専用依存へ追加した。生成で見つかったあいまい候補漏れを修正し、保存seed・固定例・直接/索引CLI比較を残した。[性質テストの確認方針](../test-plans/property-testing-test-plan.md)と[v3.0.0の公開記録](https://github.com/kz-oshiro/docs-search/releases/tag/v3.0.0)を参照する。

## 2026-10-03: Playwright導入とCargo集約

実WASM画面を操作し、Tauri/クリップボード境界だけをモック化するヘッドレスPlaywrightを自動入口へ接続した。画像・動画・トレースを保存せず、失敗時はテキストの状態・要求・応答を記録する方針とした。初回実装時点の内容は[Playwright導入報告](development/2026-10-03-ui-tests.md)、公開版の結果は[v3.0.0](https://github.com/kz-oshiro/docs-search/releases/tag/v3.0.0)を参照する。

U31で見つかった一括入力プレビューの過剰要求は120msで集約し、古い応答による上書きを防ぐ処理を追加した。

続いて画面をHTML/CSS/JavaScriptのES modulesへ移し、WASMを撤去した。Cargo workspace・lockfileと `cargo xtask` を共通入口にし、Python/PowerShellの試験・生成ランナーをRust integration tests / `test-support` へ移した。検索コア・Tauriの処理と既存の画面契約を保ち、資材stagingとビルドキャッシュを共有する構成にした。公開版の結果は[v3.0.1](https://github.com/kz-oshiro/docs-search/releases/tag/v3.0.1)を参照する。

## 2026-10-03: PowerShell版の分離

PowerShell検索版を[docs-search-ps](https://github.com/kz-oshiro/docs-search-ps)へ分離し、Rust版の現行使用・メンテ・CIから外した。ふりがなの除外・終了時の結果回収・ZIP/XML上限の変更は独立リポジトリの保守変更として扱い、Rust側のソース・設定・共通ケース原本へ混ぜなかった。移行計画は[保存版](https://github.com/kz-oshiro/docs-search/blob/059ba7540fbdc762feb5010245b1d6e31a71c716/docs/plans/powershell-repository-migration-plan.md)、公開記録は[PowerShell版のRelease](https://github.com/kz-oshiro/docs-search-ps/releases/tag/v1.0.0)を参照する。

## 2026-10-03〜2026-10-04: 試験の振り分けと文書方針

実EXE用の接続試験を設計し、バックエンド試験・フロントエンド試験で確認できる内容を結合試験から外した。旧E03/E05の検索規則・条件・順位は個別試験へ、E04/E06/E11の組合せ・表示分岐も個別試験へ振り分け、結合試験には接続の確認だけを残した。G01/G02の構造・テーマ配色はフロントエンド試験へ移した。

旧E12のTSV整形確認も結合試験から外し、検索規則・索引・文字コードなどの網羅はバックエンド、実EXEは接続の代表例を確認する設計とした。

Rust製OS操作補助は導入せず、必須のM1〜M5手動票を廃止した。試験分類と保証範囲は[結合試験設計](../test-plans/playwright-exe-test-plan.md)、フロントエンドの確認方法は[UI試験設計](../test-plans/playwright-ui-test-plan.md)を正本とする。追加ランナー・ケースの現在の実装状況は[実装状況](implementation-status.md)を参照する。

現行仕様・手順・試験設計の文書改訂を製品版番号から独立させた。改訂履歴をGitへ残し、公開版ごとの変更点・検証結果はReleaseへ分ける方針とした。公開版の記録は[v3.0.2](https://github.com/kz-oshiro/docs-search/releases/tag/v3.0.2)を参照する。

## 2026-10-04: CIとバックエンドの重複処理削減

アイコンの属性・形状計算を事前計算し、可視図形を見つけた時点で評価を打ち切るようにした。バックエンドとフロントエンド試験を同時に実行し、UI開始前のCPU使用率からworker数を決める構成へ変更した。計算方法・確認項目は[CI性能試験設計](../test-plans/ci-performance-test-plan.md)、比較測定の結果は[v3.0.3](https://github.com/kz-oshiro/docs-search/releases/tag/v3.0.3)を参照する。

一括検索の正規化・トークン化共有、繰り返し文字列の境界判定、必要時だけの変更検出ハッシュ、索引SQLとExcelメタデータの再利用を追加した。意味や契約を維持する確認項目は[バックエンド性能試験設計](../test-plans/backend-performance-test-plan.md)、検証結果と測定範囲は[v3.0.4](https://github.com/kz-oshiro/docs-search/releases/tag/v3.0.4)を参照する。
