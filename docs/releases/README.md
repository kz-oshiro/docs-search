# 公開・検証記録

配布ファイルは[最新のGitHub Release](https://github.com/kz-oshiro/docs-search/releases/latest)を参照する。文書と版情報の扱いは[文書作成ガイド](../documentation.md#更新ルール)に従う。

変更点は[GitHub Releasesの一覧](https://github.com/kz-oshiro/docs-search/releases)で案内する。今後の検証・公開記録は、各Releaseに添付する自動生成の `validation.md` / `validation.json` で案内する。実装・検証状況と手順は[生成・公開手順](../development.md#記録の自動生成と公開)を参照する。

過去の手書き記録は以下に保存する。

| 公開版 | 変更点 | 検証・公開の記録 |
| --- | --- | --- |
| v3.0.2 | [リリースノート](release-notes-v3.0.2.md) | [検証記録](validation-v3.0.2.md) |
| v3.0.1 | [リリースノート](release-notes-v3.0.1.md) | [検証記録](validation-v3.0.1.md) |
| v3.0.0 | [リリースノート](release-notes-v3.0.0.md) | [検証記録](validation-v3.0.0.md) |
| v2.0.7 | [リリースノート](release-notes-v2.0.7.md) | 同リリースノート内 |
| v2.0.6 | [リリースノート](release-notes-v2.0.6.md) | 同リリースノート内 |

未公開の自動試験は実行ごとの `outputs/runs/<ID>/report.md` / `report.json` と試験別ログに記録する。ローカルの未公開結果と公開済み成果物の検証結果を混同しない。過去の実装・移行経緯は[実装履歴](../reports/implementation-history.md)と各実装報告を参照する。
