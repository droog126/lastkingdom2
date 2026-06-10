# loop.ps1 - WANGUO ORIGINS demo closed-loop iteration script
# Usage: powershell -File loop.ps1
# Each loop: build (if needed) -> run 12s -> kill -> list new screenshots

param(
    [int]$Seconds = 12,
    # 默认 SkipBuild=true: 冷编译 lightyear 0.26 + leafwing 需要 22+ 分钟,
    # 超过 30 min cap 装不下, 也撞 lightyear + bevy 0.18 API drift 编译错。
    # 只在 binary 已经编过、增量 build 时才用 -Build
    [switch]$SkipBuild = $true,
    # 启用 Bevy 动态链接 (开发期增量 build 快, binary 会动态加载 lib 而非静态链接)
    [switch]$Dynamic = $false
)

$ProjectRoot = $PSScriptRoot
Set-Location $ProjectRoot

$env:BEVY_DISABLE_ACCESSIBILITY = "1"
$env:RUST_LOG = "info"

# 0. Kill any old lk2-client process
Get-Process -Name "lk2-client" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1

# 1. Build (default 跳过, 改用 -Build flag 显式打开)
$featureArgs = if ($Dynamic) { "--features dev-dynamic-linking" } else { "" }
if ($Dynamic) { Write-Host ">>> dynamic linking ON <<<" -ForegroundColor Cyan }
if (-not $SkipBuild) {
    Write-Host ">>> cargo build -p lk2-client $featureArgs ..." -ForegroundColor Cyan
    Invoke-Expression "cargo build -p lk2-client $featureArgs" 2>&1 | Tee-Object -FilePath "build_loop.log" | Select-Object -Last 5
    if ($LASTEXITCODE -ne 0) {
        Write-Host ">>> BUILD FAILED" -ForegroundColor Red
        exit 1
    }
}

# 2. Run + screenshot + record
$exePath = Join-Path $ProjectRoot "target\debug\lk2-client.exe"
if (-not (Test-Path $exePath)) {
    Write-Host ">>> Binary not found: $exePath" -ForegroundColor Red
    exit 1
}

Write-Host ">>> Start demo (${Seconds}s, --offline --auto-demo) ..." -ForegroundColor Green
$logPath = Join-Path $ProjectRoot "screenshots\loop_run.log"
$proc = Start-Process -FilePath $exePath -ArgumentList "--offline","--auto-demo" -PassThru -NoNewWindow -RedirectStandardOutput $logPath -RedirectStandardError "$logPath.err"
Start-Sleep -Seconds $Seconds
$proc | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1

# 3. List results
Write-Host ""
Write-Host "=== Latest screenshots ===" -ForegroundColor Yellow
Get-ChildItem "$ProjectRoot\screenshots\iter_*.png" -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending | Select-Object -First 5 |
    ForEach-Object { "  $($_.Name) ($($_.Length / 1KB | ForEach-Object {'{0:N1}KB' -f $_}))" }

Write-Host ""
Write-Host "=== Latest tick state ===" -ForegroundColor Yellow
Get-ChildItem "$ProjectRoot\screenshots\state_*.json" -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1 |
    ForEach-Object { "  $($_.Name)" }

Write-Host ""
Write-Host ">>> Done. AI: read latest screenshot + state JSON, decide next round" -ForegroundColor Magenta

#4. SCORE protocol reminder -- find latest iter and drop decision.template.md
$latestIterDir = $null
$latestIterName = $null
$ssDirs = Get-ChildItem "$ProjectRoot\screenshots\iter_*" -Directory -ErrorAction SilentlyContinue |
 Sort-Object Name -Descending
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
