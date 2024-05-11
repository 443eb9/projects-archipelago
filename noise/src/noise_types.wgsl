#define_import_path noise::types

struct NoiseSettings {
    aspect: vec2f,
    frequency: f32,
    amplitude: f32,
    fbm: FBMSettings,
}

struct FBMSettings {
    octaves: u32,
    lacularity: f32,
    gain: f32,
}

struct DomainWarpSettings {
    offset_a: vec2f,
    offset_b: vec2f,
}
