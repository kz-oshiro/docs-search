# PowerShell版の分離 開発レポート

案件ID: `2026-10-03-powershell-separation` / 移行・更新日: 2026-10-04
移行基準: `d8db3a88eebfb3d92ede697543b469e1534d6597`。過去資料から整理した記録で、移行時の再実装・再検証は行っていない。個別の承認日時・実装コミットは元資料に記録がない限り補わない。

## 目的・範囲

PowerShell検索版を別リポジトリへ分離し、Rust/Tauri版の使用・保守・CIから外す。

## 現在地

| 工程 | 状態 | 根拠・残作業 |
| --- | --- | --- |
| 要件 | 旧記録あり | リポジトリ境界の分離 |
| 設計 | 旧計画あり | 下記保存版 |
| 実装 | 分離済み（旧履歴） | 別リポジトリ |
| 検証 | 個別対応は記録なし | 本移行で別リポジトリを再検証しない |
| 公開 | 別リポジトリで公開 | 末尾 |

## 要件・設計判断

[移行元の履歴](https://github.com/kz-oshiro/docs-search/blob/d8db3a88eebfb3d92ede697543b469e1534d6597/docs/reports/implementation-history.md)と[移行計画の保存版](https://github.com/kz-oshiro/docs-search/blob/059ba7540fbdc762feb5010245b1d6e31a71c716/docs/plans/powershell-repository-migration-plan.md)を参照する。PowerShell側のふりがな除外・終了時回収・ZIP/XML上限変更は独立した保守変更で、Rust側のソース・設定・共通ケース原本へ混ぜない。

## 実装

[docs-search-ps](https://github.com/kz-oshiro/docs-search-ps)へ移した。現行Rust/Tauri版の入口は[開発手順](../../development.md)を参照する。

## 受入条件・検証

| 条件ID | 条件・仕様参照 | 実装参照 | 試験設計・ケース | 結果への参照／未確認理由 |
| --- | --- | --- | --- | --- |
| PS-01 | RustとPowerShellの使用・CI・保守を分離 | 別リポジトリ | 旧移行計画の確認項目 | 旧履歴の分離記録。今回は文書移行のみ |

旧記録にない実行ID・個別の承認は記録なし。

## 次の担当への引き継ぎ

- 最初に読む資料: 本文の仕様・試験設計、[開発手順](../../development.md)。
- 着手条件: 現在のソースと依頼範囲を確認し、移行元の古いコマンドを現在の指示へ読み替えない。
- 次の作業・完了条件: 本件は完了。PowerShell版の保守を依頼された場合は別リポジトリで扱う。

## リリース

対象リリース: [docs-search-ps v1.0.0](https://github.com/kz-oshiro/docs-search-ps/releases/tag/v1.0.0)（別リポジトリ）。Rust版の公開記録と混同しない。
