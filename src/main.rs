//! 万国起源 — Umbrella binary **已弃用**
//!
//! 历史: 这是原先单 binary 的 `minecraft_bevy` 入口。模块已按
//! `docs/plans/client-server-split.md` 拆到 `crates/core` (sim) /
//! `crates/client` (DefaultPlugins + 渲染 + 客户端 PvP) / `crates/server`
//! (MinimalPlugins + 服务端权威) 三个 crate。
//!
//! 替代方案:
//!   - 单机 demo / loop.ps1 自动化截图: `cargo run -p lk2-client -- --offline`
//!   - 联机 (server 在 :5000 listen): `cargo run -p lk2-server` + `cargo run -p lk2-client`
//!
//! 这个 binary 仅作为兼容占位存在 — 跑它会提示用户用新 binary，然后退出。

fn main() {
    eprintln!(
        "⚠  `minecraft_bevy` (umbrella) binary 已弃用。\n\
         ⚠  模块已拆到 crates/core / crates/client / crates/server。\n\
         \n\
         用法:\n\
         \n\
         单机 demo / 自动化截图:\n\
         \n\
         \x20   $env:BEVY_DISABLE_ACCESSIBILITY=\"1\"\n\
         \x20   cargo run -p lk2-client -- --offline\n\
         \n\
         联机（server / client 分进程）:\n\
         \n\
         \x20   cargo run -p lk2-server\n\
         \x20   cargo run -p lk2-client\n"
    );
    // 立刻退出（不要 .run() Bevy — 这个 binary 啥 plugin 也没 add）
    std::process::exit(0);
}
