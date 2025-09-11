// Coordinate transformation compute shader
// Transforms input coordinates to screen space coordinates

struct TransformParams {
    scale_x: f32,
    scale_y: f32,
    offset_x: f32,
    offset_y: f32,
    width: u32,
    height: u32,
    _padding_1: u32,
    _padding_2: u32,
}

struct Point2D {
    x: f32,
    y: f32,
}

@group(0) @binding(0) var<storage, read> input_points: array<Point2D>;
@group(0) @binding(1) var<storage, read_write> output_points: array<Point2D>;
@group(0) @binding(2) var<uniform> params: TransformParams;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    
    // Bounds check
    if (index >= arrayLength(&input_points)) {
        return;
    }
    
    let input_point = input_points[index];
    
    // Transform from data space to screen space
    // x_screen = (x_data - x_min) * scale_x + offset_x
    // y_screen = (y_data - y_min) * scale_y + offset_y
    let x_screen = input_point.x * params.scale_x + params.offset_x;
    let y_screen = input_point.y * params.scale_y + params.offset_y;
    
    // Clamp to screen bounds
    let clamped_x = clamp(x_screen, 0.0, f32(params.width - 1u));
    let clamped_y = clamp(y_screen, 0.0, f32(params.height - 1u));
    
    output_points[index] = Point2D(clamped_x, clamped_y);
}