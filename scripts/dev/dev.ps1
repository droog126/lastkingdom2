

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true, Position = 0)]
    [ValidateSet("build", "test", "core", "clippy", "fmt", "loop", "health", "clean", "help")]
    [string]$Command,

    [Parameter(Position = 1, ValueFromRemainingArguments = $true)]
    [string[]]$Rest
)

$ProjectRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $ProjectRoot

$featureArgs = @("--features", "dev-dynamic-linking,lk2-core/dev-dynamic-linking")

function Invoke-Cargo {
    param([string[]]$Args)
    Write-Host ">>> cargo $($Args -join ' ')" -ForegroundColor Cyan
    cargo @Args
    return $LASTEXITCODE
}

switch ($Command) {
    "build" {
        $code = Invoke-Cargo (@("build", "-p", "lk2-client") + $featureArgs)
        exit $code
    }
    "test" {
        $code = Invoke-Cargo @("test", "--workspace")
        exit $code
    }
    "core" {
        $code = Invoke-Cargo @("test", "-p", "lk2-core")
        exit $code
    }
    "clippy" {
        $code = Invoke-Cargo @("clippy", "--workspace", "--", "-D", "warnings")
        exit $code
    }
    "fmt" {
        $code = Invoke-Cargo @("fmt", "--all", "--", "--check")
        exit $code
    }
    "loop" {
        & powershell -NoProfile -File (Join-Path $ProjectRoot "loop.ps1") @Rest
        exit $LASTEXITCODE
    }
    "health" {

        $target = if ($Rest.Count -gt 0) { $Rest[0] } else {
            $latest = Get-ChildItem "$ProjectRoot\screenshots\iter_*" -Directory -ErrorAction SilentlyContinue |
                Sort-Object LastWriteTime -Descending | Select-Object -First 1
            if ($latest) { $latest.Name } else {
                Write-Error "no iter_* dir found"
                exit 1
            }
        }
        $iterDir = Join-Path $ProjectRoot "screenshots\$target"
        if (-not (Test-Path $iterDir)) {
            Write-Error "iter dir not found: $iterDir"
            exit 1
        }
        & powershell -NoProfile -File (Join-Path $ProjectRoot "scripts\loop\health_check.ps1") `
            -IterDir $iterDir
        exit $LASTEXITCODE
    }
    "clean" {
        Write-Host ">>> cargo clean (target/)" -ForegroundColor Yellow
        $confirm = Read-Host "Are you sure? [y/N]"
        if ($confirm -eq "y" -or $confirm -eq "Y") {
            cargo clean
        } else {
            Write-Host "aborted" -ForegroundColor DarkGray
        }
        exit 0
    }
    "help" {
        Write-Host "scripts/dev/dev.ps1 - one-stop dev commands" -ForegroundColor Cyan
        Write-Host ""
        Write-Host "Commands:"
        Write-Host "  build   - incremental build lk2-client (~1s with dyn-linking)"
        Write-Host "  test    - cargo test --workspace"
        Write-Host "  core    - cargo test -p lk2-core (fastest)"
        Write-Host "  clippy  - cargo clippy --workspace -- -D warnings"
        Write-Host "  fmt     - cargo fmt --all -- --check"
        Write-Host "  loop    - run loop.ps1 (forwards extra args)"
        Write-Host "  health  - run health_check on latest (or specified) iter"
        Write-Host "  clean   - cargo clean (asks first)"
        Write-Host "  help    - this message"
        Write-Host ""
        Write-Host "Examples:"
        Write-Host "  pwsh -File scripts/dev/dev.ps1 build"
        Write-Host "  pwsh -File scripts/dev/dev.ps1 loop -Offline -Seconds 12"
        Write-Host "  pwsh -File scripts/dev/dev.ps1 health iter_385"
        exit 0
    }
}
