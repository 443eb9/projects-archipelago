#define_import_path noise_funcs::value

#import noise::hash::hash12

fn noise_main(p: vec2f) -> f32 {
    let t = smoothstep(vec2f(0.), vec2f(1.), fract(p));
    let p0 = floor(p);

    return mix(
        mix(hash12(p0), hash12(p0 + vec2f(1., 0.)), t.x),
        mix(hash12(p0 + vec2f(0., 1.)), hash12(p0 + vec2f(1., 1.)), t.x),
        t.y
    );
}
