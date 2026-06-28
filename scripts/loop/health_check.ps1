

param(
    [Parameter(Mandatory = $true)]
    [string]$IterDir,
    [string]$PrevDir = "",
    [switch]$Quiet = $false
)

$ProjectRoot = $PSScriptRoot | Split-Path -Parent
$Script = Join-Path $PSScriptRoot "health_check.py"

if (-not (Test-Path $Script)) {
    Write-Error "health_check.py not found at $Script"
    exit 2
}

if (-not (Test-Path $IterDir)) {
    Write-Error "IterDir not found: $IterDir"
    exit 2
}

$py = (Get-Command python -ErrorAction SilentlyContinue).Source
if (-not $py) {
    Write-Error "python not on PATH; install Python 3.10+ with Pillow"
    exit 2
}

$argList = @($Script, $IterDir)
if ($PrevDir -and (Test-Path $PrevDir)) {
    $argList += $PrevDir
}

$proc = Start-Process -FilePath $py -ArgumentList $argList -NoNewWindow -Wait -PassThru `
    -RedirectStandardOutput "$IterDir\health_stdout.tmp" `
    -RedirectStandardError "$IterDir\health_stderr.tmp"
$stdout = Get-Content "$IterDir\health_stdout.tmp" -Raw
$stderr = Get-Content "$IterDir\health_stderr.tmp" -Raw
Remove-Item "$IterDir\health_stdout.tmp", "$IterDir\health_stderr.tmp" -ErrorAction SilentlyContinue

if ($proc.ExitCode -ne 0) {
    Write-Host "[health_check] python exited $($proc.ExitCode): $stderr" -ForegroundColor Red
    exit 2
}

$stdout | Out-File -FilePath "$IterDir\health.txt" -Encoding UTF8
if (-not $Quiet) {
    Write-Host $stdout.TrimEnd() -ForegroundColor Cyan
}

$json = Get-Content "$IterDir\health.json" -Raw | ConvertFrom-Json
switch ($json.verdict) {
    "PASS" { exit 0 }
    "PARTIAL" { exit 0 }
    "FAIL" { exit 1 }
    default { exit 1 }
}
