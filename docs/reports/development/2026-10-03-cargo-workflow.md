# 画面のJavaScript移行とCargoへの開発手順集約 開発レポート

案件ID: `2026-10-03-cargo-workflow` / 移行・更新日: 2026-10-04
移行基準: `d8db3a88eebfb3d92ede697543b469e1534d6597`。過去資料から整理した記録で、移行時の再実装・再検証は行っていない。個別の承認日時・実装コミットは元資料に記録がない限り補わない。

## 目的・範囲

WASM画面をHTML/CSS/JavaScriptへ移し、試験・データ生成・ビルドをCargoから実行できるようにする。検索コア・Tauri・既存画面の契約を維持する。

## 現在地

| 工程 | 状態 | 根拠・残作業 |
| --- | --- | --- |
| 要件 | 当時の記録あり | 契約維持と共通入口 |
| 設計 | 当時の判断あり | Cargo workspaceと資材共有 |
| 実装 | 実装済み | 現行入口 `xtask/src/main.rs` |
| 検証 | 公開版の全体記録あり | 個別条件対応は記録なし |
| 公開 | 公開済み | 末尾のRelease |

## 要件・設計判断

[移行元の履歴](https://github.com/kz-oshiro/docs-search/blob/d8db3a88eebfb3d92ede697543b469e1534d6597/docs/reports/implementation-history.md)から移行した。[現行仕様](../../specifications/rust-requirements.md)と[開発手順](../../development.md)が現在の契約。

- WASMを撤去し、画面をES modulesへ分割する。検索・抽出はネイティブRustで継続する。
- Cargo workspace・lockfileを統一し、`cargo xtask` を共通入口にする。Python/PowerShellの試験・生成ランナーはRust integration testsと `test-support` へ移す。
- 試験と配布の資材stagingを共有し、内容が変わった資材だけを更新する。ビルドキャッシュを保持する。
- バックエンド試験・フロントエンド試験・検証基盤は実装済み。実EXEの接続と追加画面ケースは[未実装案件](2026-10-04-application-integration.md)で管理する。

## 実装

`frontend/`、`xtask/`、`test-support/`、Cargo workspaceへ集約。現行の必須工程、環境準備、コマンドは開発手順と[試験在庫](../../../tests/README.md)へ置く。

## 受入条件・検証

| 条件ID | 条件・仕様参照 | 実装参照 | 試験設計・ケース | 結果への参照／未確認理由 |
| --- | --- | --- | --- | --- |
| CW-01 | 既存検索・画面契約の維持 | frontendとcore/Tauri境界 | [UI試験](../../test-plans/playwright-ui-test-plan.md)・既存バックエンド | 公開版全体記録 |
| CW-02 | 共通入口と生成器・資材の再現性 | xtask・test-support | [試験在庫](../../../tests/README.md) | 公開版全体記録。実EXE結合の成功には数えない |

公開時の原本は[v3.0.1検証記録の保存版](https://github.com/kz-oshiro/docs-search/blob/v3.0.2/docs/releases/validation-v3.0.1.md)。移行では再実行していない。

## 次の担当への引き継ぎ

- 最初に読む資料: 本文の仕様・試験設計、[開発手順](../../development.md)。
- 着手条件: 現在のソースと依頼範囲を確認し、移行元の古いコマンドを現在の指示へ読み替えない。
- 次の作業・完了条件: 本件は完了。新しい工程を追加する場合は現行入口と失敗時の扱いを確認し、対象案件の試験設計・レポートを更新する。

## リリース

対象リリース: [v3.0.1](https://github.com/kz-oshiro/docs-search/releases/tag/v3.0.1)。
