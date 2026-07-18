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
    cel_steps: f32,
    rim_strength: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var<uniform> stylized_terrain: StylizedTerrainExtension;

// Cheap, deterministic world-space variation keeps the procedural surface
// from reading as one giant flat-color sheet. Broad signals avoid visible
// tiling and remain stable when the terrain mesh is rebuilt after an edit.
fn terrain_surface_variation(position: vec2<f32>) -> f32 {
    let broad = sin(dot(position, vec2<f32>(0.075, 0.113))) * 0.5 + 0.5;
    let cross = sin(position.x * 0.041 + sin(position.y * 0.063) * 2.1) * 0.5 + 0.5;
    let detail = sin(dot(position, vec2<f32>(0.59, 0.47))) * 0.5 + 0.5;
    return 0.86 + broad * 0.18 + cross * 0.07 + detail * 0.02;
}

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

    // A small normal-driven palette lift gives the smooth heightfield a
    // readable top/side separation without turning it into hard faceted
    // bands. The altitude tint is deliberately weak and is shared by all
    // terrain categories so grass, rock and snow still belong to one world.
    let up = clamp(pbr_input.N.y, 0.0, 1.0);
    let slope = 1.0 - up;
    let slope_factor = 1.0 - slope * 0.14;
    let top_factor = 1.0 + pow(up, 3.0) * 0.06;
    let altitude = smoothstep(0.0, 30.0, pbr_input.world_position.y);
    let altitude_tint = mix(
        vec3<f32>(1.0),
        vec3<f32>(0.96, 0.985, 1.04),
        altitude * 0.18,
    );
    let surface_variation = terrain_surface_variation(pbr_input.world_position.xz);
    let tinted_base_color = pbr_input.material.base_color;
    pbr_input.material.base_color = vec4<f32>(
        tinted_base_color.rgb * slope_factor * top_factor * altitude_tint * surface_variation,
        tinted_base_color.a,
    );

    // Readable bosses, landmarks and legendary weapons receive a restrained
    // warm rim. This is material-local, so it avoids a full-screen outline
    // pass and does not halo every tree or grass blade.
    let rim_strength = clamp(stylized_terrain.rim_strength, 0.0, 1.0);
    if rim_strength > 0.0 {
        let view_alignment = clamp(dot(normalize(pbr_input.N), normalize(pbr_input.V)), 0.0, 1.0);
        let rim = pow(1.0 - view_alignment, 3.0) * rim_strength;
        let emissive = pbr_input.material.emissive;
        pbr_input.material.emissive = vec4<f32>(
            emissive.rgb + vec3<f32>(1.0, 0.42, 0.08) * rim,
            emissive.a,
        );
    }

#ifdef PREPASS_PIPELINE
    return deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);

    let floor_strength = clamp(stylized_terrain.shadow_floor, 0.0, 1.0);
    let floor_lift = max(stylized_terrain.shadow_lift, 0.0);
    let cel_steps = stylized_terrain.cel_steps;
    // Keep the default terrain fully continuous. Quantization is an explicit
    // opt-in for materials that want a cel-shaded look; applying it to every
    // fragment turns smooth normals into broad triangular color facets.
    if cel_steps > 1.0 {
        out.color = vec4<f32>(
            floor(out.color.rgb * cel_steps + vec3<f32>(0.5)) / cel_steps,
            out.color.a,
        );
    }
    let floor_color = pbr_input.material.base_color.rgb * floor_strength
        + vec3<f32>(floor_lift);
    // Blend the readability floor instead of hard-clamping to it. A hard
    // max() makes every shadowed triangle share one exact RGB value, so the
    // heightfield reads as large flat facets even when its normals are smooth.
    let floor_blend = clamp(floor_strength, 0.0, 1.0);
    out.color = vec4<f32>(
        mix(out.color.rgb, max(out.color.rgb, floor_color), floor_blend),
        out.color.a,
    );

    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
#endif
}
