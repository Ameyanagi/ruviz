struct Camera3D {
    view_projection: mat4x4<f32>,
    axis_aspect: vec4<f32>,
    viewport: vec4<f32>,
};

struct PointMaterial3D {
    color: vec4<f32>,
    parameters: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera3D;

@group(1) @binding(0)
var<uniform> material: PointMaterial3D;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec4<f32>,
) -> VertexOutput {
    let corners = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0),
    );
    let corner = corners[vertex_index];
    let center = camera.view_projection * vec4<f32>(
        position.xyz * camera.axis_aspect.xyz,
        1.0,
    );
    let ndc_per_pixel = vec2<f32>(2.0 / camera.viewport.x, 2.0 / camera.viewport.y);
    let radius = max(material.parameters.x * camera.viewport.z / 72.0 * 0.5, 0.5);
    var output: VertexOutput;
    let clip_offset = corner * radius * ndc_per_pixel * center.w;
    output.clip_position = vec4<f32>(center.xy + clip_offset, center.z, center.w);
    output.local = corner;
    return output;
}

fn marker_contains(marker: u32, point: vec2<f32>) -> bool {
    let x = abs(point.x);
    let y = abs(point.y);
    let radial = length(point);
    if marker == 0u {
        return radial <= 1.0;
    }
    if marker == 1u {
        return x <= 1.0 && y <= 1.0;
    }
    if marker == 2u {
        return point.y >= -1.0 && point.y <= 1.0 && x <= (1.0 - point.y) * 0.58;
    }
    if marker == 3u {
        return point.y >= -1.0 && point.y <= 1.0 && x <= (1.0 + point.y) * 0.58;
    }
    if marker == 4u {
        return x + y <= 1.0;
    }
    if marker == 5u {
        return (x <= 0.22 && y <= 1.0) || (y <= 0.22 && x <= 1.0);
    }
    if marker == 6u {
        return x <= 1.0 && y <= 1.0 && abs(x - y) <= 0.28;
    }
    if marker == 7u {
        if radial > 1.0 {
            return false;
        }
        let angle = atan2(point.y, point.x);
        let boundary = 0.58 + 0.42 * abs(cos(angle * 5.0));
        return radial <= boundary;
    }
    if marker == 8u {
        return radial >= 0.62 && radial <= 1.0;
    }
    if marker == 9u {
        return x <= 1.0 && y <= 1.0 && (x >= 0.68 || y >= 0.68);
    }
    if marker == 10u {
        let outer = point.y >= -1.0 && point.y <= 1.0 && x <= (1.0 - point.y) * 0.58;
        let inner = point.y >= -0.48 && point.y <= 0.62 && x <= (0.62 - point.y) * 0.44;
        return outer && !inner;
    }
    let diamond_distance = x + y;
    return diamond_distance >= 0.62 && diamond_distance <= 1.0;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let marker = u32(round(material.parameters.y));
    if !marker_contains(marker, input.local) {
        discard;
    }
    return material.color;
}
