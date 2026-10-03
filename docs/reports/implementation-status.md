# 追加機能の実装状況

2026-10-03追記: Rust版の自動テスト一式、Playwright 51件、Windows Releaseビルドが成功した。U31で見つかった一括入力プレビューの過剰要求を120msの遅延集約で修正し、再実行で確認した。結果をレビューし、[v3.0.0](https://github.com/kz-oshiro/docs-search/releases/tag/v3.0.0)を公開した。成果物・実施範囲は [v3.0.0検証記録](../releases/validation-v3.0.0.md)。アプリ起動・実GUIは未実施。

2026-10-03追記（Playwright導入時点）: [Playwright導入報告](implementation-report-playwright.md)と [自動/手動境界](../test-plans/playwright-ui-test-plan.md)を追加した。この時点ではヘッドレスUI試験コードの実行は未実施。従来の自動検証結果をUI成功へ読み替えない。

2026-10-03追記（ローカル移行時点）: PowerShell 版を [docs-search-ps](https://github.com/kz-oshiro/docs-search-ps) へ分離するローカル移行を実装・静的確認した。コピーした本体・共有テストデータはハッシュで照合した。この時点は自動テスト、Rust ビルド、GUI 確認、リモート反映が未実施だった。移行手順は [PowerShell 移行計画](../plans/powershell-repository-migration-plan.md)、移行記録は同リポジトリの `docs/migration-record.md` に記載した。その後、PowerShell版のテスト・配布ZIP作成と [v1.0.0公開](https://github.com/kz-oshiro/docs-search-ps/releases/tag/v1.0.0)を完了した。

2026-10-03追記（移行文書レビュー時点）: PowerShell の確認範囲・保存用出力・再開段階と、Rust の独立したケース原本・確認コマンドの作業ディレクトリ・自動テスト/配布ビルドの入口を修正した。P0 の予約入力拒否と P2 の書式未対応という古い確認条件も、現在の P3 / イシュー対応に合わせた。ソース・設定・生成器・ケース原本は変更せず、文書の静的確認だけを実施した。この時点は移行後の自動テスト・ビルド・GUI・リモート反映が未実施だった。

2026-10-03追記（PowerShell修正実装時点）: ユーザーの追加修正依頼により、PowerShell側でふりがなの除外、終了時の結果回収、ZIP/XML上限を実装し、自動入口へ専用回帰を追加した。詳細は `docs-search-ps/docs/powershell-source-fixes.md` と同リポジトリの移行記録を参照する。移行後の保守変更であり、Rust側のソース・設定・スクリプト・共通生成器・ケース原本は変更していない。この時点は修正後の自動テスト・ビルド・GUI・リモート反映が未実施だった。

更新日: 2026-10-03（日本時間）

現在の要求/ソース/配布版は **v3.0.0**。[Rust要求仕様](../specifications/rust-requirements-v3.0.0.md)と [検証記録](../releases/validation-v3.0.0.md)を基準とする。v2系公開記録は履歴として保持する。Rust/CLI/WASM/Playwrightの自動検証とWindows配布ビルドを確認して公開済み。実アプリGUIは未実施。

proptestをテスト専用依存へ追加し、17性質を各1,024ケースで実行した。Rustは64件成功（固定回帰/公開API47件＋17性質）。生成で発見したあいまい候補漏れを修正し、保存seed・固定例・直接/索引CLI比較を追加した。[確認方針](../test-plans/property-testing-test-plan.md)と検証記録の追記を参照。
対象: [Workから移管した実装計画](../plans/implementation-plan.md)
確認したリリース: [`v3.0.0`](https://github.com/kz-oshiro/docs-search/releases/tag/v3.0.0)、対象コミット`f96af8605840306a50c76c34356b13ca3e62bf1a`

この文書は計画の進捗を記録する。**実装済み**はコードと文書が作業ツリーにあること、**確認済み**は実際に行った検証、**公開済み**はリリースをそれぞれ指す。テスト方針の存在だけを検証完了とは扱わない。

## 段階別の状況

| 段階 | 計画上の成果 | 現在の状態 | 根拠と残作業 |
| --- | --- | --- | --- |
| P0 | 検索単位・Excel行・索引の共通基盤 | `v2.0.6` に実装・公開済み。自動確認済み、GUI未確認 | [受け入れ条件 A-13](../specifications/requirements.md)、[確認方針](../test-plans/search-foundation-test-plan.md)、[公開記録](../releases/release-notes-v2.0.6.md) |
| P1 | 結果の一括コピー、CSV/TSV、JSON調査記録 | `v2.0.6` に実装・公開済み。自動確認済み、GUI未確認 | [受け入れ条件 A-12](../specifications/requirements.md)、[確認方針](../test-plans/result-export-test-plan.md)、[公開記録](../releases/release-notes-v2.0.6.md) |
| P2 | Excel周辺セル表示と変更検出 | `v2.0.7`で実装・自動確認・Windowsビルド・公開済み。GUI未確認 | [受け入れ条件 A-14](../specifications/requirements.md)、[確認方針](../test-plans/excel-context-test-plan.md)、[検証・公開記録](../releases/release-notes-v2.0.7.md) |
| P3 | AND/OR/除外、検索箇所・Excel行・ファイル範囲 | `v2.0.7`で実装・自動確認・Windowsビルド・公開済み。GUI未確認 | [受け入れ条件 A-15](../specifications/requirements.md)、[確認方針](../test-plans/condition-search-test-plan.md)、[検証・公開記録](../releases/release-notes-v2.0.7.md) |
| P4a | Excel数式文字列、PowerPointノート | v3.0.0に実装・自動確認・Windows Releaseビルド・公開済み。実GUI未確認 | [実装報告](implementation-report-p4-p6.md)、[確認方針](../test-plans/office-ranking-batch-test-plan.md)、[検証記録](../releases/validation-v3.0.0.md)、A-16 |
| P4b | Excelコメント、Wordヘッダー・フッター・コメント | v3.0.0に実装・自動確認・Windows Releaseビルド・公開済み。実GUI未確認 | 同上。共有部品・VML重複と部分障害をCLIで確認 |
| P5 | 検索順位の根拠とコアでの順位付け | v3.0.0で公開済み。固定20クエリの優先ペアと直接/初回/再利用の順位を自動確認済み | 同上、A-17。順位の根拠・ID順はコアが通知。実アプリ画面は未確認 |
| P6 | 複数IDの一括検索 | v3.0.0で公開済み。256語・境界・件数・不確定・抽出回数を自動確認済み。負荷/実GUIは未確認 | 同上、A-18。最大256語、語別集計と語×ファイル表示・CSV/TSV |

計画の元の機能番号5「新旧設計書の差分表示」は、この計画の対象外である。

## 確認と公開の記録

- `v2.0.6` はコミット `21dff94` に対応する。[公開記録](../releases/release-notes-v2.0.6.md)では PowerShell テスト、共通バックエンド27ケース、Rust 13テスト、WASMチェック、Windowsビルド、索引なし・初回作成・再利用のCLI確認に成功したと記録している。[GitHub Release](https://github.com/kz-oshiro/docs-search/releases/tag/v2.0.6)も公開済みである。
- 同じ公開記録に、GUI操作の確認は未実施と明記されている。したがってP0・P1の画面操作は自動確認済みとは呼ばない。
- P2・P3の[確認方針](../test-plans/excel-context-test-plan.md)・[確認方針](../test-plans/condition-search-test-plan.md)を読み、2026-10-01にPowerShell回帰テストとローカルCIを実行した。共通27ケース、P2専用CLI 5項目、P3専用CLI 31項目、Rust 18テスト、WASMチェック、Windowsビルドが成功した。詳細と成果物のSHA-256は[検証記録](../releases/release-notes-v2.0.7.md)に残す。
- GUIテストは依頼者が実施しないと指定したため、アプリ起動を含め実施していない。自動確認の成功を画面操作の成功とは扱わない。
- コミット`508787b`を`main`へ反映し、同じコミットに`v2.0.7`を付けて[GitHub Release](https://github.com/kz-oshiro/docs-search/releases/tag/v2.0.7)を公開した。公開アセット`docs-search-desktop.exe`は12,070,400 bytesで、GitHub上のSHA-256 `84BB1755FAA430A290905F70E69C118F05A97062D6BB570D53E583E0925A81F8`がローカル成果物と一致する。
- 2026-10-03にRust版の自動検証一式を再実行し、Rust 64件（17性質×1,024ケース）、CLI受け入れ、WASM型検査、Playwright 51件が成功した。Windows Releaseビルドも成功。U31で検出した複数の入力イベントによるプレビュー要求の重複を120msで集約し、U30/U31と自動一式で修正を確認した。成果物・詳細は [v3.0.0検証記録](../releases/validation-v3.0.0.md)。このテスト・ビルドを終えた時点では起動・GUI・commit/push/tag/Releaseは未実施だった。
- 結果と差分のレビュー後、コミット`f96af86`をmainへpushし、同じコミットに`v3.0.0`を付けて正式Releaseを公開した。GitHub上のEXEは13,120,512 bytes、SHA-256 `3BD1F22CCC818EF9EF1A532E141E9B6B4D2B655B0C3E7DD8854C7CB859714EEE`でローカル成果物と一致した。実アプリ起動・GUI・イシュー完了操作は行っていない。

## 仕様から実装までの対応

| 段階 | 要求と契約 | 主な実装 | 確認方針 |
| --- | --- | --- | --- |
| P0 | [A-13](../specifications/requirements.md)、[境界契約](../specifications/boundary.md)、[抽出と索引](../specifications/backend.md) | [`extract.rs`](../../core/src/extract.rs)、[`index.rs`](../../core/src/index.rs) | [P0](../test-plans/search-foundation-test-plan.md) |
| P1 | [A-12](../specifications/requirements.md)、[GUI仕様](../specifications/gui.md)、[出力契約](../specifications/boundary.md) | [`report.rs`](../../core/src/report.rs)、[`frontend/src/lib.rs`](../../frontend/src/lib.rs)、[`src-tauri/src/main.rs`](../../src-tauri/src/main.rs) | [P1](../test-plans/result-export-test-plan.md) |
| P2 | [A-14](../specifications/requirements.md)、[GUI仕様](../specifications/gui.md)、[周辺取得契約](../specifications/boundary.md) | [`context.rs`](../../core/src/context.rs)、[`extract.rs`](../../core/src/extract.rs)、[`frontend/src/lib.rs`](../../frontend/src/lib.rs) | [P2](../test-plans/excel-context-test-plan.md)、[専用データ生成器](../../tests/fixtures/generate-context-fixture.py) |
| P3 | [A-15](../specifications/requirements.md)、[検索規則](../specifications/backend.md)、[条件の境界契約](../specifications/boundary.md) | [`query.rs`](../../core/src/query.rs)、[`core/src/lib.rs`](../../core/src/lib.rs)、[`frontend/src/lib.rs`](../../frontend/src/lib.rs) | [P3](../test-plans/condition-search-test-plan.md)、[専用データ生成器](../../tests/fixtures/generate-conditions-fixture.py) |

P0・P1は同じ`v2.0.6`コミットで公開した。P2・P3は同じ`v2.0.7`コミットで検証・公開した。計画の配置案にあった`group.rs`は作らず、P3の行単位の集約は`query.rs`に実装した。P3の索引経路は検索漏れを避けるため、まず対象ファイルの保存済み検索単位を全件読み込む。大きなファイルでの性能評価は未実施である。

P4a〜P4bは `extract.rs` / `index.rs` / `context.rs`、P5は `ranking.rs` と通知経路、P6は `batch.rs` / `report.rs` と画面に実装した。最新の確認方針は [P4〜P6](../test-plans/office-ranking-batch-test-plan.md)。旧公開記録の成功を今回の変更の確認済み状態として引き継がない。

## 次の確認と実装

1. 自動検証とWindows Releaseビルドは完了。実アプリのOS連携・見た目は [限定票M1〜M5](../test-plans/playwright-ui-test-plan.md#ユーザー確認票実アプリの最終5項目)に従う。アプリ起動・GUIは未実施で、自動確認と別に記録する。
2. v3.0.0のcommit・push・tag・Releaseは完了。PowerShell版の回帰と配布は独立リポジトリのv1.0.0で完了している。性能測定・実Office互換性の追加評価・イシュー完了操作は未実施。再現コマンドと公開結果は [検証記録](../releases/validation-v3.0.0.md)を参照。

この文書の状態は上記更新日時のチェックアウトに対応する。以後のコミットや公開によって変わるため、次回の作業では`git status`、`git log`、リリース記録と照合して更新する。

## 2026-10-03: GitHubイシュー #1〜#9

全9件のコード・仕様・回帰確認の定義を作業ツリーへ追加した。Excelレイアウトは相談で罫線・塗りつぶし・文字書式も含めることを確認し、保存書式の近似表示を実装した。抽出仕様版は4へ更新した。P5のファイル評価を維持し、ファイル内の箇所を文書位置順に変更した。詳細は[実装報告](implementation-report-issues.md)、次の担当の確認項目・手順・期待結果は[確認方針](../test-plans/issues-test-plan.md)。

イシュー実装時点は静的確認のみだったが、v3.0.0の自動検証で可視文字・位置順・更新日時・周辺応答・編集のコア動作を確認し、Windowsビルドと公開を完了した。proptest導入後のRust64件と各CLI・WASM型検査、Playwright 51件、公開アセットの [検証記録](../releases/validation-v3.0.0.md)を参照。実GUI・アプリ起動・性能測定・イシュー完了操作は未実施。従来の公開済みv2.0.7の成功を今回へ引き継いだものではない。
