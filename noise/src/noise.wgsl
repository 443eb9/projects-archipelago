#define_import_path noise::noise

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import noise::types::{NoiseSettings, DomainWarpSettings}

#ifdef VALUE
#import noise_funcs::value::noise_main
#endif

#ifdef PERLIN
#import noise_funcs::perlin::noise_main
#endif

#ifdef SIMPLEX
#import noise_funcs::simplex::noise_main
#endif

#ifdef VORONOI
#import noise_funcs::voronoi::noise_main
#endif

@group(0) @binding(0) var<uniform> noise: NoiseSettings;
@group(0) @binding(1) var<storage> dw_settings: array<DomainWarpSettings>;

@fragment
fn fragment(input: FullscreenVertexOutput) -> @location(0) vec4f {
    let p = input.uv * noise.aspect;
    #ifdef DOMAIN_WARP
    return vec4f(domain_warp(p, noise));
    #else ifdef FBM
    return vec4f(fbm(p, noise));
    #else
    return vec4f(noise_main(p * noise.frequency) * noise.amplitude);
    #endif
}

fn fbm(p: vec2f, settings: NoiseSettings) -> f32 {
    var freq = settings.frequency;
    var amp = settings.amplitude;
    var noise = 0.;

    for (var i = 0u; i < settings.fbm.octaves; i += 1u) {
        noise += noise_main(p * freq) * amp;
        freq *= settings.fbm.lacularity;
        amp *= settings.fbm.gain;
    }

    return noise;
}

fn warp(p: vec2f, noise: NoiseSettings, dw: DomainWarpSettings) -> vec2f {
    return vec2f(
        fbm(p + dw.offset_a, noise),
        fbm(p + dw.offset_b, noise)
    );
}

fn domain_warp(p: vec2f, noise: NoiseSettings) -> f32 {
    var p0 = p;

    for (var i = 0u; i < arrayLength(&dw_settings); i += 1u) {
        p0 = warp(p0, noise, dw_settings[i]);
    }

    return fbm(p0, noise);
}
