// DataShader-style aggregation compute shader
// Aggregates points into a 2D canvas for massive dataset visualization

struct AggregationParams {
    canvas_width: u32,
    canvas_height: u32,
    x_min: f32,
    x_max: f32,
    y_min: f32,
    y_max: f32,
    _padding: array<u32, 2>,
}

struct Point2D {
    x: f32,
    y: f32,
}

@group(0) @binding(0) var<storage, read> input_points: array<Point2D>;
@group(0) @binding(1) var<storage, read_write> canvas: array<atomic<u32>>;
@group(0) @binding(2) var<uniform> params: AggregationParams;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    
    // Bounds check
    if (index >= arrayLength(&input_points)) {
        return;
    }
    
    let point = input_points[index];
    
    // Skip points outside the data range
    if (point.x < params.x_min || point.x > params.x_max ||
        point.y < params.y_min || point.y > params.y_max) {
        return;
    }
    
    // Transform point to canvas coordinates
    let x_range = params.x_max - params.x_min;
    let y_range = params.y_max - params.y_min;
    
    let x_norm = (point.x - params.x_min) / x_range;
    let y_norm = (point.y - params.y_min) / y_range;
    
    let pixel_x = u32(x_norm * f32(params.canvas_width - 1u));
    let pixel_y = u32(y_norm * f32(params.canvas_height - 1u));
    
    // Calculate canvas index (row-major order)
    let canvas_index = pixel_y * params.canvas_width + pixel_x;
    
    // Bounds check for canvas
    if (canvas_index >= arrayLength(&canvas)) {
        return;
    }
    
    // Atomically increment the pixel count
    atomicAdd(&canvas[canvas_index], 1u);
}