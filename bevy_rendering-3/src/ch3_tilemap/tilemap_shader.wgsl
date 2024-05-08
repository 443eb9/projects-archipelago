#import bevy_sprite::mesh2d_view_bindings::view

// imported:
// @group(0) @binding(0) var<uniform> view: View;

struct TilemapVertexInput {
    @builtin(vertex_index) v_index: u32,
    @location(0) position: vec3f,
    @location(1) index: vec2u,
    @location(2) tint: vec4f,
    @location(3) texture_index: vec2i,
}

struct TilemapVertexOutput {
    @builtin(position) position: vec4f,
    @location(0) tint: vec4f,
    @location(1) texture_index: i32,
    @location(2) uv: vec2f,
}

struct TilemapUniform {
    translation: vec2f,
    slot_size: vec2f,
    time: f32,
}

@group(1) @binding(0) var<uniform> tilemap: TilemapUniform;
@group(1) @binding(1) var texture: texture_2d<f32>;
@group(1) @binding(2) var texture_sampler: sampler;

fn get_mesh_origin(input: TilemapVertexInput) -> vec2<f32> {
    return vec2<f32>(input.index.xy) * tilemap.slot_size;
}

@vertex
fn vertex(input: TilemapVertexInput) -> TilemapVertexOutput {
    var output: TilemapVertexOutput;
    var mesh_origin = get_mesh_origin(input);
    
    var translations = array<vec2<f32>, 4>(
        vec2<f32>(0., 0.),
        vec2<f32>(0., 1.),
        vec2<f32>(1., 1.),
        vec2<f32>(1., 0.),
    );

    var position_model = translations[input.v_index % 4u] * tilemap.slot_size + mesh_origin;
    var position_world = vec4<f32>(position_model + tilemap.translation, 0., 1.);

    output.position = view.view_proj * position_world;
    output.tint = input.tint;

    var uvs = array<vec2<f32>, 4>(
        vec2<f32>(0., 1.),
        vec2<f32>(0., 0.),
        vec2<f32>(1., 0.),
        vec2<f32>(1., 1.),
    );
    output.uv = uvs[(input.v_index) % 4u];

    // if input.texture_index.y != -1 {
    //     // Means that this tile is a animated tile
    //     let start = input.texture_index.x;
    //     let length = input.texture_index.y;
    //     // The number before the start index is the fps.
    //     // See `register` function in TilemapAnimations.
    //     let fps = f32(anim_seqs[start - 1]);
    //     var frame = i32(tilemap.time * fps) % length;
    //     output.texture_index = anim_seqs[start + frame];
    // } else {
        output.texture_index = input.texture_index.x;
    // }

    return output;
}

@fragment
fn fragment(input: TilemapVertexOutput) -> @location(0) vec4f {
    return input.tint;
}
