# loop.ps1 - WANGUO ORIGINS demo closed-loop iteration script
# Usage: powershell -File loop.ps1
# Each loop: build (if needed) -> run 12s -> kill -> list new screenshots
#
# 默认 (2026-06-11 wire-network-and-loop 任务后): 启 lk2-server + lk2-client
# 走 `--connect=127.0.0.1:5000` 双进程联机模式, 验证 client 能连 server
# 跑 sim tick + 截图。
# 单机模式 (--offline) 还在: 传 `-Offline` 切回, 或 `-NoServer` 启 client
# 但不启 server (client --connect= 但没 server 在听, 会 connect 失败)。

param(
    # 默认 30s: UDP connect 握手 + lightyear replication 第一次 tick 推到 client 端
    # 需要 12~30s。wire-network-and-loop 任务早期 12s 没看到 [net] applied PlayerPos
    # log 主要是 handshake 还没完成。30s 给 lightyear NetcodeClientPlugin + ReplicationReceiver
    # 留足时间 (首次 connect + 首次 heartbeat + 首次 component tick = ~3 个 RTT)。
    [int]$Seconds = 60,
    [string]$RUST_LOG = $(if ($env:RUST_LOG) { $env:RUST_LOG } else { "info,lightyear_replication=debug,lightyear_connection=debug,lightyear_send=debug,lightyear_receive=debug" }),
    # 默认 SkipBuild=true: 冷编译 lightyear 0.26 + leafwing 需要 22+ 分钟,
    # 超过 30 min cap 装不下, 也撞 lightyear + bevy 0.18 API drift 编译错。
    # 只在 binary 已经编过、增量 build 时才用 -Build
    [switch]$SkipBuild = $false,
    # 启用 Bevy 动态链接 (开发期增量 build 快, binary 会动态加载 lib 而非静态链接)
    [switch]$Dynamic = $true,
    # 联机模式 (默认 $true): 同时启 lk2-server + lk2-client --connect=...
    # 离线模式: 只启 lk2-client --offline (旧行为)
    [switch]$Offline = $false,
    # 不启 server (默认 false). 用 -NoServer 跑 client --connect= 但没 server,
    # 用于 debug client transport 行为。
    [switch]$NoServer = $false,
    # 联机模式 client 连的地址 (默认 127.0.0.1:5000, 跟 lk2-core::transport::DEFAULT_PORT)
    [string]$ServerAddr = "127.0.0.1:5000"
)

$ProjectRoot = $PSScriptRoot
Set-Location $ProjectRoot

$env:BEVY_DISABLE_ACCESSIBILITY = "1"
# RUST_LOG 已在 param 默认值里塞好 (默认读 $env:RUST_LOG 否则用 lightyear debug 默认)
# 强制写一次到 env var, 给子进程继承
$env:RUST_LOG = $RUST_LOG
$rustSysroot = (& rustc --print sysroot).Trim()
$runtimePaths = @(
    (Join-Path $ProjectRoot "target\debug\deps"),
    (Join-Path $ProjectRoot "target\debug"),
    (Join-Path $rustSysroot "bin")
) | Where-Object { Test-Path $_ }
$env:PATH = (($runtimePaths + @($env:PATH)) -join ";")

# 0. Kill any old lk2-client / lk2-server processes (loop 之前清场)
Get-Process -Name "lk2-client","lk2-server" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1

# 1. Build (default 跳过, 改用 -Build flag 显式打开)
$featureArgs = if ($Dynamic) { "--features dev-dynamic-linking" } else { "" }
if ($Dynamic) { Write-Host ">>> dynamic linking ON <<<" -ForegroundColor Cyan }

# 1a. 决定 build 哪些 crate. 联机模式需要 client + server, 离线模式只需 client
$buildTargets = if ($Offline -or $NoServer) { @("lk2-client") } else { @("lk2-client","lk2-server") }
$serverExePath = Join-Path $ProjectRoot "target\debug\lk2-server.exe"
$clientExePath = Join-Path $ProjectRoot "target\debug\lk2-client.exe"
$needServerBuild = (-not $Offline) -and (-not $NoServer) -and (-not (Test-Path $serverExePath))
$needClientBuild = -not (Test-Path $clientExePath)

if (-not $SkipBuild) {
    foreach ($t in $buildTargets) {
        Write-Host ">>> cargo build -p $t $featureArgs ..." -ForegroundColor Cyan
        $buildOutput = cargo build -p $t $featureArgs 2>&1
        $buildOutput | Tee-Object -FilePath "build_loop.log" | Select-Object -Last 5
        if ($LASTEXITCODE -ne 0) {
            Write-Host ">>> BUILD FAILED for $t" -ForegroundColor Red
            exit 1
        }
    }
} else {
    # 就算 -SkipBuild, 缺 binary 时也要建 (本会话第一次跑 loop 常见)
    if ($needClientBuild) {
        Write-Host ">>> client binary missing, building (SkipBuild override) ..." -ForegroundColor Cyan
        Invoke-Expression "cargo build -p lk2-client $featureArgs" 2>&1 | Tee-Object -FilePath "build_loop.log" | Select-Object -Last 5
        if ($LASTEXITCODE -ne 0) { Write-Host ">>> BUILD FAILED" -ForegroundColor Red; exit 1 }
    }
    if ($needServerBuild) {
        Write-Host ">>> server binary missing, building (SkipBuild override) ..." -ForegroundColor Cyan
        Invoke-Expression "cargo build -p lk2-server $featureArgs" 2>&1 | Tee-Object -FilePath "build_loop.log" | Select-Object -Last 5
        if ($LASTEXITCODE -ne 0) { Write-Host ">>> BUILD FAILED" -ForegroundColor Red; exit 1 }
    }
}

# 2. Run + screenshot + record
if (-not (Test-Path $clientExePath)) {
    Write-Host ">>> Binary not found: $clientExePath" -ForegroundColor Red
    exit 1
}

# 决定模式
$serverProc = $null
$serverLog = Join-Path $ProjectRoot "screenshots\loop_server.log"
$clientLog = Join-Path $ProjectRoot "screenshots\loop_run.log"
$mode = "online"
if ($Offline) {
    $mode = "offline"
    $clientArgs = @("--offline","--auto-demo")
    Write-Host ">>> Mode: OFFLINE (no server, client --offline --auto-demo) ${Seconds}s ..." -ForegroundColor Green
} elseif ($NoServer) {
    $mode = "noserver"
    $clientArgs = @("--connect=$ServerAddr","--auto-demo")
    Write-Host ">>> Mode: NOSERVER (no lk2-server, client --connect=$ServerAddr will fail) ${Seconds}s ..." -ForegroundColor Yellow
} else {
    if (-not (Test-Path $serverExePath)) {
        Write-Host ">>> Server binary not found: $serverExePath (use -Offline to skip server)" -ForegroundColor Red
        exit 1
    }
    $clientArgs = @("--connect=$ServerAddr","--auto-demo")
    Write-Host ">>> Mode: ONLINE (server + client --connect=$ServerAddr) ${Seconds}s ..." -ForegroundColor Green

    # 启 server 后台
    Write-Host ">>> Starting lk2-server (background) ..." -ForegroundColor Cyan
    $serverProc = Start-Process -FilePath $serverExePath -PassThru -NoNewWindow `
        -RedirectStandardOutput $serverLog -RedirectStandardError "$serverLog.err"
    # 等 server 跑完 self_check (大约 1 秒, 给 3 秒 buffer)
    Start-Sleep -Seconds 3
}

# 启 client (前台, 让截图 + state JSON 写出来)
Write-Host ">>> Starting lk2-client ($mode) ..." -ForegroundColor Cyan
$clientProc = Start-Process -FilePath $clientExePath -ArgumentList $clientArgs -PassThru -NoNewWindow `
    -RedirectStandardOutput $clientLog -RedirectStandardError "$clientLog.err"
Start-Sleep -Seconds $Seconds
$clientProc | Stop-Process -Force -ErrorAction SilentlyContinue
if ($serverProc) {
    $serverProc | Stop-Process -Force -ErrorAction SilentlyContinue
}
Start-Sleep -Seconds 1

# 3. List results
Write-Host ""
Write-Host "=== Latest screenshots ===" -ForegroundColor Yellow
# iter_NN.png 在 screenshots/iter_NN/iter_NN.png (子目录)
Get-ChildItem "$ProjectRoot\screenshots\iter_*\iter_*.png" -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending | Select-Object -First 5 |
    ForEach-Object { "  $($_.FullName.Substring($ProjectRoot.Length + 1)) ($($_.Length / 1KB | ForEach-Object {'{0:N1}KB' -f $_}))" }

Write-Host ""
Write-Host "=== Latest tick state ===" -ForegroundColor Yellow
Get-ChildItem "$ProjectRoot\screenshots\state_*.json" -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1 |
    ForEach-Object { "  $($_.Name)" }

# 联机模式多输出 server 端关键 log, 帮 AI 验证"client 连上 server"
if ($mode -in @("online","noserver")) {
    Write-Host ""
    Write-Host "=== Server log (last 8 lines) ===" -ForegroundColor Yellow
    if (Test-Path $serverLog) {
        Get-Content $serverLog -Tail 8
    } else {
        Write-Host "  (no server log file at $serverLog)" -ForegroundColor DarkGray
    }
    # 检查 server self_check / tick 关键标记
    $serverCheckOk = (Test-Path $serverLog) -and (Select-String -Path $serverLog -Pattern "自检.*100 tick 全部通过|Server UDP socket bound" -Quiet)
    if ($serverCheckOk) {
        Write-Host "  [OK] server self-check passed + socket bound" -ForegroundColor Green
    } else {
        Write-Host "  [WARN] server didn't print self-check pass / socket bound (see log above)" -ForegroundColor Yellow
    }
}

Write-Host ""
Write-Host ">>> Done. AI: read latest screenshot + state JSON, decide next round" -ForegroundColor Magenta

#4. SCORE protocol reminder -- find latest iter and drop decision.template.md
$latestIterDir = $null
$latestIterName = $null
# 按 LastWriteTime 排序 (不是 Name — 'iter_99' 字符串 > 'iter_100' 字符串, 数字排序会跑偏)
$ssDirs = Get-ChildItem "$ProjectRoot\screenshots\iter_*" -Directory -ErrorAction SilentlyContinue |
 Sort-Object LastWriteTime -Descending
if ($ssDirs.Count -gt0) {
 $latestIterDir = $ssDirs[0].FullName
 $latestIterName = $ssDirs[0].Name
 $prevIterName = if ($ssDirs.Count -gt1) { $ssDirs[1].Name } else { "" }
 $decTplPath = Join-Path $latestIterDir "decision.template.md"
 $decTpl = @"
# $latestIterName -- DECISION PLACEHOLDER (AI must fill)

> 本文件由 loop.ps1 自动生成,提醒下一轮 AI必填 `decision.md`。
>详见 `Agent.md` § 十 SCORE协议。

##必填字段

\`\`\`markdown
# $latestIterName decision

score: sky=X player=Y terrain=Z decor=W hud=V total=N.N /10
vs_prev: $prevIterName -- [升 / 平 /降] -- 一句话原因(要引用 diff.json 的 delta)
problems:
 - [具体问题1]
 - [具体问题2]
 - [具体问题3]
plan:
 - [下一轮改什么文件 / 函数]
 - [改完之后预期哪个维度 +X 分]
\`\`\`

##评分提示 (0-10)

- Sky: 全黑=0,浅蓝=5,渐变+雾=10
- Player:不可见=0, 小黑点=5, avatar清晰=10
- Terrain: 全无体素=0,边缘可见=5,平台+树+水+怪物=10
- Decor: 全空=0,1 类=5, 多类聚集=10
- HUD: 方块=0, 可读但占太多=5,紧凑不挡视线=10
"@
 Set-Content -Path $decTplPath -Value $decTpl -Encoding UTF8
 Write-Host ""
 Write-Host "=== SCORE reminder written: $decTplPath ===" -ForegroundColor Cyan
 Write-Host ">>> NEXT AI: 必须给本轮0-10 打分 +写 decision.md (不要跳过!)" -ForegroundColor Yellow
} else {
 Write-Host ""
 Write-Host "[warn] no iter_* directory found -- can't write decision.template.md" -ForegroundColor Yellow
}
