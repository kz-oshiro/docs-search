# v3.0.0 自動検証記録

実施日: 2026-10-03。対象: [Rust要求仕様 v3.0.0](https://github.com/kz-oshiro/docs-search/blob/v3.0.0/docs/rust-requirements-v3.0.0.md)。実アプリ起動/GUIは今回の依頼対象外。ヘッドレスPlaywrightは自動試験に含めた。

公開済み: **[v3.0.0](https://github.com/kz-oshiro/docs-search/releases/tag/v3.0.0)**。ソースコミット`f96af8605840306a50c76c34356b13ca3e62bf1a`。正式Releaseとアップロード済みEXEのサイズ・SHA-256を確認した。公開後の文書更新には実行ソースの変更を含めていない。

最新追記: 2026-10-03の再実行で **Rust64件（17性質×各1,024ケース）、CLI受け入れ、WASM型検査、Playwright 51件、Windows Releaseビルドが成功**。固定seedは3000002。U31で見つけた一括入力プレビューの重複要求も修正・再確認した。実施記録と成果物のSHA-256は末尾の「最新の自動テスト・Windows配布ビルド」を参照する。

公開前レビュー: 上記の生ログとPlaywright JSON、EXEのサイズ・SHA-256、Cargo/Tauriの版番号、U31修正とTauri境界・編集/一括検索/順位の関連実装を照合した。Playwrightは成功51・スキップ0・失敗0・flaky0。ビルド後に実行ソースの変更はなく、公開を止める不備は見つからなかった。READMEの未実行記述を修正し、[v3.0.0リリースノート](release-notes-v3.0.0.md)を追加した。実行ソースを変更していないため、レビューだけによるテスト・ビルドの再実行は行っていない。

要求をR3-01〜R3-20へ整理し、Cargo 3パッケージ・対応lock・Tauri設定を3.0.0へ更新した。既存の作業ツリーの追加機能を含め、要求に基づく公開API受け入れ17件と索引CLI8項目を追加して実行した。

## 初回の実行と結果（proptest導入前）

環境: Windows、Windows PowerShell 5.1、Python 3.12.10、rustc 1.91.1、cargo 1.91.1。RustのCLI/テスト用コンパイルとWASM型検査を実施した。

```powershell
powershell.exe -NoProfile -ExecutionPolicy RemoteSigned -File .\tauri\Run-Automated-Tests.ps1 -RankingReport .\outputs\v3-check-20261003-111605\ranking-comparison-final.json
```

最終実行は終了コード0。ログは [automated-tests-final.log](../../outputs/v3-check-20261003-111605/automated-tests-final.log)、固定20クエリの順位・根拠は [ranking-comparison-final.json](../../outputs/v3-check-20261003-111605/ranking-comparison-final.json) に保存した。outputsはGit管理対象外で、この記録は当該ローカル成果物への参照である。

| 確認 | 結果・対応 |
| --- | --- |
| 共通生成器 | 2テスト成功。共有fixtureの決定性・内容を維持 |
| 共通バックエンド | 27ケース成功。既存形式、場所、件数、個別エラー、入力拒否、複数ルート、索引/あいまい、中断 |
| Excel周辺CLI | 5項目成功。直接/索引初回/再利用、端/図形/アンカーなし、変更/削除検知 |
| 条件CLI | 31項目成功。範囲、AND/OR/NOT、根拠、索引/あいまいの組合せ、入力/読取失敗 |
| Office追加範囲/一括 | 専用ランナー成功。注記/数式の有効化、重複除去、本文を保持する部分障害、抽出範囲/索引、256語/識別子境界/語別集計/不確定/共有抽出 |
| 順位 | 固定20クエリすべての優先ペア成功、直接/初回/再利用で一致。oldTop5は新ヒット集合への旧規則適用であり、旧exeの実測ではない |
| ExcelイシューCLI | 4項目成功。直接/初回/再利用の可視文字・ブック/数値位置順・固定mtime、抽出版3→4の索引更新と検索ルート外の旧記録除去 |
| 新規索引CLI | 8項目成功。オフ時未作成、初回/再利用、オフ時既存DB不変＋現文書を直接読む、mtime/サイズ変更、削除掃除、破損DB、旧schema、保存先利用不可からの直接抽出 |
| Rust単体 | 29件成功。抽出/Unicode/条件/索引/順位/出力/周辺/Windows上のテキスト編集 |
| 新規公開API | 17件成功。デフォルト/開始前拒否、重複・除外、89テキスト形式、通常語句、Unicode元範囲/省略、あいまい独立、入力上限、一括集計、中断とイベント整合、個別障害/サイズ上限、位置順/反復上限、読取専用/mtime |
| WASM | `cargo check --locked --target wasm32-unknown-unknown` 成功。画面Rustの型検査であり、画面操作の成功は示さない |
| 静的確認 | Rust 3パッケージのrustfmt、Python構文、PowerShell構文、版番号のmanifest/lock/config対応、24文書のローカルリンク、差分の空白を確認 |

Rustは計46件成功、失敗/無視0件。共通の27ケースの期待値は変更していない。各CLIが個別の一時LOCALAPPDATAを使い、利用者の索引へテストデータを保存しない。CLI/テスト以外のデスクトップ実行ファイルは更新していない。

## 検出した問題と修正

1. 新規PowerShell入口の日本語をWindows PowerShell 5.1が誤読した。新規入口と更新したCIスクリプトをUTF-8 BOM付きへ揃え、構文確認と最終一式実行で成功を確認した。
2. Office専用ランナーが全assertion成功後、SQLite接続を閉じず一時ディレクトリの削除に失敗した。Pythonの接続コンテキストはトランザクション終了だけなので、Office/イシュー/新規索引ランナーへ`contextlib.closing`を追加した。修正後は各ランナーの終了・削除と一式実行が成功した。
3. 既存編集テストの要求に`recursive=false`があり、現行の開始前検証で拒否されることを静的確認で発見した。テスト入力を仕様の`true`へ修正し、編集5件の実行成功を確認した。製品の再帰検索仕様は変更していない。

初回失敗時の一時データ `C:\Users\kazum\AppData\Local\Temp\docs-search-office-9njdswpl` は残っている。後片付けの再帰削除は自動承認レビューでポリシーにより拒否された。最終成功実行が作った一時データは各ランナーで正常に削除され、初回の残存データを再利用していない。

## 初回の自動検証時点で確認していなかった範囲

以下はこの章の初回およびproptest導入後の自動検証を終えた時点で未実施だった範囲である。現在の確認結果は末尾の最新記録を参照する。

アプリ起動、GUI、性能測定、旧PowerShell実装の回帰、コミット・push・タグ・Release・GitHubイシュー操作は実施していない。

GUI除外には、折りたたみ/本文フィルターの操作、時刻の見た目、フォルダー/テーマ復元、周辺書式の表示、編集ダイアログと最新セッションtoken、クリップボード/保存先選択/既定アプリ起動、アイコンを含む。Rust編集試験はWindowsの実ファイル置換経路を通ったが、全てのOS権限/共有ロック/置換途中障害の組合せまでは検証していない。実Office文書の網羅的互換性、同じサイズ/mtimeでの外部更新検知、内部障害を注入したfailed終端の保証は今回の結果から断定しない。

この時点ではソース版3.0.0の配布/公開は未実施だった。以後の実行履歴は下記へ追記する。

## proptest導入後の検証

同日の追加依頼に対応し、[確認方針](../test-plans/property-testing-test-plan.md)に沿ってproptest 1.11.0を検索コアのdev-dependencyへ導入した。std/handle-panicsだけを有効にし、通常依存の`cargo tree --edges normal`にproptestがないことを確認した。アプリの通常依存と配布物へテストライブラリを追加したものではない。版番号は3.0.0を維持する。

| 実施 | 結果 |
| --- | --- |
| 初回の生成試験 | 14性質中13成功、候補漏れの1性質が失敗。縮小結果は本文`a_`、検索語`a!` |
| 不一致の修正 | 直接の候補判定に単一token/前方一致、索引にtoken候補の合流を追加。保存gramから落ちる区切りだけの語も直接評価へ回す。最終の照合方式・点数と抽出schema/仕様版は維持 |
| 固定回帰 | `fuzzy.rs`に最小例と前方一致の例を追加。索引CLIへ`a!`/`alpha!`/`__`の直接/初回/再利用比較を追加 |
| 保存seed再実行＋固定seed 3000001 | コアの15性質×1,024ケース（15,360新規ケース）が成功。後から追加した出力2性質はこの実行には含めない |
| 最終一式、固定seed 3000002 | Rust単体/性質47件＋公開API17件＝64件成功。17性質それぞれ1,024ケース（17,408新規ケース）、保存seedも別途再実行。失敗/無視0件 |
| 既存受け入れ | 生成器2、共通27、周辺5、条件31、Office/一括、順位20、Excelイシュー4が成功。共通期待値は変更していない |
| 索引CLI | 従来8項目＋候補漏れの直接/索引比較1項目＝9項目が成功 |
| WASM/静的確認 | WASM型検査、rustfmt、Python構文、文書リンク、差分空白と失敗seedがGit除外されないことを確認 |

最終一式の実行コマンド:

```powershell
$env:PROPTEST_CASES = '1024'
$env:PROPTEST_RNG_SEED = '3000002'
powershell.exe -NoProfile -ExecutionPolicy RemoteSigned -File .\tauri\Run-Automated-Tests.ps1 -RankingReport .\outputs\proptest-check-20261003-113218\ranking-comparison.json
```

終了コード0。[最終ログ](../../outputs/proptest-check-20261003-113218/automated-tests-seed-3000002.log)、[先行15性質のログ](../../outputs/proptest-check-20261003-113218/properties-seed-3000001.log)、[初回失敗ログ](../../outputs/proptest-check-20261003-113218/properties-first.log)、[順位記録](../../outputs/proptest-check-20261003-113218/ranking-comparison.json)、[通常依存一覧](../../outputs/proptest-check-20261003-113218/normal-dependencies.txt)を保存した。

縮小したseedは [property_tests.txt](../../core/proptest-regressions/property_tests.txt)へ保存し、除外せず回帰データとして残す。固定例・生成試験・CLIの3段階で候補漏れを検証した。seed/生成件数は実行プロセス内だけに指定し、次回の通常実行は既定の256ケース/性質を使える。

このproptest導入後の実行時点ではGUI、Windows配布ビルド、公開/リモート反映、性能・実Office互換性の追加評価は対象外だった。17,408ケースの成功は生成した範囲内の検証であり、全入力の証明ではない。

## 2026-10-03 最新の自動テスト・Windows配布ビルド

U31の初回Playwright確認では、一括入力1回から同一引数の`preview_batch`が3回送信され、テストモックの1回限りの応答後に空応答が画面件数を0へ上書きした。複数イベントを短時間にまとめる120msの遅延プレビューへ修正し、U30/U31で期待件数と要求1回を確認した。その後、固定seedで自動テスト一式を再実行して全件成功した。

実行コマンド:

```powershell
$env:PROPTEST_CASES = '1024'
$env:PROPTEST_RNG_SEED = '3000002'
powershell.exe -NoProfile -ExecutionPolicy RemoteSigned -File .\tauri\Run-Automated-Tests.ps1 -RankingReport .\outputs\v3-rust-verification-20261003-164727-98e0872d\ranking-comparison.json
powershell.exe -NoProfile -ExecutionPolicy RemoteSigned -File .\tauri\Build.ps1
```

| 確認 | 結果 |
| --- | --- |
| 共有fixture生成器 | 2テスト成功 |
| 共通バックエンド | 27ケース成功 |
| Excel周辺 | 5項目成功 |
| 条件検索 | 31項目成功 |
| Office追加範囲・順位・一括 | CLI成功、固定20クエリの順位ペア成功 |
| Excelイシュー | 4項目成功 |
| 索引 | 9項目成功 |
| Rustコア | 47テスト成功。17性質を各1,024ケース（合計17,408ケース）、固定seed 3000002で実行 |
| 公開API受け入れ | 17テスト成功。Rust全体64テスト、失敗/無視0 |
| WASM型検査 | 成功 |
| ヘッドレスPlaywright | 51件成功。実WASM画面を操作し、画像・動画・トレースを使用せずDOM/状態/要求で判定 |
| rustfmt | core / frontend / src-tauriの`cargo fmt --check`成功 |
| Windows Releaseビルド | 成功。`tauri/docs-search-desktop.exe`、13,120,512 bytes、SHA-256 `3BD1F22CCC818EF9EF1A532E141E9B6B4D2B655B0C3E7DD8854C7CB859714EEE` |

自動テストログは [automated-tests.log](../../outputs/v3-rust-verification-20261003-164727-98e0872d/automated-tests.log)、順位比較は [ranking-comparison.json](../../outputs/v3-rust-verification-20261003-164727-98e0872d/ranking-comparison.json)、ビルドログは [windows-build.log](../../outputs/v3-rust-verification-20261003-164727-98e0872d/windows-build.log) に保存した。Playwrightの生成物は `outputs/ui-tests/20261003-164805-bfb96c4ad6eb4169b16050187a4b56ac/` にある。これらの`outputs/`記録はGit除外のローカル成果物である。

このテスト・ビルドを終えた時点では実アプリ起動・GUI・性能測定・旧PowerShell実装の回帰・コミット/push/タグ/Release/GitHubイシュー操作は実施していなかった。ビルド成功はアプリ起動やOS連携の確認を意味しない。

## 2026-10-03 レビューと公開

公開前レビューで生ログ・コード・版番号・配布EXEを照合し、公開を止める不備は見つからなかった。READMEの未実行記述とPowerShell移行状況を更新した。実行ソースとテスト内容は上記の最終成功時から変更していない。

- ソースコミット`f96af8605840306a50c76c34356b13ca3e62bf1a`をmainへpushし、同じコミットを指す注釈付きタグ`v3.0.0`をpushした。
- 2026-10-03 17:09:17 JSTに [GitHub Release](https://github.com/kz-oshiro/docs-search/releases/tag/v3.0.0)を公開した。draft=false、prerelease=false、Latest=v3.0.0をAPIで確認した。
- 公開アセット`docs-search-desktop.exe`（asset ID `607423986`）はuploaded、13,120,512 bytes。GitHubのdigest `sha256:3bd1f22ccc818ef9ef1a532e141e9b6b4d2b655b0c3e7dd8854c7cb859714eee`がローカルSHA-256と一致した。
- remote mainとタグの対象コミット、`origin/main...HEAD`のahead/behind `0 0`、クリーンな作業ツリーを確認した。公開後の進捗文書更新は別コミットで反映する。

実アプリ起動・対話的GUI・OS連携・見た目、性能と実Office互換性の追加評価、イシュー完了操作は引き続き未実施。PowerShell版は別リポジトリでv1.0.0を公開済みである。
