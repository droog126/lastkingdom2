# Agent.md — 万国起源 Demo 自动迭代操作手册

> 这个文件是给**接手本项目的 agent** 的工作手册。**不要再问用户 check，自己跑完整个闭环。**

---

## 一、目标

把 `F:\rustProject\lastkingdom2` 这个 bevy 0.18.1 项目从「能跑」推进到「看着像一款 demo」。

**核心原则：自闭环。** AI 读截图 → AI 决定改什么 → AI 改代码 → AI 重建 → AI 重跑 → AI 再读图。无尽循环。

---

## 二、闭环的四个环

```
   ┌──────────────────────────────────────────────────────┐
   │ 1. CAPTURE — 跑 + 截图 + 录数据                       │
   │    - cargo run 12 秒（自动 demo 模式）                  │
   │    - 自动保存 screenshots/iter_NN.png                 │
   │    - 自动保存 screenshots/iter_NN_state.json          │
   │                                                      │
   │ 2. OBSERVE — AI 读图 + 读数据                          │
   │    - Read 工具看 iter_NN.png                          │
   │    - cat iter_NN_state.json 看 tick/资源/玩家坐标     │
   │    - 列出「这版哪里不对」                              │
   │                                                      │
   │ 3. DECIDE — AI 决定下一轮改什么                        │
   │    - 优先级：bug > 视觉缺失 > 性能 > 装饰              │
   │    - 一次迭代改 1-3 个相关改动                        │
   │                                                      │
   │ 4. ACT — AI 改代码 + 重建 + 重跑                       │
   │    - Edit / Write 工具                                │
   │    - cargo build（必须过）                            │
   │    - 跑 run_loop.ps1                                  │
   │    - 回到 1                                           │
   └──────────────────────────────────────────────────────┘
```

---

## 三、必须的代码基础设施

| 模块 | 文件 | 作用 |
| --- | --- | --- |
| 启动自检 | `src/main.rs::self_check` | 100 tick headless + invariants |
| 启动后 sim | `simulation_tick` | 每 1s 跑 1 个游戏 tick |
| 自动 demo | `src/render/mod.rs::auto_demo` | 玩家自动走 + 飞 |
| 自动 orbit | `auto_orbit_camera` | 相机绕玩家转 |
| 自动截图 | `main.rs::periodic_screenshot` | 每 5s 截 1 张 |
| HUD overlay | `main.rs::setup_hud` + `update_hud` | 左上角文字 |
| 模拟输入 | (可加) `simulate_input` system | 模拟键盘/鼠标输入用于自动测 |
| tick 录制 | (可加) `tick_recorder` | 每 N tick dump JSON |

---

## 四、每次迭代的工作流（严格顺序）

### Phase 1：观察（必须有，否则盲改）
1. `ls screenshots/` — 看最近有哪些 iter
2. 用 `Read` 工具看最新的 `iter_NN.png`（视觉）
3. `cat screenshots/iter_NN_state.json`（数据）
4. 列出 3-5 个具体问题（"水面不见了" / "玩家掉到地下" / "HUD 文字重叠"）

### Phase 2：决定改动
5. 选 1-3 个最影响"看着像 demo"的问题
6. 想清楚改什么文件、什么函数
7. 检查**该改的代码是否真的能修这个**

### Phase 3：改代码
8. 用 `Edit` / `Write` 工具改
9. 不要大改架构；优先小修
10. **不要删函数注释**，除非是错的

### Phase 4：构建（必须过）
11. `Set-Location F:\rustProject\lastkingdom2; $env:BEVY_DISABLE_ACCESSIBILITY="1"; cargo build`
12. 看 `build.log` 末尾
13. 如果失败，回到 Phase 3 修

### Phase 5：跑 + 截图
14. `Start-Process -FilePath ".\target\debug\minecraft_bevy.exe" -PassThru -NoNewWindow`
15. `Start-Sleep -Seconds 12`
16. `Stop-Process -Force`
17. `ls screenshots/` 看新 iter

### Phase 6：回到 Phase 1

---

## 五、停止条件

- 用户明确说停 → 停
- 改动 5 轮后画面没明显变好 → 反思，尝试不同方向
- build 一直失败超过 3 轮 → 回到能 build 的状态
- AI 找不到可以改的地方 → 收尾输出

---

## 六、视觉目标（参考，按优先级）

P0 — **必须修**：
- [x] 天空不是黑色
- [x] 地形是体素风（Minecraft-like）
- [x] 玩家可见
- [x] HUD 可见
- [x] 截图能自动保存

P1 — **强烈要做**：
- [ ] 出生地有**平坦区域**（用户明确要求）
- [ ] 树/水/怪物等装饰物围绕出生地
- [ ] 相机不会卡到地下或看不到玩家

P2 — **加分**：
- [ ] 阴影
- [ ] 远景雾
- [ ] 战争迷雾
- [ ] 怪物 AI 真的会移动
- [ ] 玩家挖掉方块会真实消失

P3 — **长期**：
- [ ] save/load
- [ ] 100 玩家大厅
- [ ] Aether 维度

---

## 七、踩过的坑（不要再犯）

| 问题 | 教训 |
| --- | --- |
| `ResMut<World>` 报 "Resource does not exist" | bevy 0.18 的 World 跟我的 game world 撞名，要 `use ...as GameWorld;` |
| `chain()` 找不到 | bevy 0.18 是 `IntoScheduleConfigs`，要 `use bevy::ecs::schedule::IntoScheduleConfigs;` |
| 玩家卡在方块里不能动 | demo 模式加飞行：遇 solid 向上找空位 |
| `apply_transfer(PlayerGather)` 当成转出 | 收入类 src 走 `force_add` |
| 背光面全黑 | 双灯 + 高 ambient (1.2) |
| 中文字体方块 | bevy 默认字体没 CJK，用英文 |
| HUD 被相机 HUD 文字方块 | 改英文 |
| 玩家看不到 | 加旗杆 + 高空飞行 |
| HUD 看不到 | `iter_NN.png` 包含 HUD overlay |

---

## 八、运行命令速查

```powershell
# 编译
Set-Location F:\rustProject\lastkingdom2
$env:BEVY_DISABLE_ACCESSIBILITY = "1"
cargo build

# 跑（demo 模式自动 demo + 截图）
$proc = Start-Process -FilePath ".\target\debug\minecraft_bevy.exe" -PassThru -NoNewWindow
Start-Sleep -Seconds 12
$proc | Stop-Process -Force

# 测试
cargo test

# 看截图
ls screenshots/iter_*.png
```

---

## 九、当前状态

- 5 个核心模块（resource/world/nation/monster/ai）完成，50 单元测试过
- 6 轮迭代后：体素地形 + 玩家 avatar + 树 + 云 + 水 + HUD + 红旗
- **未做**：平坦出生地、阴影、远景雾、怪物 AI 真实移动、战争迷雾
