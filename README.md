# docs-search

Rust コアと Rust→WebAssembly の画面を持つ、Windows 向けの Tauri 文書検索ツールです。起動・開発は [Tauri 実装の README](./tauri/README.md) を参照してください。PowerShell 版は独立リポジトリ [docs-search-ps](https://github.com/kz-oshiro/docs-search-ps) で管理します。

現在のソース版は **v3.0.0** です。[Rust要求仕様](./docs/rust-requirements-v3.0.0.md)にP0〜P6・既存イシュー・設定をまとめ、要求IDと自動テストの対応を記載しています。[検証記録](./docs/validation-v3.0.0.md)に今回の結果と対象外の範囲を記録します。ソースの版番号と配布済み実行ファイルの版は別です。

Rust/CLIとヘッドレスPlaywrightの自動確認（実アプリ起動・Windows配布ビルドなし）は次を実行します。初回は [UI試験の準備](./docs/playwright-ui-test-plan.md#実装と準備)が必要です。

```powershell
powershell.exe -NoProfile -ExecutionPolicy RemoteSigned -File .\tauri\Run-Automated-Tests.ps1
```

Tauri 実装の Windows 実行ファイルはローカルでテスト・ビルドし、[GitHub Releases](https://github.com/kz-oshiro/docs-search/releases)から取得できます。手順は [Tauri 実装の README](./tauri/README.md#ローカル-ci-と公開)を参照してください。リリースの版番号は Git タグと Cargo/Tauri のメタデータで管理します。

## 変更後の動作確認

機能修正では、変更内容の設計・実装に加え、確認項目・手順・期待結果を含むテスト方針の策定まで行います。

テスト実施を依頼されたエージェントは、直近の実装報告や文書にある最新のテスト方針を読み込み、確認項目・手順・期待結果を把握してから進めます。

[Playwright UI試験方針](./docs/playwright-ui-test-plan.md)に、自動判定とユーザー確認の境界を定めます。自動テストにはヘッドレス画面操作と必要なWASM生成を含めます。スクリーンショット・動画・トレースは使いません。実アプリGUIと見た目・OS連携は別に扱い、ユーザー確認は限定した5項目です。導入は [実装報告](./docs/implementation-report-playwright.md)を参照してください。2026-10-03にPlaywright 51件とWindows Releaseビルドが成功しました。詳細は [v3.0.0検証記録](./docs/validation-v3.0.0.md)を参照してください。

AI エージェントへの依頼が実装までの場合は、実装と差分・文書の静的確認で止め、テスト、ビルド、アプリの起動、GUI テスト、リモート反映は実施しません。報告には未実施の項目と次のモデルで使う確認手順を記します。テスト全体の実施をユーザーに依頼する運用ではありません。

テスト、ビルド、リモート反映まで依頼された場合は、依頼された範囲を続けて完了します。GUI テストが必要な場合だけ、実施前にユーザーへ確認します。依頼に GUI テストが明示されていれば再確認はしません。Tauri のテスト・ビルド・GUI 確認手順は [Tauri 実装の README](./tauri/README.md) を参照してください。

## 追加機能の計画と進捗

- [追加機能の実装計画](./docs/implementation-plan.md): Workで作成したP0〜P6の計画と受け入れ条件
- [実装状況](./docs/implementation-status.md): 段階ごとの実装・検証・公開状況と根拠
- [P4a〜P6の実装報告](./docs/implementation-report-p4-p6.md): Office追加範囲、コア順位、一括検索の変更と未実施の確認
- [P4〜P6の確認方針](./docs/office-ranking-batch-test-plan.md): 次の担当向けの項目・手順・期待結果

## 再構築版の要求仕様

- [全体仕様](./docs/requirements.md): 対象範囲、責務、受け入れ条件
- [GUI 仕様](./docs/gui.md): 入力、結果表示、操作、状態遷移
- [バックエンド仕様](./docs/backend.md): 対象形式、検索規則、失敗時の扱い
- [境界契約](./docs/boundary.md): GUI と検索処理の責務、要求とイベントの意味

これらは技術に依存しない受け入れ条件です。Tauri 実装とテスト方法は [Tauri 実装の README](./tauri/README.md) を参照してください。

## PowerShell 版

PowerShell 版は [docs-search-ps](https://github.com/kz-oshiro/docs-search-ps) へ分離しました。対応形式は `.xlsx` / `.xlsm` です。ローカル移行・確認・公開の段階は [移行計画](./docs/powershell-repository-migration-plan.md)を参照してください。

## Rust 版のテストデータ

[tests](./tests/README.md) に、Rust 版のテスト用文書ジェネレータとバックエンド検証ケースを置きます。Rust 側の受け入れランナーは [tauri/tests](./tauri/README.md#テスト) にあります。PowerShell 版へ移したデータ用ソースとは独立して管理します。

テストデータを手元に残して確認するときは [Generate-Test-Data.cmd](./Generate-Test-Data.cmd) をダブルクリックします。`outputs/test-data/` に生成した新しいフォルダーが開きます。生成内容は [Rust 版テストデータの説明](./tests/README.md) を参照してください。

```powershell
powershell.exe -NoProfile -ExecutionPolicy RemoteSigned -File .\tauri\Run-Automated-Tests.ps1
```

Rust 版の全自動試験は、共通バックエンドケース、Rust/WASM の確認、ヘッドレス Playwright を実行します。生成物は Git 管理対象外です。`backend-cases.json` は入力と期待結果の原本です。

## 既存イシューの一括対応

2026-10-03に #1〜#9 の折りたたみ・本文フィルター・一致箇所編集・更新日時・フォルダー復元・位置順・Excel書式/可視文字の対応を追加しました。[実装報告](docs/implementation-report-issues.md)と[確認方針](docs/issues-test-plan.md)は実装時点の記録です。現在の自動検証・Windows配布ビルドの結果は [v3.0.0検証記録](docs/validation-v3.0.0.md)を参照してください。実アプリGUI・OS連携・見た目は未確認です。
