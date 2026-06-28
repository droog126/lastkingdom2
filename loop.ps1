param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$Args
)

$ProjectRoot = $PSScriptRoot
$Script = Join-Path $ProjectRoot "scripts\loop\loop.ps1"
& powershell -NoProfile -File $Script @Args
exit $LASTEXITCODE
