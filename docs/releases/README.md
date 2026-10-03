# 公開・検証記録

配布ファイルは[最新のGitHub Release](https://github.com/kz-oshiro/docs-search/releases/latest)を参照する。現行の仕様・開発手順・試験設計は版番号を付けず同じ文書を更新し、変更履歴をGitで管理する。文書改訂だけで製品版を上げる必要はない。

リリースノート・検証記録は対象成果物を特定するため版番号を残す。検証記録には対象コミット、実行ID、環境、実施範囲、配布ファイルのSHA256を記載する。過去の記録を現行仕様へ書き換えない。依存ツール・API契約・データ形式などの技術上必要な版情報はそれぞれの設定や仕様に残す。

| 公開版 | 変更点 | 検証・公開の記録 |
| --- | --- | --- |
| v3.0.2 | [リリースノート](release-notes-v3.0.2.md) | [検証記録](validation-v3.0.2.md) |
| v3.0.1 | [リリースノート](release-notes-v3.0.1.md) | [検証記録](validation-v3.0.1.md) |
| v3.0.0 | [リリースノート](release-notes-v3.0.0.md) | [検証記録](validation-v3.0.0.md) |
| v2.0.7 | [リリースノート](release-notes-v2.0.7.md) | 同リリースノート内 |
| v2.0.6 | [リリースノート](release-notes-v2.0.6.md) | 同リリースノート内 |

未公開の自動試験は実行ごとの `outputs/runs/<ID>/report.md` / `report.json` と試験別ログに記録する。ローカルの未公開結果と公開済み成果物の検証結果を混同しない。過去の実装・移行経緯は[実装履歴](../reports/implementation-history.md)と各実装報告を参照する。
