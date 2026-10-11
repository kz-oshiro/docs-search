# Fixed program, embedded in the EXE. Paths and positions arrive only as JSON stdin.
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Console]::InputEncoding = New-Object System.Text.UTF8Encoding($false)
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
$excel = $null
$workbook = $null
$created = $false
$restoreEvents = $false
$restoreSecurity = $false
$response = $null
$message = 'Excelに接続できません。デスクトップ版Excelの導入・起動状態を確認してください。'
try {
    $target = [Console]::In.ReadToEnd() | ConvertFrom-Json
    $path = [IO.Path]::GetFullPath($target.filePath -replace '^\\\\\?\\', '')
    if (-not [IO.File]::Exists($path)) {
        $message = '元ファイルが見つかりません。再検索してください。'
        throw 'missing file'
    }
    # ROT lookup retrieves an already-open workbook, including other Excel instances.
    # Unlike BindToMoniker/GetObject(filename), it never opens a file as a side effect.
    Add-Type -TypeDefinition @'
using System;
using System.IO;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
public static class DocsSearchRunningWorkbook {
    [DllImport("ole32.dll")] static extern int GetRunningObjectTable(int reserved, out IRunningObjectTable table);
    [DllImport("ole32.dll")] static extern int CreateBindCtx(int reserved, out IBindCtx context);
    public static object Find(string path) {
        IRunningObjectTable table = null;
        IBindCtx context = null;
        IEnumMoniker iterator = null;
        try {
            Marshal.ThrowExceptionForHR(GetRunningObjectTable(0, out table));
            Marshal.ThrowExceptionForHR(CreateBindCtx(0, out context));
            table.EnumRunning(out iterator);
            IMoniker[] items = new IMoniker[1];
            while (iterator.Next(1, items, IntPtr.Zero) == 0) {
                try {
                    string name;
                    items[0].GetDisplayName(context, null, out name);
                    if (String.Equals(name, path, StringComparison.OrdinalIgnoreCase)) {
                        object value;
                        table.GetObject(items[0], out value);
                        return value;
                    }
                } catch (COMException) { }
                finally { Marshal.ReleaseComObject(items[0]); }
            }
            return null;
        } finally {
            if (iterator != null) Marshal.ReleaseComObject(iterator);
            if (context != null) Marshal.ReleaseComObject(context);
            if (table != null) Marshal.ReleaseComObject(table);
        }
    }
}
'@
    $workbook = [DocsSearchRunningWorkbook]::Find($path)
    if ($null -ne $workbook) {
        if (-not [string]::Equals($workbook.FullName, $path, [StringComparison]::OrdinalIgnoreCase)) { throw 'wrong workbook' }
        $excel = $workbook.Application
    } else {
        try { $excel = [Runtime.InteropServices.Marshal]::GetActiveObject('Excel.Application') } catch { }
        if ($null -ne $excel) {
            foreach ($candidate in $excel.Workbooks) {
                if ([string]::Equals($candidate.FullName, $path, [StringComparison]::OrdinalIgnoreCase)) {
                    $workbook = $candidate
                    break
                }
            }
        }
        if ($null -eq $workbook) {
            $processes = @(Get-Process EXCEL -ErrorAction SilentlyContinue)
            if (($null -eq $excel -and $processes.Count -gt 0) -or $processes.Count -gt 1) {
                $message = '開いているExcelブックへの接続を確定できません。対象ブックのExcelから別のウィンドウへ切り替えて再操作してください。'
                throw 'unregistered Excel instance'
            }
            if ($null -eq $excel) {
                $excel = New-Object -ComObject Excel.Application
                $created = $true
            }
        }
    }
    $events = $excel.EnableEvents
    $restoreEvents = $true
    $excel.EnableEvents = $false
    if ($null -eq $workbook) {
        $message = 'ブックを開けません。パス・保護状態・Excelの利用設定を確認してください。'
        $security = $excel.AutomationSecurity
        $restoreSecurity = $true
        $excel.AutomationSecurity = 3 # msoAutomationSecurityForceDisable
        # No link updates, read-only, explicit empty passwords (no password prompt).
        $workbook = $excel.Workbooks.Open($path, 0, $true, [Type]::Missing, '', '', $true)
    }
    $message = '対象シートが見つかりません。検索後に変更された可能性があります。再検索してください。'
    $sheet = $workbook.Worksheets.Item([string]$target.sheetName)
    $message = '非表示シートには移動できません。Excelで対象シートを確認してください。'
    if ($sheet.Visible -ne -1) { throw 'hidden sheet' }
    $message = '対象セルに移動できません。非表示の行列・シート保護・Excelの編集中状態を確認してください。'
    $cell = $sheet.Range([string]$target.cellAddress)
    if ($cell.MergeCells) { $cell = $cell.MergeArea.Cells.Item(1, 1) }
    if ($cell.EntireRow.Hidden -or $cell.EntireColumn.Hidden) { throw 'hidden cell' }
    $excel.Visible = $true
    $workbook.Activate()
    $sheet.Activate()
    $excel.Goto($cell, $true)
    # Verify the selected destination rather than reporting invocation alone as success.
    if ($excel.ActiveWorkbook.FullName -ne $workbook.FullName -or
        $excel.ActiveSheet.Name -ne $sheet.Name -or
        $excel.ActiveCell.Row -ne $cell.Row -or $excel.ActiveCell.Column -ne $cell.Column) { throw 'selection mismatch' }
    $response = @{ ok = $true }
} catch {
    $response = @{ ok = $false; message = $message }
} finally {
    # Never save, close, unhide, or quit a user's workbook. Restore application settings.
    if ($restoreSecurity) {
        try { $excel.AutomationSecurity = $security }
        catch { $response = @{ ok = $false; message = 'Excelのマクロ設定を元に戻せませんでした。Excelの状態を確認してください。' } }
    }
    if ($restoreEvents) {
        try { $excel.EnableEvents = $events }
        catch { $response = @{ ok = $false; message = 'Excelのイベント設定を元に戻せませんでした。Excelの状態を確認してください。' } }
    }
    if ($created -and $null -ne $excel) {
        try {
            if ($excel.Workbooks.Count -eq 0) { $excel.Quit() }
            else { $excel.Visible = $true }
        } catch { }
    }
}
$response | ConvertTo-Json -Compress
