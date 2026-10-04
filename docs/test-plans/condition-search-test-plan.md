# 高度な検索 P3 の確認方針

現行の実行入口は[開発手順](../development.md)、機能基準は[Rust要求仕様](../specifications/rust-requirements.md)、公開版の検証結果は[GitHub Releases](https://github.com/kz-oshiro/docs-search/releases)を参照する。試験設計は同じ文書を更新し、改訂履歴をGitで管理する。

試験の担当と保証範囲は[試験区分と振り分け](playwright-exe-test-plan.md#試験区分と振り分けの基本方針)と[OS連携の保証対象外](playwright-exe-test-plan.md#5-os連携の保証対象外)、実行依頼の範囲は[エージェントの作業ルール](../development.md#エージェントの作業ルール)に従う。以下のGUI手順・未実施・許可の記述は旧方針の参考記録として扱う。

対象は AND・OR・除外語と、検索箇所 / Excelの同じ行 / ファイル全体の判定です。この実装段階ではテスト・ビルド・GUI起動を実施していません。次の検証では、以下の確認項目・手順・期待結果を先に把握してから進めます。

対応する[実装計画 P3](../plans/implementation-plan.md)と[進捗レポート](../reports/implementation-status.md)を参照してください。

## 準備

`docs-search` を作業ディレクトリとして、自動テストのみの依頼では次を実行します。UI資材stagingとヘッドレスPlaywrightを含みます。

この入口は[`cli_conditions.rs`](../../core/tests/cli_conditions.rs)も実行し、下表のCLI項目を独立した一時データで検証します。条件入力・根拠・出力対象のDOMと要求はヘッドレスPlaywrightで確認します。実アプリのOS連携・見た目は別の確認範囲です。

```powershell
cargo xtask test
```

Windows配布ビルドまで依頼された場合は、上の入口に代えて `cargo xtask ci` を使います。

専用データは既存の `outputs/` に新しい名前で生成します。生成器は存在する出力先を上書きしません。

```powershell
cargo xtask fixtures --kind conditions --output .\outputs\conditions-p3-new
```

CLI では空の通常検索語を渡し、条件JSONを `--query-spec-json` に渡します。例:

```powershell
$spec = @{mode='conditions';scope='excelRow';all=@('顧客','必須');any=@();not=@('廃止')} | ConvertTo-Json -Compress
.\core\target\debug\docs-search-cli.exe .\outputs\conditions-p3-new\search '' --extensions xlsx --query-spec-json $spec
```

CLI の `result` 通知の `sourceKind`、`location`、`evidence`、件数と `finished.counts` を確認します。索引ありでは `--use-index`、あいまい検索ありでは `--fuzzy-search` を追加します。CLI は画面操作を検証しないため、GUI項目は別に確認します。GUI テストの実施が依頼文に明示されていなければ、実施前に確認を取ります。

## 確認項目・手順・期待結果

| 項目 | 手順 | 期待結果 |
| --- | --- | --- |
| 行 AND | `all=[顧客,必須]`、`scope=excelRow`、対象 `.xlsx` | `conditions.xlsx` の `Items` 行12と15が各1件。語句ごとに A12/D12 または A15/D15 を根拠として示す。別行13/14と別シート `Other` 行12は成立しない |
| 同じ検索箇所 | 同じ `all` で `scope=unit` | 0件。別セルの語句を1セルの一致として扱わない |
| ファイル AND | 同じ `all` で `scope=file`、対象 `.xlsx,.txt` | `conditions.xlsx` と `notes.txt` が各1件。`notes.txt` の別行にある語句も同じファイルとして成立する |
| 行 NOT | `all=[顧客,必須]`、`not=[廃止]`、`scope=excelRow` | `Items` 行12だけ。行15の E15 が除外語なので、その行は返さない。別行の除外語は行12に影響しない |
| OR | `all=[]`、`any=[顧客,必須]`、`scope=excelRow`、対象 `.xlsx` | `conditions.xlsx` の `Items` 行12～15と `Other` 行12が計5件。両語句がある行は1件で、成立した2語の根拠を保持する |
| ファイル名と本文 | `all=[filename-signal,body-signal]`、`scope=file` | `filename-signal.xlsx` が1件。`scope=unit` または `excelRow` では0件。ファイル名とセルの根拠を別々に表示する |
| 語句の解釈 | `all=[AND OR]`、`scope=unit` と、通常検索語 `AND OR` をそれぞれ実行 | どちらも1語句として B16 と `notes.txt` の3行目に一致。AND / OR を演算子として解析しない |
| 入力検証 | 正の群が空、必須と除外に同じ語句、通常語と `querySpec` の併用、未知の範囲、長すぎる語句を指定 | 開始前に `querySpec` の入力エラー。検索通知を発行しない |
| 不完全なファイル | `errors/filename-signal-broken.xlsx` を対象に `all=[filename-signal,body-signal]`、`scope=file` | 読み取り Issue が出る。ファイル名だけを根拠にファイル範囲の成立を作らない |
| 索引とあいまい検索 | 行 AND・NOT・ファイル AND を、索引オン/オフ × あいまい検索オン/オフで実行 | 各組み合わせで同じ範囲・件数・場所。あいまい検索は正の語句だけに適用され、除外語は NFC と case folding の部分一致 |
| 正の語句と除外語の判定差 | `all=[custmer]`、`scope=unit` をあいまい検索オン/オフで実行。次に `all=[customer]`、`not=[custmer]` をあいまい検索オンで実行 | 前者はオフで0件、オンで `Items!A17` が1件。後者も `Items!A17` が1件。除外語にタイプミス一致を適用しない |
| GUI・周辺セル | 高度な検索を選び `Items` 行12を表示し、根拠一覧と「周辺を表示」を開く | 各語句の元の場所・抜粋を強調。周辺表では A12 と D12 の根拠セルを強調し、列移動もできる。通常検索へ戻しても単一語句の意味を維持 |
| 出力・件数 | 高度な検索結果を TSV/CSV/JSON に出力する。中断や一部エラーも確認する | TSV/CSVに検索語と根拠列、JSONに `querySpec` と `evidence` が残る。`resultCount` は通知した結果件数と一致し、根拠数とは混同しない |

通常検索の既存ケースは変更しません。索引を使わない場合は保存済み索引への読み書きが発生せず、索引を使う場合も候補セルだけで除外を確定しないことを確認します。
