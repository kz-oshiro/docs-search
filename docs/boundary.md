# docs-search 再構築版: GUI とバックエンドの境界契約

v3.0.0の境界契約。要求IDと自動テストの範囲は [Rust要求仕様](rust-requirements-v3.0.0.md) を参照。

この契約はデータと振る舞いの意味を定める。言語、ライブラリ、IPC、HTTP、スレッド、プロセス数、シリアライズ形式は定めない。別の GUI または検索実装に差し替えても、以下の契約を満たすこと。

## 責務の境界

| GUI が行うこと | バックエンドが行うこと |
| --- | --- |
| 入力、フォルダー選択、入力エラーの提示 | 入力の最終検証 |
| 結果・進捗・エラー・終了状態の表示 | ファイル列挙、形式別抽出、照合、件数集計 |
| 元ファイルを既定アプリで開く | 元ファイルの絶対パスと場所情報を渡す |
| コピー用文字列の整形とクリップボード操作 | 場所を構造化データで渡す |
| 現在の検索 ID の通知だけを採用する | 検索 ID と通知順序を付けて送る |

GUI は文書形式の内部構造を解析しない。バックエンドは画面部品、既定アプリ、クリップボードを操作しない。

検索結果の一括コピー・保存は、終端通知後にGUIが保持している `SearchHit`・`SearchIssue`・検索条件・集計から作る。出力対象は全結果またはGUIの現在の絞り込み結果とし、画面に描画済みかどうかには依存しない。表形式の変換は共通コード、保存先選択と書き込みはデスクトップ層、クリップボード操作はGUIが担当する。検索通知とその件数の意味は変えない。

## 操作

| 操作 | 入力 | 応答 |
| --- | --- | --- |
| `startSearch` | `SearchRequest` | 受理時は検索 ID。拒否時は項目別の入力エラーで、検索は生成しない。 |
| `cancelSearch` | 検索 ID | 中断要求を受理する。同じ ID への再要求は安全。終了済み ID への要求は状態を変えない。 |
| 通知の購読 | 検索 ID | 受理済み検索のイベントを順次受け取る。開始前に受信準備を整えられること。 |

`SearchRequest` は `rootDirectory`（最初の検索フォルダーのパス）、`query`（検索語）、`recursive`（現行版では真）を持つ。`additionalDirectories`（追加の検索フォルダー）と `excludedDirectories`（対象外フォルダー）は省略可能な配列で、未指定なら空配列とする。対象外フォルダーは指定された検索フォルダーより優先し、その配下も列挙しない。検索フォルダー同士が重なる場合は同じパスのファイルを1回だけ処理する。指定したフォルダーが存在しない場合は、対応する入力項目のエラーで検索を拒否する。

`extensions`（検索対象の拡張子の配列）も指定できる。要素は先頭のドットを付けない小文字の拡張子で、[バックエンド仕様](./backend.md) の対応一覧から選ぶ。空配列や未対応の値は入力エラーとし、未指定なら従来の5種類（`xlsx`、`xlsm`、`pptx`、`docx`、`txt`）を選ぶ。GUI から形式ごとの実装名や解析設定を渡さない。

`useIndex` と `fuzzySearch` は省略可能な真偽値で、省略時はともに偽とする。`useIndex` が真ならローカル索引を作成・再利用し、偽なら保存済みの索引も読み書きしない。`fuzzySearch` が偽なら NFC と Unicode case folding による部分一致、真なら[バックエンド仕様](./backend.md)のあいまい検索を使う。2つの指定は独立している。

`querySpec` を省略または `null` とした通常検索では、`query` を空白や AND / OR という文字を含む1語句として扱う。`querySpec` を使う高度な検索では `query` を空にし、`{"mode":"conditions","scope":"unit|excelRow|file","all":[...],"any":[...],"not":[...]}` を渡す。`all`・`any`・`not` は語句の配列で、各要素の前後空白を除き、空要素と同じ群内の正規化後の重複を除く。両方の正の群が空、`all` と `not` に同じ語句がある、不明なモード・範囲・項目、群ごとに32語句または1語句200文字を超える場合は `querySpec` の入力エラーにする。一括検索は `{"mode":"batch","terms":[...],"matchMode":"identifier|text"}` を渡す。空行とNFC・case folding後の重複を除き、入力順・最初の表記を保持する。初期の照合方法はidentifier。重複除去後256語、1語200文字、元入力4096行を上限とする。識別子は先頭英字/アンダースコア、以降ASCII英数字/アンダースコア。日本語や記号を含む行はtextへ切り替える。通常queryとの同時指定は拒否する。

条件式は `all` の全語句 AND `any` のいずれか1語句 AND `not` のどれもない、である。空の `all` / `any` は条件を課さない。`unit` は1検索単位、`excelRow` は Excel の同じシート・物理行のセルと有効な数式Unitをまとめ、それ以外は1検索単位、`file` はファイル名と読み取れた文書内容を合わせて判定する。ファイル名は `unit` / `excelRow` では独立し、`file` でだけ本文と条件をまたげる。除外語は同じ範囲内の全検索単位に NFC と Unicode case folding の部分一致で判定し、あいまい検索のタイプミス候補では除外しない。

## 通知の共通規則

`includeNotes` / `includeFormulas` は省略可能な真偽値で、ともに省略時は偽。通常・条件・一括の全モードに適用する。索引の鮮度には2フラグの抽出範囲と抽出仕様版4を含める。数式は `formula`、Excelコメントは `excelComment`、PPTノートは `note`、Word追加部品は `wordHeader` / `wordFooter` / `wordComment`。数式の `location` はシート・セル番地と保存された共有式情報、ノートはスライド番号・図形ID・段落番号・部品名、Wordは部品名・種類またはコメントID・段落/表の経路、Excelコメントはシート・コメントID・取得できたセル番地を持つ。

一括検索の識別子一致はNFCとcase foldingだけを使い、前後のUnicode英数字・アンダースコアを識別子の継続文字として境界を検査する。幅・区切り・かな・タイプミスの拡張をしない。`matchType=identifier`、`matchCategory=standard`、`score=90`。textは通常検索と同じ照合。1検索語×1検索単位につき最大1 Resultで、同じ箇所で複数語が一致すれば語ごとに別Resultを返す。`termId` / `term` を添え、全体resultCountはその合計とする。フォルダー列挙・文書抽出は1回の検索で共有し、索引経路もファイルのUnitを1回だけまとめて読む。

追加の要約通知はResult/Issueとは別で、件数を増やさない。

| 通知 | 内容・時期 |
| --- | --- |
| `FileRanked` | ファイル処理終了時。`ranking` にfilePath、evidence（quality/matchedTerms/sameRowTerms/bodyEvidence/distinctEvidence）、reasons、コアが並べたresultIds。既に通知した結果だけを含む |
| `RankingSummary` | 終端前。確定順位の絶対パス配列files。未処理文書を追加しない |
| `ExecutionSummary` | 終端前。extractedFiles（抽出を試みたファイル数）、reusedFiles（索引から再利用したファイル数）。語数と抽出回数を比較できる |
| `BatchSummary` | 一括検索の終端前。入力順のterms配列。termId、term、fileCount、hitCount、status。同じファイルは語ごとに1回計上。中断/一部エラーの0件は未確定、全範囲完了・エラーなしの0件だけ指定範囲内で該当なし |

注記の部品単位Issueでは本文結果を残し、ファイル範囲の条件は成立させず、不完全な抽出を索引へ保存しない。Issueのreasonに部品名を含め、同じ部品・障害を重複計上しない。unit/行/一括では取得済みの内容を照合する。GUI・調査記録は終端前の各要約を保持し、終端後の通知を必要としない。

すべての通知に `searchId` と、同一検索内で単調増加する `sequence` を付ける。通知の種類は以下のとおり。名称は概念名であり、実装のメソッド名やデータ形式を強制しない。

| 通知 | 必須データ | 意味 |
| --- | --- | --- |
| `Started` | 受理された検索条件 | 検索を開始した。1 回だけ通知する。 |
| `Progress` | 段階、発見済み対象ファイル数、処理済みファイル数、確定していれば総数 | 列挙または検索が進んだ。通知頻度は任意でよい。 |
| `Result` | `SearchHit` | 通常検索は1検索単位、高度な検索は条件を満たす1単位・1行・1ファイルが一致した。 |
| `Issue` | `SearchIssue` | 列挙または個別ファイルの読み取りで問題が生じた。 |
| `Finished` | 終了理由と最終集計 | その検索の最後の通知。受理された検索につき必ず 1 回。 |

`Started` の後に `Progress`、`Result`、`Issue` と各要約が続き、最後に `Finished` が来る。`Finished` 後に同じ検索 ID の通知は来ない。通知順は `sequence` に従い、Resultの到着順は順位を保証しない。確定した並びは `RankingSummary` と `FileRanked.resultIds` を使う。キャンセル要求後も停止が確定するまで通知が届き得る。

## `SearchHit` の意味

| 項目 | 内容 |
| --- | --- |
| `resultId` | その検索内で一意の識別子。表示順やファイル名を識別子に代用しない。 |
| `documentOrder` | コアが与える文書位置の数値列。検索中から辞書式の数値比較で箇所を並べ、同点はtermId・unitKey・resultIdの順とする。FileRankedのresultIdsもこの規則を使う。 |
| `modifiedAt` | ファイルシステムの最終更新時刻をUnix epochからのミリ秒で渡す。取得不可はnull。検索開始時刻で代用しない。 |
| `sourceMatchRanges` | 抜粋を切り出す前の検索単位全文内のUnicode文字範囲。周辺表・編集で使い、previewTextの範囲と混同しない。 |
| `editAnchor` | 対応テキストの元行・元範囲・検索時リビジョン・元行リビジョン。条件結果では根拠ごとに付ける。GUIから任意のパス/位置を指定して保存する契約ではない。 |
| `termId` / `term` | 一括検索の語IDと元の表示語。通常・条件検索では省略する。条件検索の語IDは `evidence` に保持する。 |
| `filePath` | 元ファイルの絶対パス。ファイルを開くときの対象。 |
| `fileType` | [バックエンド仕様](./backend.md)の対応93拡張子のいずれか。 |
| `sourceKind` | `fileName`、`cell`、`formula`、`note`、`excelComment`、`wordHeader`、`wordFooter`、`wordComment`、`shape`、`slideTableCell`、`paragraph`、`wordTableParagraph`、`textLine`、`excelRow`、`fileMatch` のいずれか。 |
| `unitKey` | 同一ファイルの同一抽出結果内で検索単位を区別する識別子。ファイル編集後も同じIDである保証はしない。 |
| `partKey` / `groupKey` | Office部品の識別子と検索単位のグループ。Excelセルと数式は同じワークシート部品・物理行なら同じ `groupKey`、それ以外は検索単位ごとに異なる。 |
| `row` / `column` | Excelセルの1始まりの物理座標。取得できない場合は省略する。 |
| `contentClass` / `anchor` | 内容区分は `body`、`fileName`、`formula`、`note`。図形のアンカーなどが取得できた場合だけ `anchor` を返す。 |
| `location` | 次表の構造化した場所。ファイル名一致では空。 |
| `previewText` | 一致語と周辺が分かる抜粋。ファイル名一致ではファイル名。表示用の長さ制限を設けてよいが、一致部分を消さない。 |
| `matchRanges` | `previewText` 内で一致した範囲の `[開始, 終了)`。Unicode の文字単位で数え、GUI が一致箇所を強調表示する。 |
| `matchType` | 通常検索では最も強い一致方式、高度な検索では先頭根拠の単一語句の方式。`exact`、`caseFolded`、`normalized`、`separatorVariant`、`identifier`、`kanaVariant`、`prefix`、`substring`、`editDistance` のいずれか。 |
| `matchCategory` | `standard` または `fuzzy`。通常検索は同じ検索単位、高度な検索は同じ範囲の条件全体があいまい検索オフでも成立する場合に `standard`、オンで初めて成立する場合に `fuzzy` とする。 |
| `score` | 一致方式に対応する 0～100 の順位値。同じ検索単位に複数方式が成立しても最も高いものだけを返す。 |
| `previewTruncated` | 抜粋が元の検索単位の全文ではない場合は真。 |
| `evidence` | 高度な検索で成立した正の語句ごとの根拠。`termId`、語句、検索単位ID・部品、元の `sourceKind` と `location`、行・列、抜粋・一致範囲・方式・分類・スコアを持つ。通常検索では省略する。除外語は根拠に含めない。 |

| `sourceKind` | `location` に必要な値 |
| --- | --- |
| `cell` | シート名、セル番地 |
| `formula` | シート名、セル番地、保存されていれば式種別・共有式ID・範囲 |
| `note` | スライド番号、ノート図形名・ID、段落番号、部品名 |
| `excelComment` | シート名、コメントID・形式、部品名。取得できればセル番地 |
| `wordHeader` / `wordFooter` / `wordComment` | 部品名、段落番号または表経路。ヘッダー・フッターは参照の種類、コメントはコメントID |
| `excelRow` | シート名、1始まりの物理行。1行につき1件。 |
| `fileMatch` | ファイル全体を示す。1ファイルにつき1件。根拠の元の場所は `evidence` に保持する。 |
| `shape` | Excel ならシート名、PowerPoint なら 1 始まりのスライド番号。図形名があれば図形名、なければその文書内で識別できる番号。取得可能ならアンカー位置。 |
| `slideTableCell` | 1 始まりのスライド番号、表を識別できる名前または番号、1 始まりの行・列 |
| `paragraph` | 本文内で 1 始まりの段落番号 |
| `wordTableParagraph` | 本文内で 1 始まりの表番号から始まる表・行・列の経路。入れ子の表はその経路を繰り返す。最後にセル内の段落番号 |
| `textLine` | `.txt` とコード・マークアップファイルの 1 始まりの行番号 |

`location` は欠けた位置を推測値で埋めない。取得できない補助情報は省略してよいが、文書内で検索単位を区別するための値は必須とする。GUI は `sourceKind` と `location` から地域化した場所表示とコピー用文字列を作る。コピーは絶対パスを必ず含め、場所がある場合は後ろに添える。

Excelの結合範囲・非表示行列は内部のシートメタデータとして疎に保持する。結合セルの値を別セルへ複製して検索件数を増やさない。

## Excel 周辺セルの取得

高度な行検索の応答は根拠セルの番地を `evidenceAddresses` に含め、表示中の窓にある根拠セルを強調する。

`getResultContext(searchId, resultId, range?)` は、Excel のセル・数式文字列、`excelRow` の結果、またはアンカー位置を取得できた図形結果の周辺セルを要求時に返す。表示するセル値は保存値・保存済み計算結果である。検索通知には周辺セルの値を含めない。`range` を省略すると対象位置の前後2行・2列を基本に、端ではシート内へずらした最大5行×5列を返す。指定時は `firstRow`、`firstColumn`、`rowCount`、`columnCount` を1始まりで渡す。各辺は1～5で、行範囲には対象の行を含める。範囲外や過大な指定は `invalidRange` とする。

応答にはシート名、対象番地・種別、範囲、保存済み値を持つセルだけの `cells`、範囲と交差する `merges`、範囲内の `hiddenRows` と `hiddenColumns` を含める。各セルは行・列・番地・文字列と省略フラグを持つ。結合範囲は四隅と左上起点の番地・保存済み値を持ち、起点が表示範囲外でも識別できる。空セルを展開した配列にはしない。返す範囲は最大25セル、セル文字列は最大300文字、結合範囲は最大25件に制限する。数式は再計算せず、保存されたセル書式をGUIへ渡す。周辺応答の `layout` は範囲内の行高・列幅と最大25セルの配置・折り返し・文字書式・塗りつぶし・罫線、再現できない要素の説明を持つ。セルの `matchRanges` は可視文字列の元の一致範囲で、300文字の表示範囲内だけを返す。

最新の検索IDと結果IDだけが有効で、検索終了後も次の検索が始まるまで取得できる。新しい検索を受理した後の旧応答は破棄する。検索結果のファイルサイズまたは更新時刻と現在の値が違う、元ファイルが移動・削除された場合は `changedSinceSearch` を返し、結果そのものは残す。索引利用時にそのファイルの有効な索引があれば保存済みセルを使い、それ以外は元文書を読み直す。索引なしでは元文書を読み直す。図形のアンカーがない場合は周辺取得を提供せず、その理由を表示する。

## 編集と表示設定の境界

デスクトップ層の `prepare_result_edit(searchId, resultId, evidenceIndex?)` は終了した最新検索の保持結果だけを対象に、元ファイルと位置を選ぶ。`editAnchor`と元バイト/元行を照合し、元範囲の選択文字列・文脈・文字コードと不透明な編集tokenを返す。条件結果では根拠番号を必須とする。`save_result_edit(searchId, token, replacement)` は保存直前の全元バイトを再検査し、選択範囲だけ更新する。保存失敗はドラフトを保持する。`cancel_result_edit` はドラフトだけ破棄し、元ファイルを変更しない。新しい検索は旧tokenを無効にする。保存成功後はそのファイルの旧tokenと周辺targetを無効にし、GUIは再検索が必要な状態を示す。

`validate_folder_paths(paths)` は起動時の保存値を順序・件数を変えず検査し、有効な絶対フォルダーパスを保持して無効な要素だけ空文字へ戻す。保存場所・更新時期はGUIの責務であり、検索は開始しない。

折りたたみはフォルダーの絶対パスとファイルパスごとのGUI状態で、結果や出力件数を変えない。本文条件は元の全結果を保持したまま、一致箇所の表示抜粋に適用する。JSON記録の `filterInclude` / `filterExclude` に各1語句を保存し、filteredのrowsは残した箇所だけ、allは全件を持つ。`modifiedAt` / `documentOrder`もJSON行へ渡す。

## `SearchIssue` と終了集計

`SearchIssue` は発生段階（`discovery` または `read`）、判明している場合のパス、分類コード、利用者向けの短い理由を持つ。分類コードは少なくとも `accessDenied`、`unreadable`、`encrypted`、`unsupportedEncoding`、`resourceLimit`、`changedDuringRead`、`internalError` を区別する。内部の例外やスタックトレースを GUI にそのまま渡さない。

`Finished` の終了理由は `completed`、`cancelled`、`failed` のいずれか。`completed` に `Issue` があれば GUI は「完了（一部エラーあり）」と示す。最終集計には `discoveredFiles`、`processedFiles`、`resultCount`、`issueCount` を含める。`processedFiles` は索引の再利用を含め、検索処理を試みた対象ファイル数とする。個別エラーになったファイルも数える。`resultCount` と `issueCount` は通知した `Result` と `Issue` の件数に厳密に一致する。`processedFiles` は `discoveredFiles` を超えない。中断時も同じ集計を返す。

入力拒否は `Finished(failed)` と混同しない。検索が受理された後にバックエンド自身が停止した場合は、途中までの通知を保持して `Finished(failed)` を返す。

## 境界の適合確認

- GUI を使わずに同じ `SearchRequest` を実行し、結果、エラー、進捗、終了集計を検査できる。
- テスト用の通知源を GUI に接続し、実ファイルを読まずに結果表示、中断、古い検索 ID の無視を検査できる。
- 別実装の抽出器を接続しても `SearchHit` と終了集計の意味が変わらない。

この適合確認には [共通の生成文書とバックエンド検証ケース](../tests/README.md) を使用する。ケース内の相対パスと概念上の `location` を、実装ごとの API 入出力へ変換するのはテストアダプターの責務とする。結果の到着順や自動採番 ID を期待値として固定しない。
