# CIとアイコン生成の重複処理削減 開発レポート

案件ID: `2026-10-04-ci-optimization` / 移行・更新日: 2026-10-04
移行基準: `d8db3a88eebfb3d92ede697543b469e1534d6597`。過去資料から整理した記録で、移行時の再実装・再検証は行っていない。個別の承認日時・実装コミットは元資料に記録がない限り補わない。

## 目的・範囲

アイコン生成と自動試験の処理を効率化し、独立工程を並列実行する。コンパイル最適化設定・外部依存を変更しない。

## 現在地

| 工程 | 状態 | 根拠・残作業 |
| --- | --- | --- |
| 要件 | 旧記録あり | 契約維持と工程短縮 |
| 設計 | 試験設計あり | CPU計測・並列工程・比較 |
| 実装 | 実装済み（旧履歴） | 属性事前計算・worker選択 |
| 検証 | 公開版の自動結果・比較あり | 単一Windows環境の観測 |
| 公開 | 公開済み | 末尾 |

## 要件・設計判断

[移行元の履歴](https://github.com/kz-oshiro/docs-search/blob/d8db3a88eebfb3d92ede697543b469e1534d6597/docs/reports/implementation-history.md)から移行した。アイコンの属性・形状計算を事前に共用し、可視図形が見つかった時点で評価を打ち切る。承認済み素材の出力は維持する。バックエンドとフロントエンドを同時に開始し、全必須試験成功後にだけビルドする。UI worker数は各実行直前のCPU負荷から選択する。[CI性能試験設計](../../test-plans/ci-performance-test-plan.md)と[現行手順](../../development.md)を参照する。

## 実装

`xtask/` の工程制御、`tests/ui/` のworker方針、`test-support/` のアイコン生成を更新した。worker計算のNode試験と素材・資材照合を維持する。

## 受入条件・検証

| 条件ID | 条件・仕様参照 | 実装参照 | 試験設計・ケース | 結果への参照／未確認理由 |
| --- | --- | --- | --- | --- |
| CI-01 | 出力と独立工程の失敗処理を維持 | アイコン生成・xtask | [CI性能試験](../../test-plans/ci-performance-test-plan.md) | 公開検証記録 |
| CI-02 | 同条件の時間を比較する | 工程・worker計測 | 同設計の比較手順 | 性能JSON。数値基準は未決定 |

[公開検証記録](https://github.com/kz-oshiro/docs-search/releases/download/v3.0.3/validation.md)と[性能比較原本](https://github.com/kz-oshiro/docs-search/releases/download/v3.0.3/performance.json)を参照する。件数・中央値・短縮率をこのレポートへ転記しない。

## 次の担当への引き継ぎ

- 最初に読む資料: 本文の仕様・試験設計、[開発手順](../../development.md)。
- 着手条件: 現在のソースと依頼範囲を確認し、移行元の古いコマンドを現在の指示へ読み替えない。
- 次の作業・完了条件: 実装・公開は完了。追加比較を依頼された場合、条件・対象ソースを揃え、単一環境の観測を全端末の保証へ拡張しない。

## リリース

対象リリース: [v3.0.3](https://github.com/kz-oshiro/docs-search/releases/tag/v3.0.3)。
