# run_audit.ps1 - 启动 audit client (audit-pretty-models feature), 跑 N 秒后 kill
# 输出: screenshots/iter_*.png, screenshots/state_*.json
param(
    [int]$Seconds = 13
)

$ProjectRoot = "F:\rustProject\lastkingdom2"
Set-Location $ProjectRoot

$env:BEVY_DISABLE_ACCESSIBILITY = "1"
$env:RUST_LOG = "info"

# 1. 确保 hashed bevy_dylib 拷贝到 binary 目录
$depsDir = Join-Path $ProjectRoot "target\debug\deps"
$debugDir = Join-Path $ProjectRoot "target\debug"
$latestHashed = Get-ChildItem "$depsDir\bevy_dylib-*.dll" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
if ($latestHashed) {
    $destPath = Join-Path $debugDir $latestHashed.Name
    if (-not (Test-Path $destPath)) {
        Copy-Item $latestHashed.FullName $destPath -Force
        Write-Host ">>> copied $($latestHashed.Name) → target\debug\" -ForegroundColor Cyan
    }
}

# 2. 启 client
Write-Host ">>> starting lk2-client with audit-pretty-models feature for $Seconds s ..." -ForegroundColor Green
$proc = Start-Process -FilePath "cmd.exe" `
    -ArgumentList @("/c","cargo","run","-p","lk2-client","--features=dev-dynamic-linking,audit-pretty-models","--","--offline","--auto-demo") `
    -PassThru -NoNewWindow `
    -RedirectStandardOutput "screenshots\audit_run.out" `
    -RedirectStandardError "screenshots\audit_run.err" `
    -WorkingDirectory $ProjectRoot
Write-Host ">>> PID=$($proc.Id)"

Start-Sleep -Seconds $Seconds

$alive = Get-Process -Id $proc.Id -ErrorAction SilentlyContinue
if ($alive) {
    Write-Host ">>> killing after $Seconds s" -ForegroundColor Yellow
    # kill cargo + 它的子进程 lk2-client
    Get-Process -Name "lk2-client" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    $proc | Stop-Process -Force -ErrorAction SilentlyContinue
} else {
    Write-Host ">>> process already exited" -ForegroundColor Yellow
}

Start-Sleep -Seconds 2

Write-Host ""
Write-Host "=== latest iter_*.png ===" -ForegroundColor Yellow
Get-ChildItem "screenshots\iter_*\iter_*.png" -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 3 |
    ForEach-Object { "  $($_.FullName) ($($_.Length) bytes)" }

Write-Host ""
Write-Host "=== latest state_*.json ===" -ForegroundColor Yellow
Get-ChildItem "screenshots\state_*.json" -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 3 |
    ForEach-Object { "  $($_.Name)" }
