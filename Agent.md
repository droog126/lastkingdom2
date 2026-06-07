# Agent.md — 万国起源 Demo 自动迭代操作手册

> 这个文件是给**接手本项目的 agent** 的工作手册。**不要再问用户 check，自己跑完整个闭环。**

---

## 一、目标

把 `F:\rustProject\lastkingdom2` 这个 bevy0.18.1 项目从「能跑」推进到「看着像一款 demo」。

**核心原则：自闭环。** AI读截图 → AI决定改什么 → AI改代码 → AI重建 → AI 重跑 → AI 再读图。无尽循环。

---

## 二、闭环的四个环

```
 ┌──────────────────────────────────────────────────────┐
 │1. CAPTURE —跑 +截图 +录数据 │
 │ - cargo run12 秒（自动 demo模式） │
 │ - 自动保存 screenshots/iter_NN/iter_NN.png │
 │ - 自动保存 screenshots/iter_NN/final_state.json │
 │ │
 │2. OBSERVE —读图 +读数据 + 对比 + 打分 │
 │ - Read工具看 iter_NN.png │
 │ - cat iter_NN/final_state.json 看 tick/资源/玩家 │
 │ - cat iter_NN/diff.json 看 vs iter_(NN-1)数值变化 │
 │ - 同时 Read iter_(NN-1).png 做视觉对比 │
 │ - 按 §十 SCORE协议给本轮0-10 打分（5维度） │
 │ -列出「这版哪里不对」 │
 │ │
 │3. DECIDE —决定 +写 decision.md │
 │ -优先级：bug >视觉缺失 >性能 >装饰 │
 │ -一次迭代改1-3 个相关改动 │
 │ -必填 screenshots/iter_NN/decision.md（见 §十） │
 │ │
 │4. ACT —改代码 +重建 + 重跑 │
 │ - Edit / Write工具 │
 │ - cargo build（必须过） │
 │ -跑 loop.ps1 │
 │ -回到1 │
 └──────────────────────────────────────────────────────┘
```

---

## 三、必须的代码基础设施

| 模块 | 文件 |作用 |
| --- | --- | --- |
|启动自检 | `src/main.rs::self_check` |100 tick headless + invariants |
|启动后 sim | `simulation_tick` | 每1s跑1 个游戏 tick |
| 自动 demo | `src/render/mod.rs::auto_demo` |玩家自动走 +飞 |
| 自动 orbit | `auto_orbit_camera` |相机绕玩家转 |
| 自动截图 | `main.rs::periodic_screenshot` | 每5s截1 张 |
| HUD overlay | `main.rs::setup_hud` + `update_hud` | 左上角文字 |
|模拟输入 | (可加) `simulate_input` system |模拟键盘/鼠标输入用于自动测 |
| tick录制 | (可加) `tick_recorder` | 每 N tick dump JSON |

---

## 四、每次迭代的工作流（严格顺序）

### Phase1：观察（必须有，否则盲改）
1. `ls screenshots/` — 看最近有哪些 iter
2. 用 `Read`工具看最新的 `iter_NN.png`（视觉）
3. `cat screenshots/iter_NN/final_state.json`（数据）
4. **同时 Read `iter_(NN-1).png` 做视觉对比**（不是可选）
5.列出3-5 个具体问题（"水面不见了" / "玩家掉到地下" / "HUD文字重叠"）

### Phase2：决定改动
6.选1-3 个最影响"看着像 demo"的问题
7. 想清楚改什么文件、什么函数
8. 检查**该改的代码是否真的能修这个**

### Phase3：改代码
9. 用 `Edit` / `Write`工具改
10. 不要大改架构；优先小修
11. **不要删函数注释**，除非是错的

### Phase4：构建（必须过）
12. `Set-Location F:\rustProject\lastkingdom2; $env:BEVY_DISABLE_ACCESSIBILITY="1"; cargo build --features dev-dynamic-linking`
13. 看 `build.log`末尾
14. 如果失败，回到 Phase3修

### Phase5：跑 +截图
15. `powershell -File loop.ps1` （自动 build +12s run + screenshot + diff）
16. `ls screenshots/` 看新 iter

### Phase6：打分 + decision.md（关键，必填）
17. **给 iter_NN 打5维度分（Sky/Player/Terrain/Decor/HUD）**
18. **写 `screenshots/iter_NN/decision.md`** — 含 score / vs_prev / problems / plan
19.回到 Phase1

---

## 五、停止条件

- 用户明确说停 →停
-改动5轮后画面没明显变好 →反思，尝试不同方向
- build一直失败超过3轮 →回到能 build 的状态
- AI找不到可以改的地方 →收尾输出
- **连续3轮 decision.md 的 score 不升 →触发反思**（改错方向了）

---

## 六、视觉目标（参考，按优先级）

P0 — **必须修**：
- [x] 天空不是黑色
- [x] 地形是体素风（Minecraft-like）
- [x] 玩家可见
- [x] HUD可见
- [x] 截图能自动保存

P1 — **强烈要做**：
- [] 出生地有**平坦区域**（用户明确要求）
- [] 树/水/怪物等装饰物围绕出生地
- [] 相机不会卡到地下或看不到玩家

P2 — **加分**：
- [] 阴影
- [] 远景雾
- [] 战争迷雾
- [] 怪物 AI真的会移动
- [] 玩家挖掉方块会真实消失

P3 — **长期**：
- [] save/load
- [] 100玩家大厅
- [] Aether维度

---

## 七、踩过的坑（不要再犯）

| 问题 |教训 |
| --- | --- |
| `ResMut<World>`报 "Resource does not exist" | bevy0.18 的 World跟我的 game world撞名，要 `use ...as GameWorld;` |
| `chain()`找不到 | bevy0.18 是 `IntoScheduleConfigs`，要 `use bevy::ecs::schedule::IntoScheduleConfigs;` |
|玩家卡在方块里不能动 | demo模式加飞行：遇 solid向上找空位 |
| `apply_transfer(PlayerGather)` 当成转出 |收入类 src走 `force_add` |
|背光面全黑 | 双灯 + 高 ambient (1.2) |
| 中文字体方块 | bevy 默认字体没 CJK，用英文 |
| HUD 被相机 HUD文字方块 |改英文 |
|玩家看不到 | 加旗杆 + 高空飞行 |
| HUD看不到 | `iter_NN.png`包含 HUD overlay |

---

## 八、运行命令速查

```powershell
#闭环迭代（推荐，自动 build + run + diff）
Set-Location F:\rustProject\lastkingdom2
$env:BEVY_DISABLE_ACCESSIBILITY = "1"
$env:RUST_LOG = "info"
powershell -File loop.ps1 #12s 默认
powershell -File loop.ps1 -Seconds20 # 加长

#单独编译(dev:必须带 dev-dynamic-linking,详见 § 十二)
cargo build --features dev-dynamic-linking

#单独跑
$proc = Start-Process -FilePath ".\target\debug\minecraft_bevy.exe" -PassThru -NoNewWindow
Start-Sleep -Seconds12
$proc | Stop-Process -Force

# 测试
cargo test

# 看截图
ls screenshots\iter_*.png
```

---

## 九、当前状态

-5 个核心模块（resource/world/nation/monster/ai）完成，50单元测试过
- **96轮迭代后**:画面仍有大片空白(白/红平台主导),voxel地形视觉上几乎不可见,玩家仅是一个小黑点
- **核心问题**:96轮没有持续打分/对比 → 没有意识到"视觉没在进步" → 在原地踏步
- **未做**:平坦出生地、阴影、远景雾、怪物 AI真实移动、战争迷雾

---

## 十、SCORE协议（每轮必填，闭环才有意义）

> **没打分 = 没迭代。** 看截图不打分 = 没看。任何一次 OBSERVE 都必须产出 `screenshots/iter_NN/decision.md`。

###5维度评分（每项0-10，总分 =简单平均，保留1 位小数）

|维度 |0是什么样 |5是什么样 |10是什么样 |
| --- | --- | --- | --- |
| **Sky /天空** | 全黑 / 全白 /紫红错误色 |浅蓝可接受 |渐变天空 +远景雾 |
| **Player /玩家** |不可见 / 卡地下 / 在框外 |是个小黑点但能定位 |玩家 avatar清晰、有旗杆 /高度感 |
| **Terrain /地形** | 完全看不到体素 |边缘有1-2块体素可见 |出生地有平台、树、水、怪物围绕 |
| **Decor /装饰** | 全空 / 全单一色块 | 有 >=1 类装饰（树 / 水 /怪物） | 多类装饰聚集、有节奏 |
| **HUD / 信息层** |文字方块 /不可读 / 重叠 | 可读但占太多面积 |紧凑、信息齐、不挡视线 |

### 打分必读的两个东西

1. **`iter_NN/diff.json`**（loop.ps1写）— 看 wood / food / flag / monster 的数值 delta。
2. **`iter_(NN-1).png` vs `iter_NN.png`** — **视觉对比**。如果数值在涨但视觉没变 =玩家飞到了看不到东西的方向 = bug。

### `decision.md`必填字段

```markdown
# iter_NN decision

score: sky=X player=Y terrain=Z decor=W hud=V total=N.N /10
vs_prev: iter_(NN-1) — [升 / 平 /降] — 一句话原因（要引用 diff.json 的 delta）
problems:
 - [具体问题1，如 "玩家被白平台挡住看不见 avatar"]
 - [具体问题2]
 - [具体问题3]
plan:
 - [下一轮改什么文件 / 函数]
 - [改完之后预期哪个维度 +X 分]
```

###反思触发器

- **连续3轮 total 不升 →反思**（可能改错方向了，或改的位置没生效）。
- **单维度从5掉到0 →立刻回滚该改动**（不要在塌了的版本上继续改）。
- **数值在涨但视觉降分 →改的是隐藏数据不是视觉**，优先级降。

---

##十一、`decision.md`位置（重要）

- 每轮跑完后 `screenshots/iter_NN/decision.md` 是 **AI 的产物**，不是 loop.ps1 自动写的。
- loop.ps1会在新 iter目录里 **留一个 `decision.template.md`**（最小占位 + SCORE协议摘要）提示 AI必填。
- AI下一轮开工的第一步：先 `ls screenshots/iter_NN/`，确认上一轮的 `decision.md`存在并读完，再开始新一轮。

---

## 十二、编译加速(必读,默认行为)

**所有 dev 阶段的 `cargo build` / `cargo run` / `loop.ps1` 都必须带 `--features dev-dynamic-linking`**。`loop.ps1` 已内置这个 flag,`Phase 4` 步骤 12 也已更新。**不要**手动去掉它。

### 为什么
Bevy 本身很大,光静态链接一次要 10+ 分钟;动态链接后,只有第一次会完整编 Bevy(同样慢),**之后**只要业务代码没改到 Bevy 内部,增量构建从几分钟降到几秒。本项目已经迭代 96 轮,后面还会有几百轮 —— 没有这个 feature 根本跑不动。

### 规则
- ✅ `cargo build --features dev-dynamic-linking`(dev 唯一正确写法)
- ✅ `cargo run --features dev-dynamic-linking`
- ✅ `loop.ps1`(已自动带,不要绕过)
- ❌ `cargo build` —— 禁止,会把动态库废掉、回退到全量静态链接
- ❌ `cargo build --release` 时**任何**带 `dev-dynamic-linking` 的组合 —— release 严禁带这个 feature
- ❌ 任何 CI / 打包脚本中带 `dev-dynamic-linking` —— 失败立即停

### 异常处理
- **如果 `cargo build --features dev-dynamic-linking` 第一次失败**:检查 `Cargo.toml` 的 `[features]` 段是否还在,以及 `bevy` 依赖未被 pin 死版本(动态链接要求 `bevy` 不能是 `=x.y.z` 严格等号)。
- **如果增量构建突然变慢到分钟级**:大概率有人误删了 `[features]` 段或 `loop.ps1` 的 `--features` flag —— 立即 `git diff Cargo.toml loop.ps1` 回看。
- **如果 agent 误用了 `cargo build`**(无 feature):`target\debug` 里的 `minecraft_bevy.exe` 还是上一次带 feature 编出来的,表面看能跑,实际增量加速失效 —— 下次 `loop.ps1` 会自动改回正确 flag,但已编出来的二进制要 `cargo clean` 一下才彻底干净。
