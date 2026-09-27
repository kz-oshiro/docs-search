# doc-search v2.0

Windows 向けの文書横断検索アプリです。Rust の検索コアがローカル文書を読み取り、Tauri がデスクトップ画面との境界を担当し、画面の状態と操作は Rust からビルドした WebAssembly で動きます。Office や PowerShell を検索処理に使用しません。

## 構成

| ディレクトリ | 役割 |
| --- | --- |
| `core/` | Rust 検索コアとイベント契約。CLI からも GUI なしで実行可能 |
| `src-tauri/` | Windows のウィンドウ、フォルダー選択、既定アプリで開く操作、順序を保った検索イベントのバッチ転送 |
| `frontend/` | Rust→WASM の画面制御と HTML/CSS |
| `tests/` | 共通 `backend-cases.json` を実行するテストアダプター |

検索対象は `.xlsx`、`.xlsm`、`.pptx`、`.docx`、`.txt`、`.jsp`、`.xhtml`、`.html`、`.js`、`.java` から選べます。再帰検索、ファイル名・内容の部分一致、NFC と Unicode case folding、場所付き結果、個別エラー、進捗、中断を提供します。隠しシート・隠しスライドを含み、Office 一時ファイルとシンボリックリンクを除きます。読んだ文書には書き込みません。対象外の形式・箇所は [バックエンド仕様](../docs/backend.md) を参照してください。

## 動作条件とビルド

- Windows 10/11、WebView2、Microsoft C++ Build Tools、Rust 1.90 以降の MSVC ツールチェーン
- Python 3 は共通テストを実行するときだけ必要
- Rust の `wasm32-unknown-unknown` ターゲットと `wasm-bindgen-cli` 0.2.129

初回だけ次を実行します。

```powershell
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
```

`v2.0` を作業ディレクトリとしてビルドします。画面用ファイルは一時的な `.frontend-build/` に作られ、デスクトップアプリに取り込まれた後で削除されます。Cargo が生成する `release/` もビルド後に削除します。完成した実行ファイルだけを `v2.0/` 直下に残します。実行ファイルとビルド途中の生成物は Git の管理対象外です。`release/` を残さないため、次回のビルドでは依存関係も再コンパイルされます。

```powershell
powershell.exe -NoProfile -ExecutionPolicy RemoteSigned -File .\Build-v2.0.ps1
```

ビルド後は `v2.0\doc-search-desktop.exe` をダブルクリックして起動します。再ビルドの前には実行中のアプリを閉じてください。アプリ単体の起動ではターミナルを表示しません。`Build-v2.0.ps1 -Run` は開発用で、実行元の PowerShell ウィンドウが残ります。ビルドには crates.io へのアクセスが必要です。

## 操作

検索フォルダーを入力または「選択…」で選び、検索語を入れ、対象拡張子を選んで「検索」を押します。初期状態では従来の5拡張子が選択され、新しい5拡張子は必要に応じて追加できます。選択をすべて外すと検索は開始しません。結果は到着順で表示され、一致箇所を黄色で示します。結果を選択すると絶対パスと文書内の場所を確認でき、「元ファイルを開く」「場所をコピー」を使えます。多数の結果があるときは 200 件ずつ表示し、残りは「さらに表示」で確認できます。結果件数は表示範囲にかかわらず全件です。検索中は「中断」で停止を要求できます。

`.txt`、`.jsp`、`.xhtml`、`.html`、`.js`、`.java` は UTF-8 と UTF-8 BOM のみ対応し、保存されたテキストを行単位で検索します。タグやコードの構文解析・実行はしません。読み取れないファイルは個別のエラーとして表示して検索を続けます。暗号化ファイル、壊れたファイル、ZIP/XML の展開上限に達したファイルは検索できません。ZIP の XML 部品は 1 部品 32 MiB、合計 128 MiB、部品数 4096 を上限とします。実ファイルの互換性をすべて保証するものではありません。

## テスト

`doc-search` を作業ディレクトリとして、共通生成データと全 15 ケースを確認します。テストデータは一時ディレクトリに作成し、終了時に削除します。

```powershell
python .\v2.0\tests\run-backend-cases.py
cargo test --manifest-path .\v2.0\core\Cargo.toml
cargo check --manifest-path .\v2.0\frontend\Cargo.toml --target wasm32-unknown-unknown
```

CLI でイベントを JSON Lines として確認できます。

```powershell
.\v2.0\core\target\debug\doc-search-cli.exe C:\Documents beacon
.\v2.0\core\target\debug\doc-search-cli.exe C:\Documents beacon --extensions js,java
```

GUI はデスクトップ上で実際に起動して、フォルダー選択、検索・中断、結果の選択、元ファイル起動、クリップボードへのコピーを確認します。バックエンドの共通ケースは GUI の操作性そのものを検証しません。
