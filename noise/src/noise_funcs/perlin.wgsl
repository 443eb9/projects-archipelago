#define_import_path noise_funcs::perlin

#import noise::hash::hash22

fn dot_gradiant(p0: vec2f, p: vec2f) -> f32 {
    return dot(normalize(hash22(p0) * vec2f(2.) - vec2f(1.)), p - p0);
}

fn fade(x: f32) -> f32 {
    return 6. * pow(x, 5.) - 15. * pow(x, 4.) + 10. * pow(x, 3.);
}

fn noise_main(p: vec2f) -> f32 {
    let tx = fade(fract(p).x);
    let ty = fade(fract(p).y);
    let p0 = floor(p);

    return mix(
        mix(dot_gradiant(p0, p), dot_gradiant(p0 + vec2f(1., 0.), p), tx),
        mix(dot_gradiant(p0 + vec2f(0., 1.), p), dot_gradiant(p0 + vec2f(1.), p), tx),
        ty
    ) * 0.5 + 0.5;
}
