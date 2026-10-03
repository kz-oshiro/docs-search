# v3.0.0 Playwright導入の実装報告

2026-10-03。実装と静的確認まで。ユーザーと確定した「実WASM画面の自動操作＋OS/見た目の限定手動確認」「画像なし・テキストのみ」「自動テストに常時追加」を実装した。確認項目・手順・期待結果は [UI試験方針](../test-plans/playwright-ui-test-plan.md) に集約する。

## 変更

- Playwright 1.63.0を独立した [npmパッケージ](../../tests/ui/package.json)へ固定し、lockfileを作成した。Chromium/ヘッドレス/1 worker/再試行なし、画像・動画・トレースなし。npmはlockfile生成だけ実施し、依存インストールは行っていない。
- 配布と試験の [WASM画面生成](https://github.com/kz-oshiro/docs-search/blob/750c4674df79c3a0d7f9bd014462ace525e8dcbc/scripts/Build-Frontend.ps1)を共通化した。配布側の資材コピーを置換し、既存のEXEコピーとビルド後削除は保持した。UI側は専用出力/キャッシュへdebug WASMを生成し、Windows EXEを作らない。
- [UI入口](https://github.com/kz-oshiro/docs-search/blob/750c4674df79c3a0d7f9bd014462ace525e8dcbc/scripts/Run-Ui-Tests.ps1)を追加し、既存自動入口の末尾へ常時接続した。依存/Chromiumの不足は準備コマンド付きで失敗する。環境変数と作業ディレクトリはfinallyで復元する。
- UIソースを実WASM初期化から操作し、Tauri/クリップボード境界だけモック化。U01〜U39とパラメーター別ケースで入力、状態、結果、階層、フィルター、周辺、編集、出力、一括、テーマ、復元、キーボードを検査するコードを追加した。
- 実コアを呼ばない固定応答を、実検索・ファイル保存の成功と呼ばない。失敗記録はテキストだけを残し、ユーザー確認はM1〜M5へ絞った。
- AGENTS/README/各機能方針/要求対応表に、ヘッドレス自動と実アプリGUIの区別を追記した。従来の検証記録は当時の結果として維持する。

## 静的確認と未実施

JavaScriptの構文、PowerShellの構文/UTF-8 BOM、packageとlockの対応、モック応答と現行境界フィールド、DOMセレクターと現行画面、文書リンク、差分の空白を静的に確認した。これらはPlaywright実行・WASM生成・実アプリ起動の成功を示さない。

未実施: npm ci、Chromium導入、Playwright試験、既存自動試験の再実行、WASM/Windows配布ビルド、アプリ起動、対話的GUI、M1〜M5、性能、コミット/push/公開。

今回の既存作業ツリーには別機能の多数の未コミット変更がある。巻き戻し・一括コミット・リモート反映はしていない。UI試験の実行結果を [従来のv3.0.0検証記録](../releases/validation-v3.0.0.md)へ追加したことにもしていない。

## 次の検証担当

1. AGENTS、[本方針](../test-plans/playwright-ui-test-plan.md)、現在の機能別方針を読む。
2. テストが依頼された範囲でnpm ciとChromium準備を行い、まず `scripts/Run-Ui-Tests.ps1` を実行する。失敗時はID、期待値/実測値、修正を記録し、失敗ケースを再確認する。
3. 全自動が依頼されていれば `scripts/Run-Automated-Tests.ps1` で既存回帰とUIを完了する。配布ビルドも依頼されたときは `scripts/Run-Local-CI.ps1` を使う。
4. 結果を新しい検証記録へ保存し、ブラウザーUI成功と実アプリ未確認を分ける。GUIが明示依頼/許可された範囲だけM1〜M5へ進む。テスト全体をユーザーへ依頼しない。
