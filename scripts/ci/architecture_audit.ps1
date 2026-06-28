

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $ProjectRoot

$failed = $false

function Fail {
    param([string]$Message)
    $script:failed = $true
    Write-Host "FAIL: $Message" -ForegroundColor Red
}

function Warn {
    param([string]$Message)
    Write-Host "WARN: $Message" -ForegroundColor Yellow
}

function Pass {
    param([string]$Message)
    Write-Host "OK: $Message" -ForegroundColor Green
}

Write-Host "# Architecture audit" -ForegroundColor Cyan
Write-Host ""

$scriptFiles = @(
    Get-ChildItem scripts -Recurse -File -Filter "*.ps1" | ForEach-Object { $_.FullName }
    "loop.ps1"
    "tdd.ps1"
)
$hardcodedPathHits = @()
foreach ($file in $scriptFiles) {
    $hits = Select-String -Path $file -Pattern '[A-Za-z]:\\' -ErrorAction SilentlyContinue
    foreach ($hit in $hits) {
        $hardcodedPathHits += "{0}:{1}" -f $file, $hit.LineNumber
    }
}
if ($hardcodedPathHits.Count -gt 0) {
    Fail "scripts contain hard-coded absolute paths: $($hardcodedPathHits -join ', ')"
} else {
    Pass "scripts are free of hard-coded Windows absolute paths"
}

$rootScriptFiles = @(
    Get-ChildItem scripts -File -Filter "*.ps1" -ErrorAction SilentlyContinue | ForEach-Object { $_.Name }
    if (Test-Path "run_audit.ps1") { "run_audit.ps1" }
)
if ($rootScriptFiles.Count -gt 0) {
    Fail "active scripts must live under scripts/loop, scripts/dev, scripts/ci, or scripts/maintenance: $($rootScriptFiles -join ', ')"
} else {
    Pass "active scripts are grouped by role"
}

$legacyRoots = @()
foreach ($legacyRoot in @("document", "launchers")) {
    if (Test-Path $legacyRoot) {
        $legacyRoots += $legacyRoot
    }
}
if ($legacyRoots.Count -gt 0) {
    $sampleLegacyRoots = @($legacyRoots | Select-Object -First 8) -join ', '
    Fail "legacy root folders still tracked: $sampleLegacyRoots"
} else {
    Pass "legacy document/ and launchers/ roots are folded into docs/"
}

$clientMain = "crates/client/src/main.rs"
if (Test-Path $clientMain) {
    $mainText = Get-Content $clientMain -Raw
    $objectiveSetupCount = ([regex]::Matches($mainText, 'lk2_core::objectives::setup_default_objectives')).Count
    if ($objectiveSetupCount -gt 1) {
        Fail "client startup registers setup_default_objectives $objectiveSetupCount times"
    } else {
        Pass "client startup objective setup is registered once"
    }
}

$forbiddenRootPatterns = @("*.log", "*.err", "state_*.json", "iter_*.png")
$rootNoise = @()
foreach ($pattern in $forbiddenRootPatterns) {
    $rootNoise += Get-ChildItem -Path $ProjectRoot -File -Filter $pattern -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -ne "Cargo.lock" } |
        ForEach-Object { $_.Name }
}
if ($rootNoise.Count -gt 0) {
    Fail "runtime/build output exists in repo root: $($rootNoise -join ', ')"
} else {
    Pass "repo root has no runtime/build output files"
}

$largeRustFiles = @(
    Get-ChildItem crates -Recurse -Filter "*.rs" |
        Where-Object { (Get-Content $_.FullName).Count -gt 1200 } |
        ForEach-Object {
            $rel = Resolve-Path -Relative $_.FullName
            "{0} ({1} lines)" -f $rel.TrimStart(".\"), (Get-Content $_.FullName).Count
        }
)
if ($largeRustFiles.Count -gt 0) {
    Warn "large Rust modules should be split: $($largeRustFiles -join '; ')"
} else {
    Pass "no Rust module exceeds 1200 lines"
}

if ($failed) {
    Write-Host ""
    Write-Host "Architecture audit failed" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "Architecture audit passed" -ForegroundColor Green
