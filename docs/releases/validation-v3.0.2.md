# v3.0.2 検証・公開記録

2026-10-04。ユーザーの文書修正・リリース依頼に基づき、現行文書の版表記と参照を整理し、自動試験・Windows Releaseビルドを実施した。

## 対象と静的確認

- 基準コミット: `29de988a0965516deedb7d2660eaf64c64161119`（公開済みv3.0.1の確認記録を含むmain）。
- テスト・ビルド対象: `4b363158833312ad960725888185060590d819d8`。開始時の作業ツリーはclean。
- 現行の仕様・開発手順・試験設計から製品版番号を外し、[Rust要求仕様](../specifications/rust-requirements.md)を版番号なしのファイル名に変更した。[公開・検証記録](README.md)と[実装履歴](../reports/implementation-history.md)を追加した。
- 試験の3区分・振り分けと現行入口の実装範囲を文書に反映した。検索・画面・試験の実行コードや原本は変更していない。
- workspaceの4パッケージ、Tauri、UI package/lockの製品版を3.0.2に揃えた。Cargo.lockの外部依存とPlaywright 1.63.0は変更していない。
- `cargo metadata --no-deps --locked --format-version 1`、`cargo fmt --all -- --check`、JavaScript構文、差分空白、Markdown/HTMLのローカル参照を確認した。過去の検証記録は要求仕様の参照先だけを当時のGitタグに固定し、結果本文を変更していない。

## 実行手順と記録

```powershell
$env:PROPTEST_CASES = '1024'
$env:PROPTEST_RNG_SEED = '3000002'
cargo xtask ci
```

実行IDは `1791043954158-16528-ci`、開始は2026-10-04 01:12:34 JST。終了コード0、全16工程passed、`success=true`。xtask起動後の記録時間は133.044秒（Windowsビルド38.404秒、Playwright工程19.763秒）。今回の単発実測であり、旧版との性能比較には使わない。

[Markdownレポート](../../outputs/runs/1791043954158-16528-ci/report.md)、[JSONレポート](../../outputs/runs/1791043954158-16528-ci/report.json)、[フロントエンド試験結果](../../outputs/runs/1791043954158-16528-ci/ui/results.json)と工程別stdout/stderrを保存した。`outputs/` はGit除外のローカル記録である。

環境: Windows x86_64、rustc/Cargo 1.91.1、Node.js 26.7.0、npm 11.19.0、Playwright 1.63.0。既存のChromiumとビルドキャッシュを使用した。

## 試験結果

| 区分 | 確認項目と期待結果 | 実施結果 |
| --- | --- | --- |
| バックエンド試験 | 抽出・照合・索引・周辺・条件・順位・一括・編集・出力の意味、公開API/CLIの契約が固定期待値と一致する | Rustコア47件、公開API17件の計64件成功。17性質を各1,024ケース（合計17,408）、seed 3000002で確認。失敗・無視0 |
| バックエンド試験 | 共通・周辺・条件・Office/順位/一括・イシュー・索引の実CLIが契約を満たす | 全6群成功。共通原本27ケース、周辺5項目、条件31項目、固定20クエリの順位と一括契約、イシュー4項目、索引9項目を確認 |
| フロントエンド試験 | 実HTML/CSS/JavaScriptのDOM・操作・状態・要求引数・応答表示が境界モックの固定期待値と一致する | ヘッドレスPlaywrightの既存54件成功。skipped/unexpected/flakyは0。画像・動画・トレースを使用せず、テキスト結果のみ保存 |
| アプリケーション結合試験 | 個別試験では観測できない実資材起動・IPC/通知配送・実セッション・再起動復元の接続 | 有効なEケース9群とランナーは未実装。現行ciは実EXEのビルドまでであり、結合試験の成功には数えない |

G01/G02の構造・配色拡張も未実装で、既存54件に含めていない。両区分の追加設計は[フロントエンド試験設計](../test-plans/playwright-ui-test-plan.md)と[アプリケーション結合試験設計](../test-plans/playwright-exe-test-plan.md)を参照する。

共通の検証基盤として、文書・在庫/Office・アイコンの生成器3件が成功した。ソース、今回のUI staging、配布用stagingの8資材のSHA256が一致した。`cargo build --release --locked -p docs-search-desktop` と成果物保存が成功した。これらを画面・結合ケースの件数に加算しない。

## 配布成果物

今回のCIで保存した [docs-search-desktop.exe](../../outputs/runs/1791043954158-16528-ci/artifacts/docs-search-desktop.exe) をそのまま公開する。13,027,328 bytes、FileVersion/ProductVersion `3.0.2`。レポートの値と実ファイルのサイズ・SHA256・版情報が一致した。

```text
SHA256 b7ce33ab0bb97c63acb1c165b22c1e2832401a2aaf2c40c5068aefe0f47f96af
```

ネイティブダイアログ・実クリップボード・既定アプリ・排他ロック・OS表示は試験設計で保証対象外とする。必須の手動確認票にはしない。今回、実EXEは起動していない。

検証後の公開準備ではこの記録・リリースノート・索引だけを追加する。テスト済みの実行コード、製品版、依存、試験コードは変更せず、再ビルドしない。

## 公開の確認

- mainと注釈付きタグ `v3.0.2` を同時にpushし、タグの対象 `6b6b0aa747ffdb98b6f00a78f60cb70a1bf993cd` をremoteで確認した。テスト対象 `4b36315` との差分は上記3文書だけである。
- 2026-10-04 01:18:28 JSTに [v3.0.2 GitHub Release](https://github.com/kz-oshiro/docs-search/releases/tag/v3.0.2) を公開した。APIでdraft=false、prerelease=false、Latest=v3.0.2を確認した。
- 公開アセット [docs-search-desktop.exe](https://github.com/kz-oshiro/docs-search/releases/download/v3.0.2/docs-search-desktop.exe)（asset ID `608118140`）はuploaded、13,027,328 bytes。GitHubのdigest `sha256:b7ce33ab0bb97c63acb1c165b22c1e2832401a2aaf2c40c5068aefe0f47f96af` が今回のCIのEXEと一致した。
- 公開直後はremote mainとタグの対象が `6b6b0aa` に一致し、ahead/behind `0 0` とcleanな作業ツリーを確認した。この公開確認の追記だけを後続の文書コミットでmainに反映し、タグとアセットは維持する。
