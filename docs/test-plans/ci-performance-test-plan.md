# CI性能改善の検証方針

アイコン描画、フロントエンドのCPU負荷に応じたworker選択、バックエンドとフロントエンドの同時実行を対象とする。コンパイル最適化設定と依存ライブラリは変更しない。実装依頼では静的確認までとし、この文書の作成を実行許可とは扱わない。

通常入口と準備は[開発手順](../development.md)、DOM・操作の期待結果は[フロントエンド試験設計](playwright-ui-test-plan.md)、未実装の実EXE接続は[アプリケーション結合試験設計](playwright-exe-test-plan.md)を参照する。

## worker選択と記録

[計算と計測](../../tests/ui/workers.mjs)はNode標準機能のみを使う。[NodeのCPU API](https://nodejs.org/api/os.html#oscpus)で200msの累積CPU時間差分を3回取得する。各区間の使用率は `(全CPUの総時間差分 - idle差分) / 全CPUの総時間差分`。容量には `os.availableParallelism()` を使い、論理CPUの配列長を並列数に置き換えない。

最大使用率を `u`、利用可能CPU数を `n` とし、選択数を次で求める。

```text
workers = max(1, min(6, max(1, floor(n * 0.5)), floor(n * max(0, 0.8 - u))))
```

| 利用可能CPU | 最大使用率 | workers | 判定理由 |
| --- | --- | --- | --- |
| 8 | 0% | 4 | バックエンド用の余力を残す半数上限 |
| 8 | 50% | 2 | 80%目標までの空きに合わせる |
| 8 | 75% / 100% | 1 | 最低1で全件を実行する |
| 32 | 0% | 6 | フロントエンド6ファイルに合わせた上限 |
| 1 / 2 | 0% | 1 | 少ないCPUでは増やさない |
| 不明 / カウンター異常 | 不明 | 1 | 理由を記録してfallback |

600msの予定測定窓と実測時間、各区間・平均/最大使用率（0〜1の割合）、CPU数、選択数、policy、fallback理由を `outputs/runs/<ID>/ui/workers.json` に保存する。Playwright設定は同じファイルを全workerで読み、計算と違う選択数や異なるschemaVersionを拒否する。実行途中のworker数は固定し、次回の実行で負荷を再計測する。80%は開始時の計算目標であり、その後の他プロセスの負荷変動は次回の計測で反映する。[Playwrightのファイル間並列方式](https://playwright.dev/docs/test-parallel)を使用し、ファイル内は順次・再試行0・画像/動画/トレースなしを維持する。

## バックエンド試験

| 確認項目 | 手順 | 期待結果 |
| --- | --- | --- |
| CPU差分とworker計算 | `node --test tests/ui/workers.test.mjs`（全体入口にも必須で含む） | 8件成功。累積値ではなく差分を使用し、低負荷/高負荷/少数CPU/上限/欠損/カウンター逆行/不変/取得例外/一時的高負荷/計画改変を判定する。OS値と待機を注入するため実負荷を発生させない |
| 検索コア・公開API・CLIの維持 | 同じ `PROPTEST_CASES` / `PROPTEST_RNG_SEED` で `cargo xtask test` | 既存単体/API/性質とCLI6群がすべて成功。共通27・周辺5・条件31・Office順位20組の期待値を維持。LOCALAPPDATAと変更用データが各CLIの一時領域に分かれる |
| 独立工程の失敗収集 | 専用検証checkoutで共通CLIの期待件数1か所だけを意図的に変え、`cargo xtask ci` | 当該工程failed、他のCargo/worker-policy/UI工程も終了し記録される。全体終了コード1、desktop-build skipped。変更は検証checkoutだけに置く |

## フロントエンド試験

| 確認項目 | 手順 | 期待結果 |
| --- | --- | --- |
| 負荷連動の選択と全件実行 | `cargo xtask ui`、続いて `cargo xtask test` のレポートと `ui/workers.json` / `ui/results.json` を照合 | 54件成功、skipped/unexpected/flaky 0。policyから求めたworkersとUIの `config.workers` が一致。複数workerでもテーマ/ストレージ/要求/遅延応答/起動失敗ケースの状態が混ざらない |
| 前提不足でもバックエンドを継続 | Playwright設定のみを専用検証checkoutで意図的な構文エラーにして `cargo xtask ci` | Playwright工程failed。バックエンド/検証基盤は終了まで続き、ログが残る。全体終了コード1、desktop-build skipped。前提不足も成功にしない |
| 選択実行 | `cargo xtask ui --case "U23:"` | 選択ケースだけを実行し、同じCPU計測と計画読込みを使う。全体合格の証拠には使用しない |

## アプリケーション結合試験

今回の変更で実EXE試験は導入しない。接続上の追加ケースはない。現行 `ci` の配布ビルドは、両方の試験枝が完了し全必須成功した後に1回だけ行う。結合試験ランナーとtest/ciのRelease追加は[別設計](playwright-exe-test-plan.md)の未実装範囲として区別する。

## 共通の検証基盤

| 確認項目 | 手順 | 期待結果 |
| --- | --- | --- |
| 承認済みアイコン | `cargo test --locked -p docs-search-test-support --test icons` | 2件成功。承認済みICOの全バイト、PNGの展開画素、8サイズのICOディレクトリ、2回生成の全バイトが一致。重なった複数図形と透明図形の小さな独立ケースも期待画素に一致する |
| 生成器 | `cargo test --locked -p docs-search-test-support --test corpus`、全体入口 | 2件成功。再現性・在庫・Office構造とサイズを維持。loadを実行内で1回だけ生成し、生成器試験と共通CLIで共有する |
| 並列・ログ・完了待ち | `cargo xtask test` の `report.json` / `report.md` を確認 | backend-/frontend-ログが分離され、工程の開始時刻とdurationから2枝の重なりを確認できる。全工程が終了してからreportを保存し、異なる実行の結果を混ぜない。枝内のpanicもfailedとして残し、他方を待つ |
| ビルドの条件 | 両枝成功の `cargo xtask ci` と上記失敗注入の記録を照合 | 成功時だけ現在ソースのReleaseビルドとEXEコピー/SHA256記録を行う。失敗時に以前のEXEを成功成果物として採用しない |

## 性能比較と引継ぎ

変更前基準は `4b36315`。既存の[CI記録](../../outputs/runs/1791034713571-9428-ci/report.md)は135.567秒（ビルド37.729秒）、[自動試験記録](../../outputs/runs/1791041541534-4952-test/report.md)は76.819秒（アイコン19.03秒、UI工程19.571秒）。対象ソース/キャッシュ/実行条件が異なる参考値で、改善率の基準にはしない。

後続の検証担当は旧/新を独立checkout・独立targetに置き、同じRust/Node/Playwright、電源設定、CPU競合、proptest件数/seedで測る。既存キャッシュは削除せず、新しいtargetでcold各1回、同じtargetでwarm各3回の中央値を比較する。外側の総時間とxtask内部総時間、アイコン・CPU計測・Rust群・UI群・Release工程を分ける。並列工程のduration合計を総時間にしない。Playwrightのworker選択と測定負荷も記録し、容量や開始時負荷の違う実行を直接比較しない。

実行依頼では上記の確認項目・手順・期待結果を読み、必要なsetup、全体test/ciと性能比較を担当する。結果は実行別レポートとReleaseの検証記録に保存し、試験方針へ件数・環境・結果を再転記しない。ユーザーにテスト一式の実行を委ねない。
