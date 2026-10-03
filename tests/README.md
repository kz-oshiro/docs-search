# テストと文書データ

全自動検証はルートの `cargo xtask test`、初回 UI 依存準備は `cargo xtask setup` です。保存データは `cargo xtask fixtures` / `Generate-Test-Data.cmd` で作ります。Python/PowerShell のランナーは撤去しました。

## 原本と生成器

`tests/fixtures/backend-cases.json` は共通27ケースの入力・結果・Issue・終了集計の原本です。`test-support` の Rust 生成器は検索エンジンから独立しており、core の dev-dependency と xtask だけが使います。配布アプリにはリンクしません。

固定 seed は **20260927**。acceptance は33文書、load は148文書です。load は同じ acceptance 33件と追加115件（xlsx32、pptx32、docx31、txt20）で、正常な Office 文書は合計100件、各450〜550KBです。manifest に相対パス・profile・seed、backend-cases.json に要求と期待値を保存します。壊れた Office、BOM/UTF-8 エラー、CP932 の日本語①、隠しシート/スライド、共有文字列・リッチテキスト・VML・入れ子表、コード93拡張子の代表も含みます。sample.py は検索対象の文書です。

| 生成コマンド | 内容 |
|---|---|
| `cargo xtask fixtures` | acceptance、outputs/test-data の新しい保存フォルダー |
| `cargo xtask fixtures --profile load` | acceptance と負荷用データ |
| `cargo xtask fixtures --kind context` | 境界セル・結合・非表示・アンカーの周辺用 Excel |
| `cargo xtask fixtures --kind conditions` | 条件検索の独立文書 |
| `cargo xtask fixtures --kind office` | 注記/数式/部分エラー/20順位ペア |
| `cargo xtask fixtures --kind issues` | 可視文字・書式・205ファイル・編集用 CP932/長文 |

`--output <新規または空のフォルダー>` で保存先を指定できます。cmd のダブルクリックは保存先を開きます。通常の fixtures は開かず、--open 指定時だけ開きます。存在するデータを上書きしません。自動試験では OS の一時領域に変更可能な文書・索引を置き、共通 load は実行内で共有します。保存用データ・実行の証跡は outputs に残します。

## 実 CLI の受け入れ

| 旧ランナー | Cargo の integration test |
|---|---|
| run-backend-cases.py | `core/tests/cli_backend.rs`：共通27・93拡張子の初期選択 |
| run-context-cases.py | `core/tests/cli_context.rs`：周辺5クエリ、直接/初回索引/再利用 |
| run-condition-cases.py | `core/tests/cli_conditions.rs`：条件31、根拠・範囲・拒否 |
| run-office-search-cases.py | `core/tests/cli_office.rs`：Office opt-in・部分失敗・一括・順位20組 |
| run-issue-cases.py | `core/tests/cli_issues.rs`：可視文字・位置順・更新日時・索引旧版 |
| run-index-cases.py | `core/tests/cli_index.rs`：off・鮮度・削除・破損・利用不能 |

個別診断は `cargo test --locked -p docs-search-core --test cli_index -- --nocapture` のように実行できます。全件の結果には mandatory UI を含む `cargo xtask test` を使います。各子 CLI はそのテストの LOCALAPPDATA だけを使用し、プロセス全体の環境を変更しません。core の既存単体/API/proptest は継続します。

## UI と移行の検証

[UI 方針](../docs/test-plans/playwright-ui-test-plan.md)の既存51件の期待値を保ち、WASM 資材なし・起動資材失敗・CRLF 条件要求を3件追加しました。現在54件は未実行です。実フロントエンドをヘッドレス Chromium で操作し、Tauri/OS 境界だけをモックにします。画像・動画・トレースは使いません。

[移行検証方針](../docs/test-plans/cargo-native-ui-test-plan.md)に、旧生成器との一度の比較、ZIP 内容/順序/日時/圧縮方式、再生成の SHA256、cold/warm の性能計測手順があります。実装だけの今回ではテスト・ビルド・生成・GUI・リモート反映を実行していません。
