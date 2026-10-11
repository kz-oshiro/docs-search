use crate::excel_target::ExcelTarget;

#[cfg(target_os = "windows")]
pub fn open(target: &ExcelTarget) -> Result<(), String> {
    use std::io::Write;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    use std::sync::Mutex;

    // Settings such as EnableEvents belong to the Excel application, not a workbook.
    static AUTOMATION: Mutex<()> = Mutex::new(());
    let _guard = AUTOMATION
        .try_lock()
        .map_err(|_| "Excelの移動操作が進行中です。完了してから再操作してください。")?;
    if !std::path::Path::new(&target.file_path).is_file() {
        return Err("元ファイルが見つかりません。再検索してください。".into());
    }
    let powershell = std::path::PathBuf::from(
        std::env::var_os("SystemRoot").ok_or("Windowsの実行環境を取得できません。")?,
    )
    .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let input = serde_json::to_vec(target).map_err(|_| "Excelの移動先を準備できません。")?;
    let mut child = Command::new(powershell)
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-STA",
            "-Command",
            include_str!("excel_open.ps1"),
        ])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW; the Excel window is intentional.
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "Excelへの接続処理を起動できませんでした。")?;
    let write_result = child.stdin.take().unwrap().write_all(&input);
    // Always reap our helper. Never terminate or close a user's Excel process.
    let output = child
        .wait_with_output()
        .map_err(|_| "Excelへの接続処理に失敗しました。")?;
    write_result.map_err(|_| "Excelの移動先を渡せませんでした。")?;
    let response: serde_json::Value = serde_json::from_slice(&output.stdout).map_err(|_| {
        "Excelに接続できません。Windows PowerShellとExcelの利用設定を確認してください。"
    })?;
    if output.status.success() && response["ok"].as_bool() == Some(true) {
        Ok(())
    } else {
        Err(response["message"]
            .as_str()
            .unwrap_or("Excelのセルへ移動できませんでした。")
            .into())
    }
}

#[cfg(not(target_os = "windows"))]
pub fn open(_target: &ExcelTarget) -> Result<(), String> {
    Err("Excelのセル移動はWindowsデスクトップ版で利用できます。".into())
}
