# docs-search v3.0.1

Windows x64 向け Rust/Tauri 文書検索アプリです。開発・検証・ビルドを Cargo に集約し、画面の WASM を撤去しました。

## 変更

- 画面を HTML/CSS/JavaScript の ES modules に移行しました。検索・抽出・索引は引き続きネイティブ Rust が処理します。
- Cargo workspace と lockfile を統合し、`cargo xtask setup|test|ui|build|ci|fixtures|icons` を共通入口にしました。
- Python/PowerShell のテスト・生成ランナーを Rust の integration test と開発専用 `test-support` に移しました。Node/npm は開発時の Playwright だけに使います。
- frontend 資材は変更分だけ staging し、debug/release キャッシュを保持します。各実行のログ・JSON/Markdown・成果物の SHA256 を独立して保存します。
- 起動資材の失敗表示、起動準備中の検索抑止、古い通知の除外、CRLF の条件入力を自動確認に追加しました。

対応93拡張子、検索・順位・一括検索・周辺表示・編集・出力・テーマとフォルダー復元の要求契約を継続します。索引とあいまい検索は独立した初期オフの選択肢です。PowerShell 検索版は別リポジトリで管理します。

## 検証

- Rust コア/公開 API 64件成功。17性質を各1,024ケース、固定 seed `3000002` で実行しました。
- 実 CLI 全6群、生成器3件、ヘッドレス Playwright 54件が成功しました。失敗・無視・UI skipped/flaky は0です。
- 旧/新生成器の6種類の文書契約が一致し、新生成器453ファイルの再生成 SHA256 が一致しました。ZIP の圧縮バイト差は記録しました。
- Windows Release ビルドが成功しました。最終テスト後は文書だけを変更し、テスト済み EXE を配布します。

実アプリ GUI・OS 連携・見た目と、同一条件の旧/新 cold・warm 性能比較は未実施です。今回の工程時間から速度改善率は断定しません。詳細は [検証記録](https://github.com/kz-oshiro/docs-search/blob/v3.0.1/docs/releases/validation-v3.0.1.md)と [開発手順](https://github.com/kz-oshiro/docs-search/blob/v3.0.1/docs/development.md)を参照してください。

## 配布ファイル

`docs-search-desktop.exe` — Windows 10/11 x64、WebView2 が必要です。利用者に Rust、Node/npm、Python、PowerShell、WASM ツールの準備は不要です。

- サイズ: 13,027,840 bytes
- FileVersion / ProductVersion: `3.0.1`
- SHA256: `39ea2a6f2846193fbc3af5f50fff9bb3e959280a5a03df6b4e81cfa003974328`
