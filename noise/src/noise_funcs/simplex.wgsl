#define_import_path noise_funcs::simplex

#import noise::hash::hash22

// xy -> uv
const F = 0.3660254037;
// uv -> xy
const G = -0.211324865;

fn dot_gradiant(p0: vec2f, o: vec2f) -> f32 {
    return dot(normalize(hash22(p0) * vec2f(2.) - vec2f(1.)), o);
}

fn noise_main(p: vec2f) -> f32 {
    // Skew to figure out the cell id
    let coe1 = F * (p.x + p.y);
    let sp = p + vec2f(coe1);
    let cell_id = floor(sp);

    let coe2 = G * (cell_id.x + cell_id.y);

    // v = vertex, o = offset, d = squared distance

    // Now skew it back
    let v0 = cell_id + vec2f(coe2);
    let o0 = p - v0;
    let d0 = dot(o0, o0);

    var dx = 0.;
    var dy = 1.;
    if o0.x > o0.y {
        dx = 1.;
        dy = 0.;
    }

    let v1 = v0 + vec2f(dx, dy) + vec2f(G);
    let o1 = p - v1;
    let d1 = dot(o1, o1);

    let v2 = v0 + vec2f(1.) + vec2f(2. * G);
    let o2 = p - v2;
    let d2 = dot(o2, o2);

    var n0 = 0.;
    if d0 < 0.5 {
        n0 = dot_gradiant(v0, o0) * pow(0.5 - d0, 4.);
    }

    var n1 = 0.;
    if d1 < 0.5 {
        n1 = dot_gradiant(v1, o1) * pow(0.5 - d1, 4.);
    }

    var n2 = 0.;
    if d2 < 0.5 {
        n2 = dot_gradiant(v2, o2) * pow(0.5 - d2, 4.);
    }

    return 70. * (n0 + n1 + n2) * 0.5 + 0.5;
}
