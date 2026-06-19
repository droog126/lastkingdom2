param(
    [string]$Root = "crates/core/src"
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $PSScriptRoot
Set-Location $ProjectRoot

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

exit 0
