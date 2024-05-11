#define_import_path noise_funcs::voronoi

#import noise::hash::hash22

fn dist(a: vec2f, b: vec2f) -> f32 {
#ifdef EULER
    return distance(a, b);
#else ifdef MANHATTEN
    return abs(a.x - b.x) + abs(a.y - b.y);
#else ifdef CHEBYSHEV
    return max(abs(a.x - b.x), abs(a.y - b.y));
#endif
}

fn noise_main(p: vec2f) -> f32 {
    let p0 = floor(p);
    var min_p1 = vec2f(0.);
    var min_d = 999999.;

    for (var i = -1; i <= 1; i += 1) {
        for (var j = -1; j <= 1; j += 1) {
            let cell = p0 + vec2f(f32(i), f32(j));
            let p1 = cell + hash22(cell);
            let d = dist(p1 - p, vec2f(0.));

            if d < min_d {
                min_p1 = p1;
                min_d = d;
            }
        }
    }

    return min_d;
}
