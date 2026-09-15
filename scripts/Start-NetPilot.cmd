@echo off
REM R1 NP-252 — Core automatic startup then Desktop
setlocal
set "DIR=%~dp0"
set "NETPILOT_LOG_LEVEL=info"
if defined LOCALAPPDATA (
  if not defined NETPILOT_DATA_DIR set "NETPILOT_DATA_DIR=%LOCALAPPDATA%\NetPilot\data"
)
if defined NETPILOT_DATA_DIR (
  mkdir "%NETPILOT_DATA_DIR%\config" 2>nul
  mkdir "%NETPILOT_DATA_DIR%\logs" 2>nul
  mkdir "%NETPILOT_DATA_DIR%\cache" 2>nul
  mkdir "%NETPILOT_DATA_DIR%\run" 2>nul
)
if exist "%DIR%netpilot-core.exe" (
  start "NetPilot-Core" /MIN "%DIR%netpilot-core.exe"
  timeout /t 1 /nobreak >nul
)
if exist "%DIR%NetPilot.Desktop.Wpf.exe" (
  start "" "%DIR%NetPilot.Desktop.Wpf.exe"
  exit /b 0
)
if exist "%DIR%NetPilot.Desktop.exe" (
  start "" "%DIR%NetPilot.Desktop.exe"
  exit /b 0
)
echo Desktop executable not found in %DIR%
exit /b 1
