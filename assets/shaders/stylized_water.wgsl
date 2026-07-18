#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}
#endif

struct WaterSurfaceExtension {
    time: f32,
    wave_strength: f32,
    foam_strength: f32,
    _padding: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var<uniform> water_surface: WaterSurfaceExtension;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = alpha_discard(
        pbr_input.material,
        pbr_input.material.base_color,
    );

    let position = pbr_input.world_position.xyz;
    let wave_a = sin(position.x * 0.72 + position.z * 0.31 + water_surface.time * 0.78);
    let wave_b = sin(position.x * -0.38 + position.z * 0.91 + water_surface.time * 0.52 + 1.7);
    let wave = wave_a * 0.5 + wave_b * 0.5;
    let wave_light = 1.0 + wave * water_surface.wave_strength;

    var color = pbr_input.material.base_color.rgb * wave_light;

#ifdef VERTEX_COLORS
    // RGB must remain white because Bevy's StandardMaterial has already used
    // it to tint base_color. Opaque vertex alpha carries our custom metadata.
    let shoreline = clamp(in.color.a, 0.0, 1.0);
    let foam_pattern = smoothstep(0.54, 0.92, wave + 0.5);
    let foam = shoreline * foam_pattern * water_surface.foam_strength;
    color = mix(color, vec3<f32>(0.46, 0.86, 0.84), foam);
#endif

    pbr_input.material.base_color = vec4<f32>(color, pbr_input.material.base_color.a);

#ifdef PREPASS_PIPELINE
    return deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
#endif
}
