#define_import_path noise_funcs::perlin

#import noise::{hash::hash22, types::NoiseSettings}

fn dot_gradiant(p0: vec2f, p: vec2f) -> f32 {
    return dot(normalize(hash22(p0) * vec2f(2.) - vec2f(1.)), p - p0);
}

fn fade(x: f32) -> f32 {
    return 6. * pow(x, 5.) - 15. * pow(x, 4.) + 10. * pow(x, 3.);
}

fn noise_main(p: vec2f, settings: NoiseSettings) -> f32 {
    let np = p * settings.scale;
    let tx = fade(fract(np).x);
    let ty = fade(fract(np).y);
    let p0 = floor(np);

    return mix(
        mix(dot_gradiant(p0, np), dot_gradiant(p0 + vec2f(1., 0.), np), tx),
        mix(dot_gradiant(p0 + vec2f(0., 1.), np), dot_gradiant(p0 + vec2f(1.), np), tx),
        ty
    );
}
