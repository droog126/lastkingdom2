use bevy::prelude::*;
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

    let mut color_gradient = bevy_hanabi::prelude::Gradient::new();
    color_gradient.add_key(0.0, Vec4::new(3.5, 1.05, 0.10, 1.0));
    color_gradient.add_key(0.45, Vec4::new(2.2, 0.35, 0.025, 0.95));
    color_gradient.add_key(1.0, Vec4::new(0.15, 0.015, 0.0, 0.0));

    let mut size_gradient = bevy_hanabi::prelude::Gradient::new();
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
