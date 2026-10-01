# docs-search

文書をフォルダー単位で横断検索するツールです。Windows 向けの [Tauri 実装](./tauri/README.md) は Rust の検索コアと Rust→WebAssembly の画面で構成します。旧実装は [PowerShell 実装](./powershell/README.md) に保存しています。

Tauri 実装の Windows 実行ファイルはローカルでテスト・ビルドし、[GitHub Releases](https://github.com/kz-oshiro/docs-search/releases)から取得できます。手順は [Tauri 実装の README](./tauri/README.md#ローカル-ci-と公開)を参照してください。リリースの版番号は Git タグと Cargo/Tauri のメタデータで管理します。

## 変更後の動作確認

機能修正では、変更内容の設計・実装に加え、確認項目・手順・期待結果を含むテスト方針の策定まで行います。

テスト実施を依頼されたエージェントは、直近の実装報告や文書にある最新のテスト方針を読み込み、確認項目・手順・期待結果を把握してから進めます。

AI エージェントへの依頼が実装までの場合は、実装と差分・文書の静的確認で止め、テスト、ビルド、アプリの起動、GUI テスト、リモート反映は実施しません。報告には未実施の項目と次のモデルで使う確認手順を記します。テスト全体の実施をユーザーに依頼する運用ではありません。

テスト、ビルド、リモート反映まで依頼された場合は、依頼された範囲を続けて完了します。GUI テストが必要な場合だけ、実施前にユーザーへ確認します。依頼に GUI テストが明示されていれば再確認はしません。PowerShell のテスト手順は下記、Tauri のテスト・ビルド・GUI 確認手順は [Tauri 実装の README](./tauri/README.md) を参照してください。

## 追加機能の計画と進捗

- [追加機能の実装計画](./docs/implementation-plan.md): Workで作成したP0〜P6の計画と受け入れ条件
- [実装状況](./docs/implementation-status.md): 段階ごとの実装・検証・公開状況と根拠

## 再構築版の要求仕様

- [全体仕様](./docs/requirements.md): 対象範囲、責務、受け入れ条件
- [GUI 仕様](./docs/gui.md): 入力、結果表示、操作、状態遷移
- [バックエンド仕様](./docs/backend.md): 対象形式、検索規則、失敗時の扱い
- [境界契約](./docs/boundary.md): GUI と検索処理の責務、要求とイベントの意味

これらは技術に依存しない受け入れ条件です。Tauri 実装とテスト方法は [Tauri 実装の README](./tauri/README.md) を参照してください。

## PowerShell 実装の実行

`powershell/Start-docs-search.vbs` をダブルクリックするか、[PowerShell 実装の README](./powershell/README.md) に従って起動してください。対応形式は `.xlsx` / `.xlsm` です。

## 共通テストデータ

[tests](./tests/README.md) に、技術スタックに依存しない検索用文書ジェネレータとバックエンド検証ケースを置きます。両実装は同じ入力文書を使います。`run-powershell-tests.ps1` は生成データを使って PowerShell 実装の Excel 検索を検証します。Windows PowerShell 5.1 と Python 3 で `docs-search` ディレクトリから実行します。

テストデータを手元に残して確認するときは [Generate-Test-Data.cmd](./Generate-Test-Data.cmd) をダブルクリックします。`outputs/test-data/` に生成した新しいフォルダーが開きます。負荷確認用データの出力方法は [共通テストデータの説明](./tests/README.md) を参照してください。

```powershell
powershell.exe -NoProfile -ExecutionPolicy RemoteSigned -File .\tests\run-powershell-tests.ps1
```

テストは文書を一時生成し、終了時に削除します。生成物は Git 管理対象外です。`backend-cases.json` は各操作と検索入力、期待する結果・Issue・集計の原本で、Tauri 実装のテストアダプターも利用します。
