param(
    [ValidateSet("core", "changed", "client", "server", "workspace", "fmt", "clippy", "audit")]
    [string]$Scope = "core",
    [string]$TestName = "",
    [string]$BaseRef = "HEAD",
    [switch]$NoDefaultFeatures
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $ProjectRoot

$env:BEVY_DISABLE_ACCESSIBILITY = "1"
if (-not $env:RUST_LOG) {
    $env:RUST_LOG = "warn"
}

function Invoke-Step {
    param(
        [string]$Title,
        [string]$Command
    )

    Write-Host ""
    Write-Host ">>> $Title" -ForegroundColor Cyan
    Write-Host "    $Command" -ForegroundColor DarkGray
    $env:CHCP = "65001"
    cmd.exe /c "chcp 65001 >NUL && $Command"
    if ($LASTEXITCODE -ne 0) {
        Write-Host ">>> FAILED: $Title" -ForegroundColor Red
        exit $LASTEXITCODE
    }
}

function Cargo-TestCommand {
    param([string]$Package)

    $cmd = "cargo test -p $Package"
    if ($NoDefaultFeatures) {
        $cmd += " --no-default-features"
    }
    if ($TestName) {
        $cmd += " $TestName"
    }
    return $cmd
}

switch ($Scope) {
    "core" {
        Invoke-Step "core tests" (Cargo-TestCommand "lk2-core")
    }
    "client" {
        Invoke-Step "client tests" (Cargo-TestCommand "lk2-client")
        Invoke-Step "client build" "cargo build -p lk2-client --features dev-dynamic-linking"
    }
    "server" {
        Invoke-Step "server tests" (Cargo-TestCommand "lk2-server")
        Invoke-Step "server build" "cargo build -p lk2-server --features dev-dynamic-linking"
    }
    "workspace" {
        $cmd = "cargo test --workspace"
        if ($TestName) {
            $cmd += " $TestName"
        }
        Invoke-Step "workspace tests" $cmd
    }
    "fmt" {
        Invoke-Step "format check" "cargo fmt --check"
    }
    "clippy" {
        Invoke-Step "clippy" "cargo clippy --workspace --all-targets"
    }
    "audit" {
        & (Join-Path $ProjectRoot "scripts\ci\tdd_audit.ps1")
        if (-not $?) {
            Write-Host ">>> FAILED: test audit" -ForegroundColor Red
            exit 1
        }
        & (Join-Path $ProjectRoot "scripts\ci\architecture_audit.ps1")
        if (-not $?) {
            Write-Host ">>> FAILED: architecture audit" -ForegroundColor Red
            exit 1
        }
    }
    "changed" {
        $changed = git diff --name-only $BaseRef
        $changed += git ls-files --others --exclude-standard
        $changed = $changed | Where-Object { $_ }

        if (-not $changed) {
            Invoke-Step "core tests (no changed files detected)" (Cargo-TestCommand "lk2-core")
            break
        }

        $touchesCargo = $changed | Where-Object { $_ -eq "Cargo.toml" -or $_ -eq "Cargo.lock" -or $_ -like "crates/*/Cargo.toml" }
        $touchesCore = $changed | Where-Object { $_ -like "crates/core/*" -or $_ -eq "crates/core/Cargo.toml" }
        $touchesClient = $changed | Where-Object { $_ -like "crates/client/*" -or $_ -eq "crates/client/Cargo.toml" }
        $touchesServer = $changed | Where-Object { $_ -like "crates/server/*" -or $_ -eq "crates/server/Cargo.toml" }

        if ($touchesCore -or $touchesCargo) {
            Invoke-Step "core tests" (Cargo-TestCommand "lk2-core")
        }
        if ($touchesClient -or $touchesCargo) {
            Invoke-Step "client tests" (Cargo-TestCommand "lk2-client")
        }
        if ($touchesServer -or $touchesCargo) {
            Invoke-Step "server tests" (Cargo-TestCommand "lk2-server")
        }
        if (-not ($touchesCore -or $touchesClient -or $touchesServer -or $touchesCargo)) {
            Invoke-Step "format check for non-code changes" "cargo fmt --check"
        }
    }
}

Write-Host ""
Write-Host ">>> TDD scope '$Scope' passed" -ForegroundColor Green
