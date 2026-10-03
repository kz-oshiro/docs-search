@echo off
setlocal
cd /d "%~dp0"
cargo xtask fixtures --open %*
if errorlevel 1 (
    echo.
    echo Test data generation failed. Review the error above.
    pause
    exit /b 1
)
