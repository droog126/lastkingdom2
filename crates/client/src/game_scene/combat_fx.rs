use bevy::prelude::{Vec3, Vec4};
use bevy_hanabi::prelude::*;

pub fn create_hit_effect() -> EffectAsset {
    let writer = ExprWriter::new();

    let init_pos = SetPositionSphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        radius: writer.lit(0.10).expr(),
        dimension: ShapeDimension::Volume,
    };
    let init_vel = SetVelocitySphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        speed: writer.lit(2.8).uniform(writer.lit(5.8)).expr(),
    };
    let init_lifetime = SetAttributeModifier::new(
        Attribute::LIFETIME,
        writer.lit(0.20).uniform(writer.lit(0.38)).expr(),
    );
    let gravity = AccelModifier::new(writer.lit(Vec3::new(0.0, -5.5, 0.0)).expr());

    let mut color_gradient = Gradient::new();
    color_gradient.add_key(0.0, Vec4::new(3.5, 1.05, 0.10, 1.0));
    color_gradient.add_key(0.45, Vec4::new(2.2, 0.35, 0.025, 0.95));
    color_gradient.add_key(1.0, Vec4::new(0.15, 0.015, 0.0, 0.0));

    let mut size_gradient = Gradient::new();
    size_gradient.add_key(0.0, Vec3::new(0.12, 0.038, 0.038));
    size_gradient.add_key(0.65, Vec3::new(0.07, 0.022, 0.022));
    size_gradient.add_key(1.0, Vec3::ZERO);

    EffectAsset::new(48, SpawnerSettings::once(28.0.into()), writer.finish())
        .with_name("melee_hit_sparks")
        .init(init_pos)
        .init(init_vel)
        .init(init_lifetime)
        .update(gravity)
        .render(ColorOverLifetimeModifier::new(color_gradient))
        .render(SizeOverLifetimeModifier {
            gradient: size_gradient,
            screen_space_size: false,
        })
        .render(OrientModifier::new(OrientMode::AlongVelocity))
}

/// A larger, high-contrast burst for a confirmed enemy hit. It is separate
/// from the player's slash trail so the target's feedback remains readable in
/// third person without making every attack effect oversized.
pub fn create_enemy_hit_effect() -> EffectAsset {
    let writer = ExprWriter::new();

    let init_pos = SetPositionSphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        radius: writer.lit(0.16).expr(),
        dimension: ShapeDimension::Volume,
    };
    let init_vel = SetVelocitySphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        speed: writer.lit(4.2).uniform(writer.lit(8.8)).expr(),
    };
    let init_lifetime = SetAttributeModifier::new(
        Attribute::LIFETIME,
        writer.lit(0.24).uniform(writer.lit(0.46)).expr(),
    );
    let gravity = AccelModifier::new(writer.lit(Vec3::new(0.0, -4.0, 0.0)).expr());

    let mut color_gradient = Gradient::new();
    color_gradient.add_key(0.0, Vec4::new(5.0, 2.2, 0.22, 1.0));
    color_gradient.add_key(0.22, Vec4::new(4.0, 0.65, 0.04, 1.0));
    color_gradient.add_key(0.68, Vec4::new(1.2, 0.08, 0.01, 0.72));
    color_gradient.add_key(1.0, Vec4::new(0.12, 0.005, 0.0, 0.0));

    let mut size_gradient = Gradient::new();
    size_gradient.add_key(0.0, Vec3::new(0.18, 0.052, 0.052));
    size_gradient.add_key(0.55, Vec3::new(0.11, 0.032, 0.032));
    size_gradient.add_key(1.0, Vec3::ZERO);

    EffectAsset::new(96, SpawnerSettings::once(64.0.into()), writer.finish())
        .with_name("enemy_hit_burst")
        .init(init_pos)
        .init(init_vel)
        .init(init_lifetime)
        .update(gravity)
        .render(ColorOverLifetimeModifier::new(color_gradient))
        .render(SizeOverLifetimeModifier {
            gradient: size_gradient,
            screen_space_size: false,
        })
        .render(OrientModifier::new(OrientMode::AlongVelocity))
}

/// Slow, sparse forest motes provide depth cues between the camera and the
/// terrain without spawning one ECS entity per mote. Hanabi owns the whole
/// population on the GPU; the scene only owns this single effect entity.
pub fn create_ambient_mote_effect() -> EffectAsset {
    let writer = ExprWriter::new();

    let init_pos = SetPositionSphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        radius: writer.lit(24.0).expr(),
        dimension: ShapeDimension::Volume,
    };
    let init_vel = SetVelocitySphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        speed: writer.lit(0.015).uniform(writer.lit(0.055)).expr(),
    };
    let init_lifetime = SetAttributeModifier::new(
        Attribute::LIFETIME,
        writer.lit(4.0).uniform(writer.lit(8.0)).expr(),
    );
    let lift = AccelModifier::new(writer.lit(Vec3::new(0.0, 0.035, 0.0)).expr());

    let mut color_gradient = Gradient::new();
    color_gradient.add_key(0.0, Vec4::new(0.08, 0.45, 0.32, 0.0));
    color_gradient.add_key(0.18, Vec4::new(0.22, 1.25, 0.72, 0.72));
    color_gradient.add_key(0.78, Vec4::new(0.12, 0.72, 0.52, 0.45));
    color_gradient.add_key(1.0, Vec4::new(0.02, 0.18, 0.12, 0.0));

    let mut size_gradient = Gradient::new();
    size_gradient.add_key(0.0, Vec3::ZERO);
    size_gradient.add_key(0.20, Vec3::splat(0.045));
    size_gradient.add_key(0.80, Vec3::splat(0.032));
    size_gradient.add_key(1.0, Vec3::ZERO);

    EffectAsset::new(128, SpawnerSettings::rate(14.0.into()), writer.finish())
        .with_name("forest_ambient_motes")
        .init(init_pos)
        .init(init_vel)
        .init(init_lifetime)
        .update(lift)
        .render(ColorOverLifetimeModifier::new(color_gradient))
        .render(SizeOverLifetimeModifier {
            gradient: size_gradient,
            screen_space_size: false,
        })
}
