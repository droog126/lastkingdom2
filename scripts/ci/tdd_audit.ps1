param(
    [string]$Root = "crates/core/src"
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $ProjectRoot

function Test-DecisionTemplateSync {
    $loopPath = Join-Path $ProjectRoot "scripts\loop\loop.ps1"
    $agentPath = Join-Path $ProjectRoot "AGENTS.md"

    if (-not (Test-Path $loopPath) -or -not (Test-Path $agentPath)) {
        Write-Host "decision template audit skipped: scripts/loop/loop.ps1 or AGENTS.md missing" -ForegroundColor Yellow
        return
    }

    $loopContent = Get-Content $loopPath -Raw
    $requiredFragments = @(
        "decision",
        "task:",
        "result: pass / partial / fail",
        "score:",
        "- sky: X/10",
        "- player: X/10",
        "- terrain: X/10",
        "- decor: X/10",
        "- hud: X/10",
        "- gameplay: X/10",
        "- total: X.X/10",
        "vs_prev:",
        "- visual: improved / same / worse",
        "- state: ",
        "problems:",
        "tests:",
        "next:"
    )

    $missing = @($requiredFragments | Where-Object { -not $loopContent.Contains($_) })
    if ($missing.Count -gt 0) {
        Write-Host ""
        Write-Host "## Decision template drift" -ForegroundColor Red
        foreach ($fragment in $missing) {
            Write-Host ("- missing in scripts/loop/loop.ps1: {0}" -f $fragment)
        }
        exit 1
    }

    if (-not ($loopContent -match '# \$latestIterName decision')) {
        Write-Host ""
        Write-Host "## Decision template drift" -ForegroundColor Red
        Write-Host "- missing dynamic decision heading in scripts/loop/loop.ps1"
        exit 1
    }

    $agentContent = Get-Content $agentPath -Raw
    $agentMissing = @($requiredFragments | Where-Object { -not $agentContent.Contains($_) })
    if ($agentMissing.Count -gt 0) {
        Write-Host ""
        Write-Host "## Decision template source mismatch" -ForegroundColor Red
        foreach ($fragment in $agentMissing) {
            Write-Host ("- missing in AGENTS.md: {0}" -f $fragment)
        }
        exit 1
    }

    if (-not ($agentContent -match '# iter_NN decision')) {
        Write-Host ""
        Write-Host "## Decision template source mismatch" -ForegroundColor Red
        Write-Host "- missing canonical decision heading in AGENTS.md"
        exit 1
    }

    Write-Host ""
    Write-Host "## Decision template sync" -ForegroundColor Yellow
    Write-Host "- scripts/loop/loop.ps1 decision template matches AGENTS.md required fields"
}

function Test-FinalStateContract {
    $diagnosticsPath = Join-Path $ProjectRoot "crates\core\src\diagnostics.rs"
    if (-not (Test-Path $diagnosticsPath)) {
        Write-Host "final_state contract audit skipped: diagnostics.rs missing" -ForegroundColor Yellow
        return
    }

    $diagnosticsContent = Get-Content $diagnosticsPath -Raw
    $requiredFragments = @(
        '"tick"',
        '"player"',
        '"pool"',
        '"monsters"',
        '"creatures"',
        '"observer"',
        '"world"',
        '["creatures", "passive_current"]'
    )

    $missing = @($requiredFragments | Where-Object { -not $diagnosticsContent.Contains($_) })
    if ($missing.Count -gt 0) {
        Write-Host ""
        Write-Host "## Final state contract drift" -ForegroundColor Red
        foreach ($fragment in $missing) {
            Write-Host ("- missing in diagnostics.rs: {0}" -f $fragment)
        }
        exit 1
    }

    Write-Host ""
    Write-Host "## Final state contract" -ForegroundColor Yellow
    Write-Host "- diagnostics.rs exports monster and creature fields for final_state.json"
}

if (-not (Test-Path $Root)) {
    Write-Host "Missing audit root: $Root" -ForegroundColor Red
    exit 1
}

$files = Get-ChildItem -Path $Root -Recurse -Filter "*.rs" |
    Sort-Object FullName

$rows = foreach ($file in $files) {
    $relative = Resolve-Path -Relative $file.FullName
    $testMatches = Select-String -Path $file.FullName -Pattern "#[test]" -SimpleMatch
    $testsModuleMatches = Select-String -Path $file.FullName -Pattern "mod tests" -SimpleMatch
    [pscustomobject]@{
        File = $relative.TrimStart(".\")
        Tests = @($testMatches).Count
        HasTestsModule = @($testsModuleMatches).Count -gt 0
    }
}

$totalTests = ($rows | Measure-Object -Property Tests -Sum).Sum
$filesWithTests = @($rows | Where-Object { $_.Tests -gt 0 }).Count
$filesWithoutTests = @($rows | Where-Object { $_.Tests -eq 0 }).Count

Write-Host "# TDD audit" -ForegroundColor Cyan
Write-Host ""
Write-Host ("root: {0}" -f $Root)
Write-Host ("files: {0}" -f @($rows).Count)
Write-Host ("files_with_tests: {0}" -f $filesWithTests)
Write-Host ("files_without_tests: {0}" -f $filesWithoutTests)
Write-Host ("unit_tests_found: {0}" -f $totalTests)
Write-Host ""

Write-Host "## Modules without direct unit tests" -ForegroundColor Yellow
$missing = @($rows | Where-Object { $_.Tests -eq 0 })
if ($missing.Count -eq 0) {
    Write-Host "- none"
} else {
    foreach ($row in $missing) {
        Write-Host ("- {0}" -f $row.File)
    }
}

Write-Host ""
Write-Host "## Test counts by module" -ForegroundColor Yellow
$rows |
    Sort-Object Tests, File -Descending |
    ForEach-Object {
        Write-Host ("- {0}: {1}" -f $_.File, $_.Tests)
    }

if ($totalTests -eq 0) {
    exit 1
}

Test-DecisionTemplateSync
Test-FinalStateContract

exit 0

