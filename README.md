# docs-search

Rust の検索コアと Tauri を使う Windows 向け文書検索ツールです。画面は HTML/CSS/JavaScript の ES modules で動きます。検索と索引はネイティブ Rust で処理し、WASM は使用しません。

公開済みの [v3.0.0](https://github.com/kz-oshiro/docs-search/releases/tag/v3.0.0) は移行前の版です。今回の Cargo 集約・WASM 撤去は実装と静的確認までで、テスト・ビルド・公開は未実施です。[実装報告](docs/reports/implementation-report-cargo-native-ui.md)と[検証方針](docs/test-plans/cargo-native-ui-test-plan.md)を参照してください。過去の[検証記録](docs/releases/validation-v3.0.0.md)は今回の変更の合格証拠にはなりません。

## 開発と検証

リポジトリのルートで実行します。

```text
cargo xtask setup       # 初回の Playwright / Chromium 準備
cargo xtask test        # Rust / 実 CLI / 必須ヘッドレス UI
cargo xtask ui          # 実 HTML/CSS/JS の UI 試験
cargo xtask build       # Windows Release EXE
cargo xtask ci          # 全必須テスト成功後に Windows ビルド
cargo xtask fixtures    # 保存用 acceptance データ
cargo xtask icons      # 承認済み SVG から ICO / PNG を生成
```

Rust は `rust-toolchain.toml` の 1.91.1 に固定します。Node.js 20 以上と npm は開発時の Playwright 準備・試験だけに必要です。配布 EXE の利用者は Node/npm、Python、PowerShell、WASM ツールを用意する必要はありません。Windows ビルドには MSVC Build Tools、アプリ利用には WebView2 が必要です。

[開発手順](docs/development.md)、[テストとデータ](tests/README.md)、[Playwright 方針](docs/test-plans/playwright-ui-test-plan.md)、[構造ガイド](docs/rust-repository-structure.html)に詳しい手順があります。

## 構成

```text
docs-search/
├─ Cargo.toml / Cargo.lock / rust-toolchain.toml
├─ core/             Rust 検索コア・CLI・単体/API/CLI テスト
├─ frontend/         HTML・CSS・JS（app / view / tauri / theme）
├─ src-tauri/        Windows アプリ・OS/IPC 境界・アイコン
├─ xtask/            Cargo から呼ぶ検証・ビルド・生成の入口
├─ test-support/     検索エンジンに依存しない開発用生成器
├─ tests/fixtures/   共通の要求・期待結果 JSON
├─ tests/ui/         ヘッドレス Playwright と境界モック
├─ docs/             仕様・方針・報告・公開記録
├─ outputs/          実行ごとのログ・JSON/Markdown・データ・EXE
└─ Generate-Test-Data.cmd  cargo xtask fixtures の薄い入口
```

workspace の通常対象は `core` と `test-support`。デスクトップは明示してビルドします。`target/debug` と `target/release`、`target/frontend-dist` を保持し、毎回の検証結果は再利用しません。既存の旧配置のキャッシュと保存データも残しています。

## 機能と仕様

検索対象は Office 文書とテキスト系の 93 拡張子です。複数フォルダー・除外、独立して初期オフの索引/あいまい検索、条件検索、Office の注記/数式、順位、一括検索、結果の絞り込み/出力、対応テキストの一致箇所編集、テーマ・フォルダー復元に対応します。

- [Rust 要求仕様](docs/specifications/rust-requirements-v3.0.0.md)
- [全体仕様](docs/specifications/requirements.md)・[GUI](docs/specifications/gui.md)・[バックエンド](docs/specifications/backend.md)・[境界契約](docs/specifications/boundary.md)
- [機能実装計画](docs/plans/implementation-plan.md)・[実装状況](docs/reports/implementation-status.md)

## エージェントによる作業

実装依頼では設計・実装・静的確認・確認項目/手順/期待結果の報告まで行います。テスト、ビルド、起動、GUI、リモート反映は別の依頼範囲です。次の検証担当は最新の報告とテスト方針を読んでから、依頼済みの自動検証を完了します。テスト全体を利用者へ移しません。

全自動テストにはヘッドレス Playwright による実フロントエンド操作を含めます。スクリーンショット・画像比較・動画・トレースは使いません。実アプリ GUI と OS/見た目の確認は別に扱い、GUI の実施前だけ確認を取ります。依頼に GUI が明示されていれば再確認は不要です。

## PowerShell 版と保存用データ

PowerShell 検索版は [docs-search-ps](https://github.com/kz-oshiro/docs-search-ps) に分離済みです。現在の Rust 版の使用・メンテ・CI に含めません。ローカル配置は削除済みで、既存の成果物記録は `outputs/ps-archive-*/` に保存しています。

[Generate-Test-Data.cmd](Generate-Test-Data.cmd) をダブルクリックすると、Cargo の生成器が outputs/test-data に新しい保存先を作り、そのフォルダーを開きます。既存データを上書きしません。Cargo の fixtures コマンドだけでは開かず、--open を付けた場合に開きます。`cargo xtask fixtures --profile load` で負荷用データも生成できます。
