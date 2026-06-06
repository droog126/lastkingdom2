# loop.ps1 - WANGUO ORIGINS demo closed-loop iteration script
# Usage: powershell -File loop.ps1
# Each loop: build (if needed) -> run 12s -> kill -> list new screenshots

param(
    [int]$Seconds = 12,
    [switch]$SkipBuild = $false
)

$ProjectRoot = $PSScriptRoot
Set-Location $ProjectRoot

$env:BEVY_DISABLE_ACCESSIBILITY = "1"
$env:RUST_LOG = "info"

# 0. Kill any old process
Get-Process -Name "minecraft_bevy" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1

# 1. Build
if (-not $SkipBuild) {
    Write-Host ">>> cargo build ..." -ForegroundColor Cyan
    cargo build 2>&1 | Tee-Object -FilePath "build_loop.log" | Select-Object -Last 5
    if ($LASTEXITCODE -ne 0) {
        Write-Host ">>> BUILD FAILED" -ForegroundColor Red
        exit 1
    }
}

# 2. Run + screenshot + record
$exePath = Join-Path $ProjectRoot "target\debug\minecraft_bevy.exe"
if (-not (Test-Path $exePath)) {
    Write-Host ">>> Binary not found: $exePath" -ForegroundColor Red
    exit 1
}

Write-Host ">>> Start demo (${Seconds}s, --auto-demo) ..." -ForegroundColor Green
$logPath = Join-Path $ProjectRoot "screenshots\loop_run.log"
$proc = Start-Process -FilePath $exePath -ArgumentList "--auto-demo" -PassThru -NoNewWindow -RedirectStandardOutput $logPath -RedirectStandardError "$logPath.err"
Start-Sleep -Seconds $Seconds
$proc | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1

# 3. List results
Write-Host ""
Write-Host "=== Latest screenshots ===" -ForegroundColor Yellow
Get-ChildItem "$ProjectRoot\screenshots\iter_*.png" -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending | Select-Object -First 5 |
    ForEach-Object { "  $($_.Name) ($($_.Length / 1KB | ForEach-Object {'{0:N1}KB' -f $_}))" }

Write-Host ""
Write-Host "=== Latest tick state ===" -ForegroundColor Yellow
Get-ChildItem "$ProjectRoot\screenshots\state_*.json" -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1 |
    ForEach-Object { "  $($_.Name)" }

Write-Host ""
Write-Host ">>> Done. AI: read latest screenshot + state JSON, decide next round" -ForegroundColor Magenta
