@echo off
setlocal
cd /d "%~dp0"
if exist "netpilot-core.exe" start "NetPilot Core" /min "netpilot-core.exe"
if exist "NetPilot.Desktop.Wpf.exe" (
  start "" "NetPilot.Desktop.Wpf.exe"
) else if exist "NetPilot.Desktop.exe" (
  start "" "NetPilot.Desktop.exe"
) else (
  echo Desktop executable not found.
  pause
)
