Set-StrictMode -Version 2.0
$ErrorActionPreference = 'Stop'
trap {
    Add-Type -AssemblyName System.Windows.Forms
    [void][System.Windows.Forms.MessageBox]::Show($_.Exception.Message, 'doc-search を起動できません')
    break
}
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()

$script:enginePath = Join-Path $PSScriptRoot 'Search-Excel.ps1'
$script:searchJob = $null
$script:resultCount = 0
$script:errorCount = 0

$form = New-Object System.Windows.Forms.Form
$form.Text = 'doc-search | Excel 検索'
$form.StartPosition = 'CenterScreen'
$form.Size = New-Object System.Drawing.Size(1150, 730)
$form.MinimumSize = New-Object System.Drawing.Size(850, 540)

$folderLabel = New-Object System.Windows.Forms.Label
$folderLabel.Text = '検索フォルダー'
$folderLabel.Location = New-Object System.Drawing.Point(14, 18)
$folderLabel.AutoSize = $true
$form.Controls.Add($folderLabel)

$folderBox = New-Object System.Windows.Forms.TextBox
$folderBox.Location = New-Object System.Drawing.Point(110, 14)
$folderBox.Size = New-Object System.Drawing.Size(895, 24)
$folderBox.Anchor = 'Top,Left,Right'
$folderBox.Text = [Environment]::GetFolderPath('MyDocuments')
$form.Controls.Add($folderBox)

$browseButton = New-Object System.Windows.Forms.Button
$browseButton.Text = '参照...'
$browseButton.Location = New-Object System.Drawing.Point(1010, 12)
$browseButton.Size = New-Object System.Drawing.Size(110, 28)
$browseButton.Anchor = 'Top,Right'
$form.Controls.Add($browseButton)

$queryLabel = New-Object System.Windows.Forms.Label
$queryLabel.Text = '検索語'
$queryLabel.Location = New-Object System.Drawing.Point(14, 58)
$queryLabel.AutoSize = $true
$form.Controls.Add($queryLabel)

$queryBox = New-Object System.Windows.Forms.TextBox
$queryBox.Location = New-Object System.Drawing.Point(110, 54)
$queryBox.Size = New-Object System.Drawing.Size(650, 24)
$queryBox.Anchor = 'Top,Left,Right'
$form.Controls.Add($queryBox)

$searchButton = New-Object System.Windows.Forms.Button
$searchButton.Text = '検索'
$searchButton.Location = New-Object System.Drawing.Point(770, 52)
$searchButton.Size = New-Object System.Drawing.Size(110, 28)
$searchButton.Anchor = 'Top,Right'
$form.Controls.Add($searchButton)

$stopButton = New-Object System.Windows.Forms.Button
$stopButton.Text = '中断'
$stopButton.Location = New-Object System.Drawing.Point(890, 52)
$stopButton.Size = New-Object System.Drawing.Size(110, 28)
$stopButton.Anchor = 'Top,Right'
$stopButton.Enabled = $false
$form.Controls.Add($stopButton)

$copyButton = New-Object System.Windows.Forms.Button
$copyButton.Text = '場所をコピー'
$copyButton.Location = New-Object System.Drawing.Point(1010, 52)
$copyButton.Size = New-Object System.Drawing.Size(110, 28)
$copyButton.Anchor = 'Top,Right'
$form.Controls.Add($copyButton)

$statusLabel = New-Object System.Windows.Forms.Label
$statusLabel.Text = '検索するフォルダーと語句を指定してください。'
$statusLabel.Location = New-Object System.Drawing.Point(14, 90)
$statusLabel.Size = New-Object System.Drawing.Size(1100, 24)
$statusLabel.Anchor = 'Top,Left,Right'
$form.Controls.Add($statusLabel)

$grid = New-Object System.Windows.Forms.DataGridView
$grid.Location = New-Object System.Drawing.Point(14, 118)
$grid.Size = New-Object System.Drawing.Size(1106, 440)
$grid.Anchor = 'Top,Bottom,Left,Right'
$grid.ReadOnly = $true
$grid.AllowUserToAddRows = $false
$grid.AllowUserToDeleteRows = $false
$grid.MultiSelect = $false
$grid.SelectionMode = 'FullRowSelect'
$grid.RowHeadersVisible = $false
$grid.AutoSizeRowsMode = 'DisplayedCells'
$grid.DefaultCellStyle.WrapMode = 'True'
$grid.Columns.Add('Kind', '種類') | Out-Null
$grid.Columns.Add('Sheet', 'シート') | Out-Null
$grid.Columns.Add('Location', 'セル・図形') | Out-Null
$grid.Columns.Add('Value', '該当文字列') | Out-Null
$linkColumn = New-Object System.Windows.Forms.DataGridViewLinkColumn
$linkColumn.Name = 'Path'
$linkColumn.HeaderText = 'ファイルパス（クリックで開く）'
$linkColumn.TrackVisitedState = $false
$grid.Columns.Add($linkColumn) | Out-Null
$grid.Columns['Kind'].Width = 75
$grid.Columns['Sheet'].Width = 120
$grid.Columns['Location'].Width = 145
$grid.Columns['Value'].Width = 285
$grid.Columns['Path'].AutoSizeMode = 'Fill'
$form.Controls.Add($grid)

$errorsLabel = New-Object System.Windows.Forms.Label
$errorsLabel.Text = '読み取りエラー'
$errorsLabel.Location = New-Object System.Drawing.Point(14, 567)
$errorsLabel.Anchor = 'Bottom,Left'
$errorsLabel.AutoSize = $true
$form.Controls.Add($errorsLabel)

$errorsBox = New-Object System.Windows.Forms.TextBox
$errorsBox.Location = New-Object System.Drawing.Point(14, 588)
$errorsBox.Size = New-Object System.Drawing.Size(1106, 93)
$errorsBox.Anchor = 'Bottom,Left,Right'
$errorsBox.Multiline = $true
$errorsBox.ReadOnly = $true
$errorsBox.ScrollBars = 'Vertical'
$form.Controls.Add($errorsBox)

function Finish-Search([string]$Message) {
    $timer.Stop()
    if ($null -ne $script:searchJob) {
        Remove-Job -Job $script:searchJob -Force -ErrorAction SilentlyContinue
        $script:searchJob = $null
    }
    $searchButton.Enabled = $true
    $stopButton.Enabled = $false
    $browseButton.Enabled = $true
    $statusLabel.Text = "$Message　結果: $script:resultCount 件 / エラー: $script:errorCount 件"
}

function Receive-SearchOutput {
    if ($null -eq $script:searchJob) { return }
    $jobErrors = @()
    $items = @(Receive-Job -Job $script:searchJob -ErrorAction SilentlyContinue -ErrorVariable jobErrors)
    foreach ($item in $items) {
        switch ($item.Type) {
            'Result' {
                [void]$grid.Rows.Add($item.Kind, $item.Sheet, $item.Location, $item.Value, $item.Path)
                $script:resultCount++
            }
            'Progress' {
                $statusLabel.Text = "検索中: $($item.Current) / $($item.Total) ファイル　結果: $script:resultCount 件"
            }
            'Error' {
                $script:errorCount++
                $errorsBox.AppendText("$($item.Path): $($item.Message)`r`n")
            }
        }
    }
    foreach ($issue in $jobErrors) {
        $script:errorCount++
        $errorsBox.AppendText("$($issue.Exception.Message)`r`n")
    }
    if ($script:searchJob.State -ne 'Running') {
        $message = if ($script:searchJob.State -eq 'Completed') { '検索が完了しました。' }
                   elseif ($script:searchJob.State -eq 'Stopped') { '検索を中断しました。' }
                   else { '検索処理が停止しました。' }
        Finish-Search $message
    }
}

$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 200
$timer.Add_Tick({ Receive-SearchOutput })

$browseButton.Add_Click({
    $dialog = New-Object System.Windows.Forms.FolderBrowserDialog
    $dialog.Description = '検索するフォルダーを選択してください'
    if (Test-Path -LiteralPath $folderBox.Text -PathType Container) { $dialog.SelectedPath = $folderBox.Text }
    if ($dialog.ShowDialog($form) -eq [System.Windows.Forms.DialogResult]::OK) { $folderBox.Text = $dialog.SelectedPath }
    $dialog.Dispose()
})

$searchButton.Add_Click({
    $root = $folderBox.Text.Trim()
    $query = $queryBox.Text.Trim()
    if (-not (Test-Path -LiteralPath $root -PathType Container)) {
        [void][System.Windows.Forms.MessageBox]::Show($form, '存在するフォルダーを指定してください。', 'doc-search')
        return
    }
    if (-not $query) {
        [void][System.Windows.Forms.MessageBox]::Show($form, '検索語を入力してください。', 'doc-search')
        return
    }
    $grid.Rows.Clear()
    $errorsBox.Clear()
    $script:resultCount = 0
    $script:errorCount = 0
    try {
        $script:searchJob = Start-Job -ScriptBlock {
            param($engine, $folder, $term)
            & $engine -Root $folder -Query $term
        } -ArgumentList $script:enginePath, $root, $query
        $searchButton.Enabled = $false
        $stopButton.Enabled = $true
        $browseButton.Enabled = $false
        $statusLabel.Text = '検索を開始しています...'
        $timer.Start()
    }
    catch {
        [void][System.Windows.Forms.MessageBox]::Show($form, $_.Exception.Message, '検索を開始できません')
    }
})

$stopButton.Add_Click({
    if ($null -ne $script:searchJob -and $script:searchJob.State -eq 'Running') {
        $statusLabel.Text = '中断しています...'
        Stop-Job -Job $script:searchJob -ErrorAction SilentlyContinue
        Receive-SearchOutput
    }
})

$copyButton.Add_Click({
    if ($grid.SelectedRows.Count -eq 0) { return }
    $row = $grid.SelectedRows[0]
    $path = [string]$row.Cells['Path'].Value
    $sheet = [string]$row.Cells['Sheet'].Value
    $location = [string]$row.Cells['Location'].Value
    $reference = if ($sheet -and $location) { "$path [$sheet!$location]" }
                 elseif ($sheet) { "$path [$sheet]" } else { $path }
    [System.Windows.Forms.Clipboard]::SetText($reference)
    $statusLabel.Text = '場所をコピーしました。'
})

$grid.Add_CellContentClick({
    param($sender, $eventArgs)
    if ($eventArgs.RowIndex -lt 0 -or $eventArgs.ColumnIndex -ne $grid.Columns['Path'].Index) { return }
    $path = [string]$grid.Rows[$eventArgs.RowIndex].Cells['Path'].Value
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        [void][System.Windows.Forms.MessageBox]::Show($form, "ファイルが見つかりません: $path", 'doc-search')
        return
    }
    try { Start-Process -FilePath $path -ErrorAction Stop | Out-Null }
    catch { [void][System.Windows.Forms.MessageBox]::Show($form, $_.Exception.Message, 'ファイルを開けません') }
})

$form.Add_FormClosing({
    $timer.Stop()
    if ($null -ne $script:searchJob) {
        Stop-Job -Job $script:searchJob -ErrorAction SilentlyContinue
        Remove-Job -Job $script:searchJob -Force -ErrorAction SilentlyContinue
    }
})

[void]$form.ShowDialog()
$form.Dispose()
