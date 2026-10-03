# docs-search v3.0.0

Windows x64向けのRust/Tauri版です。PowerShell版は独立リポジトリ [docs-search-ps](https://github.com/kz-oshiro/docs-search-ps)へ移行しました。

## 変更点

- Officeの検索範囲にExcel数式・コメント、Wordヘッダー・フッター・コメント、PowerPointノートを追加しました。追加範囲は設定で選べます。
- 検索コアでファイルの順位と根拠を計算し、ファイル内の一致箇所は文書位置順に表示します。
- 最大256語の一括検索、識別子の境界一致、語別集計、語×ファイルの行列表示とCSV/TSV保存を追加しました。
- ファイル単位の折りたたみ、本文だけの結果フィルター、更新日時、前回フォルダーの復元に対応しました。
- 対応テキスト形式は検索終了後に一致範囲を編集・保存できます。元ファイルの変更と文字コードを確認し、保存後は再検索が必要です。Officeファイルの直接編集は対象外です。
- Excelの可視文字抽出と周辺表を改善し、保存済みの罫線・塗りつぶし・文字書式を近似表示します。
- テーマ設定とアイコンを追加し、実WASMを操作するヘッドレスPlaywright試験を自動テストへ組み込みました。
- 生成試験で見つかったあいまい検索の候補漏れを修正しました。一括入力のプレビュー要求は120msで集約し、古い応答による上書きを防ぎます。

## 確認結果

- 共通バックエンド27ケース、Excel周辺5項目、条件検索31項目、Office・一括検索CLI、固定20クエリの順位ペア、Excelイシュー4項目、索引9項目が成功しました。
- Rustは64テスト成功。うち17性質を各1,024ケース、固定seed `3000002`で実行しました。失敗・無視は0です。
- WASM型検査、ヘッドレスPlaywright 51件、3パッケージのrustfmt、Windows Releaseビルドが成功しました。Playwrightのスキップ・失敗・flakyは0です。
- 実アプリの起動、対話的GUI、OS連携と見た目の確認、性能測定・実Office互換性の追加評価は未実施です。ヘッドレス試験ではTauri境界を置き換えています。

詳細は [検証記録](https://github.com/kz-oshiro/docs-search/blob/v3.0.0/docs/validation-v3.0.0.md)と [Rust要求仕様](https://github.com/kz-oshiro/docs-search/blob/v3.0.0/docs/rust-requirements-v3.0.0.md)を参照してください。

## 配布物

`docs-search-desktop.exe`（13,120,512 bytes）

SHA-256: `3BD1F22CCC818EF9EF1A532E141E9B6B4D2B655B0C3E7DD8854C7CB859714EEE`

動作にはWindows 10/11とWebView2が必要です。
