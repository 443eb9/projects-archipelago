#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::globals::Globals

struct Settings {
    speed: f32,
}

@group(0) @binding(0)
var main_tex: texture_2d<f32>;

@group(0) @binding(1)
var main_tex_sampler: sampler;

@group(0) @binding(2)
var<uniform> settings: Settings;

@group(0) @binding(3)
var<uniform> globals: Globals;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4f {
    let color = textureSample(main_tex, main_tex_sampler, in.uv).rgb;
    let gray = color.r * 0.299 + color.g * 0.587 + color.b * 0.114;
    return vec4f(gray, gray, gray, 1.) * sin(globals.time * settings.speed);
}
