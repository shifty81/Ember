@echo off
setlocal EnableExtensions
cd /d "%~dp0"

set "EMBER_PCC=%~dp0tools\control_center\ember_pcc.py"
if not exist "%EMBER_PCC%" (
    echo [ERROR] Ember Project Control Center is missing:
    echo         %EMBER_PCC%
    exit /b 2
)

where py >nul 2>nul
if not errorlevel 1 (
    py -3 "%EMBER_PCC%" %*
    exit /b %ERRORLEVEL%
)

where python >nul 2>nul
if not errorlevel 1 (
    python "%EMBER_PCC%" %*
    exit /b %ERRORLEVEL%
)

echo [ERROR] Python 3 is required by the Ember Project Control Center.
where winget >nul 2>nul
if errorlevel 1 (
    echo Install Python 3.12+ and rerun PROJECT_CONTROL_CENTER.cmd.
    exit /b 2
)

echo.
choice /C YN /N /M "Install Python 3 with winget now? [Y/N]: "
if errorlevel 2 exit /b 2
winget install --id Python.Python.3.12 -e --accept-package-agreements --accept-source-agreements
if errorlevel 1 exit /b %ERRORLEVEL%
echo.
echo Python installation completed. Reopen this command window, then run PROJECT_CONTROL_CENTER.cmd again.
exit /b 0
