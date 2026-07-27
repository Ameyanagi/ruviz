struct Camera3D {
    view_projection: mat4x4<f32>,
    axis_aspect: vec4<f32>,
    viewport: vec4<f32>,
};

struct LineMaterial3D {
    color: vec4<f32>,
    parameters: vec4<f32>,
    dash0: vec4<f32>,
    dash1: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera3D;

@group(1) @binding(0)
var<uniform> material: LineMaterial3D;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(linear) segment_coordinate: vec2<f32>,
    @location(1) @interpolate(flat) segment_length: f32,
};

/// Smallest homogeneous divisor a perspective divide may use.
const W_EPSILON: f32 = 1.0e-6;

/// In front of the near plane (z < 0), so the primitive is discarded whole.
const DEGENERATE_POSITION: vec4<f32> = vec4<f32>(0.0, 0.0, -1.0, 1.0);

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @location(0) start: vec4<f32>,
    @location(1) end: vec4<f32>,
) -> VertexOutput {
    let start_clip = camera.view_projection * vec4<f32>(
        start.xyz * camera.axis_aspect.xyz,
        1.0,
    );
    let end_clip = camera.view_projection * vec4<f32>(
        end.xyz * camera.axis_aspect.xyz,
        1.0,
    );
    // Either endpoint at or behind the eye makes the perspective divide below
    // meaningless: a negative `w` mirrors the segment and smears the expanded
    // quad across the whole frame. Emit a degenerate, fully clipped quad
    // instead. The CPU rasterizer reaches the same conclusion by clipping the
    // segment against the near plane before it projects.
    if !(start_clip.w > W_EPSILON && end_clip.w > W_EPSILON) {
        var clipped: VertexOutput;
        clipped.clip_position = DEGENERATE_POSITION;
        clipped.segment_coordinate = vec2<f32>(0.0, 0.0);
        clipped.segment_length = 0.0;
        return clipped;
    }
    let start_ndc = start_clip.xy / start_clip.w;
    let end_ndc = end_clip.xy / end_clip.w;
    let half_viewport = camera.viewport.xy * 0.5;
    let delta_pixels = (end_ndc - start_ndc) * half_viewport;
    let segment_length = length(delta_pixels);
    var direction = vec2<f32>(1.0, 0.0);
    if segment_length > 1.0e-6 {
        direction = delta_pixels / segment_length;
    }
    let perpendicular = vec2<f32>(-direction.y, direction.x);
    let endpoint = vertex_index >= 2u;
    let side = select(-1.0, 1.0, (vertex_index & 1u) == 1u);
    let line_width = max(material.parameters.x * camera.viewport.z / 72.0, 1.0);
    let radius = line_width * 0.5;
    let endpoint_clip = select(start_clip, end_clip, endpoint);
    let endpoint_ndc = select(start_ndc, end_ndc, endpoint);
    let along_offset = select(-radius, radius, endpoint);
    let pixel_offset = direction * along_offset + perpendicular * side * radius;
    let ndc_offset = pixel_offset / half_viewport;

    var output: VertexOutput;
    let clip_xy = (endpoint_ndc + ndc_offset) * endpoint_clip.w;
    output.clip_position = vec4<f32>(clip_xy, endpoint_clip.z, endpoint_clip.w);
    output.segment_coordinate = vec2<f32>(
        select(-radius, segment_length + radius, endpoint),
        side * radius,
    );
    output.segment_length = segment_length;
    return output;
}

fn dash_value(index: u32) -> f32 {
    var value = material.dash1.w;
    if index == 0u { value = material.dash0.x; }
    if index == 1u { value = material.dash0.y; }
    if index == 2u { value = material.dash0.z; }
    if index == 3u { value = material.dash0.w; }
    if index == 4u { value = material.dash1.x; }
    if index == 5u { value = material.dash1.y; }
    if index == 6u { value = material.dash1.z; }
    return max(value * camera.viewport.z / 100.0, 0.25);
}

fn dash_is_on(distance: f32) -> bool {
    let dash_count = u32(round(material.parameters.y));
    let total = material.parameters.z * camera.viewport.z / 100.0;
    if dash_count == 0u || total <= 1.0e-6 {
        return true;
    }
    var position = distance - floor(distance / total) * total;
    for (var index = 0u; index < 8u; index += 1u) {
        if index >= dash_count {
            break;
        }
        let dash_length = dash_value(index);
        if position <= dash_length {
            return (index & 1u) == 0u;
        }
        position -= dash_length;
    }
    return true;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let radius = max(material.parameters.x * camera.viewport.z / 72.0, 1.0) * 0.5;
    let along = clamp(input.segment_coordinate.x, 0.0, input.segment_length);
    let cap_distance = input.segment_coordinate.x - along;
    if length(vec2<f32>(cap_distance, input.segment_coordinate.y)) > radius {
        discard;
    }
    if !dash_is_on(along) {
        discard;
    }
    // The scene pipelines blend and resolve premultiplied.
    return vec4<f32>(material.color.rgb * material.color.a, material.color.a);
}
