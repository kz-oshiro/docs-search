# テストと文書データ

全自動検証はルートの `cargo xtask test`、初回 UI 依存準備は `cargo xtask setup` です。保存データは `cargo xtask fixtures` / `Generate-Test-Data.cmd` で作ります。

実行順序・失敗時の扱いは[開発手順](../docs/development.md#テスト)、worker選択・並列実行・性能比較の確認は[CI性能検証方針](../docs/test-plans/ci-performance-test-plan.md)を参照してください。

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

| 対象 | Cargo の integration test |
|---|---|
| 共通検索 | `core/tests/cli_backend.rs`：共通27・93拡張子の初期選択 |
| Excel周辺 | `core/tests/cli_context.rs`：周辺5クエリ、直接/初回索引/再利用 |
| 条件検索 | `core/tests/cli_conditions.rs`：条件31、根拠・範囲・拒否 |
| Office・一括・順位 | `core/tests/cli_office.rs`：Office opt-in・部分失敗・一括・順位20組 |
| イシュー対応 | `core/tests/cli_issues.rs`：可視文字・位置順・更新日時・索引旧版 |
| 索引 | `core/tests/cli_index.rs`：off・鮮度・削除・破損・利用不能 |

個別診断は `cargo test --locked -p docs-search-core --test cli_index -- --nocapture` のように実行できます。全件の結果には mandatory UI を含む `cargo xtask test` を使います。各子 CLI はそのテストの LOCALAPPDATA だけを使用し、プロセス全体の環境を変更しません。core の既存単体/API/proptest は継続します。

worker数の計算・CPU差分・測定不能時のfallbackを `node --test tests/ui/workers.test.mjs` で個別診断できます。OS値と待機を注入した8件の単体試験であり、実CPUへの負荷やPlaywright起動は発生させません。この計算の試験はバックエンド区分とし、`cargo xtask test` / `ci` の `worker-policy` 工程に含めます。

## フロントエンド試験とアプリケーション結合試験

ケースと判定方法は[フロントエンド試験設計](../docs/test-plans/playwright-ui-test-plan.md)、試験ランナーの実装済み・設計のみの区別は[実装状況](../docs/reports/implementation-status.md#試験の実装と振り分け)を参照してください。実施結果は[開発手順の実行記録](../docs/development.md#記録の自動生成と公開)と[GitHub Releases](https://github.com/kz-oshiro/docs-search/releases)から確認します。

試験の担当と結果報告は[試験区分と振り分け](../docs/test-plans/playwright-exe-test-plan.md#試験区分と振り分けの基本方針)、OS連携の範囲は[保証対象外の定義](../docs/test-plans/playwright-exe-test-plan.md#5-os連携の保証対象外)を参照してください。
