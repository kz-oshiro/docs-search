# P4〜P6: Office検索範囲・順位・一括検索の確認方針

現行の実行入口は[開発手順](../development.md)、機能基準は[Rust要求仕様](../specifications/rust-requirements.md)、公開版の検証結果は[GitHub Releases](https://github.com/kz-oshiro/docs-search/releases)を参照する。試験設計は同じ文書を更新し、改訂履歴をGitで管理する。

試験の担当と保証範囲は[試験区分と振り分け](playwright-exe-test-plan.md#試験区分と振り分けの基本方針)と[OS連携の保証対象外](playwright-exe-test-plan.md#5-os連携の保証対象外)、実行依頼の範囲は[エージェントの作業ルール](../development.md#エージェントの作業ルール)に従う。以下のGUI手順・未実施・許可の記述は旧方針の参考記録として扱う。

作成日: 2026-10-03。対象: [Issue](https://github.com/kz-oshiro/docs-search/issues/13)のP4a・P4b・P5・P6、要求A-16〜A-18。
自動テストは `cargo xtask test` を使う。要求対応は[Rust要求仕様](../specifications/rust-requirements.md)、実行結果の保存・公開は[開発手順](../development.md#記録の自動生成と公開)を参照。初回実装時点の状態は案件Issueへ分ける。
実装内容と制限は[Issue](https://github.com/kz-oshiro/docs-search/issues/13)を先に読む。

## 次の担当モデルの実行順

1. `AGENTS.md`、本方針、案件Issue、既存P0〜P3の確認方針を読む。既存のチェックアウト差分を保存・確認する。
2. 自動テストのみの依頼では、ルートから次を実行する。UI資材stagingとヘッドレスPlaywrightを含む。

```powershell
cargo xtask test
```

Windows配布ビルドまで依頼された場合は、上の入口に代えて `cargo xtask ci` を使う。順位記録も保存する場合は、次の3の入口と配布ビルド手順を使う。

自動入口は共通load生成、Rust単体/API/proptest、生成器検証、共通27、P2/P3、P4〜P6、イシュー、索引、ヘッドレスUIの順。ローカルCIはその後にWindowsビルドを加える。配布ビルド前に既存アプリが起動している場合は終了する。P4〜P6は `test-support/src/specialized.rs` と `core/tests/cli_office.rs` を使う。保存先が既存なら生成器は拒否し、共通fixtureの件数・期待値は変更しない。

3. P5の固定20クエリについて結果と根拠の保存も依頼された場合、2の自動テスト入口に代えて次を使う。保存先は既存ファイルを上書きしない新しいパスを使う。

```powershell
cargo xtask test
```

この入口は自動一式の同じ実行内で順位記録を保存する。配布ビルドも依頼された場合は、成功後に `cargo xtask build` を実行し、成功した自動一式をローカルCIで繰り返さない。既存スクリプトを書き換える必要はない。成功したテスト・未実施の性能/実アプリGUI・検出した不一致を分けて報告する。

## 自動確認の項目・手順・期待結果

| 項目 | 手順 | 期待結果 |
| --- | --- | --- |
| 従来の互換性 | 共通27ケースとP0〜P3を実行。新フラグは未指定 | 既存結果・Issue・終端集計を維持。通常欄のAND等は1語句 |
| Excel数式・保存値 | FORMULA_SIGNAL、SHARED_SIGNAL、FORMULA_WITHOUT_CACHEと42を検索。数式フラグをオン/オフ | 式はformula、保存値はcell。式だけのセルも検索でき、共有式は基点だけ。従属式の式を再構成しない。xlsmも同じ |
| Excel行の数式 | 同じ行のTAB_01とFORMULA_SIGNALでANDを指定 | セルと有効な数式Unitが同じ行で成立し、元のセル番地と内容区分を根拠に返す |
| PPTノート | NOTE_SIGNAL、IGNORED_SIGNAL、ORPHAN_SIGNALを注記オン/オフで検索 | ノート本文だけが1件。日時/ヘッダー/フッター/番号/スライド画像プレースホルダーと未参照部品は除く。スライド・図形ID・段落が分かる |
| Excelコメント・VML | 従来・スレッド親・返信とVML_SIGNALを検索 | 種別とシート/番地/コメントIDが分かる。返信は親の番地。注記オン時は同番地のVMLコメントを二重計上しない。通常テキストボックスは維持 |
| Word注記 | HEADER_SIGNAL、HEADER_TABLE_SIGNAL、FOOTER_SIGNAL、WORD_COMMENT_SIGNALを検索 | 種類・部品・段落/表経路・コメントIDを保持。2セクションの共有部品でも1回のみ |
| 注記部分障害 | malformed/missing/external/invalid-utf8/oversized/word-errorでBODY_SIGNALを検索 | 本文1件と部品Issue1件を保持。不完全なUnitをSQLiteに保存しない。同じ障害を重複通知しない。外部ネットワークを取得しない |
| 不完全な条件範囲 | 上記障害でfile範囲の正条件＋NOTを検索 | ファイル範囲は成立しない。本文のunit/行・通常/一括の通知済み根拠は保持 |
| 索引 | 4通りの注記/数式フラグ、直接/初回索引/再利用で検索 | 件数・種別・場所・一致分類・順位が同じ。範囲変更時は再抽出。同範囲では再利用。部分障害は再利用しない |
| 順位20クエリ | ranking-cases.jsonの各ペアを直接/初回/再利用で実行 | 指定した優先ペアが逆転しない。弱いAND、正の語の充足、同じ行、本文対名前/注記、反復上限、かな/識別子/タイプミス、パスとA2/A10の順を確認 |
| 語句の入力 | 空行・TAB_01/tab_01・256/257語、日本語を入力 | 空行/重複を除き入力順と最初の表記を維持。256語以内のみ受理。識別子で扱えない行は文字列検索へ案内。開始前の件数はコアと同じ |
| 識別子境界 | TAB_01/TAB_010を識別子一致・文字列検索で検索 | 識別子一致では相互の部分一致なし。全角/記号/タイプミス拡張なし。文字列検索は既存の部分一致・あいまい検索規則を使う |
| 語別件数 | 同じ場所で2語が一致する文書を検索 | 1語×1Unitで最大1Result。全resultCountは語別hitCountの合計。fileCountは語ごとの重複なし。別語が同じ場所なら2Result |
| 共有抽出 | 同じ3文書を1語と256語で直接検索。ExecutionSummaryを比較 | extractedFilesはともに3、reusedFiles=0。索引再利用は抽出0/再利用3。フォルダー列挙も1回の共通処理 |
| 中断・不確定 | cancel-on-start、壊れた注記、該当0語を指定 | BatchSummaryはFinishedより前、Finishedは最後に1回。中断・エラーの0件は未確定。正常完了・エラーなしの0件だけ指定範囲内で該当なし |
| 出力 | RustのreportテストとGUI確認で全件/絞り込み・CSV/TSV/JSON/語×ファイルを比較 | 語ID、場所、注記/数式区分、順位理由を保持。未描画の結果も対象。表の引用・改行・数式先頭文字は無害化。JSONは検索全体の順位・語別集計・抽出統計も保持 |

20クエリの `oldTop5` は同じ新しいヒット集合に旧GUIの並べ替え規則を適用した比較であり、旧実行ファイルを起動した実測ではない。`newTop5` とevidenceを保存し、ペア逆転・改善/悪化を区別する。

追加の索引確認: fixtureを新規出力し、同範囲で3回検索後にファイル変更/削除、抽出仕様版2のDB、破損DBを用意して再検索する。サイズ・mtime/仕様版・範囲が違えば再抽出し、DB破損時は直接検索へ戻る。既存の索引確認手順も参照する。

## GUI確認（明示依頼または事前確認が必要）

自動確認とWindowsビルドが成功しても、画面操作の確認完了とは扱わない。実施する場合は新規保存先に専用fixtureを生成し、ビルドした実行ファイルを使用する。

専用fixtureは抽出に必要な部品を中心にした最小のパッケージであり、Officeアプリで開けることの確認用ではない。元文書の起動・Officeとの実互換性には、既存の共通fixtureや実際のOfficeで保存した文書も使用する。

1. 通常/高度/一括を切り替え、初期値（注記・数式・索引・あいまいはオフ、識別子一致）と入力行エラー/語数を確認する。
2. 注記・数式結果の場所表示、根拠の区分、コピー、数式からの周辺表を確認する。周辺表は保存値を示し、再計算しない。
3. 検索中は到着順、完了・中断後はコア順位・理由を確認。語別詳細を開いて50件送り、語×ファイル表の20×20送りを確認する。
4. 200ファイルを超える結果で、未描画の結果・語別詳細・ファイル絞り込み・全件/絞り込み出力の整合を確認する。
5. 保存/コピー/取消し、壊れた注記、取消し後の0件、次の検索へ切り替えた古い通知・周辺応答の破棄を確認する。

## 性能確認

1・32・256語、索引なし/初回/再利用、文書数/サイズ別の比較は[バックエンド性能の確認方針](backend-performance-test-plan.md#性能比較の手順期待結果)に集約する。通常/識別子の一致規則、順位、語別集計の期待値は本方針を維持する。速度・最大メモリ・中断応答の定量基準は計測前の基準取得で定める。

## 抽出仕様の参照

- [Microsoft: CellFormula](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.spreadsheet.cellformula?view=openxml-3.0.1)
- [Microsoft: Notes slides](https://learn.microsoft.com/en-us/office/open-xml/presentation/working-with-notes-slides)
- [Microsoft: PlaceholderValues](https://learn.microsoft.com/ja-jp/dotnet/api/documentformat.openxml.presentation.placeholdervalues?view=openxml-3.0.1)
