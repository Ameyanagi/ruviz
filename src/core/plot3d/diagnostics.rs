/// Structured counters shared by 3D benchmarks and backend assertions.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderDiagnostics3D {
    pub scene_compiles: u64,
    pub triangulations: u64,
    pub normal_recomputations: u64,
    pub bvh_rebuilds: u64,
    pub vertex_upload_bytes: u64,
    pub index_upload_bytes: u64,
    pub buffer_creations: u64,
    pub camera_uniform_writes: u64,
    pub draw_calls: u64,
    pub points_submitted: u64,
    pub triangles_submitted: u64,
    pub primitives_culled: u64,
    pub readback_bytes: u64,
    pub presentation_vertex_upload_bytes: u64,
    pub presentation_texture_upload_bytes: u64,
    pub surface_presents: u64,
    pub surface_reconfigurations: u64,
    pub queue_waits: u64,
    pub actual_backend: String,
    pub adapter_name: Option<String>,
    pub sample_count: u32,
    pub fallback_reason: Option<String>,
    pub sampling_mode: String,
}

impl Default for RenderDiagnostics3D {
    fn default() -> Self {
        Self {
            scene_compiles: 0,
            triangulations: 0,
            normal_recomputations: 0,
            bvh_rebuilds: 0,
            vertex_upload_bytes: 0,
            index_upload_bytes: 0,
            buffer_creations: 0,
            camera_uniform_writes: 0,
            draw_calls: 0,
            points_submitted: 0,
            triangles_submitted: 0,
            primitives_culled: 0,
            readback_bytes: 0,
            presentation_vertex_upload_bytes: 0,
            presentation_texture_upload_bytes: 0,
            surface_presents: 0,
            surface_reconfigurations: 0,
            queue_waits: 0,
            actual_backend: "unresolved".to_string(),
            adapter_name: None,
            sample_count: 0,
            fallback_reason: None,
            sampling_mode: "full".to_string(),
        }
    }
}

#[cfg(all(test, feature = "serde"))]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_serialize_with_stable_field_names() {
        let json = serde_json::to_string(&RenderDiagnostics3D::default())
            .expect("serialize 3D diagnostics");
        assert!(json.contains("\"scene_compiles\":0"));
        assert!(json.contains("\"actual_backend\":\"unresolved\""));
        assert!(json.contains("\"adapter_name\":null"));
        assert!(json.contains("\"sample_count\":0"));
        assert!(json.contains("\"sampling_mode\":\"full\""));
        assert!(json.contains("\"presentation_texture_upload_bytes\":0"));
        assert!(json.contains("\"surface_presents\":0"));
    }
}
