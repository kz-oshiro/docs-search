@echo off
setlocal
powershell.exe -NoProfile -ExecutionPolicy RemoteSigned -File "%~dp0tests\Generate-Test-Data.ps1" %*
if errorlevel 1 (
    echo.
    echo Test data generation failed. Review the error above.
    pause
    exit /b 1
)
