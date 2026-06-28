








use bevy::prelude::*;
use lk2_core::player::PlayerState;
use lk2_core::world::{Biome, World as GameWorld};

use crate::render::scalar_field::effective_ground_height;

#[cfg(feature = "audit-pretty-models")]
mod audit_pretty;


#[derive(Resource, Debug, Clone)]
pub struct PrettyConfig {
    pub show_water: bool,
    pub show_player_avatar: bool,
    pub show_monster_cubes: bool,
}

impl Default for PrettyConfig {
    fn default() -> Self {
        Self { show_water: true, show_player_avatar: true, show_monster_cubes: true }
    }
}


#[derive(Component)]
pub struct WaterMarker;


#[derive(Component)]
pub struct GroundDiscOuter;


#[derive(Component)]
pub struct GroundDiscInner;







pub fn follow_ground_discs(
    player: Res<PlayerState>,
    game_world: Res<GameWorld>,
    mut outer: Query<&mut Transform, (With<GroundDiscOuter>, Without<GroundDiscInner>)>,
    mut inner: Query<&mut Transform, (With<GroundDiscInner>, Without<GroundDiscOuter>)>,
) {
    let ground_top = effective_ground_height(&game_world, player.block_pos[0], player.block_pos[2]);
    for mut t in &mut outer {
        t.translation = Vec3::new(player.pos.x, ground_top - 0.05, player.pos.z);
    }
    for mut t in &mut inner {
        t.translation = Vec3::new(player.pos.x, ground_top + 0.25, player.pos.z);
    }
}





pub fn follow_water(player: Res<PlayerState>, mut q: Query<&mut Transform, With<WaterMarker>>) {
    let Ok(mut tf) = q.single_mut() else {
        return;
    };
    tf.translation.x = player.pos.x;
    tf.translation.z = player.pos.z;

}


#[derive(Component)]
pub struct AvatarPart {
    pub offset: Vec3,
}

const AVATAR_VISUAL_SCALE: f32 = 0.55;

fn avatar_offset(offset: Vec3) -> Vec3 {
    offset * AVATAR_VISUAL_SCALE
}


#[derive(Component)]
pub struct MonsterCube {
    pub base: Vec3,
}


#[derive(Component)]
pub struct CloudPuff {

    pub base: Vec3,

    pub phase: f32,
}

#[derive(Component)]
pub struct V2WorldMarker;


pub fn spawn_pretty(
    mut commands: Commands,
    game_world: Res<GameWorld>,
    player: Res<PlayerState>,
    cfg: Res<PrettyConfig>,
    _asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {





    if cfg.show_water {






        let s = 56.0_f32;
        let water_y = lk2_core::constant::WATER_Y;
        let cx = player.pos.x;
        let cz = player.pos.z;
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(s, s))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(0.22, 0.42, 0.62, 0.72),
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 0.85,
                metallic: 0.0,
                reflectance: 0.18,
                ..default()
            })),
            Transform::from_translation(Vec3::new(cx, water_y, cz)),
            WaterMarker,
        ));
        info!(
            "🌊 水面已 spawn (y={}, size={}, 跟随玩家 @ ({:.1}, {:.1}))",
            water_y, s, cx, cz
        );
    }




    {

        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.8, 0.05))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(0.32, 0.48, 0.20, 0.65),
                emissive: Color::srgb(0.20, 0.40, 0.10).into(),
                perceptual_roughness: 0.95,
                metallic: 0.0,
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            Transform::from_translation(Vec3::new(player.pos.x, player.pos.y - 0.05, player.pos.z)),
            GroundDiscOuter,
        ));


        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.3, 0.05))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(0.55, 0.75, 0.30, 0.85),
                emissive: Color::srgb(0.30, 0.50, 0.15).into(),
                perceptual_roughness: 0.92,
                metallic: 0.0,
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            Transform::from_translation(Vec3::new(
                player.pos.x,
                player.pos.y + 0.005,
                player.pos.z,
            )),
            GroundDiscInner,
        ));
    }



    if cfg.show_player_avatar {
        let base = player.pos;

        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.30))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.85, 0.75),
                emissive: LinearRgba::from(Color::srgb(0.40, 0.34, 0.30)) * 0.5,
                perceptual_roughness: 0.5,
                metallic: 0.0,
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(0.0, 0.70, 0.0)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 0.93, 0.93)),
            AvatarPart { offset: avatar_offset(Vec3::new(0.0, 0.70, 0.0)) },
        ));

        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.30))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.45, 0.30, 0.20),
                emissive: LinearRgba::from(Color::srgb(0.18, 0.12, 0.08)) * 0.4,
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(0.0, 0.80, 0.0)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 0.60, 1.0)),
            AvatarPart { offset: avatar_offset(Vec3::new(0.0, 0.80, 0.0)) },
        ));

        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.09))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.05, 0.02, 0.04),
                emissive: Color::BLACK.into(),
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(-0.10, 0.72, 0.22)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(0.78, 1.0, 0.55)),
            AvatarPart { offset: avatar_offset(Vec3::new(-0.10, 0.72, 0.22)) },
        ));

        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.09))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.05, 0.02, 0.04),
                emissive: Color::BLACK.into(),
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(0.10, 0.72, 0.22)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(0.78, 1.0, 0.55)),
            AvatarPart { offset: avatar_offset(Vec3::new(0.10, 0.72, 0.22)) },
        ));

        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.03))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: Color::WHITE.to_linear() * 1.5,
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(-0.085, 0.76, 0.27)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(0.6, 0.7, 0.3)),
            AvatarPart { offset: avatar_offset(Vec3::new(-0.085, 0.76, 0.27)) },
        ));

        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.03))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: Color::WHITE.to_linear() * 1.5,
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(0.115, 0.76, 0.27)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(0.6, 0.7, 0.3)),
            AvatarPart { offset: avatar_offset(Vec3::new(0.115, 0.76, 0.27)) },
        ));

        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.07))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.65, 0.70),
                emissive: LinearRgba::from(Color::srgb(0.50, 0.30, 0.30)) * 0.5,
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(-0.22, 0.66, 0.18)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 0.7, 0.5)),
            AvatarPart { offset: avatar_offset(Vec3::new(-0.22, 0.66, 0.18)) },
        ));

        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.07))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.65, 0.70),
                emissive: LinearRgba::from(Color::srgb(0.50, 0.30, 0.30)) * 0.5,
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(0.22, 0.66, 0.18)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 0.7, 0.5)),
            AvatarPart { offset: avatar_offset(Vec3::new(0.22, 0.66, 0.18)) },
        ));

        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.04))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.85, 0.30, 0.40),
                emissive: LinearRgba::from(Color::srgb(0.40, 0.10, 0.15)) * 0.4,
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(0.0, 0.62, 0.27)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(0.4, 0.3, 0.3)),
            AvatarPart { offset: avatar_offset(Vec3::new(0.0, 0.62, 0.27)) },
        ));

        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.30))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.40, 0.40),
                emissive: LinearRgba::from(Color::srgb(0.50, 0.20, 0.20)) * 0.5,
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(0.0, 0.40, 0.0)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 0.93, 0.83)),
            AvatarPart { offset: avatar_offset(Vec3::new(0.0, 0.40, 0.0)) },
        ));

        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.10))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.40, 0.55, 0.95),
                emissive: LinearRgba::from(Color::srgb(0.16, 0.22, 0.38)) * 0.4,
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(-0.10, 0.10, 0.0)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 1.0, 1.0)),
            AvatarPart { offset: avatar_offset(Vec3::new(-0.10, 0.10, 0.0)) },
        ));

        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.10))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.40, 0.55, 0.95),
                emissive: LinearRgba::from(Color::srgb(0.16, 0.22, 0.38)) * 0.4,
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(0.10, 0.10, 0.0)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 1.0, 1.0)),
            AvatarPart { offset: avatar_offset(Vec3::new(0.10, 0.10, 0.0)) },
        ));

        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.13))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.40, 0.55, 0.95),
                emissive: LinearRgba::from(Color::srgb(0.16, 0.22, 0.38)) * 0.4,
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(-0.10, 0.05, 0.05)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 0.6, 1.2)),
            AvatarPart { offset: avatar_offset(Vec3::new(-0.10, 0.05, 0.05)) },
        ));

        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.13))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.40, 0.55, 0.95),
                emissive: LinearRgba::from(Color::srgb(0.16, 0.22, 0.38)) * 0.4,
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(0.10, 0.05, 0.05)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 0.6, 1.2)),
            AvatarPart { offset: avatar_offset(Vec3::new(0.10, 0.05, 0.05)) },
        ));

        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.10))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.85, 0.75),
                emissive: LinearRgba::from(Color::srgb(0.40, 0.34, 0.30)) * 0.4,
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(-0.30, 0.42, 0.0)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 1.0, 1.0)),
            AvatarPart { offset: avatar_offset(Vec3::new(-0.30, 0.42, 0.0)) },
        ));

        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.10))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.85, 0.75),
                emissive: LinearRgba::from(Color::srgb(0.40, 0.34, 0.30)) * 0.4,
                ..default()
            })),
            Transform::from_translation(base + avatar_offset(Vec3::new(0.30, 0.42, 0.0)))
                .with_scale(Vec3::splat(AVATAR_VISUAL_SCALE) * Vec3::new(1.0, 1.0, 1.0)),
            AvatarPart { offset: avatar_offset(Vec3::new(0.30, 0.42, 0.0)) },
        ));
        info!(
            "🧍 玩家 avatar (v5-cute Q 版球体) 已 spawn at {:?}",
            player.pos
        );
    }



    if cfg.show_monster_cubes {
        let monster_kinds = [
            (Color::srgb(0.5, 0.85, 0.2), "Snake"),
            (Color::srgb(0.3, 0.7, 0.95), "FrostElf"),
            (Color::srgb(0.95, 0.7, 0.2), "SandWurm"),
            (Color::srgb(0.4, 0.25, 0.1), "Treant"),
            (Color::srgb(0.7, 0.3, 0.85), "AetherWraith"),
        ];
        for (i, (color, _name)) in monster_kinds.iter().enumerate() {
            let angle = (i as f32) * 1.2566;
            let r = 7.5 + (i as f32) * 1.8;
            let offset = Vec3::new(angle.cos() * r, 1.3, angle.sin() * r);
            let pos = player.pos + offset;

            let entity = commands
                .spawn((
                    Mesh3d(meshes.add(Sphere::new(1.3))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: *color,
                        emissive: LinearRgba::from(*color) * 0.4,
                        perceptual_roughness: 0.6,
                        metallic: 0.0,
                        ..default()
                    })),
                    Transform::from_translation(pos),
                    MonsterCube { base: pos },
                ))
                .id();
            let _ = entity;
        }
        info!("👹 5 个怪物球体已 spawn (7.5-14.7m 圆周, 半径 1.3m, 朝 player 走)");
    }





    let cloud_layouts: [(f32, f32, f32, f32, f32); 4] = [

        (0.7, 22.0, 18.0, 1.0, 0.7),
        (2.1, 18.0, 20.0, 0.8, 1.0),
        (3.8, 25.0, 22.0, 1.2, 0.8),
        (5.4, 16.0, 19.0, 0.7, 0.7),
    ];
    for (angle, r, base_y, ex, ez) in cloud_layouts.iter().copied() {
        let cx = player.pos.x + angle.cos() * r;
        let cz = player.pos.z + angle.sin() * r;
        let cy = base_y;

        let cloud_color = Color::srgba(0.92, 0.95, 1.0, 0.85);
        let cloud_mat = materials.add(StandardMaterial {
            base_color: cloud_color,
            emissive: (cloud_color.to_linear() * 0.18).into(),
            perceptual_roughness: 0.95,
            metallic: 0.0,
            alpha_mode: AlphaMode::Blend,
            ..default()
        });

        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(1.6))),
            MeshMaterial3d(cloud_mat.clone()),
            Transform::from_translation(Vec3::new(cx, cy, cz)),
            CloudPuff { base: Vec3::new(cx, cy, cz), phase: angle * 1.3 },
        ));

        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(1.1))),
            MeshMaterial3d(cloud_mat.clone()),
            Transform::from_translation(Vec3::new(cx - 1.5 * ex, cy + 0.2, cz)),
            CloudPuff { base: Vec3::new(cx - 1.5 * ex, cy + 0.2, cz), phase: angle * 1.3 + 1.7 },
        ));

        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(1.2))),
            MeshMaterial3d(cloud_mat.clone()),
            Transform::from_translation(Vec3::new(cx + 1.6 * ex, cy - 0.1, cz + 0.5 * ez)),
            CloudPuff {
                base: Vec3::new(cx + 1.6 * ex, cy - 0.1, cz + 0.5 * ez),
                phase: angle * 1.3 + 3.1,
            },
        ));

        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.9))),
            MeshMaterial3d(cloud_mat),
            Transform::from_translation(Vec3::new(cx + 0.3, cy + 1.0, cz - 0.2 * ez)),
            CloudPuff {
                base: Vec3::new(cx + 0.3, cy + 1.0, cz - 0.2 * ez),
                phase: angle * 1.3 + 4.5,
            },
        ));
    }




    let ground_y = effective_ground_height(&game_world, player.block_pos[0], player.block_pos[2]);


    for i in 0..8 {
        let angle = (i as f32) * (std::f32::consts::TAU / 8.0);
        let r = 13.0;
        let t_x = player.pos.x + angle.cos() * r;
        let t_z = player.pos.z + angle.sin() * r;

        for h in 0..3 {
            spawn_cube(
                &mut commands,
                &mut meshes,
                &mut materials,
                Vec3::new(t_x, ground_y + 1.10 + h as f32, t_z),
                Vec3::new(1.10, 2.10, 1.10),
                Color::srgb(0.45, 0.27, 0.10),
            );
        }

        for dy in 0..3 {
            spawn_cube(
                &mut commands,
                &mut meshes,
                &mut materials,
                Vec3::new(t_x, ground_y + 3.55 + dy as f32, t_z),
                Vec3::new(2.40, 2.20, 2.40),
                Color::srgb(0.25, 0.55, 0.20),
            );
        }
    }




    let rock_positions: [(f32, f32, f32); 10] = [
        (3.0, 3.0, 1.2),
        (-3.5, 3.2, 0.95),
        (2.5, -3.8, 1.1),
        (-2.2, -4.5, 0.75),
        (4.0, -1.0, 1.45),
        (-4.2, -0.7, 0.85),
        (4.2, 3.8, 1.05),
        (-3.8, -2.4, 1.3),
        (1.0, 4.5, 0.7),
        (-1.0, -4.8, 0.85),
    ];
    for (i, (rx, rz, scale)) in rock_positions.iter().enumerate() {

        let rock_color = match i % 3 {
            0 => Color::srgb(0.42, 0.42, 0.45),
            1 => Color::srgb(0.58, 0.55, 0.50),
            _ => Color::srgb(0.50, 0.52, 0.48),
        };
        let r_x = player.pos.x + rx * 1.8;
        let r_z = player.pos.z + rz * 1.8;
        spawn_cube(
            &mut commands,
            &mut meshes,
            &mut materials,
            Vec3::new(r_x, ground_y + 0.4 * scale, r_z),
            Vec3::new(*scale * 1.55, *scale * 1.20, *scale * 1.55),
            rock_color,
        );
    }




    let flower_positions: [(f32, f32); 10] = [
        (1.8, 1.8),
        (-2.0, 2.3),
        (2.3, -1.0),
        (-2.5, -1.8),
        (3.0, -0.5),
        (-3.4, 1.0),
        (0.8, -3.2),
        (3.6, 2.5),
        (-3.0, -3.6),
        (2.7, 3.7),
    ];
    let flower_colors = [
        Color::srgb(0.98, 0.30, 0.55),
        Color::srgb(1.0, 0.85, 0.20),
        Color::srgb(0.55, 0.30, 0.98),
        Color::srgb(1.0, 0.45, 0.20),
        Color::srgb(0.95, 0.30, 0.30),
    ];
    for (i, (fx, fz)) in flower_positions.iter().enumerate() {
        let f_color = flower_colors[i % flower_colors.len()];
        let f_x = player.pos.x + fx * 2.0;
        let f_z = player.pos.z + fz * 2.0;
        spawn_cube(
            &mut commands,
            &mut meshes,
            &mut materials,
            Vec3::new(f_x, ground_y + 0.6, f_z),
            Vec3::new(0.95, 1.30, 0.95),
            f_color,
        );
    }






    let hill_distance = 28.0;
    let hill_offsets: [(f32, f32); 4] = [
        (hill_distance, hill_distance),
        (-hill_distance, hill_distance),
        (hill_distance, -hill_distance),
        (-hill_distance, -hill_distance),
    ];
    for (hx, hz) in hill_offsets.iter() {
        let h_x = player.pos.x + hx;
        let h_z = player.pos.z + hz;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(8.0, 4.5, 8.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.20, 0.42, 0.18),
                emissive: Color::srgb(0.04, 0.08, 0.03).into(),
                perceptual_roughness: 0.95,
                metallic: 0.0,
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            Transform::from_translation(Vec3::new(h_x, ground_y + 2.25, h_z)),
        ));
    }
    spawn_v2_crown_season_markers(
        &mut commands,
        &mut meshes,
        &mut materials,
        player.pos,
        ground_y,
    );





    #[cfg(feature = "audit-pretty-models")]
    audit_pretty::spawn_audit_ring(
        &mut commands,
        &mut meshes,
        &mut materials,
        _asset_server,
        player.pos,
        ground_y,
    );
}

fn spawn_v2_crown_season_markers(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    player_pos: Vec3,
    ground_y: f32,
) {
    spawn_v2_cube(
        commands,
        meshes,
        materials,
        Vec3::new(player_pos.x, ground_y + 0.28, player_pos.z),
        Vec3::new(7.5, 0.26, 7.5),
        Color::srgb(0.28, 0.34, 0.24),
        Color::srgb(0.16, 0.26, 0.12),
    );





    let pois = [
        (
            Vec3::new(12.5, 0.85, -12.5),
            Vec3::new(0.65, 1.70, 0.65),
            Color::srgb(0.70, 0.30, 0.18),
            Color::srgb(0.95, 0.50, 0.24),
        ),
        (
            Vec3::new(-12.5, 0.75, -12.5),
            Vec3::new(0.70, 1.50, 0.70),
            Color::srgb(0.15, 0.46, 0.42),
            Color::srgb(0.28, 0.82, 0.72),
        ),
        (
            Vec3::new(12.5, 0.70, 12.5),
            Vec3::new(0.80, 1.40, 0.80),
            Color::srgb(0.55, 0.16, 0.28),
            Color::srgb(0.90, 0.28, 0.44),
        ),
        (
            Vec3::new(-12.5, 0.65, 12.5),
            Vec3::new(0.70, 1.30, 0.70),
            Color::srgb(0.62, 0.48, 0.20),
            Color::srgb(0.95, 0.76, 0.26),
        ),
    ];

    for (offset, size, base, glow) in pois {
        spawn_v2_cube(
            commands,
            meshes,
            materials,
            Vec3::new(
                player_pos.x + offset.x,
                ground_y + offset.y,
                player_pos.z + offset.z,
            ),
            size,
            base,
            glow,
        );
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.62))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: glow,
                emissive: (glow.to_linear() * 1.15).into(),
                perceptual_roughness: 0.38,
                metallic: 0.0,
                ..default()
            })),
            Transform::from_translation(Vec3::new(
                player_pos.x + offset.x,
                ground_y + offset.y + size.y * 0.58,
                player_pos.z + offset.z,
            )),
            V2WorldMarker,
        ));
    }
}

fn spawn_v2_cube(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    size: Vec3,
    base: Color,
    glow: Color,
) -> Entity {
    commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: base,
                emissive: (glow.to_linear() * 0.65).into(),
                perceptual_roughness: 0.62,
                metallic: 0.0,
                ..default()
            })),
            Transform::from_translation(pos),
            V2WorldMarker,
        ))
        .id()
}

fn spawn_avatar_cube(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    size: Vec3,
    color: Color,
    offset: Vec3,
) {
    let entity = spawn_cube(commands, meshes, materials, pos, size, color);
    commands.entity(entity).insert(AvatarPart { offset });
}

fn spawn_cube(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    size: Vec3,
    color: Color,
) -> Entity {
    commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,

                emissive: (color.to_linear() * 0.25).into(),
                perceptual_roughness: 0.6,
                metallic: 0.1,
                ..default()
            })),
            Transform::from_translation(pos),
        ))
        .id()
}




pub fn follow_player_avatar(
    mut q: Query<(&mut Transform, &AvatarPart)>,
    player: Res<PlayerState>,
    game_world: Res<GameWorld>,
) {
    let ground_top = effective_ground_height(&game_world, player.block_pos[0], player.block_pos[2]);

    for (mut t, part) in q.iter_mut() {
        t.translation = Vec3::new(player.pos.x, ground_top + part.offset.y, player.pos.z)
            + Vec3::new(part.offset.x, 0.0, part.offset.z);
    }
}





pub fn follow_monster_cubes(
    mut q: Query<(&mut Transform, &mut MonsterCube)>,
    game_world: Res<GameWorld>,
    player: Res<PlayerState>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    for (mut t, mut mc) in q.iter_mut() {

        let to_player = player.pos - mc.base;
        let dist = (to_player.x * to_player.x + to_player.z * to_player.z).sqrt();
        let dir = if dist > 0.1 {
            Vec3::new(to_player.x / dist, 0.0, to_player.z / dist)
        } else {
            Vec3::ZERO
        };

        let speed = if dist < 12.0 { 0.4 } else { 0.0 };
        let new_base = mc.base + dir * speed * dt;

        let new_dist =
            ((new_base.x - player.pos.x).powi(2) + (new_base.z - player.pos.z).powi(2)).sqrt();
        mc.base = if new_dist < 2.5 { mc.base } else { new_base };
        let ground_top = effective_ground_height(&game_world, mc.base.x as i32, mc.base.z as i32);
        t.translation = Vec3::new(mc.base.x, ground_top + 0.5, mc.base.z);
    }
}


pub fn animate_monsters(time: Res<Time>, mut q: Query<(&mut Transform, &MonsterCube)>) {
    let t = time.elapsed_secs();
    for (i, (mut transform, monster)) in q.iter_mut().enumerate() {
        let phase = (i as f32) * 0.7;

        let bob = (t * 1.5 + phase).sin() * 0.12;
        transform.translation = monster.base + Vec3::Y * bob;

        transform.rotate_y(0.4 * time.delta_secs());
    }
}


pub fn animate_cloud_puffs(time: Res<Time>, mut q: Query<(&mut Transform, &CloudPuff)>) {
    let t = time.elapsed_secs();
    for (mut tf, puff) in q.iter_mut() {
        let bob = (t * 0.6 + puff.phase).sin() * 0.15;
        tf.translation = puff.base + Vec3::Y * bob;
    }
}






pub fn animate_avatar(
    time: Res<Time>,
    mut q: Query<&mut Transform, With<AvatarPart>>,
    player: Res<PlayerState>,
) {
    let t = time.elapsed_secs();
    let bob = (t * 2.0).sin() * 0.05;
    let upper_bob = bob;
    let base = player.pos;
    for (i, mut transform) in q.iter_mut().enumerate() {
        let offset = match i {
            0 => avatar_offset(Vec3::new(0.0, 0.70 + upper_bob, 0.0)),
            1 => avatar_offset(Vec3::new(0.0, 0.80 + upper_bob, 0.0)),
            2 => avatar_offset(Vec3::new(-0.10, 0.72 + upper_bob, 0.22)),
            3 => avatar_offset(Vec3::new(0.10, 0.72 + upper_bob, 0.22)),
            4 => avatar_offset(Vec3::new(-0.085, 0.76 + upper_bob, 0.27)),
            5 => avatar_offset(Vec3::new(0.115, 0.76 + upper_bob, 0.27)),
            6 => avatar_offset(Vec3::new(-0.22, 0.66 + upper_bob, 0.18)),
            7 => avatar_offset(Vec3::new(0.22, 0.66 + upper_bob, 0.18)),
            8 => avatar_offset(Vec3::new(0.0, 0.62 + upper_bob, 0.27)),
            9 => avatar_offset(Vec3::new(0.0, 0.40 + upper_bob, 0.0)),
            10 => avatar_offset(Vec3::new(-0.10, 0.10, 0.0)),
            11 => avatar_offset(Vec3::new(0.10, 0.10, 0.0)),
            12 => avatar_offset(Vec3::new(-0.10, 0.05, 0.05)),
            13 => avatar_offset(Vec3::new(0.10, 0.05, 0.05)),
            14 => avatar_offset(Vec3::new(-0.30, 0.42 + upper_bob, 0.0)),
            15 => avatar_offset(Vec3::new(0.30, 0.42 + upper_bob, 0.0)),
            _ => Vec3::ZERO,
        };
        transform.translation = base + offset;
    }
}
