# 実装状況

現行の実装状況を示す。機能の基準は[Rust要求仕様](../specifications/rust-requirements.md)、詳細手順は[開発手順](../development.md)、公開版の対象コミット・検証結果・成果物は[GitHub Releases](https://github.com/kz-oshiro/docs-search/releases)を参照する。過去の経緯は[実装履歴](implementation-history.md)へ分けて保存する。

実装済みはコードと文書が存在すること、確認済みは実際に行った検証、公開済みは配布を指す。試験設計の存在だけを検証完了とは扱わない。

## 開発レポート

案件の要件・設計・実装・検証と次の作業を1枚で確認する。担当者・モデルが変わるときは[レポートの運用](../documentation.md#開発レポートのひな型)に従い、対象コミットと現在の差分を確認する。

| 案件 | 状況・次の作業 |
| --- | --- |
| [文書管理・利用者向け案内](development/2026-10-04-documentation.md) | 本整理の実装・静的確認。記録生成の実行検証とリモート反映は未実施 |
| [検索処理の重複削減](development/2026-10-04-backend-optimization.md) | v3.0.4で公開。性能比較・数値基準は未実施・未決定 |

過去の案件は[実装履歴](implementation-history.md)から参照する。公開検証の入口は[v3.0.4検証MD](validation/v3.0.4.md)と[Release一覧](https://github.com/kz-oshiro/docs-search/releases)。

## 機能の実装

| 機能 | 状況 | 確認方針 |
| --- | --- | --- |
| 検索単位・Excel行・索引基盤（P0） | 実装済み | [検索基盤](../test-plans/search-foundation-test-plan.md) |
| あいまい検索 | 実装済み | [あいまい検索](../test-plans/fuzzy-search-test-plan.md) |
| 結果ラベル・エラー表示 | 実装済み | [結果ラベル・エラー](../test-plans/result-match-error-test-plan.md) |
| 一括コピー・CSV/TSV・JSON出力（P1） | 実装済み | [出力](../test-plans/result-export-test-plan.md) |
| Excel周辺セル・変更検出（P2） | 実装済み | [周辺](../test-plans/excel-context-test-plan.md) |
| AND/OR/除外・判定範囲（P3） | 実装済み | [条件](../test-plans/condition-search-test-plan.md) |
| Office注記・数式・順位・一括検索（P4〜P6） | 実装済み | [Office/順位/一括](../test-plans/office-ranking-batch-test-plan.md) |
| 折りたたみ・編集・絞り込み・書式・日時・復元・位置順・ふりがな除外（#1〜#9） | 実装済み | [イシュー](../test-plans/issues-test-plan.md) |
| テーマ・アイコン | 実装済み | [テーマ](../test-plans/theme-settings-test-plan.md) |
| バックエンドの境界判定・正規化共有・テキスト読込み・索引SQLの重複削減 | 実装済み | [バックエンド性能](../test-plans/backend-performance-test-plan.md) |

元の計画にある新旧設計書の意味的差分表示は対象外。PowerShell版は別リポジトリで管理する。画面はHTML/CSS/JavaScript、検索コアと検証入口はRust/Cargoで構成し、WASMは使用しない。

## 試験の実装と振り分け

- バックエンド試験はコアの単体/API/性質、実CLIの6群が実装済み。[性質テストの設計](../test-plans/property-testing-test-plan.md)と[試験コード・データの対応](../../tests/README.md)を参照する。
- フロントエンド試験は実HTML/CSS/JavaScript＋Tauri境界モックのUケースが実装済み。[G01/G02の構造・配色拡張](../test-plans/playwright-ui-test-plan.md#設計中のフロントエンド試験の拡張g01g02)は設計のみで未実装。
- [アプリケーション結合試験](../test-plans/playwright-exe-test-plan.md)の有効なE01/E02/E04/E06〜E11の9ケース群、実EXEランナーとtest/ciへの組込みは設計のみで未実装。
- 現行の `cargo xtask test` は実装済みの個別試験と生成器検証、`cargo xtask ci` はその成功後のWindows配布ビルドを実行する。現行入口が実EXEを試験しているとは扱わない。
- CI性能改善としてアイコンの属性事前計算/描画打切り、バックエンドとフロントエンドの同時実行、CPU負荷を計測したUI worker選択を実装した。コンパイル最適化設定は変更しない。[性能検証方針](../test-plans/ci-performance-test-plan.md)に新規8件のworker計算試験と検証基盤・並列実行の確認手順を記載した。
- 性能の合格基準は未決定。

試験の担当・結果報告の分類は[試験区分と振り分け](../test-plans/playwright-exe-test-plan.md#試験区分と振り分けの基本方針)、OS連携は[保証対象外の定義](../test-plans/playwright-exe-test-plan.md#5-os連携の保証対象外)を参照する。

次の実装は、有効な結合ケースとCargoランナー、フロントエンドのG01/G02拡張を対象とする。実装後の検証依頼ではE01の実接続を確認し、全必須工程へ進む。
