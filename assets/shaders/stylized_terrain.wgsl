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

struct StylizedTerrainExtension {
    shadow_floor: f32,
    shadow_lift: f32,
    _padding: vec2<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var<uniform> stylized_terrain: StylizedTerrainExtension;

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

#ifdef PREPASS_PIPELINE
    return deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);

    let floor_strength = clamp(stylized_terrain.shadow_floor, 0.0, 1.0);
    let floor_lift = max(stylized_terrain.shadow_lift, 0.0);
    let floor_color = pbr_input.material.base_color.rgb * floor_strength
        + vec3<f32>(floor_lift);
    out.color = vec4<f32>(max(out.color.rgb, floor_color), out.color.a);

    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
#endif
}
