# ディレクトリ整理の実装報告

作業日: 2026-10-03。今回の確認は実装と静的確認まで。以前のv3.0.0検証結果は配置変更後の動作確認として扱わない。

## 変更内容

- `tauri/` 内の3 crateを `core/`、`frontend/`、`src-tauri/` へ移動。Rustモジュール、公開API、依存関係、Cargo.lock、版番号は変更しない。Cargo workspaceは導入しない。
- ビルド・自動試験・保存用データ生成・アイコン生成の入口を `scripts/` へ集約。旧入口の転送スクリプトは置かない。ルートの `Generate-Test-Data.cmd` は維持する。
- テストを `tests/fixtures/`、`tests/cli/`、`tests/ui/` へ分類。文書を仕様・試験方針・計画・実装報告・リリース記録に分類し、開発手順を `docs/development.md` に集約する。構造ガイドHTMLの内容とリンクも更新する。
- 新規配布EXEは `outputs/build/docs-search-desktop.exe`、一時画面は `outputs/build/.frontend-build/`。TauriのfrontendDist、スクリプトのルート解決、CLI/生成器参照、後片付け対象、成果物確認を更新する。
- 生成器の省略時出力先はリポジトリルートの `outputs/test-data/generated/`。保存用ランチャーは既存の日時・識別子付き出力を維持し、既存の非空フォルダーへは生成しない。
- 空の `powershell/` を除去。移行バックアップ、既存outputs、旧Cargoキャッシュ、node_modules、配布EXEは移動・削除しない。旧 `tauri/` にローカル生成物と空フォルダーが残る場合がある。旧配置の生成物用Git除外も保持する。
- 過去のリリース・移行記録の事実と当時のコマンドは保持し、文書リンクのみ新配置へ更新する。

## 確認範囲

管理対象97ファイルの移動先を照合し、92ファイルの移動と欠落がないことを確認した。検索コア・画面・Tauriのソース/資産、Cargoメタデータ/lock、Playwright lock、ケース原本の計34ファイルは、Gitのチェックアウト改行を除き内容不変を確認した。ローカル文書リンク、Python 13ファイル、PowerShell 6ファイル、JavaScript、Cargo TOML・JSONの構文と参照、変更行の空白を静的確認し、git diff --checkも成功した。

## 次の担当への引き継ぎ

[配置変更の試験方針](../test-plans/directory-layout-test-plan.md)を読み、依頼された範囲を実施する。テスト全体の実施をユーザーへ移さない。新しい `tests/ui/` には依存を移していないため、必要なら依存準備を行う。

## 実行確認（2026-10-03）

- `npm ci` とChromium準備後、`scripts/Run-Local-CI.ps1` が終了コード0で完了。生成器検証2件、共通ケース27件、P2/P3・Office順位一括・イシュー・索引のCLI確認、Rust単体47件、公開API17件、WASM型検査、実WASMのヘッドレスPlaywright51件が成功した。
- acceptance 33文書、load 148文書をそれぞれ日時付きの新規フォルダーに生成。既定先 `outputs/test-data/generated/` にも33文書を生成し、非空の既定先への再実行が終了コード1で拒否され、manifestが不変であることを確認した。
- Windows Release EXEを `outputs/build/docs-search-desktop.exe` に生成。サイズ13,119,488 bytes、SHA-256 `8BA2D0AA281F134857E92BF4A7684C69DC5352A233395DB4B0A1FEFEE5C50A0F`。
- Playwrightのテキスト結果は `outputs/ui-tests/20261003-193252-69996de4f125421491fd9355969166d9/results.json` に保存した。実アプリ起動・対話GUI・OS連携・見た目の確認は未実施。
