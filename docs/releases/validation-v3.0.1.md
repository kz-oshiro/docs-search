# v3.0.1 検証・公開記録

2026-10-03。Cargo 集約・WASM 撤去後の自動テスト・Windows Release ビルドを実施し、ユーザーの公開依頼に基づいて結果をレビューした。実装前基準は `750c4674df79c3a0d7f9bd014462ace525e8dcbc`。検索の要求契約は [v3.0.0](../specifications/rust-requirements-v3.0.0.md)を継続する。

## 最終実行

テスト・ビルド対象ソースは `7c550f669605df1e7247ebed7c59b4b8f5f01bf9`。開始時の作業ツリーは clean。実行後の公開準備は文書だけを変更し、実行ソース・版番号・依存・試験内容を変更していない。

```powershell
$env:PROPTEST_CASES = '1024'
$env:PROPTEST_RNG_SEED = '3000002'
cargo xtask ci
```

実行先は `outputs/runs/1791034713571-9428-ci/`。終了コード0、全16工程 passed、全体 `success=true`。[Markdown レポート](../../outputs/runs/1791034713571-9428-ci/report.md)、[JSON レポート](../../outputs/runs/1791034713571-9428-ci/report.json)、[UI 結果](../../outputs/runs/1791034713571-9428-ci/ui/results.json)と工程別 stdout/stderr を保存した。`outputs/` は Git 除外のローカル記録である。

環境: Windows x86_64、rustc/Cargo 1.91.1、Node.js 26.7.0、npm 11.19.0、Playwright 1.63.0。Playwright/Chromium は `cargo xtask setup` で準備済み。

| 確認 | 結果 |
| --- | --- |
| コア/性質 | 47テスト成功。17性質を各1,024ケース（合計17,408）、固定 seed 3000002で確認 |
| 公開 API | 17テスト成功。コアと合計64件、失敗/無視0 |
| 生成器 | 文書・在庫/Office・アイコンの3テスト成功 |
| 共通 CLI | 原本27ケース、拡張子カタログと要求/終了契約を確認 |
| 周辺 CLI | 5項目成功 |
| 条件 CLI | 31項目成功 |
| Office/順位/一括 CLI | 追加抽出範囲・固定20クエリの優先ペア・直接/初回/再利用順位・一括契約が成功 |
| イシュー CLI | 4項目成功 |
| 索引 CLI | 9項目成功。子プロセスごとに一時 LOCALAPPDATA を分離 |
| ヘッドレス Playwright | 実 HTML/CSS/JS の54件成功。skipped/unexpected/flaky 0、画像・動画・トレースなし |
| Windows Release | `cargo build --release --locked -p docs-search-desktop` 成功 |

xtask 起動後の記録時間は135.567秒、うち Windows ビルド37.729秒、Playwright 工程19.694秒。外側の xtask 自身のコンパイル時間を含まない。既存キャッシュを使った今回の単発実測であり、旧版との速度比較や改善率の根拠にはしない。

## 初回実行・旧生成器比較のレビュー

先行 CI `1791033784432-14852-ci` の全工程成功、ログ、UI54件と成果物をレビューした。その実行までに移行時の型差を修正している。イシュー CLI の更新日時は JSON の数値を `as_f64` で照合し、順位報告は OS の区切り文字に合わせたパスを照合した。未使用 import を削除した。検索の期待値を緩和した修正ではない。版番号を3.0.1に揃えてコミットし、上記の clean な最終 CI を再実行した。

旧 Python は移行比較だけに使用した。独立した一時 checkout と新規出力先を使い、元の保存データを上書きしていない。旧 checkout の autoCRLF により共通原本の入力バイトが異なる初回比較があったため、Git 正規化後に同じ内容であることを確認し、元の作業ツリーと同じ原本バイトに揃えて旧データを再生成した。原本の tracked 内容は変更していない。

| 種類 | 比較レポートの実行ID | 契約 | ZIP全体のバイト差 |
| --- | --- | --- | --- |
| 共通 acceptance | `1791033720976-16048-compare-fixtures` | 一致 | 5ファイル |
| 共通 load | `1791033721048-5676-compare-fixtures` | 一致 | 100ファイル |
| context | `1791033558060-8044-compare-fixtures` | 一致 | 1ファイル |
| conditions | `1791033557922-13448-compare-fixtures` | 一致 | 2ファイル |
| office | `1791033721011-7484-compare-fixtures` | 一致 | 24ファイル |
| issues | `1791033557845-14400-compare-fixtures` | 一致 | 1ファイル |

各 `outputs/runs/<実行ID>/fixture-comparison.json` は `sameContract=true`、`differences=[]`。非ZIPの全バイト、ZIP部品の順序・名前・展開内容・固定日時・圧縮方式が一致した。ZIP全体の SHA256 差は Python/Rust の圧縮・シリアライズ差として別記録した。新生成器を別の新規先に再実行し、453ファイル（共通35/150、context1、conditions4、office51、issues212）の全 SHA256 が一致した。生成データは `outputs/migration-validation/20261003/` に保存した。

## 公開前の差分レビュー

- Tauri の invoke/event 名、wire fields、capability とグローバル API の設定を照合した。production の検索コア/Tauri 処理は今回の移行で変更していない。
- 旧 inline JavaScript と移行後の表示処理を比較し、既存 DOM/CSS/表示・保存キーを継続していることを確認した。購読準備・フォルダー復元、起動失敗、古い通知、CRLF 入力は実画面の自動試験で確認した。
- 最終 UI 試験の資材と配布用 staging の8ファイルがソースと一致した。WASM/wasm-bindgen 資材を配布/要求しない。
- 統合 Cargo.lock の外部パッケージ版を旧 lockfile 群と比較した。新しい外部パッケージ版の追加はなく、旧エントリー2件が除かれた。通常依存 tree に test-support/proptest が入らない。
- workspace 4パッケージ、Tauri、UI package/lock と EXE の版は3.0.1。metadata `--locked`、整形・構文、差分空白と文書参照を確認した。履歴文書の撤去済みファイルへの参照は基準コミットへのリンクへ直した。

以上の自動検証・ビルド・差分レビューに、公開を止める問題は見つからなかった。

## 配布成果物と未確認範囲

最終成果物は [docs-search-desktop.exe](../../outputs/runs/1791034713571-9428-ci/artifacts/docs-search-desktop.exe)。13,027,840 bytes、FileVersion/ProductVersion `3.0.1`。

```text
SHA256 39ea2a6f2846193fbc3af5f50fff9bb3e959280a5a03df6b4e81cfa003974328
```

実アプリを起動していない。実 WebView2 の接続、OS のダイアログ/クリップボード/ファイル操作、DPI・見た目は [M1〜M5](../test-plans/playwright-ui-test-plan.md#ユーザー確認票実アプリの最終5項目)の未確認範囲として残る。同一条件の旧/新 cold・warm 性能比較、実 Office 互換性の追加評価、GitHub Actions の作成とイシュー完了操作も未実施である。

## 公開の確認

公開先は [v3.0.1 GitHub Release](https://github.com/kz-oshiro/docs-search/releases/tag/v3.0.1)。この文書の公開準備時点では push・タグ・Release の確認はまだ行っていない。公開後の API/remote 照合結果を追記する。
