Option Explicit

Dim shell, files, scriptPath, powerShellPath, command
Set shell = CreateObject("WScript.Shell")
Set files = CreateObject("Scripting.FileSystemObject")

scriptPath = files.BuildPath(files.GetParentFolderName(WScript.ScriptFullName), "doc-search.ps1")
powerShellPath = shell.ExpandEnvironmentStrings("%SystemRoot%") & "\System32\WindowsPowerShell\v1.0\powershell.exe"
command = Chr(34) & powerShellPath & Chr(34) & " -NoProfile -STA -ExecutionPolicy RemoteSigned -WindowStyle Hidden -File " & Chr(34) & scriptPath & Chr(34)

On Error Resume Next
shell.Run command, 0, False
If Err.Number <> 0 Then
    MsgBox Err.Description, vbCritical, "doc-search"
End If
