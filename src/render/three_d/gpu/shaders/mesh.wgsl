struct Camera3D {
    view_projection: mat4x4<f32>,
    axis_aspect: vec4<f32>,
    viewport: vec4<f32>,
};

struct MeshMaterial3D {
    color: vec4<f32>,
    parameters: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera3D;

@group(1) @binding(0)
var<uniform> material: MeshMaterial3D;

@group(1) @binding(1)
var colormap: texture_2d<f32>;

@group(1) @binding(2)
var colormap_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) scalar: f32,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) scalar: f32,
) -> VertexOutput {
    var output: VertexOutput;
    let local = position * camera.axis_aspect.xyz;
    output.clip_position = camera.view_projection * vec4<f32>(local, 1.0);
    // Positions are stretched by `axis_aspect`, so normals transform by the
    // inverse transpose of that scale. `render/three_d/color.rs` performs the
    // identical correction for the CPU rasterizer.
    output.normal = normal / camera.axis_aspect.xyz;
    output.scalar = scalar;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    var color = material.color;
    if material.parameters.y > 0.5 {
        color = textureSample(colormap, colormap_sampler, vec2<f32>(clamp(input.scalar, 0.0, 1.0), 0.5));
    }
    if material.parameters.x < 0.5 {
        return premultiplied(color);
    }
    let fallback_normal = vec3<f32>(0.0, 0.0, 1.0);
    var normal = fallback_normal;
    if dot(input.normal, input.normal) > 1.0e-12 {
        normal = normalize(input.normal);
    }
    // Shared with `AMBIENT_INTENSITY`, `DIFFUSE_INTENSITY`, and `KEY_LIGHT` in
    // `render/three_d/color.rs`; both sides shade in linear light.
    let light = normalize(vec3<f32>(0.35, -0.45, 0.82));
    var diffuse = dot(normal, light);
    if material.parameters.z > 0.5 {
        diffuse = abs(diffuse);
    } else {
        diffuse = max(diffuse, 0.0);
    }
    let intensity = clamp(0.35 + 0.65 * diffuse, 0.0, 1.0);
    return premultiplied(vec4<f32>(color.rgb * intensity, color.a));
}

// The scene pipelines blend and resolve premultiplied, so every shader has to
// emit premultiplied colour.
fn premultiplied(color: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(color.rgb * color.a, color.a);
}
