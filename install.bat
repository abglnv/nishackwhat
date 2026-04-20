@echo off
REM ===== NisHack Teacher Backend Installer (standalone bootstrap) =====
REM Drop this single file on a fresh Windows machine, right-click ->
REM Run as administrator. It will:
REM   - install Git + Rust via winget if missing
REM   - clone the source from GitHub into %ProgramData%\NisHack\teacher
REM   - prompt for a Redis URI and write it into config.toml
REM   - build the release binary
REM   - open TCP 8080 in Windows Firewall
REM   - register a scheduled task that auto-starts at boot as SYSTEM,
REM     running run.bat with a crash-restart loop

title NisHack Teacher Backend Installer

REM --- must be Administrator ---
net session >nul 2>&1
if errorlevel 1 (
    echo.
    echo This installer must be run as Administrator.
    echo Right-click install.bat and choose "Run as administrator".
    echo.
    pause
    exit /b 1
)

set "REPO_URL=https://github.com/abglnv/nishackwhat"
set "ROOT=%ProgramData%\NisHack"
set "INSTALL_DIR=%ROOT%\teacher"

echo.
echo === NisHack Teacher Backend Installer ===
echo Repo:        %REPO_URL%
echo Install dir: %INSTALL_DIR%
echo.

REM --- winget check ---
where winget >nul 2>&1
if errorlevel 1 (
    echo winget is not available on this system.
    echo Install "App Installer" from the Microsoft Store and retry.
    pause
    exit /b 1
)

REM --- Git ---
where git >nul 2>&1
if errorlevel 1 (
    echo [1/8] Installing Git...
    winget install --id Git.Git -e --silent --accept-source-agreements --accept-package-agreements
    if exist "C:\Program Files\Git\cmd\git.exe" set "PATH=C:\Program Files\Git\cmd;%PATH%"
) else (
    echo [1/8] Git already installed.
)
where git >nul 2>&1
if errorlevel 1 (
    echo Git still not on PATH after install. Open a new admin terminal and retry.
    pause
    exit /b 1
)

REM --- Rust (GNU toolchain, no Visual Studio needed) ---
set "CARGO_BIN=%USERPROFILE%\.cargo\bin"
set "PATH=%CARGO_BIN%;%PATH%"

where cargo >nul 2>&1
if errorlevel 1 (
    echo [2/8] Installing Rust toolchain...
    winget install --id Rustlang.Rustup -e --silent --accept-source-agreements --accept-package-agreements
    set "PATH=%CARGO_BIN%;%PATH%"
) else (
    echo [2/8] Rust already installed.
)

"%CARGO_BIN%\rustup.exe" toolchain install stable-x86_64-pc-windows-gnu
"%CARGO_BIN%\rustup.exe" default stable-x86_64-pc-windows-gnu
if errorlevel 1 (
    echo Rust toolchain setup failed.
    pause
    exit /b 1
)

REM --- Clone or update source ---
echo.
echo [3/8] Fetching source code...
if not exist "%ROOT%" mkdir "%ROOT%"

if exist "%INSTALL_DIR%\.git" (
    echo Repo already present, pulling latest...
    pushd "%INSTALL_DIR%"
    git pull --ff-only
    popd
) else (
    git clone "%REPO_URL%" "%INSTALL_DIR%"
    if errorlevel 1 (
        echo git clone failed.
        pause
        exit /b 1
    )
)
if not exist "%INSTALL_DIR%\Cargo.toml" (
    echo Cargo.toml missing in "%INSTALL_DIR%" after clone.
    pause
    exit /b 1
)

REM --- Redis URI prompt ---
echo.
echo [4/8] Redis configuration
echo Example: redis://default:password@my-redis.upstash.io:6379
echo (paste WITHOUT surrounding quotes)
set "REDIS_URI="
set /p REDIS_URI=Enter Redis URI:
if defined REDIS_URI set REDIS_URI=%REDIS_URI:"=%

if "%REDIS_URI%"=="" (
    echo Redis URI cannot be empty.
    pause
    exit /b 1
)

REM --- Patch config.toml (top-level redis_url) ---
powershell -NoProfile -ExecutionPolicy Bypass -Command ^
  "$uri = $env:REDIS_URI;" ^
  "$path = Join-Path $env:INSTALL_DIR 'config.toml';" ^
  "$c = Get-Content -Raw $path;" ^
  "$c = [regex]::Replace($c, '(?m)^redis_url\s*=\s*\"[^\"]*\"', ('redis_url = \"' + $uri + '\"'));" ^
  "Set-Content -NoNewline -Path $path -Value $c"
if errorlevel 1 (
    echo Failed to update config.toml.
    pause
    exit /b 1
)
echo Redis URI written to config.toml.

REM --- Build ---
echo.
echo [5/8] Building release binary (this can take several minutes)...
pushd "%INSTALL_DIR%"
cargo build --release
if errorlevel 1 (
    echo Build failed.
    popd
    pause
    exit /b 1
)
popd

if not exist "%INSTALL_DIR%\target\release\nishack-backend.exe" (
    echo nishack-backend.exe not found after build.
    pause
    exit /b 1
)

if not exist "%INSTALL_DIR%\run.bat" (
    echo run.bat is missing from the repo. Commit and push run.bat to %REPO_URL% and re-run.
    pause
    exit /b 1
)

REM --- Firewall rule for port 8080 ---
echo.
echo [6/8] Opening TCP port 8080 in Windows Firewall...
netsh advfirewall firewall delete rule name="NisHack Teacher Backend" >nul 2>&1
netsh advfirewall firewall add rule name="NisHack Teacher Backend" dir=in action=allow protocol=TCP localport=8080

REM --- Scheduled task (auto-start at boot as SYSTEM) ---
echo.
echo [7/8] Registering auto-start scheduled task...
schtasks /Delete /TN "NisHack Teacher Backend" /F >nul 2>&1
schtasks /Create /F ^
    /TN "NisHack Teacher Backend" ^
    /TR "\"%INSTALL_DIR%\run.bat\"" ^
    /SC ONSTART ^
    /RU SYSTEM ^
    /RL HIGHEST
if errorlevel 1 (
    echo Scheduled task registration failed.
    pause
    exit /b 1
)

REM --- Start now ---
echo.
echo [8/8] Starting backend now in a background window...
start "NisHack Teacher Backend" /MIN cmd /c "\"%INSTALL_DIR%\run.bat\""

echo.
echo === Install complete ===
echo Installed at:    %INSTALL_DIR%
echo Dashboard:       http://localhost:8080  (and LAN IP on :8080)
echo Auto-start:      at every boot (Task Scheduler: "NisHack Teacher Backend")
echo Auto-restart:    yes (run.bat loop)
echo.
echo To stop:         schtasks /End /TN "NisHack Teacher Backend"
echo To uninstall:    schtasks /Delete /TN "NisHack Teacher Backend" /F
echo                  netsh advfirewall firewall delete rule name="NisHack Teacher Backend"
echo                  rmdir /S /Q "%INSTALL_DIR%"
echo.
pause
