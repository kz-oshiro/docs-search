# Excelセル移動の確認方針

[Issue #23](https://github.com/kz-oshiro/docs-search/issues/23)の確認項目を定める。操作と対象範囲は[GUI仕様](../specifications/gui.md#excelで一致箇所を開く)、位置解決と接続方式は[境界仕様](../specifications/boundary.md#excelセル移動の境界)を正本とする。工程の現在地・実行結果・対象コミットはIssueで管理する。

## 自動試験の担当

[試験区分](playwright-exe-test-plan.md#試験区分と振り分けの基本方針)に従い、位置解決はバックエンド、ボタン・要求・応答はフロントエンドで確認する。固定応答を実Excel接続成功へ読み替えない。実Officeは[OS連携の保証対象外](playwright-exe-test-plan.md#5-os連携の保証対象外)に属し、通常CIの必須・skipped件数には含めない。

| 担当・実装 | 固定入力・手順 | 期待結果 |
| --- | --- | --- |
| バックエンド・[excel_target.rs](../../core/tests/excel_target.rs) | 日本語/空白パス・アポストロフィ付きシート名・最大セル、範囲式・小文字・列/行上限超過・位置不明・非Excelを指定 | 構造化した正しい位置だけを採用。表示文字列・文字列再検索で代用せず、不正なセル式をCOMへ渡さない |
| バックエンド・同上 | 行/ファイル条件の2根拠へ番号なし・範囲外・第2根拠を指定。数式・コメント・アンカー有無を比較 | 根拠番号必須、指定した根拠のシートと番地を使う。アンカーなしを任意セルへ移動しない |
| フロントエンド・[U48](../../tests/ui/specs/excel-results.spec.mjs) | 別ヒット・別根拠の操作、位置なし、拒否・遅延・新検索を投入 | IDと根拠番号・位置案内・二重操作防止・古い応答の抑止が正しい |
| 静的確認・[main.rs](../../src-tauri/src/main.rs) | コマンド登録、最新セッションからのID解決、入力の受渡しを読む | 任意のパス/番地をUIから直接受理せず、旧ID・不正な根拠を拒否。Excelの処理中に検索状態のMutexを保持しない |
| 静的確認・[補助処理](../../src-tauri/src/excel_open.ps1) | ROT/既存ブック優先、読み取り専用Open、マクロ/リンク/イベント設定、finally、Gotoと選択位置照合を読む | ユーザーのブックを保存・閉じる・再読込しない。設定を復元し、接続先が不明なら拒否。入力データをスクリプトに展開しない |

自動実行は[開発手順](../development.md#テスト)の `cargo xtask test`。`excel_target` はcore-api-properties工程、U48はフロントエンド工程へ組み込む。実行結果は対象ソースに対応する生成レポートを参照する。

## 実Excelとの互換性確認

COMを公開するWindowsデスクトップ版Excelとxlsx/xlsmを対象とする。Web版Excel・Excel未導入・他の表計算アプリではセル移動できず、通常のファイル名リンクを案内する。Excelの製品版・ビルド・bit数・OSを実施時に記録する。対応を想定することと各版での確認済みを分ける。

この表はOffice連携を実地確認する依頼があったときの担当者向け手順であり、通常CI後にユーザーへ必須確認票として依頼しない。対話的GUI操作は[共通ルール](../development.md#エージェントの作業ルール)の範囲で実施する。各ケースは検証用コピーを使い、個人のブックや設定を変更しない。

| 条件ID | 手順 | 期待結果 |
| --- | --- | --- |
| AC-01/02 | 同語が複数のシート・セルにある検証ブックで、画面外のD900等を順に選ぶ。日本語/空白パス・アポストロフィ付きシートでも行う | 選択したブック・シート・セルがアクティブで画面内に表示される。文字列再検索による別セルへの移動なし |
| AC-03 | 開いたブックで未保存編集を行い、別ヒットから移動。複数Excelインスタンスでも既存対象を開く | 編集内容と保存状態を保持し、既存ブックを使用。接続できない場合は拒否し別コピーを開かない |
| AC-04 | 新規/既存・xlsx/xlsmで移動前後の元ファイルのバイト/mtime・内容・書式・保存状態を比較。検証用のVBAイベント、外部リンクとアプリ設定も確認 | セル移動で保存・内容/書式変更をしない。新規は読み取り専用。リンク更新・VBA/イベントを抑止し、EnableEvents/AutomationSecurityを元へ戻す。Excel 4.0マクロはAutomationSecurityだけで無条件無効化されないためOfficeの警告を回避・承認しない |
| AC-05 | Excel未導入・ブック削除・シート削除/改名・非表示シート/行列・編集中・接続制限を試す | 理由と検索時の位置を案内し、通常起動を利用できる。シートを勝手に表示したり未保存内容を捨てたりしない |
| AC-06 | 結合セル、保護により選択可能/不可のセル、数式、番地付き/なしコメント、図形アンカー有無、行/ファイル条件の複数根拠を操作 | 結合は左上起点、非表示設定を変更しない。選択不可は拒否。根拠ごとのボタンで指定した位置を使い、不明位置を推測しない |

方式の一次資料はMicrosoftの[Workbooks.Open](https://learn.microsoft.com/en-us/office/vba/api/excel.workbooks.open)、[AutomationSecurity](https://learn.microsoft.com/en-us/office/vba/api/excel.application.automationsecurity)、[Application.Goto](https://learn.microsoft.com/en-us/office/vba/api/excel.application.goto)。実地結果なしにOffice互換性を確認済みとしない。
