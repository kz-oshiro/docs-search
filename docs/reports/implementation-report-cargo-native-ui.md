# Cargo 集約・ネイティブ UI 接続の実装報告

2026-10-03。実装前基準 `750c4674df79c3a0d7f9bd014462ace525e8dcbc`。この報告は実装と静的確認までを依頼された時点の記録である。

同日の後続依頼で Cargo 自動一式、旧生成器比較、Windows Release ビルドと結果レビューを完了した。Rust 64件、生成器3件、CLI 全6群、Playwright 54件が成功し、v3.0.1 の公開用 EXE を作成した。実行後の修正・成果物・未確認範囲は [v3.0.1 検証記録](../releases/validation-v3.0.1.md)を優先する。以下の未実施記述は実装報告時点に限る。

## 変更

- root Cargo workspace/lock と Rust 1.91.1 に統一。通常対象は core/test-support、Tauriは明示ビルド。既存の依存版を維持し、開発用crateを追加した。
- frontend のRust/WASM crateを撤去。既存inline JSの表示処理を view.js に抽出し、app.js に入力/状態/通知、tauri.js に IPC を移した。DOM/CSS/表示文言/保存キーと wire fields は継続する。購読の準備前と保存フォルダーの検証中は検索を開始せず、旧ID・終了後の通知と旧開始失敗を無視する。bootは資材失敗も表示する。
- cargo xtaskに setup/test/ui/build/ci/fixtures/icons と移行用 compare-fixtures を実装。UI資材のallow-listと内容比較、debug/release保持、独立テスト群の続行、全必須成功後のビルド、実行別JSON/Markdown/ログ/順位/UI記録、成功EXEのSHA256を追加した。
- 6 Python CLIランナーをRust integration testへ、文書生成器とアイコン生成器を独立した開発専用test-supportへ移行。共通原本・固定seed・ケース期待値を保持。共通loadを実行内で共有し、可変文書/索引を隔離。production appにtest-support依存を追加していない。
- Playwrightの既存51期待値を保ち、起動資材失敗・WASM資材なし・CRLF要求の3件を追加。実HTML/CSS/JSを操作する。画像/動画/トレースは無効。
- 旧PS/Python workerとUI crate/個別lockを削除し、cmdはCargoの薄い入口へ変更。現行文書/構造ガイド/AGENTSを更新。保存済みoutputs、旧target、node_modules、PS成果物アーカイブは保持した。GitHub Actionsとリモート反映は実施していない。

## 確認と次の検証

静的確認は Cargo lock のworkspace解決、metadata --no-deps --locked、Rust整形/構文、JS構文、資材/依存/文書参照と差分。コンパイルによる型検査・自動試験・fixture生成・アイコン再生成・Windowsビルド・アプリ起動・GUI・pushは未実施。静的確認は実行成功の証拠にはならない。

次の検証担当は[今回の確認項目・手順・期待結果](../test-plans/cargo-native-ui-test-plan.md)を読み、依頼された範囲で setup→test→旧生成器比較→build/ci と性能計測を進める。テストを利用者へ移さない。実行記録は outputs/runs に保存し、今回の変更と過去のv3.0.0成功を区別する。

型/API差、ZIP/Zlibのバイト差、JavaScriptへの状態移植、Windows/WebView2の実接続は未検証の箇所として残る。検索アルゴリズムの変更はなく、性能改善率はまだ測定していない。
