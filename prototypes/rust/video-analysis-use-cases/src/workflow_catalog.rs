//! Internal module support for workflow catalog.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
/// Variants describing port kind.
pub enum PortKind {
    /// The run trigger variant.
    RunTrigger,
    /// The run config variant.
    RunConfig,
    /// The youtube URL variant.
    YoutubeUrl,
    /// The youtube collection URL variant.
    YoutubeCollectionUrl,
    /// The video file variant.
    VideoFile,
    /// The image file variant.
    ImageFile,
    /// The image batch variant.
    ImageBatch,
    /// The audio file variant.
    AudioFile,
    /// The audio waveform variant.
    AudioWaveform,
    /// The media file variant.
    MediaFile,
    /// The mask tensor variant.
    MaskTensor,
    /// The latent batch variant.
    LatentBatch,
    /// The conditioning variant.
    Conditioning,
    /// The model ref variant.
    ModelRef,
    /// The vae ref variant.
    VaeRef,
    /// The clip ref variant.
    ClipRef,
    /// The clip vision ref variant.
    ClipVisionRef,
    /// The upscale model ref variant.
    UpscaleModelRef,
    /// The model patch ref variant.
    ModelPatchRef,
    /// The transcript variant.
    Transcript,
    /// The subtitle file variant.
    SubtitleFile,
    /// The scene list variant.
    SceneList,
    /// The observation list variant.
    ObservationList,
    /// The audio events variant.
    AudioEvents,
    /// The text events variant.
    TextEvents,
    /// The data buckets variant.
    DataBuckets,
    /// The JSON report variant.
    JsonReport,
    /// The collection manifest variant.
    CollectionManifest,
    /// The collection items variant.
    CollectionItems,
    /// The song fingerprint variant.
    SongFingerprint,
    /// The song catalog variant.
    SongCatalog,
    /// The song matches variant.
    SongMatches,
    /// The music features variant.
    MusicFeatures,
    /// The template string variant.
    TemplateString,
    /// The string variant.
    String,
    /// The number variant.
    Number,
    /// The boolean variant.
    Boolean,
    /// The array variant.
    Array,
    /// The object variant.
    Object,
    /// The http request variant.
    HttpRequest,
    /// The http response variant.
    HttpResponse,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
/// Variants describing workflow semantic kind.
pub enum WorkflowSemanticKind {
    /// The run request variant.
    RunRequest,
    /// The run config variant.
    RunConfig,
    /// The template string variant.
    TemplateString,
    /// The youtube URL variant.
    YoutubeUrl,
    /// The youtube collection URL variant.
    YoutubeCollectionUrl,
    /// The video file variant.
    VideoFile,
    /// The image file variant.
    ImageFile,
    /// The image batch variant.
    ImageBatch,
    /// The audio file variant.
    AudioFile,
    /// The audio waveform variant.
    AudioWaveform,
    /// The audio wav variant.
    AudioWav,
    /// The media file variant.
    MediaFile,
    /// The mask tensor variant.
    MaskTensor,
    /// The latent batch variant.
    LatentBatch,
    /// The conditioning variant.
    Conditioning,
    /// The model ref variant.
    ModelRef,
    /// The vae ref variant.
    VaeRef,
    /// The clip ref variant.
    ClipRef,
    /// The clip vision ref variant.
    ClipVisionRef,
    /// The upscale model ref variant.
    UpscaleModelRef,
    /// The model patch ref variant.
    ModelPatchRef,
    /// The transcript variant.
    Transcript,
    /// The subtitle file variant.
    SubtitleFile,
    /// The video metadata variant.
    VideoMetadata,
    /// The scene result variant.
    SceneResult,
    /// The video observation variant.
    VideoObservation,
    /// The model request variant.
    ModelRequest,
    /// The model prediction variant.
    ModelPrediction,
    /// The transcript segment variant.
    TranscriptSegment,
    /// The audio event variant.
    AudioEvent,
    /// The text event variant.
    TextEvent,
    /// The collection manifest variant.
    CollectionManifest,
    /// The collection item variant.
    CollectionItem,
    /// The collection report variant.
    CollectionReport,
    /// The collection table variant.
    CollectionTable,
    /// The song fingerprint variant.
    SongFingerprint,
    /// The song catalog variant.
    SongCatalog,
    /// The song match variant.
    SongMatch,
    /// The music feature variant.
    MusicFeature,
    /// The http request variant.
    HttpRequest,
    /// The http response variant.
    HttpResponse,
    /// The JSON report variant.
    JsonReport,
    /// The dashboard view variant.
    DashboardView,
    /// The data record variant.
    DataRecord,
    /// The data bucket variant.
    DataBucket,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// Data type for workflow object property.
pub struct WorkflowObjectProperty {
    /// Human-readable name for this value.
    pub name: String,
    /// The value value.
    pub value: WorkflowTypeSpec,
    #[serde(default)]
    /// The required value.
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
/// Variants describing workflow type spec.
pub enum WorkflowTypeSpec {
    /// The any variant.
    Any,
    /// The null variant.
    Null,
    /// The boolean variant.
    Boolean,
    /// The number variant.
    Number,
    /// The string variant.
    String {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        /// Semantic meaning associated with this variant.
        semantic: Option<WorkflowSemanticKind>,
    },
    /// The binary variant.
    Binary {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        /// The media type value for this variant.
        media_type: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        /// Semantic meaning associated with this variant.
        semantic: Option<WorkflowSemanticKind>,
    },
    /// The array variant.
    Array {
        /// Item type represented by this value.
        items: Box<WorkflowTypeSpec>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        /// The minimum items value for this variant.
        min_items: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        /// The maximum items value for this variant.
        max_items: Option<u32>,
    },
    /// The object variant.
    Object {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        /// Object properties represented by this value.
        properties: Vec<WorkflowObjectProperty>,
        #[serde(default)]
        /// Whether additional object properties are allowed.
        additional_properties: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        /// Semantic meaning associated with this variant.
        semantic: Option<WorkflowSemanticKind>,
    },
    /// The union variant.
    Union {
        /// Candidate variants represented by this value.
        variants: Vec<WorkflowTypeSpec>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
/// Variants describing workflow input surface.
pub enum WorkflowInputSurface {
    /// The source URL variant.
    SourceUrl,
    /// The collection URL variant.
    CollectionUrl,
    /// The file variant.
    File,
    /// The config variant.
    Config,
    /// The generic variant.
    Generic,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
/// Variants describing workflow runtime validation.
pub enum WorkflowRuntimeValidation {
    /// The strict variant.
    Strict,
    /// The unsafe variant.
    Unsafe,
    /// The external input variant.
    ExternalInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// Data type for port spec.
pub struct PortSpec {
    /// Identifier for this value.
    pub id: String,
    /// Label assigned to this value.
    pub label: String,
    /// The kind value.
    pub kind: PortKind,
    /// The value type value.
    pub value_type: WorkflowTypeSpec,
    #[serde(default)]
    /// The optional value.
    pub optional: bool,
    #[serde(default)]
    /// The stream value.
    pub stream: bool,
    #[serde(default)]
    /// The exposable input value.
    pub exposable_input: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The input surface value.
    pub input_surface: Option<WorkflowInputSurface>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// The suggested adapters value.
    pub suggested_adapters: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The runtime validation value.
    pub runtime_validation: Option<WorkflowRuntimeValidation>,
}

impl PortSpec {
    /// Creates a new value.
    pub fn new(id: impl Into<String>, label: impl Into<String>, kind: PortKind) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value_type: workflow_type_for_port_kind(kind),
            kind,
            optional: false,
            stream: false,
            exposable_input: false,
            input_surface: None,
            suggested_adapters: Vec::new(),
            runtime_validation: None,
        }
    }

    /// Returns optional.
    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    /// Returns stream.
    pub fn stream(mut self) -> Self {
        self.stream = true;
        self
    }

    /// Returns exposable input.
    pub fn exposable_input(mut self, surface: WorkflowInputSurface) -> Self {
        self.exposable_input = true;
        self.input_surface = Some(surface);
        self
    }

    /// Returns value type.
    pub fn value_type(mut self, value_type: WorkflowTypeSpec) -> Self {
        self.value_type = value_type;
        self
    }

    /// Returns suggested adapters.
    pub fn suggested_adapters<I, S>(mut self, suggested_adapters: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.suggested_adapters = suggested_adapters.into_iter().map(Into::into).collect();
        self
    }

    /// Returns runtime validation.
    pub fn runtime_validation(mut self, mode: WorkflowRuntimeValidation) -> Self {
        self.runtime_validation = Some(mode);
        self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
/// Variants describing workflow category.
pub enum WorkflowCategory {
    /// The app variant.
    App,
    /// The ingest variant.
    Ingest,
    /// The youtube variant.
    Youtube,
    /// The video variant.
    Video,
    /// The audio variant.
    Audio,
    /// The text variant.
    Text,
    /// The song variant.
    Song,
    /// The collection variant.
    Collection,
    /// The output variant.
    Output,
    /// The data variant.
    Data,
    /// The logic variant.
    Logic,
    /// The network variant.
    Network,
    /// The ui variant.
    Ui,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// Data type for node spec.
pub struct NodeSpec {
    /// Identifier for this value.
    pub id: String,
    /// The title value.
    pub title: String,
    /// The package name value.
    pub package_name: String,
    /// The category value.
    pub category: WorkflowCategory,
    #[serde(default)]
    /// The ui owned value.
    pub ui_owned: bool,
    #[serde(default)]
    /// The inputs value.
    pub inputs: Vec<PortSpec>,
    #[serde(default)]
    /// The outputs value.
    pub outputs: Vec<PortSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// Data type for workflow catalog.
pub struct WorkflowCatalog {
    /// The version value.
    pub version: u32,
    /// The port kinds value.
    pub port_kinds: Vec<PortKind>,
    /// The nodes value.
    pub nodes: Vec<NodeSpec>,
    /// The compatibility value.
    pub compatibility: Vec<PortCompatibility>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// Data type for port compatibility.
pub struct PortCompatibility {
    /// The source value.
    pub source: PortKind,
    /// The target value.
    pub target: PortKind,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct WorkflowCatalogDefinition {
    version: u32,
    nodes: Vec<NodeDefinition>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct NodeDefinition {
    id: String,
    title: String,
    package_name: String,
    category: WorkflowCategory,
    #[serde(default)]
    ui_owned: bool,
    #[serde(default)]
    inputs: Vec<PortDefinition>,
    #[serde(default)]
    outputs: Vec<PortDefinition>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct PortDefinition {
    id: String,
    label: String,
    kind: PortKind,
    #[serde(default)]
    optional: bool,
    #[serde(default)]
    stream: bool,
    #[serde(default)]
    exposable_input: bool,
    #[serde(default)]
    input_surface: Option<WorkflowInputSurface>,
    #[serde(default)]
    suggested_adapters: Vec<String>,
    #[serde(default)]
    runtime_validation: Option<WorkflowRuntimeValidation>,
}

impl From<NodeDefinition> for NodeSpec {
    fn from(value: NodeDefinition) -> Self {
        Self {
            id: value.id,
            title: value.title,
            package_name: value.package_name,
            category: value.category,
            ui_owned: value.ui_owned,
            inputs: value.inputs.into_iter().map(Into::into).collect(),
            outputs: value.outputs.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<PortDefinition> for PortSpec {
    fn from(value: PortDefinition) -> Self {
        Self {
            id: value.id,
            label: value.label,
            value_type: workflow_type_for_port_kind(value.kind),
            kind: value.kind,
            optional: value.optional,
            stream: value.stream,
            exposable_input: value.exposable_input,
            input_surface: value.input_surface,
            suggested_adapters: value.suggested_adapters,
            runtime_validation: value.runtime_validation,
        }
    }
}

/// Returns default workflow catalog.
pub fn default_workflow_catalog() -> WorkflowCatalog {
    let definition =
        serde_json::from_str::<WorkflowCatalogDefinition>(include_str!("workflow_catalog.json"))
            .expect("embedded workflow catalog JSON is valid");

    WorkflowCatalog {
        version: definition.version,
        port_kinds: all_port_kinds(),
        compatibility: compatibility_pairs(),
        nodes: definition.nodes.into_iter().map(Into::into).collect(),
    }
}

/// Returns ports compatible.
pub fn ports_compatible(source: PortKind, target: PortKind) -> bool {
    workflow_types_assignable(
        &workflow_type_for_port_kind(source),
        &workflow_type_for_port_kind(target),
    )
}

/// Returns workflow types assignable.
pub fn workflow_types_assignable(source: &WorkflowTypeSpec, target: &WorkflowTypeSpec) -> bool {
    match target {
        WorkflowTypeSpec::Any => return true,
        WorkflowTypeSpec::Union { variants } => {
            return variants
                .iter()
                .any(|variant| workflow_types_assignable(source, variant));
        }
        _ => {}
    }

    match source {
        WorkflowTypeSpec::Union { variants } => {
            return variants
                .iter()
                .all(|variant| workflow_types_assignable(variant, target));
        }
        WorkflowTypeSpec::Any => return false,
        _ => {}
    }

    match (source, target) {
        (WorkflowTypeSpec::Null, WorkflowTypeSpec::Null)
        | (WorkflowTypeSpec::Boolean, WorkflowTypeSpec::Boolean)
        | (WorkflowTypeSpec::Number, WorkflowTypeSpec::Number) => true,
        (
            WorkflowTypeSpec::String {
                semantic: source_semantic,
            },
            WorkflowTypeSpec::String {
                semantic: target_semantic,
            },
        ) => semantic_assignable(source_semantic, target_semantic),
        (
            WorkflowTypeSpec::Binary {
                media_type: source_media_type,
                semantic: source_semantic,
            },
            WorkflowTypeSpec::Binary {
                media_type: target_media_type,
                semantic: target_semantic,
            },
        ) => {
            media_type_assignable(source_media_type, target_media_type)
                && semantic_assignable(source_semantic, target_semantic)
        }
        (
            WorkflowTypeSpec::Array {
                items: source_items,
                min_items: source_min_items,
                max_items: source_max_items,
            },
            WorkflowTypeSpec::Array {
                items: target_items,
                min_items: target_min_items,
                max_items: target_max_items,
            },
        ) => {
            workflow_types_assignable(source_items, target_items)
                && source_min_items.unwrap_or(0) >= target_min_items.unwrap_or(0)
                && max_items_assignable(*source_max_items, *target_max_items)
        }
        (
            WorkflowTypeSpec::Object {
                properties: source_properties,
                additional_properties: source_additional_properties,
                semantic: source_semantic,
            },
            WorkflowTypeSpec::Object {
                properties: target_properties,
                additional_properties: target_additional_properties,
                semantic: target_semantic,
            },
        ) => {
            semantic_assignable(source_semantic, target_semantic)
                && object_properties_assignable(
                    source_properties,
                    *source_additional_properties,
                    target_properties,
                    *target_additional_properties,
                )
        }
        _ => false,
    }
}

fn all_port_kinds() -> Vec<PortKind> {
    use PortKind::*;
    vec![
        RunTrigger,
        RunConfig,
        YoutubeUrl,
        YoutubeCollectionUrl,
        VideoFile,
        ImageFile,
        ImageBatch,
        AudioFile,
        AudioWaveform,
        MediaFile,
        MaskTensor,
        LatentBatch,
        Conditioning,
        ModelRef,
        VaeRef,
        ClipRef,
        ClipVisionRef,
        UpscaleModelRef,
        ModelPatchRef,
        Transcript,
        SubtitleFile,
        SceneList,
        ObservationList,
        AudioEvents,
        TextEvents,
        DataBuckets,
        JsonReport,
        CollectionManifest,
        CollectionItems,
        SongFingerprint,
        SongCatalog,
        SongMatches,
        MusicFeatures,
        TemplateString,
        String,
        Number,
        Boolean,
        Array,
        Object,
        HttpRequest,
        HttpResponse,
    ]
}

fn compatibility_pairs() -> Vec<PortCompatibility> {
    let kinds = all_port_kinds();
    kinds
        .iter()
        .flat_map(|source| {
            kinds.iter().filter_map(move |target| {
                if source != target && ports_compatible(*source, *target) {
                    Some(PortCompatibility {
                        source: *source,
                        target: *target,
                    })
                } else {
                    None
                }
            })
        })
        .collect()
}

fn workflow_type_for_port_kind(kind: PortKind) -> WorkflowTypeSpec {
    use PortKind::*;
    match kind {
        RunTrigger => open_object(Some(WorkflowSemanticKind::RunRequest)),
        RunConfig => open_object(Some(WorkflowSemanticKind::RunConfig)),
        YoutubeUrl => string_type(Some(WorkflowSemanticKind::YoutubeUrl)),
        YoutubeCollectionUrl => string_type(Some(WorkflowSemanticKind::YoutubeCollectionUrl)),
        VideoFile => binary_type(Some("video/*"), Some(WorkflowSemanticKind::VideoFile)),
        ImageFile => binary_type(Some("image/*"), Some(WorkflowSemanticKind::ImageFile)),
        ImageBatch => open_object(Some(WorkflowSemanticKind::ImageBatch)),
        AudioFile => binary_type(Some("audio/*"), Some(WorkflowSemanticKind::AudioFile)),
        AudioWaveform => open_object(Some(WorkflowSemanticKind::AudioWaveform)),
        MediaFile => binary_type(None, Some(WorkflowSemanticKind::MediaFile)),
        MaskTensor => open_object(Some(WorkflowSemanticKind::MaskTensor)),
        LatentBatch => open_object(Some(WorkflowSemanticKind::LatentBatch)),
        Conditioning => open_object(Some(WorkflowSemanticKind::Conditioning)),
        ModelRef => generic_model_ref_type(),
        VaeRef => open_object(Some(WorkflowSemanticKind::VaeRef)),
        ClipRef => open_object(Some(WorkflowSemanticKind::ClipRef)),
        ClipVisionRef => open_object(Some(WorkflowSemanticKind::ClipVisionRef)),
        UpscaleModelRef => open_object(Some(WorkflowSemanticKind::UpscaleModelRef)),
        ModelPatchRef => open_object(Some(WorkflowSemanticKind::ModelPatchRef)),
        Transcript => open_object(Some(WorkflowSemanticKind::Transcript)),
        SubtitleFile => string_type(Some(WorkflowSemanticKind::SubtitleFile)),
        SceneList => open_object(Some(WorkflowSemanticKind::SceneResult)),
        ObservationList => open_object(Some(WorkflowSemanticKind::VideoObservation)),
        AudioEvents => open_object(Some(WorkflowSemanticKind::AudioEvent)),
        TextEvents => open_object(Some(WorkflowSemanticKind::TextEvent)),
        DataBuckets => open_object(Some(WorkflowSemanticKind::DataBucket)),
        JsonReport => open_object(Some(WorkflowSemanticKind::JsonReport)),
        CollectionManifest => open_object(Some(WorkflowSemanticKind::CollectionManifest)),
        CollectionItems => open_object(Some(WorkflowSemanticKind::CollectionItem)),
        SongFingerprint => open_object(Some(WorkflowSemanticKind::SongFingerprint)),
        SongCatalog => open_object(Some(WorkflowSemanticKind::SongCatalog)),
        SongMatches => open_object(Some(WorkflowSemanticKind::SongMatch)),
        MusicFeatures => open_object(Some(WorkflowSemanticKind::MusicFeature)),
        TemplateString => string_type(Some(WorkflowSemanticKind::TemplateString)),
        String => string_type(None),
        Number => WorkflowTypeSpec::Number,
        Boolean => WorkflowTypeSpec::Boolean,
        Array => WorkflowTypeSpec::Array {
            items: Box::new(WorkflowTypeSpec::Any),
            min_items: None,
            max_items: None,
        },
        Object => open_object(None),
        HttpRequest => open_object(Some(WorkflowSemanticKind::HttpRequest)),
        HttpResponse => open_object(Some(WorkflowSemanticKind::HttpResponse)),
    }
}

fn string_type(semantic: Option<WorkflowSemanticKind>) -> WorkflowTypeSpec {
    WorkflowTypeSpec::String { semantic }
}

fn binary_type(
    media_type: Option<&str>,
    semantic: Option<WorkflowSemanticKind>,
) -> WorkflowTypeSpec {
    WorkflowTypeSpec::Binary {
        media_type: media_type.map(str::to_string),
        semantic,
    }
}

fn open_object(semantic: Option<WorkflowSemanticKind>) -> WorkflowTypeSpec {
    WorkflowTypeSpec::Object {
        properties: Vec::new(),
        additional_properties: true,
        semantic,
    }
}

fn generic_model_ref_type() -> WorkflowTypeSpec {
    WorkflowTypeSpec::Union {
        variants: vec![
            open_object(Some(WorkflowSemanticKind::ModelRef)),
            open_object(Some(WorkflowSemanticKind::VaeRef)),
            open_object(Some(WorkflowSemanticKind::ClipRef)),
            open_object(Some(WorkflowSemanticKind::ClipVisionRef)),
            open_object(Some(WorkflowSemanticKind::UpscaleModelRef)),
            open_object(Some(WorkflowSemanticKind::ModelPatchRef)),
        ],
    }
}

fn semantic_assignable(
    source: &Option<WorkflowSemanticKind>,
    target: &Option<WorkflowSemanticKind>,
) -> bool {
    match target {
        None => true,
        Some(target_semantic) => source == &Some(*target_semantic),
    }
}

fn media_type_assignable(source: &Option<String>, target: &Option<String>) -> bool {
    match target {
        None => true,
        Some(target_media_type) => source.as_ref() == Some(target_media_type),
    }
}

fn max_items_assignable(source: Option<u32>, target: Option<u32>) -> bool {
    match target {
        None => true,
        Some(target_max_items) => {
            source.is_some_and(|source_max_items| source_max_items <= target_max_items)
        }
    }
}

fn object_properties_assignable(
    source_properties: &[WorkflowObjectProperty],
    source_additional_properties: bool,
    target_properties: &[WorkflowObjectProperty],
    target_additional_properties: bool,
) -> bool {
    for target_property in target_properties {
        match source_properties
            .iter()
            .find(|source_property| source_property.name == target_property.name)
        {
            Some(source_property)
                if !workflow_types_assignable(&source_property.value, &target_property.value) =>
            {
                return false;
            }
            Some(_) => {}
            None if target_property.required => return false,
            None => {}
        }
    }

    if !target_additional_properties {
        if source_additional_properties {
            return false;
        }
        for source_property in source_properties {
            if !target_properties
                .iter()
                .any(|target_property| target_property.name == source_property.name)
            {
                return false;
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_scalar_compatibility_is_shape_based() {
        assert!(workflow_types_assignable(
            &WorkflowTypeSpec::Number,
            &WorkflowTypeSpec::Number
        ));
        assert!(!workflow_types_assignable(
            &WorkflowTypeSpec::Number,
            &WorkflowTypeSpec::String { semantic: None }
        ));
    }

    #[test]
    fn semantic_assignability_is_directional() {
        let plain = WorkflowTypeSpec::String { semantic: None };
        let youtube = WorkflowTypeSpec::String {
            semantic: Some(WorkflowSemanticKind::YoutubeUrl),
        };

        assert!(workflow_types_assignable(&youtube, &plain));
        assert!(!workflow_types_assignable(&plain, &youtube));
    }

    #[test]
    fn array_assignability_checks_items_and_bounds() {
        let source = WorkflowTypeSpec::Array {
            items: Box::new(WorkflowTypeSpec::String { semantic: None }),
            min_items: Some(2),
            max_items: Some(4),
        };
        let target = WorkflowTypeSpec::Array {
            items: Box::new(WorkflowTypeSpec::String { semantic: None }),
            min_items: Some(1),
            max_items: Some(5),
        };
        let too_wide = WorkflowTypeSpec::Array {
            items: Box::new(WorkflowTypeSpec::String { semantic: None }),
            min_items: Some(1),
            max_items: Some(3),
        };

        assert!(workflow_types_assignable(&source, &target));
        assert!(!workflow_types_assignable(&source, &too_wide));
    }

    #[test]
    fn object_assignability_supports_width_subtyping() {
        let source = WorkflowTypeSpec::Object {
            properties: vec![
                WorkflowObjectProperty {
                    name: "id".to_string(),
                    value: WorkflowTypeSpec::String { semantic: None },
                    required: true,
                },
                WorkflowObjectProperty {
                    name: "title".to_string(),
                    value: WorkflowTypeSpec::String { semantic: None },
                    required: true,
                },
            ],
            additional_properties: false,
            semantic: None,
        };
        let target = WorkflowTypeSpec::Object {
            properties: vec![WorkflowObjectProperty {
                name: "id".to_string(),
                value: WorkflowTypeSpec::String { semantic: None },
                required: true,
            }],
            additional_properties: true,
            semantic: None,
        };
        let closed_target = WorkflowTypeSpec::Object {
            properties: vec![WorkflowObjectProperty {
                name: "id".to_string(),
                value: WorkflowTypeSpec::String { semantic: None },
                required: true,
            }],
            additional_properties: false,
            semantic: None,
        };

        assert!(workflow_types_assignable(&source, &target));
        assert!(!workflow_types_assignable(&source, &closed_target));
    }

    #[test]
    fn union_targets_accept_compatible_sources() {
        let source = WorkflowTypeSpec::String {
            semantic: Some(WorkflowSemanticKind::TemplateString),
        };
        let target = WorkflowTypeSpec::Union {
            variants: vec![
                WorkflowTypeSpec::Number,
                WorkflowTypeSpec::String { semantic: None },
            ],
        };

        assert!(workflow_types_assignable(&source, &target));
    }

    #[test]
    fn binary_media_type_assignability_is_directional() {
        let generic_audio = WorkflowTypeSpec::Binary {
            media_type: Some("audio/*".to_string()),
            semantic: Some(WorkflowSemanticKind::AudioFile),
        };
        let wav_audio = WorkflowTypeSpec::Binary {
            media_type: Some("audio/wav".to_string()),
            semantic: Some(WorkflowSemanticKind::AudioWav),
        };

        assert!(!workflow_types_assignable(&generic_audio, &wav_audio));
        assert!(!workflow_types_assignable(&wav_audio, &generic_audio));
    }

    #[test]
    fn catalog_serialization_includes_value_type_and_input_metadata() {
        let catalog = default_workflow_catalog();
        let json = serde_json::to_value(catalog).expect("catalog serializes");
        let first_port = &json["nodes"][0]["outputs"][0];
        let config_port = &json["nodes"][0]["outputs"][1];

        assert!(first_port.get("value_type").is_some());
        assert_eq!(config_port["exposable_input"], true);
        assert_eq!(config_port["input_surface"], "config");
        assert_eq!(config_port["runtime_validation"], "external_input");
        assert_eq!(json["version"], 5);
    }

    #[test]
    fn workflow_catalog_includes_new_use_case_nodes_and_image_ports() {
        let catalog = default_workflow_catalog();
        assert!(catalog.port_kinds.contains(&PortKind::ImageFile));
        assert!(catalog.port_kinds.contains(&PortKind::AudioWaveform));
        assert!(catalog.port_kinds.contains(&PortKind::LatentBatch));
        assert!(catalog.port_kinds.contains(&PortKind::ModelPatchRef));
        assert!(catalog
            .nodes
            .iter()
            .any(|node| node.id == "video-red-cars-workflow"));
        assert!(catalog
            .nodes
            .iter()
            .any(|node| node.id == "audio-voice-analysis-workflow"));
        assert!(catalog
            .nodes
            .iter()
            .any(|node| node.id == "image-person-edit-workflow"));
    }

    #[test]
    fn specialized_model_refs_are_assignable_to_generic_model_refs() {
        assert!(workflow_types_assignable(
            &workflow_type_for_port_kind(PortKind::VaeRef),
            &workflow_type_for_port_kind(PortKind::ModelRef),
        ));
        assert!(!workflow_types_assignable(
            &workflow_type_for_port_kind(PortKind::ModelRef),
            &workflow_type_for_port_kind(PortKind::VaeRef),
        ));
    }

    #[test]
    fn in_memory_media_types_do_not_match_file_asset_types() {
        assert!(!workflow_types_assignable(
            &workflow_type_for_port_kind(PortKind::AudioWaveform),
            &workflow_type_for_port_kind(PortKind::AudioFile),
        ));
        assert!(!workflow_types_assignable(
            &workflow_type_for_port_kind(PortKind::ImageBatch),
            &workflow_type_for_port_kind(PortKind::ImageFile),
        ));
        assert!(!workflow_types_assignable(
            &workflow_type_for_port_kind(PortKind::LatentBatch),
            &workflow_type_for_port_kind(PortKind::RunConfig),
        ));
    }
}
