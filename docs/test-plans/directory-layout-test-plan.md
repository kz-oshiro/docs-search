# ディレクトリ整理の試験方針

対象: [実装報告](../reports/implementation-report-directory-layout.md)。配置変更後の実行試験は未実施。コマンドはリポジトリルートで実行する。

## 確認項目・手順・期待結果

| 確認項目 | 手順 | 期待結果 |
| --- | --- | --- |
| 全自動試験 | `powershell.exe -NoProfile -ExecutionPolicy RemoteSigned -File .\scripts\Run-Automated-Tests.ps1` | 共通生成器、CLI受け入れ、Rust単体/公開API、WASM型検査、実WASMのヘッドレスPlaywrightが新配置で成功。パス不在による失敗なし |
| 配布ビルドを含む確認 | ビルドまで依頼された場合、`powershell.exe -NoProfile -ExecutionPolicy RemoteSigned -File .\scripts\Run-Local-CI.ps1` | 全自動試験後に `outputs/build/docs-search-desktop.exe` を生成。一時画面と従来どおりのCargo release生成物を片付ける。既存outputsの試験記録・保存文書は保持 |
| 保存データ生成 | `Generate-Test-Data.cmd -NoOpen` と `Generate-Test-Data.cmd -Profile load -NoOpen`（PowerShellでは先頭に `.\`） | 毎回別の `outputs/test-data/` 配下へacceptance 33文書、load 148文書を生成。ケース原本は27件で一致し、既存データを上書きしない。フォルダーを開かない |
| 省略時出力先 | `outputs/test-data/generated/` が未作成の場合だけ `python .\tests\fixtures\generate-fixtures.py` | カレントディレクトリに依存せず既定の生成先へ出力。非空の既定先へ再実行すると拒否し、既存データを維持。既存なら削除せずこの確認を未実施として記録 |
| 実行中アプリ | 実アプリ確認を許可された場合だけ、新配置のEXEを起動してBuild.ps1の事前検出を確認 | 起動中はビルド前に終了案内を表示。閉じた後はビルド可能 |

配布ビルド込みの依頼ではRun-Local-CI.ps1を1回実行し、全自動試験を重複実行しない。途中で失敗した場合は原因を直して依頼範囲を再確認する。履歴のv3.0.0成功記録を今回の成功結果に流用しない。

## 依存準備・判定境界

新配置でNode依存がない場合、[Playwright準備](playwright-ui-test-plan.md#実装と準備)に従って `tests/ui/` でnpm ciとChromium導入を行う。自動試験には必要なWASM生成を含める。旧node_modulesやCargo targetを自動で移動・削除しない。

ヘッドレスPlaywrightはDOM・状態・要求引数とテキスト記録で判定し、スクリーンショット・画像比較・動画・トレースを使わない。実アプリ起動・OS連携・見た目は別の確認であり、明示依頼または事前許可が必要。通常の自動試験でGUI確認は行わない。
