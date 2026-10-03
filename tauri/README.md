# docs-search Tauri 実装

ソース版: **v3.0.0**。[Rust要求仕様](../docs/rust-requirements-v3.0.0.md)を機能・上限・テスト対応の基準とし、[検証記録](../docs/validation-v3.0.0.md)へ今回の自動確認を記録します。

Windows 向けの文書横断検索アプリです。Rust の検索コアがローカル文書を読み取り、Tauri がデスクトップ画面との境界を担当し、画面の検索状態と操作は Rust からビルドした WebAssembly で動きます。テーマ設定は JavaScript で保存・復元します。Office や PowerShell を検索処理に使用しません。

## 構成

| ディレクトリ | 役割 |
| --- | --- |
| `core/` | Rust 検索コアとイベント契約。CLI からも GUI なしで実行可能 |
| `src-tauri/` | Windows のウィンドウ、フォルダー選択、既定アプリで開く操作、順序を保った検索イベントのバッチ転送 |
| `frontend/` | Rust→WASM の画面制御と HTML/CSS |
| `tests/` | CLIの受け入れランナーと `ui/` のヘッドレスPlaywright試験。共通ケース原本はリポジトリ直下の `tests/backend-cases.json` |

検索対象は Office 文書とテキスト系の計93拡張子から選べます。`.vue`、`.mjs`、`.jsx`、`.tsx`、`.json` などを含む一覧と対象外の形式・箇所は [バックエンド仕様](../docs/backend.md) を参照してください。通常検索は NFC と大文字小文字を吸収した部分一致です。チェックボックスであいまい検索を有効にすると、NFKC・識別子表記・かな種別の違いや限定した英字のタイプミス候補も扱います。検索用索引も別のチェックボックスで作成・利用を選べます。両方とも初期状態はオフです。場所と一致理由付きの結果、個別エラー、進捗、中断を提供します。保存済みの索引は画面から削除できます。隠しシート・隠しスライドを含み、Office 一時ファイルとシンボリックリンクを除きます。検索処理は元文書へ書き込みません。検索終了後の明示した編集・保存は、対応テキスト形式の一致範囲だけを更新します。

## 動作条件とビルド

- Windows 10/11、WebView2、Microsoft C++ Build Tools、Rust 1.90 以降の MSVC ツールチェーン
- Python 3 は共通テストを実行するときだけ必要
- Node.js 20以上とnpmはPlaywright自動UI試験に必要。初回は [UI試験の準備](../docs/playwright-ui-test-plan.md#実装と準備)を行う
- Rust の `wasm32-unknown-unknown` ターゲットと `wasm-bindgen-cli` 0.2.129

初回だけ次を実行します。

```powershell
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
```

以下のビルド・試験・公開コマンドは、`docs-search` のルートを作業ディレクトリとして実行します。画面用ファイルは一時的な `tauri/.frontend-build/` に作られ、デスクトップアプリに取り込まれた後で削除されます。Cargo が生成する `release/` もビルド後に削除します。完成した実行ファイルだけを `tauri/` 直下に残します。実行ファイルとビルド途中の生成物は Git の管理対象外です。`release/` を残さないため、次回のビルドでは依存関係も再コンパイルされます。

```powershell
powershell.exe -NoProfile -ExecutionPolicy RemoteSigned -File .\tauri\Build.ps1
```

ビルド後は `tauri\docs-search-desktop.exe` をダブルクリックして起動します。再ビルドの前には実行中のアプリを閉じてください。アプリ単体の起動ではターミナルを表示しません。`.\tauri\Build.ps1 -Run` は開発用で、実行元の PowerShell ウィンドウが残ります。ビルドには crates.io へのアクセスが必要です。

## ローカル CI と公開

自動テストだけを実行する場合は次を使います。共通・P2/P3・Office/順位/一括・イシュー・索引のCLI試験、Rust単体/公開API試験、WASM型検査、ヘッドレスPlaywrightを含みます。CLI/テストのコンパイルと試験用WASM生成は行い、実アプリ起動・Windows配布ビルドは行いません。`-RankingReport <新規JSONパス>` を付けると固定20クエリの順位記録を同じ実行で保存します。

```powershell
powershell.exe -NoProfile -ExecutionPolicy RemoteSigned -File .\tauri\Run-Automated-Tests.ps1
```

UIだけの入口は `tauri/Run-Ui-Tests.ps1` です。実際のHTML/CSS/JS/WASMをChromiumで操作し、Tauriとクリップボードの境界だけ置き換えます。画像・動画・トレースは無効で、失敗はテキスト記録へ保存します。[UI試験方針とM1〜M5のユーザー確認票](../docs/playwright-ui-test-plan.md)を参照してください。U01〜U39の51件は2026-10-03に成功した。実EXEの接続・OS連携と見た目の手動確認は未実施である。詳細は [v3.0.0検証記録](../docs/validation-v3.0.0.md)。

配布ビルドまで依頼された場合は、次の入口で `Run-Automated-Tests.ps1` の全自動試験（ヘッドレスPlaywrightを含む）に続けて Windows 実行ファイルをビルドします。自動テストの入口を別に繰り返す必要はありません。いずれかが失敗するとそこで停止します。初回は上記のビルドツールとUI試験の準備が必要です。

```powershell
powershell.exe -NoProfile -ExecutionPolicy RemoteSigned -File .\tauri\Run-Local-CI.ps1
```

成功後に残る `tauri\docs-search-desktop.exe` は Git の履歴には含めません。ローカル CI と差分確認を終えたソースを `main` に push してから、そのコミットをタグ付けし、ローカルでビルドした実行ファイルを GitHub Release のアセットとして登録します。版番号は Cargo/Tauri のメタデータと Git タグで管理します。GitHub CLI の認証が必要です。

```powershell
$version = '3.0.0'  # この版の配布ビルド・公開が依頼された場合
git tag "v$version"
git push origin "v$version"
gh release create "v$version" .\tauri\docs-search-desktop.exe --verify-tag --title "docs-search v$version" --notes "Windows x64 実行ファイル"
```

公開済みの実行ファイルは [Releases](https://github.com/kz-oshiro/docs-search/releases)から取得できます。新しい版を公開するときは、その版のタグとリリースを作成します。
v2.0.0 と v2.0.1 は移動前の名称 `doc-search-desktop.exe` で公開された版です。v2.0.2 から `docs-search-desktop.exe` です。

## 操作

画面右上の「設定」からテーマカラーをサンオレンジ（公式）・アンバー・夕焼け・青緑・ブルー・フォレストの6色へ切り替えられます。初期色はサンオレンジです。変更はすぐに反映し、次回起動時にも同じ色を使います。以前に保存した有効な色も引き継ぎます。保存できない場合は設定内に表示し、その起動中だけ適用します。配色の変更で検索条件や結果は変わりません。アプリアイコンはサンオレンジの白い書類＋茶色の虫眼鏡で固定します。

公式アイコンの編集元は `src-tauri/icons/icon.svg` です。形を更新したら、リポジトリ直下で `python .\tauri\scripts\make-icon.py` を実行して `icon.ico`（16・20・24・32・48・64・128・256px）と `icon.png`（256px）を生成します。生成に追加の Python パッケージは不要です。このスクリプトは現在のロゴに使う矩形・円・丸い線・多角形だけを扱います。アプリへの取り込みは次回の Windows ビルドで行います。

検索フォルダー欄の「追加」で複数のフォルダーを指定できます。対象外フォルダー欄も「追加」で指定でき、その配下は検索しません。追加した欄は「削除」で外せます。検索フォルダー同士が重なる場合は同じファイルを1回だけ処理し、対象外の指定を優先します。

検索フォルダーを入力または「参照…」で選び、検索語を入れ、用途別の分類から対象拡張子を選んで「検索」を押します。拡張子は全体または分類ごとにまとめて選択・解除でき、選択数も表示されます。初期状態では対応する93拡張子がすべて選択されています。選択をすべて外すと検索は開始しません。結果は親フォルダーとファイルの2階層で折りたためます。ファイルに更新日時を示し、検索中から一致箇所を文書位置順に表示します。完了・中断時にはファイルをコアの一致品質・語数・同じ行・本文等の順位へ並べ、理由を表示します。各一致箇所に「一致検索」（通常検索でも見つかる）または「あいまい検索」（オンにしたときだけ見つかる）と一致理由を示し、原文の一致箇所を黄色で強調します。ファイル名はリンクで、クリックすると元ファイルを開けます。拡張子はファイル名の左にタグとして表示し、パスはファイル名の下に表示します。各一致箇所のコピーアイコンを押すと、絶対パスと文書内の場所をコピーできます。読み取り・列挙エラーがあれば、進捗表示の下に赤系の件数付き欄が現れ、展開して種別・パス・理由を確認できます。一致箇所のあるファイルで読み取りエラーも起きた場合、そのファイルの結果は赤系の色と「エラーあり」の表示で区別します。多数のファイルがあるときは 200 ファイルずつ表示し、残りは「さらに表示」で確認できます。結果件数は表示範囲にかかわらず一致箇所の全件です。検索中は「中断」で停止を要求できます。

検索結果の上にある欄では、ファイル名・パスの文字列と拡張子、本文に含む語句・含まない語句で一致箇所を絞り込めます。両方を指定すると両条件に合うファイルを表示します。絞り込み後の件数と全件数は別に示され、「絞り込みを解除」で全件に戻せます。新しい検索を始めると絞り込みは解除されます。

検索終了後は「現在の絞り込み結果」または「全結果」を選び、TSVの一括コピー、CSV/TSV保存、JSON調査記録の保存ができます。画面にまだ描画されていない結果も対象です。CSVはUTF-8 BOM付き、TSVはUTF-8で保存します。JSONには検索条件・終了状態・全体集計・Issue一覧と選択された結果を含みます。検索中は出力操作を使えません。保存先の選択を取り消しても検索結果は残ります。

検索単位には部品・Excel物理行・セル座標などの内部識別情報を付け、索引にも保存します。保存済みの旧形式の索引は次に索引を有効にした検索で元文書から再構築します。通常検索の意味は維持します。

「検索方法」で高度な検索を選ぶと、「すべて含む」「いずれか含む」「含まない」を改行区切りで入力できます。判定範囲は検索箇所、Excelの同じ行、ファイル全体から選びます。同じ行では別セルの語句をまとめて1件とし、各語句の根拠を結果内に表示します。除外語は同じ範囲全体から探します。通常検索欄は従来どおり1語句として扱います。

Excel のセル一致とアンカー付き図形一致には「周辺を表示」が付きます。開くと一致位置を含む最大5行×5列の保存済みセル値を表で示し、「前の列」「次の列」で横へ移動できます。結合範囲と起点、非表示の行・列を表示します。周辺情報は開いた時に取得し、元ファイルが検索後に変わった場合はその結果内で再検索を促します。アンカーのない図形には取得できない理由を表示します。

Office 以外の対応拡張子は UTF-8、UTF-8 BOM、Shift_JIS（Windows-31J 相当）に対応し、保存されたテキストを行単位で検索します。ファイルごとに UTF-8 BOM、UTF-8、Shift_JIS の順で判定するため、異なる文字コードのファイルが同じフォルダーにあっても検索できます。タグやコードの構文解析・実行はしません。読み取れないファイルは個別のエラーとして表示して検索を続けます。暗号化ファイル、壊れたファイル、ZIP/XML の展開上限に達したファイルは検索できません。ZIP の XML 部品は 1 部品 32 MiB、合計 128 MiB、部品数 4096 を上限とします。実ファイルの互換性をすべて保証するものではありません。

「注記も検索」でExcelコメント、PowerPointノート、Wordヘッダー・フッター・コメントを追加し、「数式文字列も検索」でExcelの保存式を追加できます。どちらも初期状態ではオフです。式は保存値と別の一致箇所として表示し、再計算しません。

「一括検索」では1行1語で最大256語を検索します。初期値の識別子一致は `TAB_01` と `TAB_010` を区別します。日本語等は通常の文字列検索へ切り替えます。終了後は語別件数・確認状態・詳細と語×ファイル表を表示し、行列全体をCSV/TSVへ保存できます。中断やエラーを含む0件は未確定として扱います。

## テスト

検索コアにはテスト専用のproptestを導入しています。[確認方針](../docs/property-testing-test-plan.md)に、生成する入力・期待する性質・件数/seedの指定・失敗入力の保存と再実行を記載しています。既定は17性質それぞれ256ケースで、通常の`cargo test`と自動テスト入口に含まれます。固定例とOffice/CLIの受け入れケースも維持します。

追加機能の順序と現状は [実装計画](../docs/implementation-plan.md) と [進捗レポート](../docs/implementation-status.md) に記録します。以下は段階ごとの確認方針です。

一括出力の確認項目・手順・期待結果は [検索結果の一括出力の確認方針](../docs/result-export-test-plan.md) に記載しています。
テーマ切り替え・保存・表示の確認項目と未実施範囲は [テーマ設定の確認方針](../docs/theme-settings-test-plan.md) に記載しています。
検索単位と索引の共通基盤は [P0の確認方針](../docs/search-foundation-test-plan.md) に記載しています。
Excel 周辺セルの確認項目・手順・期待結果は [P2の確認方針](../docs/excel-context-test-plan.md) に記載しています。
高度な検索の確認項目・手順・期待結果は [P3の確認方針](../docs/condition-search-test-plan.md) に記載しています。
Officeの追加範囲、コア順位、一括検索は [P4〜P6の確認方針](../docs/office-ranking-batch-test-plan.md) と [実装報告](../docs/implementation-report-p4-p6.md) に記載しています。実装報告の未実施項目は報告時点の記録で、現在の自動検証状況は [v3.0.0検証記録](../docs/validation-v3.0.0.md)を参照してください。

全自動確認には上記の `Run-Automated-Tests.ps1` を使います。個別に原因を調べる場合は、`docs-search` を作業ディレクトリとして次を実行できます。CLIのテストデータは一時ディレクトリに作成し、終了時に削除します。UIの生成物と失敗記録は `outputs/ui-tests/` に残します。

```powershell
python .\tauri\tests\run-backend-cases.py
python .\tauri\tests\run-context-cases.py
python .\tauri\tests\run-condition-cases.py
python .\tauri\tests\run-office-search-cases.py
python .\tauri\tests\run-issue-cases.py
python .\tauri\tests\run-index-cases.py
cargo test --locked --manifest-path .\tauri\core\Cargo.toml
cargo check --locked --manifest-path .\tauri\frontend\Cargo.toml --target wasm32-unknown-unknown
powershell.exe -NoProfile -ExecutionPolicy RemoteSigned -File .\tauri\Run-Ui-Tests.ps1
```

CLI でイベントを JSON Lines として確認できます。

```powershell
.\tauri\core\target\debug\docs-search-cli.exe C:\Documents beacon
.\tauri\core\target\debug\docs-search-cli.exe C:\Documents beacon --extensions js,java
```

画面の操作・状態・要求はPlaywrightで自動確認し、実EXEの接続・フォルダー/保存先選択・元ファイル起動・実クリップボード・編集保存・見た目は [限定したユーザー確認票](../docs/playwright-ui-test-plan.md#ユーザー確認票実アプリの最終5項目)で確認します。ブラウザーUI試験の成功を実Tauri/WebView2の成功と扱わないでください。

## 既存イシューの一括対応

2026-10-03に #1〜#9 の折りたたみ・本文フィルター・一致箇所編集・更新日時・フォルダー復元・位置順・Excel書式/可視文字の対応を追加しました。[実装報告](../docs/implementation-report-issues.md)と[確認方針](../docs/issues-test-plan.md)を参照してください。現在の自動確認は [v3.0.0検証記録](../docs/validation-v3.0.0.md)に集約し、GUI・配布ビルド・公開とは区別します。
