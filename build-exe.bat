@echo off
rem Builds the finished program and puts Folio.exe next to this file.
cd /d "%~dp0"
cargo build --release
if errorlevel 1 (
  echo.
  echo The build failed. See the messages above.
  pause
  exit /b 1
)
copy /y "target\release\folio.exe" "Folio.exe" >nul
echo.
echo Done! Your program is here: %~dp0Folio.exe
pause
