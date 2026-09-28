# docs-search Tauri 実装

Windows 向けの文書横断検索アプリです。Rust の検索コアがローカル文書を読み取り、Tauri がデスクトップ画面との境界を担当し、画面の状態と操作は Rust からビルドした WebAssembly で動きます。Office や PowerShell を検索処理に使用しません。

## 構成

| ディレクトリ | 役割 |
| --- | --- |
| `core/` | Rust 検索コアとイベント契約。CLI からも GUI なしで実行可能 |
| `src-tauri/` | Windows のウィンドウ、フォルダー選択、既定アプリで開く操作、順序を保った検索イベントのバッチ転送 |
| `frontend/` | Rust→WASM の画面制御と HTML/CSS |
| `tests/` | 共通 `backend-cases.json` を実行するテストアダプター |

検索対象は Office 文書とテキスト系の計93拡張子から選べます。`.vue`、`.mjs`、`.jsx`、`.tsx`、`.json` などを含む一覧と対象外の形式・箇所は [バックエンド仕様](../docs/backend.md) を参照してください。再帰検索、ファイル名・内容の部分一致、NFC と Unicode case folding、場所付き結果、個別エラー、進捗、中断を提供します。隠しシート・隠しスライドを含み、Office 一時ファイルとシンボリックリンクを除きます。読んだ文書には書き込みません。

## 動作条件とビルド

- Windows 10/11、WebView2、Microsoft C++ Build Tools、Rust 1.90 以降の MSVC ツールチェーン
- Python 3 は共通テストを実行するときだけ必要
- Rust の `wasm32-unknown-unknown` ターゲットと `wasm-bindgen-cli` 0.2.129

初回だけ次を実行します。

```powershell
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
```

`tauri` を作業ディレクトリとしてビルドします。画面用ファイルは一時的な `.frontend-build/` に作られ、デスクトップアプリに取り込まれた後で削除されます。Cargo が生成する `release/` もビルド後に削除します。完成した実行ファイルだけを `tauri/` 直下に残します。実行ファイルとビルド途中の生成物は Git の管理対象外です。`release/` を残さないため、次回のビルドでは依存関係も再コンパイルされます。

```powershell
powershell.exe -NoProfile -ExecutionPolicy RemoteSigned -File .\Build.ps1
```

ビルド後は `tauri\docs-search-desktop.exe` をダブルクリックして起動します。再ビルドの前には実行中のアプリを閉じてください。アプリ単体の起動ではターミナルを表示しません。`Build.ps1 -Run` は開発用で、実行元の PowerShell ウィンドウが残ります。ビルドには crates.io へのアクセスが必要です。

## ローカル CI と公開

`docs-search` ディレクトリから次を実行すると、共通テストデータの検証、バックエンドの全ケース、検索コアのテスト、WASM のチェック、Windows 実行ファイルのビルドを順に行います。いずれかが失敗するとそこで停止します。初回は上記のビルドツールを準備してください。

```powershell
powershell.exe -NoProfile -ExecutionPolicy RemoteSigned -File .\tauri\Run-Local-CI.ps1
```

成功後に残る `tauri\docs-search-desktop.exe` は Git の履歴には含めません。ローカル CI と差分確認を終えたソースを `main` に push してから、そのコミットをタグ付けし、ローカルでビルドした実行ファイルを GitHub Release のアセットとして登録します。版番号は Cargo/Tauri のメタデータと Git タグで管理します。GitHub CLI の認証が必要です。

```powershell
$version = '2.0.4'  # 次に公開する版番号に置き換える
git tag "v$version"
git push origin "v$version"
gh release create "v$version" .\tauri\docs-search-desktop.exe --verify-tag --title "docs-search v$version" --notes "Windows x64 実行ファイル"
```

公開済みの実行ファイルは [Releases](https://github.com/kz-oshiro/docs-search/releases)から取得できます。新しい版を公開するときは、その版のタグとリリースを作成します。
v2.0.0 と v2.0.1 は移動前の名称 `doc-search-desktop.exe` で公開された版です。v2.0.2 から `docs-search-desktop.exe` です。

## 操作

検索フォルダーを入力または「選択…」で選び、検索語を入れ、用途別の分類から対象拡張子を選んで「検索」を押します。初期状態では従来の5拡張子が選択され、他の形式は必要に応じて追加できます。選択をすべて外すと検索は開始しません。結果は到着順で表示され、一致箇所を黄色で示します。各結果のファイル名はリンクで、クリックすると元ファイルを開けます。拡張子はファイル名の左にタグとして表示し、パスはファイル名の下に表示します。各結果のコピーアイコンを押すと、絶対パスと文書内の場所をコピーできます。読み取りエラーがあれば、進捗表示の下に件数付きの欄が現れ、展開して詳細を確認できます。多数の結果があるときは 200 件ずつ表示し、残りは「さらに表示」で確認できます。結果件数は表示範囲にかかわらず全件です。検索中は「中断」で停止を要求できます。

Office 以外の対応拡張子は UTF-8、UTF-8 BOM、Shift_JIS（Windows-31J 相当）に対応し、保存されたテキストを行単位で検索します。ファイルごとに UTF-8 BOM、UTF-8、Shift_JIS の順で判定するため、異なる文字コードのファイルが同じフォルダーにあっても検索できます。タグやコードの構文解析・実行はしません。読み取れないファイルは個別のエラーとして表示して検索を続けます。暗号化ファイル、壊れたファイル、ZIP/XML の展開上限に達したファイルは検索できません。ZIP の XML 部品は 1 部品 32 MiB、合計 128 MiB、部品数 4096 を上限とします。実ファイルの互換性をすべて保証するものではありません。

## テスト

`docs-search` を作業ディレクトリとして、共通生成データと全 17 ケースを確認します。テストデータは一時ディレクトリに作成し、終了時に削除します。

```powershell
python .\tauri\tests\run-backend-cases.py
cargo test --manifest-path .\tauri\core\Cargo.toml
cargo check --manifest-path .\tauri\frontend\Cargo.toml --target wasm32-unknown-unknown
```

CLI でイベントを JSON Lines として確認できます。

```powershell
.\tauri\core\target\debug\docs-search-cli.exe C:\Documents beacon
.\tauri\core\target\debug\docs-search-cli.exe C:\Documents beacon --extensions js,java
```

GUI はデスクトップ上で実際に起動して、フォルダー選択、検索・中断、ファイル名リンクからの元ファイル起動、各結果からのクリップボードへのコピー、読み取りエラー欄を確認します。バックエンドの共通ケースは GUI の操作性そのものを検証しません。
