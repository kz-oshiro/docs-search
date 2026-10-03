# Cargo 集約後の配置確認

今回の実行確認は未実施。[移行検証方針](cargo-native-ui-test-plan.md)を優先する。旧配置変更の記録は[当時の報告](../reports/implementation-report-directory-layout.md)で保存する。

| 手順 | 期待結果 |
|---|---|
| root Cargo metadata、README、構造ガイドの参照を確認 | core/src-tauri/xtask/test-support のworkspace、root lock、既定core/test-support。frontendにRust crateなし |
| cargo xtask setup / test | Python/PowerShell/WASM生成なし、実CLIと必須ヘッドレスUIを実行。索引は子プロセスの一時LOCALAPPDATA |
| cargo xtask build / ci | frontend8資材のみをstaging。cache保持。現在成功したEXEとSHA256を実行別に保存 |
| cargo xtask fixtures と --profile load / --kind issues | 新規保存先を表示。acceptance33/load148、既存非空先を拒否 |
| Generate-Test-Data.cmd を明示した手動範囲で実行 | Cargoの入口だけを呼び、既存データを変更せず保存先を表示 |
| outputs / 旧target / node_modules を確認 | 過去の保存データ・キャッシュ・PS成果物アーカイブを自動削除しない |

テスト/ビルド/起動/GUI/リモート反映は実装依頼だけでは行わない。GUI確認を別途依頼された場合にだけ実EXEの接続・OS操作・見た目を確認する。
