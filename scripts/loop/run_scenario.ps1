

param(
    [string]$Json = "scenarios/*.json",
    [int]$Seconds = 60,
    [switch]$SkipBuild = $false,
    [switch]$Offline = $true
)

$ProjectRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $ProjectRoot

$RunLogsDir = Join-Path $ProjectRoot "run-logs"
if (-not (Test-Path $RunLogsDir)) {
    New-Item -ItemType Directory -Path $RunLogsDir -Force | Out-Null
}
$BuildLog = Join-Path $RunLogsDir "build_scenario.log"
$clientLog = Join-Path $RunLogsDir "scenario_run.log"

$env:BEVY_DISABLE_ACCESSIBILITY = "1"
$env:RUST_LOG = $(if ($env:RUST_LOG) { $env:RUST_LOG } else { "info" })

$rustSysroot = (& rustc --print sysroot).Trim()
$runtimePaths = @(
    (Join-Path $ProjectRoot "target\debug\deps"),
    (Join-Path $ProjectRoot "target\debug"),
    (Join-Path $rustSysroot "bin")
) | Where-Object { Test-Path $_ }
$env:PATH = (($runtimePaths + @($env:PATH)) -join ";")

Get-Process -Name "lk2-client" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1

$clientExePath = Join-Path $ProjectRoot "target\debug\lk2-client.exe"
if (-not $SkipBuild -or -not (Test-Path $clientExePath)) {
    Write-Host ">>> cargo build -p lk2-client --features dev-dynamic-linking ..." -ForegroundColor Cyan
    cmd /c "cargo build -p lk2-client --features dev-dynamic-linking 2>&1" |
        Tee-Object -FilePath $BuildLog | Select-Object -Last 5
    if ($LASTEXITCODE -ne 0) { Write-Host ">>> BUILD FAILED" -ForegroundColor Red; exit 1 }
}

$jsonFiles = Get-Item $Json -ErrorAction SilentlyContinue
if (-not $jsonFiles) {
    Write-Host ">>> 没有匹配的 JSON: $Json" -ForegroundColor Red
    exit 1
}

$debugDir = Split-Path -Parent $clientExePath
$candidateDlls = Get-ChildItem "$debugDir\bevy_dylib-*.dll" -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending
if ($candidateDlls.Count -gt 0) {
    $vanilla = Join-Path $debugDir "bevy_dylib.dll"
    if (-not (Test-Path $vanilla)) {
        Copy-Item $candidateDlls[0].FullName $vanilla -Force
        Write-Host ">>> copied bevy_dylib.dll (vanilla alias)" -ForegroundColor DarkGray
    }
}

foreach ($jf in $jsonFiles) {
    $relJson = $jf.FullName.Substring($ProjectRoot.Length + 1)
    $baseName = [System.IO.Path]::GetFileNameWithoutExtension($jf.Name)
    Write-Host ""
    Write-Host ">>> Running scenario: $relJson" -ForegroundColor Green

    $proc = Start-Process -FilePath $clientExePath `
        -ArgumentList @("--offline", $jf.FullName) `
        -PassThru -NoNewWindow `
        -RedirectStandardOutput $clientLog `
        -RedirectStandardError "$clientLog.err" `
        -WorkingDirectory $ProjectRoot

    $deadline = (Get-Date).AddSeconds($Seconds)
    while ((Get-Date) -lt $deadline) {
        if ($proc.HasExited) { break }
        Start-Sleep -Seconds 1
    }
    if (-not $proc.HasExited) {
        Write-Host ">>> 超时 $Seconds s, 强杀" -ForegroundColor Yellow
        $proc | Stop-Process -Force -ErrorAction SilentlyContinue
        Get-Process -Name "lk2-client" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    }
    Start-Sleep -Seconds 1
}

Write-Host ""
Write-Host "=== Latest scenario screenshots ===" -ForegroundColor Yellow
Get-ChildItem "$ProjectRoot\screenshots\scenario_*" -Directory -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending | Select-Object -First 3 |
    ForEach-Object { "  $($_.FullName.Substring($ProjectRoot.Length + 1))" }

Write-Host ""
Write-Host ">>> Done. 看 screenshots/scenario_<name>/ 下的截图和 record_*.jsonl" -ForegroundColor Magenta
