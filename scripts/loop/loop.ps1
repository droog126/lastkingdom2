









param(




    [int]$Seconds = 60,
    [int]$MaxExtraWait = 60,
    [string]$RUST_LOG = $(if ($env:RUST_LOG) { $env:RUST_LOG } else { "info,lightyear_replication=debug,lightyear_connection=debug,lightyear_send=debug,lightyear_receive=debug" }),



    [switch]$SkipBuild = $false,

    [switch]$Dynamic = $true,
    [switch]$Online = $false,


    [switch]$Offline = $false,


    [switch]$NoServer = $false,

    [string]$ServerAddr = "127.0.0.1:5000",

    [switch]$FirstPerson = $false,
    [switch]$AuditPrettyModels = $false
)

$ProjectRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $ProjectRoot

$LogDir = Join-Path $ProjectRoot "run-logs"
if (-not (Test-Path $LogDir)) {
    New-Item -ItemType Directory -Path $LogDir -Force | Out-Null
}




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
            Write-Host " See AGENTS.md decision template and completion criteria." -ForegroundColor Red
            Write-Host "" -ForegroundColor Red
            exit 1
        }
        Write-Host ">>> [OK] previous $prevIterName/decision.md exists -- decision gate green" -ForegroundColor Green
    }
}

$UseOffline = (-not $Online) -and (-not $NoServer)
if ($Offline) { $UseOffline = $true }

$env:BEVY_DISABLE_ACCESSIBILITY = "1"

$env:RUST_LOG = $RUST_LOG
$rustSysroot = (& rustc --print sysroot).Trim()
$runtimePaths = @(
    (Join-Path $ProjectRoot "target\debug\deps"),
    (Join-Path $ProjectRoot "target\debug"),
    (Join-Path $rustSysroot "bin")
) | Where-Object { Test-Path $_ }
$env:PATH = (($runtimePaths + @($env:PATH)) -join ";")


Get-Process -Name "lk2-client","lk2-server" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1





$features = @()
if ($Dynamic) {
    $features += "dev-dynamic-linking"
    $features += "lk2-core/dev-dynamic-linking"
}
if ($AuditPrettyModels) {
    $features += "audit-pretty-models"
}
$featureArgs = if ($features.Count -gt 0) { "--features " + ($features -join ",") } else { "" }
if ($Dynamic) { Write-Host ">>> dynamic linking ON <<<" -ForegroundColor Cyan }
if ($AuditPrettyModels) { Write-Host ">>> audit pretty models ON <<<" -ForegroundColor Cyan }


$buildTargets = if ($UseOffline -or $NoServer) { @("lk2-client") } else { @("lk2-client","lk2-server") }
$serverExePath = Join-Path $ProjectRoot "target\debug\lk2-server.exe"
$clientExePath = Join-Path $ProjectRoot "target\debug\lk2-client.exe"
$needServerBuild = (-not $UseOffline) -and (-not $NoServer) -and (-not (Test-Path $serverExePath))
$needClientBuild = -not (Test-Path $clientExePath)

if (-not $SkipBuild) {
    foreach ($t in $buildTargets) {
        Write-Host ">>> cargo build -p $t $featureArgs ..." -ForegroundColor Cyan
        $buildOutput = cmd /c "cargo build -p $t $featureArgs 2>&1"
        $buildOutput | Tee-Object -FilePath (Join-Path $LogDir "build_loop.log") | Select-Object -Last 5
        if ($LASTEXITCODE -ne 0) {
            Write-Host ">>> BUILD FAILED for $t" -ForegroundColor Red
            exit 1
        }
    }
} else {

    if ($needClientBuild) {
        Write-Host ">>> client binary missing, building (SkipBuild override) ..." -ForegroundColor Cyan
        cmd /c "cargo build -p lk2-client $featureArgs 2>&1" | Tee-Object -FilePath (Join-Path $LogDir "build_loop.log") | Select-Object -Last 5
        if ($LASTEXITCODE -ne 0) { Write-Host ">>> BUILD FAILED" -ForegroundColor Red; exit 1 }
    }
    if ($needServerBuild) {
        Write-Host ">>> server binary missing, building (SkipBuild override) ..." -ForegroundColor Cyan
        cmd /c "cargo build -p lk2-server $featureArgs 2>&1" | Tee-Object -FilePath (Join-Path $LogDir "build_loop.log") | Select-Object -Last 5
        if ($LASTEXITCODE -ne 0) { Write-Host ">>> BUILD FAILED" -ForegroundColor Red; exit 1 }
    }
}


if (-not (Test-Path $clientExePath)) {
    Write-Host ">>> Binary not found: $clientExePath" -ForegroundColor Red
    exit 1
}


$serverProc = $null
$serverLog = Join-Path $ProjectRoot "screenshots\loop_server.log"
$clientLog = Join-Path $ProjectRoot "screenshots\loop_run.log"
$latestIterBeforeNumber = 0
$latestIterBefore = Get-ChildItem "$ProjectRoot\screenshots\iter_*" -Directory -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1
if ($latestIterBefore -and $latestIterBefore.Name -match '^iter_(\d+)$') {
    $latestIterBeforeNumber = [int]$Matches[1]
}
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


    Write-Host ">>> Starting lk2-server (background) ..." -ForegroundColor Cyan
    $serverProc = Start-Process -FilePath $serverExePath -PassThru -NoNewWindow `
        -RedirectStandardOutput $serverLog -RedirectStandardError "$serverLog.err"

    Start-Sleep -Seconds 3
}

if ($FirstPerson) {
    $clientArgs += "--first-person"
    Write-Host ">>> FirstPerson ON (camera at player eye height + mouse look)" -ForegroundColor Cyan
}


Write-Host ">>> Starting lk2-client ($mode) ..." -ForegroundColor Cyan




$debugDir = Split-Path -Parent $clientExePath
$expectedName = $null


$candidateDlls = Get-ChildItem "$debugDir\bevy_dylib-*.dll" -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending
if ($candidateDlls.Count -gt 0) {
    $latestDll = $candidateDlls[0].FullName


    $latestBaseName = $candidateDlls[0].Name
    Write-Host ">>> found bevy dll: $latestBaseName" -ForegroundColor DarkGray

    $vanilla = Join-Path $debugDir "bevy_dylib.dll"
    if (-not (Test-Path $vanilla)) {
        Copy-Item $latestDll $vanilla -Force
        Write-Host ">>> copied bevy_dylib.dll (vanilla alias)" -ForegroundColor DarkGray
    }



}

$clientProc = Start-Process -FilePath $clientExePath -ArgumentList $clientArgs -PassThru -NoNewWindow `
    -RedirectStandardOutput $clientLog -RedirectStandardError "$clientLog.err" `
    -WorkingDirectory $ProjectRoot
$startedAt = Get-Date
$minStopAt = $startedAt.AddSeconds($Seconds)
$maxStopAt = $minStopAt.AddSeconds($MaxExtraWait)
$readyIter = $null
do {
    Start-Sleep -Seconds 1
    if ($clientProc.HasExited) {
        break
    }

    $now = Get-Date
    if ($now -lt $minStopAt) {
        continue
    }

    $candidate = Get-ChildItem "$ProjectRoot\screenshots\iter_*" -Directory -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -match '^iter_(\d+)$' -and [int]$Matches[1] -gt $latestIterBeforeNumber } |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if (-not $candidate) {
        continue
    }

    $statePath = Join-Path $candidate.FullName "final_state.json"
    $png = Get-ChildItem (Join-Path $candidate.FullName "iter_*.png") -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if (-not (Test-Path $statePath) -or -not $png -or $png.Length -lt 30KB) {
        continue
    }

    try {
        $state = Get-Content $statePath -Raw | ConvertFrom-Json
        if ([int64]$state.tick -ge 500) {
            $readyIter = $candidate
            break
        }
    } catch {
        continue
    }
} while ((Get-Date) -lt $maxStopAt)

if ($readyIter) {
    Write-Host ">>> Loop capture ready: $($readyIter.Name)" -ForegroundColor Green
} else {
    Write-Host ">>> Loop capture did not reach ready state before timeout; stopping for health check" -ForegroundColor Yellow
}
$clientProc | Stop-Process -Force -ErrorAction SilentlyContinue
if ($serverProc) {
    $serverProc | Stop-Process -Force -ErrorAction SilentlyContinue
}
Start-Sleep -Seconds 1


Write-Host ""
Write-Host "=== Latest screenshots ===" -ForegroundColor Yellow

Get-ChildItem "$ProjectRoot\screenshots\iter_*\iter_*.png" -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending | Select-Object -First 5 |
    ForEach-Object { "  $($_.FullName.Substring($ProjectRoot.Length + 1)) ($($_.Length / 1KB | ForEach-Object {'{0:N1}KB' -f $_}))" }

Write-Host ""
Write-Host "=== Latest tick state ===" -ForegroundColor Yellow
Get-ChildItem "$ProjectRoot\screenshots\state_*.json" -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1 |
    ForEach-Object { "  $($_.Name)" }






$hcScript = Join-Path $ProjectRoot "scripts\loop\health_check.ps1"
if (Test-Path $hcScript) {
    Write-Host ""
    Write-Host "=== Health check ===" -ForegroundColor Yellow
    $iterDirs = Get-ChildItem "$ProjectRoot\screenshots\iter_*" -Directory -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending
    if ($iterDirs.Count -gt 0) {
        $latestIter = $iterDirs[0].FullName
        $prevIter = if ($iterDirs.Count -gt 1) { $iterDirs[1].FullName } else { "" }
        $hcArgs = @("-NoProfile", "-File", $hcScript, "-IterDir", $latestIter)
        if ($prevIter) { $hcArgs += @("-PrevDir", $prevIter) }
        $hcOut = & powershell @hcArgs 2>&1
        $hcExit = $LASTEXITCODE
        $hcColor = switch ($hcExit) {
            0 { if ($hcOut -match "PARTIAL") { "Yellow" } else { "Green" } }
            1 { "Red" }
            default { "Magenta" }
        }
        Write-Host "  $hcOut" -ForegroundColor $hcColor
        if ($hcExit -eq 1) {
            Write-Host "  [HEALTH FAIL] visual regression detected -- check health.json for reasons" -ForegroundColor Red
        } elseif ($hcExit -eq 2) {
            Write-Host "  [HEALTH ERR] health_check.py failed -- check Pillow / logs" -ForegroundColor Magenta
        }
    } else {
        Write-Host "  (no iter_* dir to check)" -ForegroundColor DarkGray
    }
} else {
    Write-Host ""
    Write-Host "=== Health check ===" -ForegroundColor Yellow
    Write-Host "  (scripts\loop\health_check.ps1 not found -- skipping)" -ForegroundColor DarkGray
}


if ($mode -in @("online","noserver")) {
    Write-Host ""
    Write-Host "=== Server log (last 8 lines) ===" -ForegroundColor Yellow
    if (Test-Path $serverLog) {
        Get-Content $serverLog -Tail 8
    } else {
        Write-Host "  (no server log file at $serverLog)" -ForegroundColor DarkGray
    }

    $serverCheckOk = (Test-Path $serverLog) -and (Select-String -Path $serverLog -Pattern "閼奉亝顥?*100 tick 閸忋劑鍎撮柅姘崇箖|Server UDP socket bound" -Quiet)
    if ($serverCheckOk) {
        Write-Host "  [OK] server self-check passed + socket bound" -ForegroundColor Green
    } else {
        Write-Host "  [WARN] server didn't print self-check pass / socket bound (see log above)" -ForegroundColor Yellow
    }
}

Write-Host ""
Write-Host ">>> Done. AI: read latest health.json first; if PARTIAL/FAIL, read assertions.json before PNG/state." -ForegroundColor Magenta


$latestIterDir = $null
$latestIterName = $null

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

