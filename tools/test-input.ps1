param(
    [switch]$Launch,
    [switch]$Online,
    [switch]$FirstPerson,
    [switch]$AllowFocus,
    [int]$HoldMillis = 2200,
    [int]$LookHoldMillis = 900,
    [int]$StartupTimeoutSeconds = 80,
    [int]$StateTimeoutSeconds = 25,
    [double]$MinDistance = 0.25
)

$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName Microsoft.VisualBasic
Add-Type -Path (Join-Path $PSScriptRoot '.input-test\Win32.cs')

$Root = Resolve-Path (Join-Path $PSScriptRoot '..')
$OutDir = Join-Path $PSScriptRoot '.input-test'
$ScreenshotsDir = Join-Path $Root 'screenshots'
$ClientExe = Join-Path $Root 'target\debug\lk2-client.exe'
$SW_RESTORE = 9
$StartedPid = $null

function Ensure-Dir([string]$Path) {
    if (-not (Test-Path $Path)) {
        New-Item -ItemType Directory -Path $Path | Out-Null
    }
}

function Remove-Old-StateFiles {
    foreach ($file in Get-ChildItem $ScreenshotsDir -Filter 'state_t*.json' -ErrorAction SilentlyContinue) {
        Remove-Item -LiteralPath $file.FullName -Force -ErrorAction SilentlyContinue
    }
    foreach ($file in Get-ChildItem $ScreenshotsDir -Filter 'server_state_t*.json' -ErrorAction SilentlyContinue) {
        Remove-Item -LiteralPath $file.FullName -Force -ErrorAction SilentlyContinue
    }
}

function Focus-Window([IntPtr]$hWnd, [string]$title) {
    if (-not $AllowFocus) {
        return
    }
    [void][Native.Win32]::ShowWindowAsync($hWnd, $SW_RESTORE)
    [void][Native.Win32]::SetForegroundWindow($hWnd)
    Start-Sleep -Milliseconds 250
    try { [void][Microsoft.VisualBasic.Interaction]::AppActivate($title) } catch { }
    Start-Sleep -Milliseconds 250
}

function Send-Key([UInt16]$vk, [bool]$up) {
    if (-not $AllowFocus) {
        throw "Refusing to send key input without -AllowFocus because it can steal the active window."
    }
    if ($up) {
        [void][Native.Win32]::KeyUp($vk)
    } else {
        [void][Native.Win32]::KeyDown($vk)
    }
}

function Hold-Key([string]$key, [int]$millis) {
    $vkMap = @{ W = 0x57; A = 0x41; S = 0x53; D = 0x44; R = 0x52; T = 0x54; PageUp = 0x21; PageDown = 0x22 }
    if (-not $vkMap.ContainsKey($key)) { throw "Unsupported key: $key" }
    $vk = [UInt16]$vkMap[$key]
    Send-Key $vk $false
    Start-Sleep -Milliseconds $millis
    Send-Key $vk $true
}

function Tap-Key([string]$key) {
    # For keys that use just_pressed semantics (F5, F8, etc.).
    $vkMap = @{ F5 = 0x74; F8 = 0x77; F6 = 0x75; F11 = 0x7A }
    if (-not $vkMap.ContainsKey($key)) { throw "Unsupported tap key: $key" }
    $vk = [UInt16]$vkMap[$key]
    Send-Key $vk $false
    Start-Sleep -Milliseconds 60
    Send-Key $vk $true
}

function Move-Mouse([int]$dx, [int]$dy) {
    [Native.Win32]::MouseMove($dx, $dy)
}

function Latest-StateFile([datetime]$after) {
    $filter = if ($Online) { 'server_state_t*.json' } else { 'state_t*.json' }
    Get-ChildItem $ScreenshotsDir -Filter $filter -ErrorAction SilentlyContinue |
        Where-Object { $_.LastWriteTime -ge $after } |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
}

function Wait-State([datetime]$after, [int]$timeoutSeconds) {
    $deadline = (Get-Date).AddSeconds($timeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        $file = Latest-StateFile $after
        if ($file) {
            $json = Get-Content -Raw $file.FullName | ConvertFrom-Json
            return [pscustomobject]@{ File = $file.FullName; Json = $json }
        }
        Start-Sleep -Milliseconds 250
    }
    throw "Timed out waiting for state_t*.json after $after"
}

function Player-Pos($state) {
    $p = $state.Json.player.pos
    return [double[]]@($p[0], $p[1], $p[2])
}

function Player-BlockPos($state) {
    $p = $state.Json.player.block_pos
    return [int[]]@($p[0], $p[1], $p[2])
}

function Distance3([double[]]$a, [double[]]$b) {
    $dx = $b[0] - $a[0]
    $dy = $b[1] - $a[1]
    $dz = $b[2] - $a[2]
    return [Math]::Sqrt($dx * $dx + $dy * $dy + $dz * $dz)
}

function Camera-Summary($state) {
    if ($null -eq $state.Json.camera) { return "(no camera field)" }
    $cam = $state.Json.camera
    $hit = $cam.center_ray_hit
    $hitText = if ($null -eq $hit) {
        "hit=null"
    } else {
        "hit=$($hit.block) block=[$($hit.block_pos -join ', ')] dist=$('{0:N2}' -f [double]$hit.distance)"
    }
    return "mode=$($cam.mode) yaw=$('{0:N3}' -f [double]$cam.yaw) pitch=$('{0:N3}' -f [double]$cam.pitch) $hitText"
}

Ensure-Dir $OutDir
Ensure-Dir $ScreenshotsDir
Remove-Old-StateFiles

if ($Launch) {
    if (-not (Test-Path $ClientExe)) {
        throw "Missing $ClientExe. Run cargo build -p lk2-client first."
    }
    Get-Process -Name lk2-client -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Milliseconds 500
    $env:BEVY_DISABLE_ACCESSIBILITY = '1'
    $env:LK2_SCREENSHOT_INTERVAL = '60'
    $env:RUST_LOG = 'info'
    $args = if ($Online) {
        @('--connect=127.0.0.1:5000')
    } else {
        @('--offline', '--input-test-idle')
    }
    if ($FirstPerson) {
        $args += '--first-person'
    }
    $started = Start-Process -FilePath $ClientExe -ArgumentList $args -WorkingDirectory $Root -WindowStyle Normal -PassThru
    $StartedPid = $started.Id
}

$deadline = (Get-Date).AddSeconds($StartupTimeoutSeconds)
$proc = $null
while ((Get-Date) -lt $deadline) {
    if ($StartedPid) {
        $proc = Get-Process -Id $StartedPid -ErrorAction SilentlyContinue
        if ($proc -and ($proc.MainWindowHandle -eq 0 -or [string]::IsNullOrWhiteSpace($proc.MainWindowTitle))) {
            $proc = $null
        }
    } else {
        $proc = Get-Process -Name lk2-client -ErrorAction SilentlyContinue |
            Where-Object { $_.MainWindowHandle -ne 0 -and -not [string]::IsNullOrWhiteSpace($_.MainWindowTitle) } |
            Sort-Object StartTime -Descending |
            Select-Object -First 1
    }
    if ($proc) { break }
    Start-Sleep -Milliseconds 500
}
if (-not $proc) {
    throw "lk2-client window not found within ${StartupTimeoutSeconds}s"
}

$hWnd = $proc.MainWindowHandle
$title = $proc.MainWindowTitle
"PID=$($proc.Id) hWnd=$hWnd title=[$title] allow_focus=$AllowFocus"
Focus-Window $hWnd $title

$baseline = Wait-State (Get-Date).AddSeconds(-1) $StateTimeoutSeconds
$beforePos = Player-Pos $baseline
$beforeBlock = Player-BlockPos $baseline
"before file=$($baseline.File) tick=$($baseline.Json.tick) pos=[$($beforePos -join ', ')] block=[$($beforeBlock -join ', ')]"
"before camera $(Camera-Summary $baseline)"

if ($FirstPerson -and -not $Online) {
    if ($null -eq $baseline.Json.camera.center_ray_hit -or $baseline.Json.camera.center_ray_hit.block -ne 'Leaves') {
        throw "FAIL: first-person center ray did not hit grass platform. $(Camera-Summary $baseline)"
    }
    $beforePos = Player-Pos $baseline
}

if ((-not $Online) -and $AllowFocus) {
    # Push the player to a clear spawn via F5 (in-game `emergency_teleport`)
    # so we are not stuck inside a structure that the default scenario dropped us in.
    Focus-Window $hWnd $title
    Tap-Key 'F5'
    Start-Sleep -Milliseconds 1200

    $afterTeleport = Wait-State (Get-Item $baseline.File).LastWriteTime.AddMilliseconds(1) $StateTimeoutSeconds
    $teleportPos = Player-Pos $afterTeleport
    $teleportBlock = Player-BlockPos $afterTeleport
    "after F5 file=$($afterTeleport.File) tick=$($afterTeleport.Json.tick) pos=[$($teleportPos -join ', ')] block=[$($teleportBlock -join ', ')]"
} else {
    $afterTeleport = $baseline
    $teleportPos = $beforePos
}

if ($AllowFocus) {
    Focus-Window $hWnd $title
    Hold-Key 'W' $HoldMillis
    Start-Sleep -Milliseconds 900

    $afterW = Wait-State (Get-Item $afterTeleport.File).LastWriteTime.AddMilliseconds(1) $StateTimeoutSeconds
    $afterWPos = Player-Pos $afterW
    $afterWBlock = Player-BlockPos $afterW
    $distW = Distance3 $teleportPos $afterWPos
    "after W file=$($afterW.File) tick=$($afterW.Json.tick) pos=[$($afterWPos -join ', ')] block=[$($afterWBlock -join ', ')] dist=$('{0:N3}' -f $distW)"

    Focus-Window $hWnd $title
    Hold-Key 'D' $HoldMillis
    Start-Sleep -Milliseconds 900

    $afterD = Wait-State (Get-Item $afterW.File).LastWriteTime.AddMilliseconds(1) $StateTimeoutSeconds
    $afterDPos = Player-Pos $afterD
    $afterDBlock = Player-BlockPos $afterD
    $distD = Distance3 $afterWPos $afterDPos
    $distTotal = Distance3 $teleportPos $afterDPos
    "after D file=$($afterD.File) tick=$($afterD.Json.tick) pos=[$($afterDPos -join ', ')] block=[$($afterDBlock -join ', ')] dist=$('{0:N3}' -f $distD) total=$('{0:N3}' -f $distTotal)"

    if ($distW -lt $MinDistance) {
        throw "FAIL: W did not move forward enough. min=$MinDistance W=$distW D=$distD total=$distTotal"
    }

    "PASS: W moved player coordinates. W=$('{0:N3}' -f $distW) total=$('{0:N3}' -f $distTotal)"
} else {
    "PASS: observed state without stealing focus. Input movement checks skipped; use pure examples or pass -AllowFocus explicitly."
}
