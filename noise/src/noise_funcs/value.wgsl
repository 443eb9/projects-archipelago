#define_import_path noise_funcs::value

#import noise::{hash::hash12, types::NoiseSettings}

fn noise_main(p: vec2f, settings: NoiseSettings) -> f32 {
    let np = p * settings.scale;
    let t = smoothstep(vec2f(0.), vec2f(1.), fract(np));
    let p0 = floor(np);

    return mix(
        mix(hash12(p0), hash12(p0 + vec2f(1., 0.)), t.x),
        mix(hash12(p0 + vec2f(0., 1.)), hash12(p0 + vec2f(1., 1.)), t.x),
        t.y
    );
}
