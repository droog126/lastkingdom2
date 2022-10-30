### 巨神峰

https://mm.edrawsoft.cn/map.html?obj=qq159560A96324E74E448428137707B62E/Personal/%E6%9C%AA%E5%91%BD%E5%90%8D%E6%96%87%E4%BB%B62.emmx

通过命令行创建 1W 只蛇,每只蛇具有隐藏血条会跳出伤害,lowHp 会跳出别的属性,有声音类似于 apex，玩家具有体力,双人同步旋风斩割 1W 只蛇。性能要求是达标 40fps,完美情况是 60fps。

### 开发主旨

最低程度且高扩展得完成主线程，目标是构建开放具有 Z 轴得 2D 世界+Kingdom PVP。

### P0

1. 蛇攻击
   1.1 攻击盒子
   1.1.1 初步完成完成通用实例创建函数 (doing)
   攻击特效  
   no
   生命属性系统
   done
   todo:创建后一定时间内销毁，或者碰到东西立即销毁 攻击盒子

   gms 解包
   untiy 解包
   godot 解包

2. 人攻击 2. 武器系统

### p1

粒子系统

# 进度

1. 输入系统
2. 动画系统
3. 移动
4. 相机跟随 相机 debug

# todo

5.  地图编辑(等待 1.0 支持)

    5.1 4+9 动态地图加载
    坍塌函数也没有看
    这个研究还是太早了点，等联机 ok 了再搞这个
    已经支持了，下一个 todo

6.  碰撞
    https://github.com/jcornaz/impacted
    6.1 把碰撞形状实装父实体
    6.2 把碰撞形状放进 aabbs 输出碰撞事件 反馈给父实体
    6.3 碰撞性能优化成功，里程碑 0.2

7.  ui 试点
    https://github.com/mvlabat/bevy_egui.git
    7.1 按 M 呼出菜单
    7.2 优化文字显示(done)
    7.3 碰撞实例个数显示(done)

8.  怪物
    8.1 素材
    蛇
    8.2 ai
    8.2.1. 随机移动(done)
    8.2.2 看到玩家进行攻击(done)
    8.2.2.1 攻击事件分发系统 (done)
    8.2.2.2 生命属性系统 (wip)
    8.2.2.3 粒子系统 (wip)

        8.2.3   血量过低，逃跑

9.  pvp
10. 联网
11. debug 改成命令行
12. 开发工具
    12.1 地形编辑器

### 实验室

1. g7 的转点绳索
2. z 轴
3. 曲线闭合相交判定
   数组去建立区间
   或者用库
   https://parry.rs/
   这个可以
   矩形 内包裹不规则图形进行碰撞检测
4. 可不可以用 tokio 库 完成这样一个目标:当我需要别的实例的数据时，我不立刻获取，我记录下这个消费，然后再总线发出这个实例数据

### 记录

bevy_prototype_lyon 画图性能很烂
bevy_ecs_ldtk= "0.3.0" 性能也有点烂

### 源代码现存问题

4. 镜头需要跟着 y+z 移动才行

# 启动
cargo install cargo-watch
cargo watch -x 'run'

cargo run
cargo run --features bevy/trace_chrome

# debug

1.相机解锁 f3 DebugStatus.camera_debug
2.fps 显示 f11 DebugStatus.fps_show 3.碰撞体积显示 f12 DebugStatus.collision_debug 4.放置怪物 f10 DebugStatus.instance_debug

# 性能现状 0.1

无内存泄露
20000sprite 20000collision = 70-90fps
40000sprite 40000collision = 37-43fps

20000sprite 20000collision 理想碰撞规则 = 66-78fps
40000sprite 40000collision 理想碰撞规则 = 30-37fps

// here
20000sprite 20000collision 实际碰撞规则 = 66-78fps
40000sprite 40000collision 实际碰撞规则 = 30-37fps

### 开发理念

1. 能加字段就加字段解决
2. ecs 的模式很适合生产消费模式
   明确系统需要做什么?
   西兰花提供什么能力?
   接受一组矩形 然后调用回调函数(可以用来输出点心)

   碰撞系统需要接受一组形状，判断是否碰撞.
   碰撞之后要做什么? 1.防止移动 2.排斥力 3.侦察是否包含
   做到这些需要什么?
   碰撞两人的形状

   生产者就需要提供自己的形状 作为生产者的 生产因子

   生产者需要输入的，这个输入需要捋清楚所有线路，才会知道生产因子。

3. 如果需要不同类型的实例需要相互修改就放在 总线 或者是 并行总线 里去做，不要放在并行系统里搞。

4. 在一个地方产生事件 并存在当时的仓库里 到一个系统里去处理事件 然后分发(这个需要总线怎么处理呢) 给实例的仓库让他去消费事件。
   先抓住这个开发路线

### 历史

# 性能现状 0

1w 可以用实例
2w 碰撞检测
有 42 到 47 帧数左右

10000:42

none
10000:122-135
20000:60-70

only collision
10000:90-100
20000:45-50
使用 Mutex
10000:35

only snake_step
10000:100
20000:54
优化手段是什么呢？

完美优化成功 里程碑达到

### 源代码现存问题

1. 相机抖动(done)
   解决办法:镜头移动快速点 小于一定值直接等于
2. 碰撞抖动问题(done)

3. 命名改造(P0 done)
   实例的 children 有动画 阴影 范围
   主体包括 碰撞 ai 属性 动画实例索引

### P0

1.  完成碰撞规则实装(done)
2.  完成碰撞效果(done)
3.  重写怪物 Ai(done)
4.  人物需要增加脚下阴影(done)


### 性能分析工具
https://github.com/bevyengine/bevy/blob/main/docs/profiling.md