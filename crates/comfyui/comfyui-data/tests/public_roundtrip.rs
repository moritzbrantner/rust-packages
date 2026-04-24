use comfyui_data::{prompt_from_json_str, write_prompt_pretty, ComfyWorkflow, WorkflowNodeId};

fn comfy_workflow_json() -> &'static str {
    r#"{
  "version": 0.4,
  "nodes": [
    {"id": 1, "type": "LoadImage", "outputs": [{"name": "IMAGE", "type": "IMAGE", "links": [1]}]},
    {"id": 2, "type": "SaveImage", "inputs": [{"name": "images", "type": "IMAGE", "link": 1}]}
  ],
  "links": [[1, 1, 0, 2, 0, "IMAGE"]],
  "last_node_id": 2,
  "last_link_id": 1
}"#
}

fn comfy_prompt_json() -> &'static str {
    r#"{
  "1": {"class_type": "LoadImage", "inputs": {"image": "input.png"}},
  "2": {"class_type": "SaveImage", "inputs": {"images": ["1", 0]}}
}"#
}

#[test]
fn workflow_and_prompt_json_round_trip_through_public_api() {
    let workflow = ComfyWorkflow::from_json_str(comfy_workflow_json()).unwrap();
    workflow.validate().unwrap();
    assert_eq!(workflow.nodes.len(), 2);
    assert_eq!(workflow.links[0].origin_id, WorkflowNodeId::Number(1));

    let mut workflow_json = Vec::new();
    workflow.write_pretty(&mut workflow_json).unwrap();
    let workflow_again =
        ComfyWorkflow::from_json_str(std::str::from_utf8(&workflow_json).unwrap()).unwrap();
    workflow_again.validate().unwrap();
    assert_eq!(workflow_again.links, workflow.links);

    let prompt = prompt_from_json_str(comfy_prompt_json()).unwrap();
    let mut prompt_json = Vec::new();
    write_prompt_pretty(&prompt, &mut prompt_json).unwrap();
    let prompt_again = prompt_from_json_str(std::str::from_utf8(&prompt_json).unwrap()).unwrap();
    assert_eq!(prompt_again.len(), 2);
}
