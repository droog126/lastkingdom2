$tokens = $null
$errors = $null
$path = $args[0]
[System.Management.Automation.Language.Parser]::ParseFile($path, [ref]$tokens, [ref]$errors) | Out-Null
if ($errors) {
    Write-Host "PARSE ERRORS in $path" -ForegroundColor Red
    foreach ($err in $errors) {
        Write-Host "  $($err.Extent.StartLineNumber):$($err.Extent.StartColumnNumber)  $($err.Message)"
    }
    exit 1
} else {
    Write-Host "OK: $path"
    exit 0
}