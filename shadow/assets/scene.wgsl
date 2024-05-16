struct Camera {
    view: mat4x4f,
    proj: mat4x4f,
}

struct VertexInput {
    @location(0) position: vec3f,
    @location(1) normal: vec3f,
}

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) normal: vec3f,
}

@group(0) @binding(0) var<uniform> camera: Camera;

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = camera.proj * camera.view * vec4f(input.position, 1.);
    output.normal = input.normal;
    return normal;
}

@fragment
fn fragment(input: VertexOutput) -> @location(0) vec4f {
    return vec4f(1.);
}
