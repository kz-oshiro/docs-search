# docs-search の開発

Rust 検索コア、Tauri の OS/IPC 境界、HTML/CSS/JavaScript の画面で構成します。`frontend/app.js` が要求と検索状態、`view.js` が DOM・周辺・編集・出力、`tauri.js` が接続、`theme.js` がテーマを担当します。利用者向けの検索仕様は [Rust 要求仕様](specifications/rust-requirements.md)を参照してください。

## 準備

Rust/MSVC 1.91.1（rustup が toolchain ファイルに従う）、Windows C++ Build Tools、WebView2 を用意します。WASM target / wasm-bindgen-cli / Python / PowerShell の準備は不要です。UI 試験を行う開発環境には Node.js 20 以上と npm を用意し、初回だけルートで次を実行します。

```text
cargo xtask setup
```

固定 `package-lock.json` で `npm ci` を実行し、Playwright 1.63.0 の Chromium を準備します。通常の test / ui / ci は依存を自動インストールしません。不足は失敗/理由付き skipped として記録し、全体を成功にしません。

## テスト

```text
cargo xtask test
cargo xtask ui
cargo xtask ui --case "^U23:"
```

`test` は core の単体・公開 API・proptest、生成器の再現性/在庫/Office 検証、実 CLI の共通27・周辺5・条件31・Office/一括/順位20組・イシュー・索引、必須ヘッドレス Playwright を実行します。実 CLI は Cargo の `CARGO_BIN_EXE_docs-search-cli` を使い、ランナー内で別のビルドをしません。CLI の LOCALAPPDATA は子プロセスごとに独立しています。共通 load データは実行内で共有し、変更する文書と索引は別の一時領域に置きます。

フロントエンド試験の選択実行は絞った診断用です。全体合格の証拠には `cargo xtask test` / `ci` の全件実行を使います。Tauri/OS の応答をモックにして実 HTML/CSS/JS を操作し、DOM・状態・要求引数で判定します。画像/動画/トレースは無効です。[フロントエンド試験設計](test-plans/playwright-ui-test-plan.md)と[今回の移行検証](test-plans/cargo-native-ui-test-plan.md)を読んでから実施します。

試験区分はバックエンド試験（Rustの単体/API/性質/実CLI）、フロントエンド試験（Playwright＋Tauri境界モック）、アプリケーション結合試験（Playwright＋実Tauri）に統一します。バックエンド試験・フロントエンド試験でカバーできる確認は各区分で担当し、個別試験では確認できない接続上の不具合だけを結合試験に回します。個別試験の不足はその区分で補い、結合試験で代用しません。構造・6テーマの配色G01/G02はフロントエンド試験、生成器・資材・ハッシュは共通の検証基盤で担当します。

上記コマンドは現行ランナーの説明です。[アプリケーション結合試験設計](test-plans/playwright-exe-test-plan.md)では、Rustの入口を維持し、test/ciの両方にReleaseビルドと結合試験を追加します。追加ランナー、診断入口cargo xtask exe、フロントエンド試験のG01/G02拡張は未実装です。導入後は試験済みEXEを成果物として再利用します。結果を3区分で報告し、検証基盤の確認を併記します。Rust製OS操作補助は作らず、OS固有の操作・表示は保証対象外として文書化し、人の確認を通常の完了条件にしません。

## Windows ビルドと CI

```text
cargo xtask build
cargo xtask ci
```

`build` は frontend の8資材だけを `target/frontend-dist/` へ内容が変わった場合にコピーし、`cargo build --release --locked -p docs-search-desktop` を実行します。成功した現在の EXE だけを実行別 `outputs/runs/<実行ID>/artifacts/docs-search-desktop.exe` に保存し、SHA256 とサイズを記録します。Node/npm はこのビルドに不要です。実行中の EXE を再ビルドするときは閉じてください。

`ci` は独立したテスト群を失敗後も続け、必須テストがすべて成功した場合だけビルドします。全体の失敗を一つの終了コードで返します。GitHub Actions はまだ作成していません。push、タグ、Release は検証入口に含めず、ユーザーの公開依頼を受けて別途行います。

各実行は新規 `outputs/runs/<実行ID>/` に `report.json` / `report.md`、工程別 stdout/stderr、Git commit/dirty、Rust/Cargo/Node/npm、Playwright の前提と実測版、所要時間、順位 Top-5、UI JSON/テキスト失敗記録を残します。レポートは異なる実行の成功ログを混ぜません。

`target/debug` / `target/release` / staged frontend を削除しません。旧配置の target・outputs・UI cache も自動削除しません。キャッシュの保持はコンパイルの再利用であり、テストは毎回実行します。

## データとアイコン

```text
cargo xtask fixtures
cargo xtask fixtures --profile load --output outputs/test-data/new-load
cargo xtask fixtures --kind issues --output outputs/manual/new-issues
cargo xtask icons
```

生成先は新規または空のフォルダーに限ります。省略時は outputs/test-data の新しい実行別フォルダーです。`--kind common|context|conditions|office|issues` を指定できます。[在庫と役割](../tests/README.md)を参照してください。`Generate-Test-Data.cmd` は Cargo を呼ぶだけの入口で、ダブルクリックでは保存先を開き、従来の `-Profile` / `-NoOpen` も受け付けます。通常の cargo xtask fixtures は開かず、--open 指定時だけ開きます。アイコン生成は承認済み SVG の限定した図形のみを Rust でラスタライズし、通常の test / build では書き換えません。

## 静的確認と検証範囲

実装依頼で行うのは `cargo metadata --no-deps --locked`、`cargo fmt --all -- --check`、`node --check`、文書/差分の静的確認までです。型検査・テスト・ビルド・起動は依頼された範囲で実施します。コマンド、対象ソース、ログ、成果物は実行ごとのレポートと[公開・検証記録](releases/README.md)に記録します。現行文書に製品版番号を付けず、改訂はGitで管理します。
