Set-Location F:\rustProject\lastkingdom2
# Get original Cargo.lock content
$origContent = git show HEAD:Cargo.lock
$origContent | Select-String -Pattern 'name = "bevy"' | Select-Object -First 5
"---"
$origContent | Select-String -Pattern 'name = "(avian3d|lightyear|leafwing|bevy_egui|fastnoise|tracing|rkyv|postcard|sled|rhai|dashmap|criterion|zstd|compt)"' | Select-Object -First 20
