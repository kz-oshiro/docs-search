# docs-search の開発

Rust 検索コア、Tauri の OS/IPC 境界、HTML/CSS/JavaScript の画面で構成します。`frontend/app.js` が要求と検索状態、`view.js` が DOM・周辺・編集・出力、`tauri.js` が接続、`theme.js` がテーマを担当します。利用者向けの案内は[README](../README.md)、機能の契約は[Rust要求仕様](specifications/rust-requirements.md)を参照してください。

## エージェントの作業ルール

- 機能修正では設計・実装・変更に応じた確認項目/手順/期待結果を用意します。実装依頼は必要な差分・文書の静的確認と報告までとし、型検査・テスト・ビルド・アプリ起動・GUI操作・リモート反映へ進みません。
- 要件・設計・実装・検証の各担当は、対象案件の[Issue](https://github.com/kz-oshiro/docs-search/issues?q=is%3Aissue+label%3Adevelopment)を取得し、[ひな型](documentation.md#案件issueのひな型)に従って更新し、成果物・対象コミット・未実施項目・次の作業と完了条件を残します。次の担当は記録されたソースと現在の差分を確認します。テスト・ビルド以降を別のモデルへ引き継ぎ、テスト全体をユーザーへ依頼しません。
- テスト依頼では、対象Issueと対象の試験設計を読み、確認項目・手順・期待結果を把握してから実行します。テスト・ビルド・リモート反映は依頼された範囲を途中で止めず完了し、非対話的なテスト・ビルドの実行前に再確認を求めません。
- 自動テストには実HTML/CSS/JavaScriptを操作するヘッドレスPlaywright、資材staging、Releaseビルド・実EXE起動・Playwright結合試験を常時含め、GUI操作の再確認を求めません。実装と検証の現在地は[試験拡張のIssue](https://github.com/kz-oshiro/docs-search/issues/18)で区別します。
- 対話的にAIが操作するGUIテストだけは実施前に確認します。今回の依頼で明示されていれば再確認は不要です。確認を待つ間も依頼済みの自動テストを進めます。
- 試験設計と結果報告は[試験区分と振り分け](test-plans/playwright-exe-test-plan.md#試験区分と振り分けの基本方針)に従います。[フロントエンドの判定方法](test-plans/playwright-ui-test-plan.md#確認境界と実行依頼の意味)と[OS連携の保証対象外](test-plans/playwright-exe-test-plan.md#5-os連携の保証対象外)を読み、必須の手動確認票や一律の「GUI未検証」を報告に追加しません。
- 文書の作成・更新は[文書作成ガイド](documentation.md)、テスト・ビルド結果の扱いと公開は[記録の自動生成と公開](#記録の自動生成と公開)に従います。

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
cargo xtask ui --case "U23:"
cargo xtask exe
```

`test` は core の単体・公開 API・proptest、生成器の再現性/在庫/Office 検証、実 CLI の共通27・周辺5・条件31・Office/一括/順位20組・イシュー・索引、必須ヘッドレス Playwright を実行します。個別試験の証跡が全必須成功なら、Release EXEを1回ビルドして必須9ケースのアプリケーション結合試験へ進みます。実 CLI は Cargo の `CARGO_BIN_EXE_docs-search-cli` を使い、CLIランナー内で別のビルドをしません。LOCALAPPDATAと変更用文書は子プロセス・ケースごとに独立し、共通loadは実行内で共有します。

バックエンドのCargo試験とフロントエンド試験は別の実行枝で同時に開始し、両方の終了後に結果を合流します。Cargoの各工程は順次実行します。CPU使用率の計算・worker選択のNode単体試験もバックエンド側の必須工程です。いずれかが失敗しても他方と独立した残りの工程を続けます。

フロントエンドのworker数は開始前に200ms間隔で3回CPU時間の差分を測って決めます。利用可能CPU数は `os.availableParallelism()`、負荷は `os.cpus()` の累積時間を使います。3区間の最大使用率から80%目標までの空きを計算し、利用可能CPUの半分・6 workersを上限、1 workerを下限にします。測定不能時は理由付きで1にします。各実行で再計測し、同じPlaywright実行中は決定値を共有します。ファイル内の試験順序は維持し、ファイル間だけを並列化します。[CI性能検証方針](test-plans/ci-performance-test-plan.md)に計算例と確認手順を記載しています。

フロントエンド試験の選択実行は絞った診断用です。全体合格の証拠には `cargo xtask test` / `ci` の全件実行を使います。判定方法・確認項目は[フロントエンド試験設計](test-plans/playwright-ui-test-plan.md)、対象機能の試験設計は[案件Issue一覧](https://github.com/kz-oshiro/docs-search/issues?q=is%3Aissue+label%3Adevelopment)を参照してください。

試験の分類と結果報告は[試験区分と振り分け](test-plans/playwright-exe-test-plan.md#試験区分と振り分けの基本方針)を参照してください。

`exe`は現在のソースをReleaseビルドして結合9ケースだけを全件実行する診断入口です。Windowsの操作可能なセッションとWebView2が必要です。前提不足・接続不可・未実行は理由付きskippedと非ゼロ終了で残します。`ui --case` / `exe`だけの成功を全体合格へ読み替えません。実装と検証の状態は[試験拡張のIssue](https://github.com/kz-oshiro/docs-search/issues/18)、実行契約と観測範囲は[アプリケーション結合試験設計](test-plans/playwright-exe-test-plan.md#1-現行と導入後の実行契約)を参照してください。

## Windows ビルドと CI

```text
cargo xtask build
cargo xtask ci
```

`build` は frontend の8資材だけを `target/frontend-dist/` へ内容が変わった場合にコピーし、`cargo build --release --locked -p docs-search-desktop` を実行します。成功した現在の EXE だけを実行別 `outputs/runs/<実行ID>/artifacts/docs-search-desktop.exe` に保存し、SHA256 とサイズを記録します。Node/npm はこのビルドに不要です。実行中の EXE を再ビルドするときは閉じてください。

`ci` は`test`と同じ必須工程です。独立した個別テスト群を失敗後も続け、その証跡がすべて成功した場合だけReleaseをビルドし、結合試験で使用した同じEXEを成果物にします。結合ケースも独立した残りを続け、全体の失敗を一つの終了コードで返します。GitHub Actions はまだ作成していません。push、タグ、Release は検証入口に含めず、ユーザーの公開依頼を受けて別途行います。

各実行は新規 `outputs/runs/<実行ID>/` に `report.json` / `report.md`、工程別 stdout/stderr、Git commit/dirty、Rust/Cargo/Node/npm、Playwright の前提と実測版、所要時間、順位 Top-5、UI JSON/テキスト失敗記録を残します。レポートは異なる実行の成功ログを混ぜません。

並列試験のログには `backend-` / `frontend-` を付け、工程の開始時刻と経過時間を記録します。工程時間は重なるため総時間として加算しません。総時間は `finishedAtUnixMs - metadata.startedAtUnixMs`、外側のCargo起動・xtaskコンパイル込みは別に計測します。`ui/workers.json` にCPU各区間・平均/最大使用率・利用可能CPU数・選択worker数・fallback理由を保存し、トップレポートから参照します。proptestの件数/seedの環境変数もmetadataに残します。

`target/debug` / `target/release` / staged frontend を削除しません。キャッシュの保持はコンパイルの再利用であり、テストは毎回実行します。

## 記録の自動生成と公開

公開版の利用者向け案内は[GitHub Releases](https://github.com/kz-oshiro/docs-search/releases)、案件の要件・設計・実装・検証への対応は[案件Issue一覧](https://github.com/kz-oshiro/docs-search/issues?q=is%3Aissue+label%3Adevelopment)に置きます。詳細な公開検証は `docs/reports/validation/vX.Y.Z.md`、原データ・ログは同じ実行の `release/vX.Y.Z/validation.json` にローカル保存し、Releaseには添付しません。版別のリリースノートをリポジトリへ複製しません。

テスト・ビルド結果の原本は `outputs/runs/<ID>/report.md` / `report.json` です。案件Issueには実行ID・対象コミット・原本の場所と検証MDへの参照を残し、件数・環境・ハッシュ・正常ログを転記しません。読む範囲と会話報告は[文書作成ガイド](documentation.md#検証結果の読み方と会話報告)に従います。

`cargo xtask release-record` と `record-contracts` 工程は実装済みです。生成処理はpush・タグ作成・Release作成・添付・Git管理文書の更新を行いません。v3.0.0〜v3.0.2の旧手書き記録は既存Releaseの添付とGitの保存版に残し、新形式の証拠へ読み替えません。v3.0.4の[検証MD](reports/validation/v3.0.4.md)は既存の公開記録を移したもので、再実行結果ではありません。

### 記録生成

公開依頼を受けた担当は対象ソースをコミットして `cargo xtask ci` を実行し、短いUTF-8の利用者向け本文を `outputs/release-changes.md` に用意します。本文の読者と内容は[文書作成ガイド](documentation.md#利用者向けrelease本文)に従います。`--notes` の入力にタイトルは不要で、必要環境・利用者向け変更点・更新方法・既知の制限を記載します。

```text
cargo xtask release-record --run outputs/runs/<ID> --tag vX.Y.Z --notes outputs/release-changes.md
```

同じ実行の `release/vX.Y.Z/` に `release-notes.md`、`validation.md`、`validation.json` を生成します。本文へ試験の技術表を自動挿入せず、表は検証MDだけに載せます。JSONにはCI原本、工程別stdout/stderr、Playwright全ケース、worker計測、順位Top-5、生成manifestを格納します。MDには原データがローカル保存であることを明記し、非公開のJSONへのダウンロードリンクを載せません。追加依存はありません。`--repo OWNER/REPO` の既定値は `kz-oshiro/docs-search` です。

成功した新形式（`metadata.schemaVersion=2`）のci、開始/終了とも同じcleanなコミット、全必須工程、件数、資材ハッシュ、保存EXEを照合します。対象は存在するローカルタグ、未作成なら現在のHEADです。試験後の差分は `docs/` のMarkdown/HTML・ルートREADME/AGENTSだけを許容し、製品・依存・試験・ビルドコードの変更時は新しいCIが必要です。旧レポートを新形式の証拠へ読み替えません。

### 公開と読める検証記録

公開依頼の範囲で次の順に進めます。既存版の本文整理だけなら、既存の検証記録を使い、EXEを作り直したりタグを動かしたりしません。

1. main/タグをpushし、`gh release create --verify-tag --notes-file <生成先>/release-notes.md` で同じ実行のEXEだけを添付して公開します。`validation.json` はローカルに保持します。検証MDは後続手順でリポジトリから案内します。既存のMD添付は履歴として保持できますが、新しい公開記録の正本はリポジトリのMDとします。
2. 同じ `release-record` に `--published` を付けて、公開状態をread-onlyの `gh api` と `git ls-remote` で照合します。公開照合後の生成JSONもローカルに保持し、添付しません。不一致・取得失敗は理由を保存し、非ゼロで終了します。
3. 公開照合後の生成 `validation.md` を `docs/reports/validation/vX.Y.Z.md` へ保存し、コミット・pushします。対象Issueへ検証MD・コミット・Releaseの参照を追加します。記録内の照合時刻・remote mainは観測時点の値で、後続の文書コミットと一致する必要はありません。
4. 文書を含む完全なコミットSHAを取得し、利用者向けRelease本文へ `[詳しい検証記録](https://github.com/OWNER/REPO/blob/<文書コミットSHA>/docs/reports/validation/vX.Y.Z.md)` を追加します。読者向け本文を確定したUTF-8ファイルに保存し、`gh release edit vX.Y.Z --notes-file <本文ファイル>` で反映します。本文確定のために `release-record` を再実行して観測時刻を更新する必要はありません。
5. GitHubのMD表示ページ・EXEへ到達できること、`validation.json` が添付されていないこと、本文の利用条件がその配布版に一致すること、EXEのID・サイズ・digestとタグが変わっていないことを確認します。固定リンクが未確定の状態で公開手順を完了にしません。

試験・ビルドの件数は検証記録へ、引き継ぎの判断は案件Issueへ集約します。公開ごとの文書コミットは読みやすい検証MDを保存するために行い、Issueに件数・ログを転記しません。

### 次の検証担当の確認方針

| 区分 | 手順 | 期待結果 |
| --- | --- | --- |
| バックエンド試験 | 依頼範囲の `cargo xtask test` / `ci` | 既存Rust/CLI/worker試験が成功。Rust関数数とNodeケース数を別々に実測集計 |
| フロントエンド試験 | 上記全自動入口の必須Playwright | 既存全件を実行。statsとerrorsから実測を集計し、JSON欠落・skipped/flakyを全体成功にしない |
| アプリケーション結合試験 | 全自動入口、または診断用`cargo xtask exe` | 実EXEの9IDを各1回実行。`exe/results.json`の欠落・重複・skipped/flaky・再試行を成功にしない。EXEビルドだけを接続成功に数えない |
| 共通の検証基盤 | `cargo test --locked -p xtask`（全自動入口にも `record-contracts` として組込み）と資材最終照合 | 集計・欠落・旧形式・dirty・版不一致・公開digest不一致の契約試験が成功。内部PASS行を加算せず、ソース/UI/配布の8資材が一致 |
| 公開記録 | cleanな対象の成功ciに対して生成。公開依頼時だけ `--published` とEXEの照合 | 3ファイル生成、原ログをJSONに格納。同じEXE/タグを確認し、取得失敗を成功として書かない |

## リポジトリの構成

| 配置 | 役割 |
| --- | --- |
| `core/` | Rust検索コア・CLI・単体/API/CLI試験 |
| `frontend/` | 画面のHTML/CSS/JavaScript |
| `src-tauri/` | Windowsアプリ、OS/IPC境界、アイコン |
| `xtask/` | Cargoから呼ぶ試験・ビルド・記録生成の入口 |
| `test-support/`、`tests/` | 独立した生成器、固定ケース、画面試験 |
| `docs/` | 仕様・設計・手順・公開検証MD（案件管理はGitHub Issue） |
| `outputs/` | Git管理外の実行原本・成果物・保存用データ |

workspaceの通常対象は `core` と `test-support`。デスクトップは明示してビルドします。詳しい配置は[構造ガイド](rust-repository-structure.html)、試験とデータは[在庫](../tests/README.md)を参照してください。

## データとアイコン

```text
cargo xtask fixtures
cargo xtask fixtures --profile load --output outputs/test-data/new-load
cargo xtask fixtures --kind issues --output outputs/manual/new-issues
cargo xtask icons
```

生成先は新規または空のフォルダーに限ります。省略時は outputs/test-data の新しい実行別フォルダーです。`--kind common|context|conditions|office|issues` を指定できます。[在庫と役割](../tests/README.md)を参照してください。`Generate-Test-Data.cmd` は Cargo を呼ぶだけの入口で、ダブルクリックでは保存先を開き、従来の `-Profile` / `-NoOpen` も受け付けます。通常の cargo xtask fixtures は開かず、--open 指定時だけ開きます。アイコン生成は承認済み SVG の限定した図形のみを Rust でラスタライズし、通常の test / build では書き換えません。

アイコンはSVGの解析結果を8サイズで共有し、属性と図形固有の計算をサンプル処理の外へ移しています。各サンプルでは後ろに定義された可視図形から評価します。4×4サンプリング・画素の丸め・ICO/PNG形式を維持し、承認済み画素と2回生成の再現性を毎回検証します。Cargoのコンパイル最適化設定は変更していません。

## 静的確認と検証範囲

変更に応じて `cargo metadata --no-deps --locked`、`cargo fmt --all -- --check`、`node --check`、文書のリンク/内容照合、`git diff --check` を使います。書換えを行う整形コマンドは静的確認に含めません。実行範囲は[エージェントの作業ルール](#エージェントの作業ルール)、文書の確認項目は[文書作成ガイド](documentation.md#静的確認)を参照してください。
