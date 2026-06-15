use std::time::{Duration, Instant};

use image_analysis_comfyui::{
    build_generation_workflow, ComfyUiClient, ComfyUiClientOptions, ImageGenerationRequest,
};
use serde_json::Value;

#[test]
#[ignore = "requires a running ComfyUI server"]
fn comfyui_submits_generation_workflow_when_configured() {
    let Some(base_url) = std::env::var("COMFYUI_URL").ok() else {
        eprintln!("skipping ComfyUI external smoke because COMFYUI_URL is unset");
        return;
    };

    let checkpoint = std::env::var("COMFYUI_CHECKPOINT")
        .unwrap_or_else(|_| "v1-5-pruned-emaonly.safetensors".to_string());
    let workflow = build_generation_workflow(
        &ImageGenerationRequest::new("external smoke test")
            .checkpoint(checkpoint)
            .output_prefix("external-smoke"),
    )
    .expect("build ComfyUI workflow");

    let client = ComfyUiClient::new(ComfyUiClientOptions {
        base_url: Some(base_url),
        timeout: Duration::from_secs(30),
        ..ComfyUiClientOptions::default()
    });
    let submission = client
        .submit_prompt(&workflow)
        .expect("submit ComfyUI prompt")
        .expect("COMFYUI_URL should produce a submission");
    assert!(!submission.prompt_id.trim().is_empty());
    assert!(submission.response.is_object());

    if std::env::var_os("COMFYUI_WAIT_FOR_OUTPUT").is_some() {
        let history =
            wait_for_history(&client, &submission.prompt_id).expect("wait for ComfyUI history");
        assert!(history.is_object());
        assert!(
            history.get(&submission.prompt_id).is_some() || non_empty_object(&history),
            "ComfyUI history did not contain prompt output: {history}"
        );
    }
}

fn wait_for_history(
    client: &ComfyUiClient,
    prompt_id: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    let timeout = std::env::var("COMFYUI_WAIT_TIMEOUT_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(60));
    let start = Instant::now();
    loop {
        let history = client.prompt_history(prompt_id)?.unwrap_or(Value::Null);
        if history.get(prompt_id).is_some() || non_empty_object(&history) {
            return Ok(history);
        }
        if start.elapsed() >= timeout {
            return Ok(history);
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

fn non_empty_object(value: &Value) -> bool {
    value.as_object().is_some_and(|object| !object.is_empty())
}
