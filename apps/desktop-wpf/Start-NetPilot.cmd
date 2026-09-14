@echo off
setlocal
cd /d "%~dp0"

if exist "netpilot-core.exe" (
  echo Starting netpilot-core.exe (named pipe \\.\pipe\netpilot-core )...
  start "NetPilot Core" "netpilot-core.exe"
  timeout /t 2 /nobreak >nul
) else (
  echo WARNING: netpilot-core.exe not found in %cd%
)

if exist "NetPilot.Desktop.Wpf.exe" (
  start "" "NetPilot.Desktop.Wpf.exe"
) else if exist "NetPilot.Desktop.exe" (
  start "" "NetPilot.Desktop.exe"
) else (
  echo Desktop executable not found.
  pause
)
