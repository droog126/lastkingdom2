# loop.ps1 - WANGUO ORIGINS demo closed-loop iteration script
# Usage: powershell -File loop.ps1
# Each loop: build (if needed) -> run 12s -> kill -> list new screenshots
#
# Default: online mode starts lk2-server + lk2-client.
# Client connects to 127.0.0.1:5000 unless -Offline or -NoServer is used.
# The loop runs sim ticks and captures screenshots/state JSON.
# Offline mode runs only lk2-client --offline --auto-demo.
# -NoServer runs the client connect path without starting a server.

param(
    # Default 60s gives network startup and replication enough time.
    # Earlier short loops missed initial replicated PlayerPos in online mode.
    # Keep verbose lightyear logs unless RUST_LOG is already set.
    # The loop may be shortened with -Seconds for offline visual checks.
    [int]$Seconds = 60,
    [string]$RUST_LOG = $(if ($env:RUST_LOG) { $env:RUST_LOG } else { "info,lightyear_replication=debug,lightyear_connection=debug,lightyear_send=debug,lightyear_receive=debug" }),
    # Build is enabled by default here; cold builds can be slow.
    # Use -SkipBuild only when binaries already exist and no code changed.
    # Incremental dev builds are usually fast with dynamic linking.
    [switch]$SkipBuild = $false,
    # Enable Bevy dynamic linking for faster dev builds.
    [switch]$Dynamic = $true,
    [switch]$Online = $false,
    # Online mode starts lk2-server and lk2-client together.
    # Offline mode starts only lk2-client --offline.
    [switch]$Offline = $false,
    # NoServer starts the client connect path without a local server.
    # Useful for debugging client transport behavior.
    [switch]$NoServer = $false,
    # Server address used by online client mode.
    [string]$ServerAddr = "127.0.0.1:5000",
    # Force first-person camera for FP debugging.
    [switch]$FirstPerson = $false
)

$ProjectRoot = $PSScriptRoot
Set-Location $ProjectRoot

# Decision gate: require the previous iter_NN/decision.md before another loop.
# This keeps the closed-loop AI workflow honest and reviewable.
# Disable only for urgent debug or CI dry runs.
[switch]$RequireDecision = $true

if ($RequireDecision) {
    $prevDirs = Get-ChildItem "$ProjectRoot\screenshots\iter_*" -Directory -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending
    if ($prevDirs.Count -gt 0) {
        $prevIterDir = $prevDirs[0].FullName
        $prevIterName = $prevDirs[0].Name
        $prevDecision = Join-Path $prevIterDir "decision.md"
        if (-not (Test-Path $prevDecision)) {
            Write-Host "" -ForegroundColor Red
            Write-Host "=============================================" -ForegroundColor Red
            Write-Host "  FAIL: $prevIterName/decision.md MISSING" -ForegroundColor Red
            Write-Host "=============================================" -ForegroundColor Red
            Write-Host " Previous loop ($prevIterName) has no decision.md; refusing next loop." -ForegroundColor Red
            Write-Host " Write $prevDecision, then run loop.ps1 again." -ForegroundColor Red
            Write-Host " Template: $prevIterDir\decision.template.md" -ForegroundColor Red
            Write-Host " See Agent.md decision template and completion criteria." -ForegroundColor Red
            Write-Host "" -ForegroundColor Red
            exit 1
        }
        Write-Host ">>> [OK] previous $prevIterName/decision.md exists -- decision gate green" -ForegroundColor Green
    }
}

$UseOffline = (-not $Online) -and (-not $NoServer)
if ($Offline) { $UseOffline = $true }

$env:BEVY_DISABLE_ACCESSIBILITY = "1"
# RUST_LOG was selected in the param default; pass it to child processes.
# 瀵搫鍩楅崘娆庣濞嗏€冲煂 env var, 缂佹瑥鐡欐潻娑氣柤缂佈勫
$env:RUST_LOG = $RUST_LOG
$rustSysroot = (& rustc --print sysroot).Trim()
$runtimePaths = @(
    (Join-Path $ProjectRoot "target\debug\deps"),
    (Join-Path $ProjectRoot "target\debug"),
    (Join-Path $rustSysroot "bin")
) | Where-Object { Test-Path $_ }
$env:PATH = (($runtimePaths + @($env:PATH)) -join ";")

# 0. Kill any old lk2-client / lk2-server processes before the loop.
Get-Process -Name "lk2-client","lk2-server" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1

# 1. Build targets unless -SkipBuild is set.
# Key point: when client uses bevy dynamic_linking, lk2-core must enable it too.
# Otherwise core may link static Bevy while client links dynamic Bevy, causing LNK2019.
# Cargo unifies dependency features, but not crate-local feature names automatically.
$featureArgs = if ($Dynamic) { "--features dev-dynamic-linking,lk2-core/dev-dynamic-linking" } else { ""
    # Static linking path: core does not need dev-dynamic-linking.
}
if ($Dynamic) { Write-Host ">>> dynamic linking ON <<<" -ForegroundColor Cyan }

# 1a. Decide build targets: online needs client+server; offline only needs client.
$buildTargets = if ($UseOffline -or $NoServer) { @("lk2-client") } else { @("lk2-client","lk2-server") }
$serverExePath = Join-Path $ProjectRoot "target\debug\lk2-server.exe"
$clientExePath = Join-Path $ProjectRoot "target\debug\lk2-client.exe"
$needServerBuild = (-not $UseOffline) -and (-not $NoServer) -and (-not (Test-Path $serverExePath))
$needClientBuild = -not (Test-Path $clientExePath)

if (-not $SkipBuild) {
    foreach ($t in $buildTargets) {
        Write-Host ">>> cargo build -p $t $featureArgs ..." -ForegroundColor Cyan
        $buildOutput = cmd /c "cargo build -p $t $featureArgs 2>&1"
        $buildOutput | Tee-Object -FilePath "build_loop.log" | Select-Object -Last 5
        if ($LASTEXITCODE -ne 0) {
            Write-Host ">>> BUILD FAILED for $t" -ForegroundColor Red
            exit 1
        }
    }
} else {
    # Even with -SkipBuild, build missing binaries on the first loop run.
    if ($needClientBuild) {
        Write-Host ">>> client binary missing, building (SkipBuild override) ..." -ForegroundColor Cyan
        cmd /c "cargo build -p lk2-client $featureArgs 2>&1" | Tee-Object -FilePath "build_loop.log" | Select-Object -Last 5
        if ($LASTEXITCODE -ne 0) { Write-Host ">>> BUILD FAILED" -ForegroundColor Red; exit 1 }
    }
    if ($needServerBuild) {
        Write-Host ">>> server binary missing, building (SkipBuild override) ..." -ForegroundColor Cyan
        cmd /c "cargo build -p lk2-server $featureArgs 2>&1" | Tee-Object -FilePath "build_loop.log" | Select-Object -Last 5
        if ($LASTEXITCODE -ne 0) { Write-Host ">>> BUILD FAILED" -ForegroundColor Red; exit 1 }
    }
}

# 2. Run + screenshot + record
if (-not (Test-Path $clientExePath)) {
    Write-Host ">>> Binary not found: $clientExePath" -ForegroundColor Red
    exit 1
}

# 閸愬啿鐣惧Ο鈥崇础
$serverProc = $null
$serverLog = Join-Path $ProjectRoot "screenshots\loop_server.log"
$clientLog = Join-Path $ProjectRoot "screenshots\loop_run.log"
$mode = "online"
if ($UseOffline) {
    $mode = "offline"
    $clientArgs = @("--offline","--auto-demo")
    Write-Host ">>> Mode: OFFLINE (no server, client --offline --auto-demo) ${Seconds}s ..." -ForegroundColor Green
} elseif ($NoServer) {
    $mode = "noserver"
    $clientArgs = @("--connect=$ServerAddr","--auto-demo")
    Write-Host ">>> Mode: NOSERVER (no lk2-server, client --connect=$ServerAddr will fail) ${Seconds}s ..." -ForegroundColor Yellow
} else {
    if (-not (Test-Path $serverExePath)) {
        Write-Host ">>> Server binary not found: $serverExePath (use -Offline to skip server)" -ForegroundColor Red
        exit 1
    }
    $clientArgs = @("--connect=$ServerAddr","--auto-demo")
    Write-Host ">>> Mode: ONLINE (server + client --connect=$ServerAddr) ${Seconds}s ..." -ForegroundColor Green

    # Start server in the background.
    Write-Host ">>> Starting lk2-server (background) ..." -ForegroundColor Cyan
    $serverProc = Start-Process -FilePath $serverExePath -PassThru -NoNewWindow `
        -RedirectStandardOutput $serverLog -RedirectStandardError "$serverLog.err"
    # Give server self_check a short startup buffer.
    Start-Sleep -Seconds 3
}

if ($FirstPerson) {
    $clientArgs += "--first-person"
    Write-Host ">>> FirstPerson ON (camera at player eye height + mouse look)" -ForegroundColor Cyan
}

# Start client in the foreground so screenshots and state JSON are written.
Write-Host ">>> Starting lk2-client ($mode) ..." -ForegroundColor Cyan

# Bevy dev-dynamic-linking records a hashed bevy_dylib name in the binary.
# If the newest dll hash changes, Windows may fail to locate the expected dll.
# Keep aliases in target/debug so the client can start reliably.
$debugDir = Split-Path -Parent $clientExePath
$expectedName = $null
# 1) Find the newest bevy_dylib-*.dll in the binary directory.
#    This is enough for the local dev loop.
$candidateDlls = Get-ChildItem "$debugDir\bevy_dylib-*.dll" -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending
if ($candidateDlls.Count -gt 0) {
    $latestDll = $candidateDlls[0].FullName
    # The binary expects a hashed dll name from Bevy build.rs.
    # Use the newest dll basename from this build session.
    $latestBaseName = $candidateDlls[0].Name
    Write-Host ">>> found bevy dll: $latestBaseName" -ForegroundColor DarkGray
    # 2) 閸氬本妞傛穱婵堟殌 bevy_dylib.dll 閸掝偄鎮?(cargo 閼奉亜鐢?
    $vanilla = Join-Path $debugDir "bevy_dylib.dll"
    if (-not (Test-Path $vanilla)) {
        Copy-Item $latestDll $vanilla -Force
        Write-Host ">>> copied bevy_dylib.dll (vanilla alias)" -ForegroundColor DarkGray
    }
    # 3) Keep the hashed dll name available in target/debug.
    #    If newest dll hash and binary expectation diverge, PATH/debugDir handles lookup.
    #    (binary 闁俺绻?bevy_dylib-<hash>.dll 鏉╂瑤閲滈崥宥呯摟 lookup)
    #    The debug directory is on PATH before client launch.
}

$clientProc = Start-Process -FilePath $clientExePath -ArgumentList $clientArgs -PassThru -NoNewWindow `
    -RedirectStandardOutput $clientLog -RedirectStandardError "$clientLog.err" `
    -WorkingDirectory $ProjectRoot
Start-Sleep -Seconds $Seconds
$clientProc | Stop-Process -Force -ErrorAction SilentlyContinue
if ($serverProc) {
    $serverProc | Stop-Process -Force -ErrorAction SilentlyContinue
}
Start-Sleep -Seconds 1

# 3. List results
Write-Host ""
Write-Host "=== Latest screenshots ===" -ForegroundColor Yellow
# iter_NN.png is copied to screenshots/iter_NN/iter_NN.png.
Get-ChildItem "$ProjectRoot\screenshots\iter_*\iter_*.png" -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending | Select-Object -First 5 |
    ForEach-Object { "  $($_.FullName.Substring($ProjectRoot.Length + 1)) ($($_.Length / 1KB | ForEach-Object {'{0:N1}KB' -f $_}))" }

Write-Host ""
Write-Host "=== Latest tick state ===" -ForegroundColor Yellow
Get-ChildItem "$ProjectRoot\screenshots\state_*.json" -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1 |
    ForEach-Object { "  $($_.Name)" }

# In online mode, also print key server log lines for connection validation.
if ($mode -in @("online","noserver")) {
    Write-Host ""
    Write-Host "=== Server log (last 8 lines) ===" -ForegroundColor Yellow
    if (Test-Path $serverLog) {
        Get-Content $serverLog -Tail 8
    } else {
        Write-Host "  (no server log file at $serverLog)" -ForegroundColor DarkGray
    }
    # Check server self_check / tick markers.
    $serverCheckOk = (Test-Path $serverLog) -and (Select-String -Path $serverLog -Pattern "閼奉亝顥?*100 tick 閸忋劑鍎撮柅姘崇箖|Server UDP socket bound" -Quiet)
    if ($serverCheckOk) {
        Write-Host "  [OK] server self-check passed + socket bound" -ForegroundColor Green
    } else {
        Write-Host "  [WARN] server didn't print self-check pass / socket bound (see log above)" -ForegroundColor Yellow
    }
}

Write-Host ""
Write-Host ">>> Done. AI: read latest screenshot + state JSON, decide next round" -ForegroundColor Magenta

#4. SCORE protocol reminder -- find latest iter and drop decision.template.md
$latestIterDir = $null
$latestIterName = $null
# Sort by LastWriteTime; iter_99 vs iter_100 string ordering is misleading.
$ssDirs = Get-ChildItem "$ProjectRoot\screenshots\iter_*" -Directory -ErrorAction SilentlyContinue |
 Sort-Object LastWriteTime -Descending
if ($ssDirs.Count -gt0) {
 $latestIterDir = $ssDirs[0].FullName
 $latestIterName = $ssDirs[0].Name
 $prevIterName = if ($ssDirs.Count -gt1) { $ssDirs[1].Name } else { "" }
 $decTplPath = Join-Path $latestIterDir "decision.template.md"
 $decTpl = @"
# $latestIterName decision

task: [loop goal]
result: pass / partial / fail

score:
- sky: X/10
- player: X/10
- terrain: X/10
- decor: X/10
- hud: X/10
- gameplay: X/10
- total: X.X/10

vs_prev:
- visual: improved / same / worse, with reason
- state: key delta from diff.json$(if ($prevIterName) { " compared with $prevIterName" } else { "" })

problems:
- [concrete problem 1]
- [concrete problem 2]
- [concrete problem 3]

tests:
- [command run]
- [result]

next:
- [single best next action]

## Scoring guide (0-10)

- Sky: readable sky; not black/white screen.
- Player: player is visible with direction and height cues.
- Terrain: terrain is recognizable and spawn point is standable.
- Decor: trees, water, animals, monsters, and props have layers.
- HUD: readable and not blocking the key scene.
- Gameplay: auto demo moves state forward and changes are explainable.
"@
 Set-Content -Path $decTplPath -Value $decTpl -Encoding UTF8
 Write-Host ""
 Write-Host "=== SCORE reminder written: $decTplPath ===" -ForegroundColor Cyan
 Write-Host ">>> NEXT AI: score this loop from 0-10 and write decision.md before continuing." -ForegroundColor Yellow
} else {
 Write-Host ""
 Write-Host "[warn] no iter_* directory found -- can't write decision.template.md" -ForegroundColor Yellow
}
