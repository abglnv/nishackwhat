@echo off
REM ===== NisHack Teacher Backend Runner =====
REM Runs nishack-backend.exe in an infinite restart loop so the server
REM comes back whenever it exits or crashes. Registered to auto-start
REM at boot by install.bat; can also be launched manually.

title NisHack Teacher Backend
setlocal

set "BASE=%~dp0"
if "%BASE:~-1%"=="\" set "BASE=%BASE:~0,-1%"
set "BIN=%BASE%\target\release\nishack-backend.exe"
set "LOGDIR=%BASE%\logs"

if not exist "%BIN%" (
    echo nishack-backend.exe not found at "%BIN%".
    echo Run install.bat first.
    pause
    exit /b 1
)

if not exist "%LOGDIR%" mkdir "%LOGDIR%"

cd /d "%BASE%"

:loop
echo [%date% %time%] Starting nishack-backend.exe >> "%LOGDIR%\run.log"
"%BIN%" >> "%LOGDIR%\backend.log" 2>&1
set "RC=%ERRORLEVEL%"
echo [%date% %time%] nishack-backend.exe exited (code %RC%). Restarting in 5s... >> "%LOGDIR%\run.log"
timeout /t 5 /nobreak >nul
goto loop
