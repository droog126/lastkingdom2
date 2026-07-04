    Mesh 生成：你以为只是 spawn_cube，实际是 Greedy Meshing + AO + 面剔除 + LOD。100×100×100 的区块如果 naive 生成，帧率直接个位数。
    体素碰撞：每个方块一个 Collider 是自杀。需要做稀疏高度场或合并相邻方块的大 AABB，否则 avian3d 的 Broad Phase 会爆。