use video_analysis as va;

#[test]
fn root_facade_covers_comfyui_reexports() {
    let workflow = va::image_comfyui::build_generation_workflow(
        &va::image_comfyui::ImageGenerationRequest::new("red cube"),
    )
    .unwrap();
    assert!(!workflow.nodes.is_empty());
    assert!(workflow
        .observed_socket_types()
        .contains(&va::comfyui_data::ComfySocketType::Latent));

    let latent = va::comfyui_latents::LatentBatch::new(
        tensor_data::F32Tensor::from_dims([1, 4, 64, 64], vec![0.0; 16_384]).unwrap(),
    )
    .unwrap();
    assert_eq!(latent.image_size().unwrap().width, 512);
    assert_eq!(
        va::comfyui_models::ComfyModelRole::Vae.kind(),
        va::comfyui_models::ComfyModelKind::Vae
    );
}
