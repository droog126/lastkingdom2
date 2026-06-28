param(
    [string]$Ref = "HEAD"
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $ProjectRoot

$lockContent = git show "${Ref}:Cargo.lock"
$lockContent | Select-String -Pattern 'name = "bevy"' | Select-Object -First 5
"---"
$lockContent |
    Select-String -Pattern 'name = "(avian3d|lightyear|leafwing|bevy_egui|fastnoise|tracing|rkyv|postcard|sled|rhai|dashmap|criterion|zstd|compt)"' |
    Select-Object -First 20
