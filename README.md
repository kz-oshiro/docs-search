# doc-search

文書をフォルダー単位で横断検索するツールです。Windows 向けの [v2.0](./v2.0/README.md) は Rust の検索コアと Rust→WebAssembly の画面で構成します。旧実装は [v1.0](./v1.0/README.md) に保存しています。

## 再構築版の要求仕様

- [全体仕様](./docs/requirements.md): 対象範囲、責務、受け入れ条件
- [GUI 仕様](./docs/gui.md): 入力、結果表示、操作、状態遷移
- [バックエンド仕様](./docs/backend.md): 対象形式、検索規則、失敗時の扱い
- [境界契約](./docs/boundary.md): GUI と検索処理の責務、要求とイベントの意味

これらは技術に依存しない受け入れ条件です。v2.0 の実装とテスト方法は [v2.0 の README](./v2.0/README.md) を参照してください。

## v1.0 の実行

`v1.0/Start-doc-search.vbs` をダブルクリックするか、[v1.0 の README](./v1.0/README.md) に従って起動してください。v1.0 の対応形式は `.xlsx` / `.xlsm` です。

## 共通テストデータ

[tests](./tests/README.md) に、技術スタックに依存しない検索用文書ジェネレータとバックエンド検証ケースを置きます。v1.0 と v2.0 は同じ入力文書を使います。`run-v1-tests.ps1` は生成データを使って v1.0 の Excel 検索を検証します。Windows PowerShell 5.1 と Python 3 で `doc-search` ディレクトリから実行します。

```powershell
powershell.exe -NoProfile -ExecutionPolicy RemoteSigned -File .\tests\run-v1-tests.ps1
```

テストは文書を一時生成し、終了時に削除します。生成物は Git 管理対象外です。`backend-cases.json` は各操作と検索入力、期待する結果・Issue・集計の原本で、v2.0 のテストアダプターも利用します。
