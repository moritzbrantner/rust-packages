use comfyui_data::{prompt_from_json_str, write_prompt_pretty, ComfyWorkflow, WorkflowNodeId};

#[test]
fn workflow_and_prompt_json_round_trip_through_public_api() {
    let workflow =
        ComfyWorkflow::from_json_str(video_analysis_test_support::comfy_workflow_json()).unwrap();
    workflow.validate().unwrap();
    assert_eq!(workflow.nodes.len(), 2);
    assert_eq!(workflow.links[0].origin_id, WorkflowNodeId::Number(1));

    let mut workflow_json = Vec::new();
    workflow.write_pretty(&mut workflow_json).unwrap();
    let workflow_again =
        ComfyWorkflow::from_json_str(std::str::from_utf8(&workflow_json).unwrap()).unwrap();
    workflow_again.validate().unwrap();
    assert_eq!(workflow_again.links, workflow.links);

    let prompt = prompt_from_json_str(video_analysis_test_support::comfy_prompt_json()).unwrap();
    let mut prompt_json = Vec::new();
    write_prompt_pretty(&prompt, &mut prompt_json).unwrap();
    let prompt_again = prompt_from_json_str(std::str::from_utf8(&prompt_json).unwrap()).unwrap();
    assert_eq!(prompt_again.len(), 2);
}
