#import bevy_pbr::{
    mesh_bindings::mesh,
    mesh_functions,
    forward_io::{Vertex, VertexOutput},
    view_transformations::position_world_to_clip,
}

struct GrassWind { amplitude: f32, strength: f32, speed: f32, _padding: f32 }
@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var<uniform> grass_wind: GrassWind;

@vertex
fn vertex(vertex_in: Vertex) -> VertexOutput {
    var vertex = vertex_in;
    let t = grass_wind.amplitude;
    let height = clamp(vertex.position.y / 0.72, 0.0, 1.0);
    let sway = sin(vertex.position.x * 11.0 + vertex.position.z * 7.0 + t)
        * grass_wind.strength * grass_wind.speed * height * height;
    vertex.position.x += sway;
    vertex.position.z += sway * 0.35;

    var out: VertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4(vertex.position, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);
#ifdef VERTEX_NORMALS
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex_in.instance_index);
#endif
#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif
#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif
    return out;
}
