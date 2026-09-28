# docs-search PowerShell 実装

旧 v1.0 の Excel 検索実装を保存したディレクトリです。再構築版の要求仕様は [上位の README](../README.md) を参照してください。以下の起動・テスト用コマンドは、この `powershell` ディレクトリを作業ディレクトリとして実行します。

追加ライブラリを入れずに、Excel ブックを横断検索する Windows 用ツールです。

## 起動

エクスプローラーで **`Start-docs-search.vbs` をダブルクリック**します。追加のインストールやパス設定は不要です。起動用スクリプトはすぐ終了し、検索画面だけが残ります。検索画面を閉じると PowerShell のプロセスも終了します。

コマンドから起動する場合は、Windows PowerShell 5.1 でこのフォルダーから次を実行します。

```powershell
powershell.exe -NoProfile -STA -ExecutionPolicy RemoteSigned -File .\docs-search.ps1
```

起動ファイルの `RemoteSigned` は、この起動プロセスにだけ適用されます。組織のポリシーでスクリプト実行が禁止されている場合は、その設定が優先されます。

検索フォルダーと検索語を入力して「検索」を押します。サブフォルダー内の `.xlsx` / `.xlsm` も対象です。結果のファイルパスをクリックすると Windows の既定のアプリでそのファイルを開きます。「場所をコピー」はファイルパスと、分かる場合はシート・セルまたは図形名をコピーします。

検索はファイル名、セルの保存値・保存済み計算結果、標準図形とテキストボックス内の文字を対象にします。大文字・小文字を区別しない部分一致です。検索時に Excel やマクロは起動せず、ブック・索引・検索履歴を書き込みません。

`.xls`、グラフ、SmartArt、セルの表示書式を反映した文字列は対象外です。パスワードで暗号化されたファイルや読めないファイルは、画面下部にエラーとして表示します。

## 開発時の依存関係

ツール本体に追加パッケージは不要です。テストでは Python 3 の標準ライブラリで共通の文書データを一時生成し、終了時に削除します。生成方法とバックエンド検証ケースは [共通テストデータ](../tests/README.md) を参照してください。

Windows PowerShell 5.1 でこのディレクトリからテストを実行します。

```powershell
powershell.exe -NoProfile -ExecutionPolicy RemoteSigned -File ..\tests\run-powershell-tests.ps1
```

## 負荷確認用データ

共通ジェネレータの `--profile load` で、Excel を含む 4 種類の負荷確認用データを生成できます。コマンドと内容は [共通テストデータ](../tests/README.md) を参照してください。PowerPoint、Word、テキストは再構築版の検討用で、v1.0 の検索対象は `.xlsx` / `.xlsm` のままです。
