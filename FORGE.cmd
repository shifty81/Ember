@echo off
setlocal
cd /d "%~dp0"
rem Backward-compatible alias. Ember's project-local PCC is authoritative.
call "%~dp0PROJECT_CONTROL_CENTER.cmd" %*
exit /b %ERRORLEVEL%
