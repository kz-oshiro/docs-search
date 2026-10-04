# 検索基盤・出力・周辺表示・条件検索 開発レポート

案件ID: `2026-10-01-search-foundation-export-conditions` / 移行・更新日: 2026-10-04
移行基準: `d8db3a88eebfb3d92ede697543b469e1534d6597`。過去資料から整理した記録で、移行時の再実装・再検証は行っていない。個別の承認日時・実装コミットは元資料に記録がない限り補わない。

## 目的・範囲

元計画のP0〜P3をまとめる。検索単位・Excel行・索引基盤、一括コピーとCSV/TSV/JSON出力、Excel周辺表と変更検出、AND/OR/除外と判定範囲を追加する。新旧設計書の意味的差分表示は対象外。

## 現在地

| 工程 | 状態 | 根拠・残作業 |
| --- | --- | --- |
| 要件 | 当時の計画あり | 要求A-12〜A-15。個別の承認日時は記録なし |
| 設計 | 当時の判断あり | 下記の依存関係と採用方針 |
| 実装 | 実装済み（旧履歴） | P0〜P3の実装を移行元に記録 |
| 検証 | 公開版の全体記録あり | 個別条件と実行ケースの対応は記録なし |
| 公開 | 公開済み（旧履歴） | 版別の対応は末尾 |

## 要件・設計判断

[元計画の保存版](https://github.com/kz-oshiro/docs-search/blob/d8db3a88eebfb3d92ede697543b469e1534d6597/docs/plans/implementation-plan.md)と[移行元の履歴](https://github.com/kz-oshiro/docs-search/blob/d8db3a88eebfb3d92ede697543b469e1534d6597/docs/reports/implementation-history.md)から移行した。元計画は2026-09-30作成、基準コミット `aab5d1aabe540b50cee62ba6a91e32129fee402f`。作成時点は実装前の計画で、その承認・検証の完了を意味しない。現行契約は[要求仕様](../../specifications/requirements.md)、[抽出・索引](../../specifications/backend.md)、[境界](../../specifications/boundary.md)、[GUI](../../specifications/gui.md)を正本とする。

### 元計画から引き継ぐ判断

- 辞書登録・学習データ・手動タグを要求せず、端末内で検索する。索引とあいまい検索は独立した初期オフの選択肢。通常欄は入力全体を1語句として扱う。
- P1を先に独立実装し、P0で共有構造を揃えてからP2→P3へ進む計画だった。後続はP4a→P4b→P5→P6で、P6はP1・P3・P5に依存する。これは当時の計画順で、現在の残作業ではない。
- 安定した検索単位ID・セル座標・行グループ・部品種別を共用し、Excel行を文字列連結して一致を捏造しない。抽出範囲と抽出仕様版も索引の鮮度条件に含める。
- 出力は描画済みDOMではなく保持した全結果から作る。周辺表は疎なセル・結合範囲・非表示行列を扱い、取得上限と元文書の変更を確認する。
- 条件候補と最終評価を分け、AND/NOTは指定したセル・行・ファイル全体で判定する。除外語は通常の部分一致とし、取得失敗したファイル範囲を成立させない。
- 計画案の `group.rs` は作らず、行集約を `query.rs` に実装した。当初の索引経路では漏れを避けるため保存済み検索単位を全件読み込んだ。
- 原計画の全受入条件・配置案は保存版に残す。仕様と重なる全文をこのレポートへ複製せず、変更時の条件・期待結果は下記試験設計を参照する。

## 実装

| 段階 | 主な実装 | 確認方針 |
| --- | --- | --- |
| P0 | `core/src/extract.rs`、`index.rs` | [検索基盤](../../test-plans/search-foundation-test-plan.md) |
| P1 | `core/src/report.rs`、`frontend/view.js`、`src-tauri/src/main.rs` | [結果出力](../../test-plans/result-export-test-plan.md) |
| P2 | `core/src/context.rs`、`extract.rs`、`frontend/view.js` | [Excel周辺](../../test-plans/excel-context-test-plan.md) |
| P3 | `core/src/query.rs`、`lib.rs`、`frontend/app.js` | [条件検索](../../test-plans/condition-search-test-plan.md) |

専用データ生成は `test-support/src/specialized.rs`。あいまい検索と結果ラベル・エラーの既存契約は[あいまい検索試験](../../test-plans/fuzzy-search-test-plan.md)と[結果・エラー試験](../../test-plans/result-match-error-test-plan.md)を参照する。

## 受入条件・検証

| 条件ID | 条件・仕様参照 | 実装参照 | 試験設計・ケース | 結果への参照／未確認理由 |
| --- | --- | --- | --- | --- |
| A-13 | 検索単位・行・索引の同値性 | P0 | 検索基盤の固定ケース | 公開版全体記録。個別対応は記録なし |
| A-12 | 全件・絞り込み後の出力範囲と形式 | P1 | 結果出力のコピー・保存・JSON | 同上 |
| A-14 | 周辺表・結合・非表示・変更検出 | P2 | Excel周辺の直接・索引経路 | 同上 |
| A-15 | 条件範囲・NOT・根拠と件数 | P3 | 条件検索の範囲と組合せ | 同上 |

当時の確認結果は末尾のReleaseを参照する。移行元にない実行IDや個別成功は推定しない。

## 次の担当への引き継ぎ

- 最初に読む資料: 本文の仕様・試験設計、[開発手順](../../development.md)。
- 着手条件: 現在のソースと依頼範囲を確認し、移行元の古いコマンドを現在の指示へ読み替えない。
- 次の作業・完了条件: この案件の実装は終了。変更時は該当要求・試験設計を更新し、依頼範囲の実行結果を変更後のソースに対応付ける。性能の数値基準は未決定で、必要な負荷比較は新しい依頼範囲で設計する。

## リリース

対象リリース: P0・P1: [v2.0.6](https://github.com/kz-oshiro/docs-search/releases/tag/v2.0.6)。P2・P3: [v2.0.7](https://github.com/kz-oshiro/docs-search/releases/tag/v2.0.7)。
