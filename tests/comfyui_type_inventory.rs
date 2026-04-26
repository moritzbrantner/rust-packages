use std::collections::BTreeSet;

use video_analysis as va;

fn observed_types_from_workflows() -> BTreeSet<va::comfyui_data::ComfySocketType> {
    [
        va::image_comfyui::build_generation_workflow(
            &va::image_comfyui::ImageGenerationRequest::new("red cube"),
        )
        .unwrap(),
        va::image_comfyui::build_generation_workflow(
            &va::image_comfyui::ImageGenerationRequest::new("repair")
                .mode(va::image_comfyui::ImageGenerationMode::Inpaint)
                .input_image("input.png")
                .mask_image("mask.png"),
        )
        .unwrap(),
        va::image_comfyui::build_generation_workflow(
            &va::image_comfyui::ImageGenerationRequest::new("upscale")
                .mode(va::image_comfyui::ImageGenerationMode::Upscale)
                .input_image("input.png"),
        )
        .unwrap(),
    ]
    .iter()
    .flat_map(|workflow| workflow.observed_socket_types().all())
    .collect()
}

#[test]
fn observed_comfyui_types_map_to_matrix_rows() {
    let matrix = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/COMFYUI_TYPE_MATRIX.md"
    ))
    .unwrap();
    let observed = observed_types_from_workflows();

    let expected_rows = [
        (
            va::comfyui_data::ComfySocketType::Image,
            "image-analysis-core",
        ),
        (
            va::comfyui_data::ComfySocketType::Mask,
            "`image-analysis-core` + `tensor-data`",
        ),
        (
            va::comfyui_data::ComfySocketType::Latent,
            "`comfyui-latents` + `tensor-data`",
        ),
        (
            va::comfyui_data::ComfySocketType::Model,
            "`comfyui-data` + `comfyui-models`",
        ),
        (va::comfyui_data::ComfySocketType::Clip, "`comfyui-data`"),
        (
            va::comfyui_data::ComfySocketType::Vae,
            "`comfyui-data` + `comfyui-models`",
        ),
        (
            va::comfyui_data::ComfySocketType::Conditioning,
            "Minimal tensor-backed runtime schema",
        ),
        (
            va::comfyui_data::ComfySocketType::UpscaleModel,
            "`comfyui-data` + `comfyui-models`",
        ),
    ];

    for (socket_type, owner_or_status) in expected_rows {
        assert!(observed.contains(&socket_type));
        let row = matrix
            .lines()
            .find(|line| line.contains(&format!("| `{}` |", socket_type)))
            .unwrap_or("");
        assert!(!row.is_empty(), "missing row for {socket_type}");
        assert!(
            row.contains(owner_or_status),
            "missing owner/status marker `{owner_or_status}` for {socket_type}"
        );
    }
}
