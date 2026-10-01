# 追加機能の実装状況

更新日: 2026-10-01（日本時間）
対象: [Workから移管した実装計画](./implementation-plan.md)
確認したリリース: [`v2.0.7`](https://github.com/kz-oshiro/docs-search/releases/tag/v2.0.7)、対象コミット`508787bfeb3045bf2ec823aaf8d67a7a2eecd074`

この文書は計画の進捗を記録する。**実装済み**はコードと文書が作業ツリーにあること、**確認済み**は実際に行った検証、**公開済み**はリリースをそれぞれ指す。テスト方針の存在だけを検証完了とは扱わない。

## 段階別の状況

| 段階 | 計画上の成果 | 現在の状態 | 根拠と残作業 |
| --- | --- | --- | --- |
| P0 | 検索単位・Excel行・索引の共通基盤 | `v2.0.6` に実装・公開済み。自動確認済み、GUI未確認 | [受け入れ条件 A-13](./requirements.md)、[確認方針](./search-foundation-test-plan.md)、[公開記録](./release-notes-v2.0.6.md) |
| P1 | 結果の一括コピー、CSV/TSV、JSON調査記録 | `v2.0.6` に実装・公開済み。自動確認済み、GUI未確認 | [受け入れ条件 A-12](./requirements.md)、[確認方針](./result-export-test-plan.md)、[公開記録](./release-notes-v2.0.6.md) |
| P2 | Excel周辺セル表示と変更検出 | `v2.0.7`で実装・自動確認・Windowsビルド・公開済み。GUI未確認 | [受け入れ条件 A-14](./requirements.md)、[確認方針](./excel-context-test-plan.md)、[検証・公開記録](./release-notes-v2.0.7.md) |
| P3 | AND/OR/除外、検索箇所・Excel行・ファイル範囲 | `v2.0.7`で実装・自動確認・Windowsビルド・公開済み。GUI未確認 | [受け入れ条件 A-15](./requirements.md)、[確認方針](./condition-search-test-plan.md)、[検証・公開記録](./release-notes-v2.0.7.md) |
| P4a | Excel数式文字列、PowerPointノート | 未着手 | [計画のP4a節](./implementation-plan.md)から着手する |
| P4b | Excelコメント、Wordヘッダー・フッター・コメント | 未着手 | P4aの後に着手する |
| P5 | 検索順位の根拠とコアでの順位付け | 未着手 | P3・P4bの後に着手する。現在のGUI内の順位付けはP5完了を意味しない |
| P6 | 複数IDの一括検索 | 未着手 | P1・P3・P5の後に着手する |

計画の元の機能番号5「新旧設計書の差分表示」は、この計画の対象外である。

## 確認と公開の記録

- `v2.0.6` はコミット `21dff94` に対応する。[公開記録](./release-notes-v2.0.6.md)では PowerShell テスト、共通バックエンド27ケース、Rust 13テスト、WASMチェック、Windowsビルド、索引なし・初回作成・再利用のCLI確認に成功したと記録している。[GitHub Release](https://github.com/kz-oshiro/docs-search/releases/tag/v2.0.6)も公開済みである。
- 同じ公開記録に、GUI操作の確認は未実施と明記されている。したがってP0・P1の画面操作は自動確認済みとは呼ばない。
- P2・P3の[確認方針](./excel-context-test-plan.md)・[確認方針](./condition-search-test-plan.md)を読み、2026-10-01にPowerShell回帰テストとローカルCIを実行した。共通27ケース、P2専用CLI 5項目、P3専用CLI 31項目、Rust 18テスト、WASMチェック、Windowsビルドが成功した。詳細と成果物のSHA-256は[検証記録](./release-notes-v2.0.7.md)に残す。
- GUIテストは依頼者が実施しないと指定したため、アプリ起動を含め実施していない。自動確認の成功を画面操作の成功とは扱わない。
- コミット`508787b`を`main`へ反映し、同じコミットに`v2.0.7`を付けて[GitHub Release](https://github.com/kz-oshiro/docs-search/releases/tag/v2.0.7)を公開した。公開アセット`docs-search-desktop.exe`は12,070,400 bytesで、GitHub上のSHA-256 `84BB1755FAA430A290905F70E69C118F05A97062D6BB570D53E583E0925A81F8`がローカル成果物と一致する。

## 仕様から実装までの対応

| 段階 | 要求と契約 | 主な実装 | 確認方針 |
| --- | --- | --- | --- |
| P0 | [A-13](./requirements.md)、[境界契約](./boundary.md)、[抽出と索引](./backend.md) | [`extract.rs`](../tauri/core/src/extract.rs)、[`index.rs`](../tauri/core/src/index.rs) | [P0](./search-foundation-test-plan.md) |
| P1 | [A-12](./requirements.md)、[GUI仕様](./gui.md)、[出力契約](./boundary.md) | [`report.rs`](../tauri/core/src/report.rs)、[`frontend/src/lib.rs`](../tauri/frontend/src/lib.rs)、[`src-tauri/src/main.rs`](../tauri/src-tauri/src/main.rs) | [P1](./result-export-test-plan.md) |
| P2 | [A-14](./requirements.md)、[GUI仕様](./gui.md)、[周辺取得契約](./boundary.md) | [`context.rs`](../tauri/core/src/context.rs)、[`extract.rs`](../tauri/core/src/extract.rs)、[`frontend/src/lib.rs`](../tauri/frontend/src/lib.rs) | [P2](./excel-context-test-plan.md)、[専用データ生成器](../tests/generate-context-fixture.py) |
| P3 | [A-15](./requirements.md)、[検索規則](./backend.md)、[条件の境界契約](./boundary.md) | [`query.rs`](../tauri/core/src/query.rs)、[`core/src/lib.rs`](../tauri/core/src/lib.rs)、[`frontend/src/lib.rs`](../tauri/frontend/src/lib.rs) | [P3](./condition-search-test-plan.md)、[専用データ生成器](../tests/generate-conditions-fixture.py) |

P0・P1は同じ`v2.0.6`コミットで公開した。P2・P3は同じ`v2.0.7`コミットで検証・公開した。計画の配置案にあった`group.rs`は作らず、P3の行単位の集約は`query.rs`に実装した。P3の索引経路は検索漏れを避けるため、まず対象ファイルの保存済み検索単位を全件読み込む。大きなファイルでの性能評価は未実施である。

## 次の確認と実装

1. GUIの周辺表、条件入力、根拠表示、出力は未確認。後日確認するときは現在の`AGENTS.md`に従い、GUIテストの依頼が明示されていなければ実施前に確認し、自動確認と別に記録する。
2. 次の機能実装はP4a。計画の対象・抽出規則と、現行の検索契約を読んでから仕様・コード・確認方針を同じ変更で整える。

この文書の状態は上記更新日時のチェックアウトに対応する。以後のコミットや公開によって変わるため、次回の作業では`git status`、`git log`、リリース記録と照合して更新する。
