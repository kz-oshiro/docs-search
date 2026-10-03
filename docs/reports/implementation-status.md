# 実装状況

現行の実装状況を示す。機能の基準は[Rust要求仕様](../specifications/rust-requirements.md)、詳細手順は[開発手順](../development.md)、対象コミット・実行結果・公開成果物は[公開・検証記録](../releases/README.md)を参照する。過去の経緯は[実装履歴](implementation-history.md)へ分けて保存する。

実装済みはコードと文書が存在すること、確認済みは実際に行った検証、公開済みは配布を指す。試験設計の存在だけを検証完了とは扱わない。

## 機能の実装

| 機能 | 状況 | 確認方針 |
| --- | --- | --- |
| 検索単位・Excel行・索引基盤（P0） | 実装済み | [検索基盤](../test-plans/search-foundation-test-plan.md) |
| 一括コピー・CSV/TSV・JSON出力（P1） | 実装済み | [出力](../test-plans/result-export-test-plan.md) |
| Excel周辺セル・変更検出（P2） | 実装済み | [周辺](../test-plans/excel-context-test-plan.md) |
| AND/OR/除外・判定範囲（P3） | 実装済み | [条件](../test-plans/condition-search-test-plan.md) |
| Office注記・数式・順位・一括検索（P4〜P6） | 実装済み | [Office/順位/一括](../test-plans/office-ranking-batch-test-plan.md) |
| 折りたたみ・編集・絞り込み・書式・日時・復元・位置順・ふりがな除外（#1〜#9） | 実装済み | [イシュー](../test-plans/issues-test-plan.md) |
| テーマ・アイコン | 実装済み | [テーマ](../test-plans/theme-settings-test-plan.md) |

元の計画にある新旧設計書の意味的差分表示は対象外。PowerShell版は別リポジトリで管理する。画面はHTML/CSS/JavaScript、検索コアと検証入口はRust/Cargoで構成し、WASMは使用しない。

## 試験の実装と振り分け

- バックエンド試験はコアの単体/API/性質、実CLIの6群が実装済み。接続を必要としないTauriコマンド内の処理/状態もこの区分で検証する。
- フロントエンド試験は実HTML/CSS/JavaScript＋Tauri境界モックのUケースが実装済み。[G01/G02の構造・配色拡張](../test-plans/playwright-ui-test-plan.md#設計中のフロントエンド試験の拡張g01g02)は設計のみで未実装。
- 個別のバックエンド試験・フロントエンド試験でカバーできないもののみ、[アプリケーション結合試験](../test-plans/playwright-exe-test-plan.md)に回す。有効なE01/E02/E04/E06〜E11の9ケース群、実EXEランナーとtest/ciへの組込みは設計のみで未実装。廃止E03/E05/E12は再利用しない。
- 現行の `cargo xtask test` は実装済みの個別試験と生成器検証、`cargo xtask ci` はその成功後のWindows配布ビルドを実行する。現行入口が実EXEを試験しているとは扱わない。
- ネイティブダイアログ・実クリップボード・既定アプリ・排他ロック・OS表示は設計で保証対象外とし、人の確認を通常の完了条件にしない。性能の合格基準は未決定。

次の実装は、有効な結合ケースとCargoランナー、フロントエンドのG01/G02拡張を対象とする。実装後の検証依頼ではE01の実接続を確認し、全必須工程へ進む。
