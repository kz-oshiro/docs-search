# docs-search

Rust の検索コアと Tauri を使う Windows 向け文書検索ツールです。画面は HTML/CSS/JavaScript の ES modules で動きます。検索と索引はネイティブ Rust で処理し、WASM は使用しません。

配布ファイルは [最新のGitHub Release](https://github.com/kz-oshiro/docs-search/releases/latest)から取得できます。リリースごとの変更点・対象ソース・検証結果は[GitHub Releases](https://github.com/kz-oshiro/docs-search/releases)を参照してください。

## 開発と検証

リポジトリのルートで実行します。

```text
cargo xtask setup       # 初回の Playwright / Chromium 準備
cargo xtask test        # バックエンド試験 / 必須フロントエンド試験 / 生成器
cargo xtask ui          # フロントエンド試験（実 HTML/CSS/JS）
cargo xtask build       # Windows Release EXE
cargo xtask ci          # 全必須テスト成功後に Windows ビルド
cargo xtask fixtures    # 保存用 acceptance データ
cargo xtask icons      # 承認済み SVG から ICO / PNG を生成
```

Rust は `rust-toolchain.toml` の 1.91.1 に固定します。Node.js 20 以上と npm は開発時の Playwright 準備・試験だけに必要です。配布 EXE の利用者は Node/npm、Python、PowerShell、WASM ツールを用意する必要はありません。Windows ビルドには MSVC Build Tools、アプリ利用には WebView2 が必要です。

[開発手順](docs/development.md)、[テストとデータ](tests/README.md)、[フロントエンド試験設計](docs/test-plans/playwright-ui-test-plan.md)、[構造ガイド](docs/rust-repository-structure.html)に詳しい手順があります。

上記コマンドは現行の実行入口です。実EXE用ランナーなどの追加設計は[実装状況](docs/reports/implementation-status.md#試験の実装と振り分け)、試験の分類・保証範囲は[アプリケーション結合試験設計](docs/test-plans/playwright-exe-test-plan.md)を参照してください。

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
├─ docs/             仕様・手順・試験設計・開発報告
├─ outputs/          Git管理外のデータ・成果物（試験原本は runs/）
└─ Generate-Test-Data.cmd  cargo xtask fixtures の薄い入口
```

workspace の通常対象は `core` と `test-support`。デスクトップは明示してビルドします。`target/debug` と `target/release`、`target/frontend-dist` を保持し、毎回の検証結果は再利用しません。既存の旧配置のキャッシュと保存データも残しています。

## 機能と仕様

検索対象は Office 文書とテキスト系の 93 拡張子です。複数フォルダー・除外、独立して初期オフの索引/あいまい検索、条件検索、Office の注記/数式、順位、一括検索、結果の絞り込み/出力、対応テキストの一致箇所編集、テーマ・フォルダー復元に対応します。

- [Rust 要求仕様](docs/specifications/rust-requirements.md)
- [全体仕様](docs/specifications/requirements.md)・[GUI](docs/specifications/gui.md)・[バックエンド](docs/specifications/backend.md)・[境界契約](docs/specifications/boundary.md)
- [機能実装計画](docs/plans/implementation-plan.md)・[実装状況](docs/reports/implementation-status.md)

## エージェントによる作業

[AGENTS.md](AGENTS.md)からタスク別の資料を参照してください。共通の作業ルールは[開発手順](docs/development.md#エージェントの作業ルール)、文書の役割・更新方法・実装報告のひな型は[文書作成ガイド](docs/documentation.md)にまとめています。

## PowerShell 版と保存用データ

PowerShell 検索版は [docs-search-ps](https://github.com/kz-oshiro/docs-search-ps) に分離済みです。現在の Rust 版の使用・メンテ・CI に含めません。ローカル配置は削除済みで、既存の成果物記録は `outputs/ps-archive-*/` に保存しています。

[Generate-Test-Data.cmd](Generate-Test-Data.cmd) をダブルクリックすると、Cargo の生成器が outputs/test-data に新しい保存先を作り、そのフォルダーを開きます。既存データを上書きしません。Cargo の fixtures コマンドだけでは開かず、--open を付けた場合に開きます。`cargo xtask fixtures --profile load` で負荷用データも生成できます。
