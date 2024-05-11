#define_import_path noise::noise

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import noise::types::NoiseSettings

#ifdef VALUE
#import noise_funcs::value::noise_main
#endif

#ifdef PERLIN
#import noise_funcs::perlin::noise_main
#endif

#ifdef SIMPLEX
#import noise_funcs::simplex::noise_main
#endif

@group(0) @binding(0) var<uniform> noise: NoiseSettings;

@fragment
fn fragment(input: FullscreenVertexOutput) -> @location(0) vec4f {
    let p = input.uv * noise.aspect;
    return vec4f(noise_main(p, noise));
}
