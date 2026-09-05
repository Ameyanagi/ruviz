// Analytic, instanced sphere impostors. Keep ray/lighting math in sync with
// three_d/sphere.rs; CPU/GPU image tests exercise the same scenes.
struct Camera {
    matrix: mat4x4<f32>,
    inverse: mat4x4<f32>,
    normal_to_view: mat4x4<f32>,
    viewport: vec4<f32>,
};
struct Material {
    unused: vec4<f32>,
    parameters: vec4<f32>, // shaded, specular strength, gloss, unused
};
@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<uniform> material: Material;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) center: vec3<f32>,
    @location(1) @interpolate(flat) radii: vec3<f32>,
    @location(2) @interpolate(flat) color: vec4<f32>,
    @location(3) @interpolate(linear, sample) ndc: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32,
    @location(0) center: vec4<f32>, @location(1) radii: vec4<f32>,
    @location(2) color: vec4<f32>) -> VertexOutput {
    var low = vec2<f32>(1e30);
    var high = vec2<f32>(-1e30);
    for (var i = 0u; i < 8u; i++) {
        let sign = vec3<f32>(select(-1.0, 1.0, (i & 1u) != 0u),
            select(-1.0, 1.0, (i & 2u) != 0u), select(-1.0, 1.0, (i & 4u) != 0u));
        let clip = camera.matrix * vec4<f32>(center.xyz + radii.xyz * sign, 1.0);
        if clip.w <= 1e-6 {
            low = vec2<f32>(-1.0);
            high = vec2<f32>(1.0);
            break;
        }
        low = min(low, clip.xy / clip.w);
        high = max(high, clip.xy / clip.w);
    }
    // Keep raster coordinates finite even for extreme zoom/offscreen centers.
    low = clamp(low, vec2<f32>(-1.0), vec2<f32>(1.0));
    high = clamp(high, vec2<f32>(-1.0), vec2<f32>(1.0));
    let corner = vec2<f32>(select(low.x, high.x, (vertex & 1u) != 0u),
        select(low.y, high.y, (vertex & 2u) != 0u));
    var output: VertexOutput;
    output.position = vec4<f32>(corner, 0.0, 1.0);
    output.center = center.xyz;
    output.radii = radii.xyz;
    output.color = color;
    output.ndc = corner;
    return output;
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
};

@fragment
// Request per-sample evaluation so the analytic silhouette receives MSAA.
fn fs_main(input: VertexOutput) -> FragmentOutput {
    if input.color.a == 0.0 { discard; }
    let ndc = input.ndc;
    let near_h = camera.inverse * vec4<f32>(ndc, 0.0, 1.0);
    let far_h = camera.inverse * vec4<f32>(ndc, 1.0, 1.0);
    let origin = near_h.xyz / near_h.w;
    let direction = normalize(far_h.xyz / far_h.w - origin);
    let o = (origin - input.center) / input.radii;
    let d = direction / input.radii;
    let a = dot(d, d);
    let b = dot(o, d);
    let c = dot(o, o) - 1.0;
    let discriminant = b*b - a*c;
    if discriminant < 0.0 || a <= 0.0 { discard; }
    let root = sqrt(discriminant);
    let near = (-b-root)/a;
    let far = (-b+root)/a;
    let t = select(far, near, near >= 0.0);
    if t < 0.0 { discard; }
    let surface = origin + direction*t;
    let clip = camera.matrix * vec4<f32>(surface, 1.0);
    let depth = clip.z / clip.w;
    if depth < 0.0 || depth > 1.0 { discard; }
    var color = input.color.rgb;
    if material.parameters.x > 0.5 {
        let normal_local = (surface-input.center)/(input.radii*input.radii);
        let normal = normalize((camera.normal_to_view * vec4<f32>(normal_local, 0.0)).xyz);
        let light = normalize(vec3<f32>(-0.35, 0.45, 0.82));
        let diffuse = max(dot(normal, light), 0.0);
        var highlight = 0.0;
        if diffuse > 0.0 {
            let halfway = normalize(light + vec3<f32>(0.0, 0.0, 1.0));
            highlight = material.parameters.y * pow(max(dot(normal, halfway), 0.0), material.parameters.z);
        }
        color = clamp(color * (0.3 + 0.7*diffuse) + vec3<f32>(highlight), vec3<f32>(0.0), vec3<f32>(1.0));
    }
    var output: FragmentOutput;
    output.color = vec4<f32>(color*input.color.a, input.color.a);
    output.depth = depth;
    return output;
}
