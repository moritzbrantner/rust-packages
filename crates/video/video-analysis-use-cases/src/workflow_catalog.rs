use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PortKind {
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
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowSemanticKind {
    RunRequest,
    RunConfig,
    TemplateString,
    YoutubeUrl,
    YoutubeCollectionUrl,
    VideoFile,
    ImageFile,
    ImageBatch,
    AudioFile,
    AudioWaveform,
    AudioWav,
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
    VideoMetadata,
    SceneResult,
    VideoObservation,
    ModelRequest,
    ModelPrediction,
    TranscriptSegment,
    AudioEvent,
    TextEvent,
    CollectionManifest,
    CollectionItem,
    CollectionReport,
    CollectionTable,
    SongFingerprint,
    SongCatalog,
    SongMatch,
    MusicFeature,
    HttpRequest,
    HttpResponse,
    JsonReport,
    DashboardView,
    DataRecord,
    DataBucket,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowObjectProperty {
    pub name: String,
    pub value: WorkflowTypeSpec,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkflowTypeSpec {
    Any,
    Null,
    Boolean,
    Number,
    String {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        semantic: Option<WorkflowSemanticKind>,
    },
    Binary {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        media_type: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        semantic: Option<WorkflowSemanticKind>,
    },
    Array {
        items: Box<WorkflowTypeSpec>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min_items: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_items: Option<u32>,
    },
    Object {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        properties: Vec<WorkflowObjectProperty>,
        #[serde(default)]
        additional_properties: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        semantic: Option<WorkflowSemanticKind>,
    },
    Union {
        variants: Vec<WorkflowTypeSpec>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowInputSurface {
    SourceUrl,
    CollectionUrl,
    File,
    Config,
    Generic,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowRuntimeValidation {
    Strict,
    Unsafe,
    ExternalInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PortSpec {
    pub id: String,
    pub label: String,
    pub kind: PortKind,
    pub value_type: WorkflowTypeSpec,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub exposable_input: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_surface: Option<WorkflowInputSurface>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suggested_adapters: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_validation: Option<WorkflowRuntimeValidation>,
}

impl PortSpec {
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

    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    pub fn stream(mut self) -> Self {
        self.stream = true;
        self
    }

    pub fn exposable_input(mut self, surface: WorkflowInputSurface) -> Self {
        self.exposable_input = true;
        self.input_surface = Some(surface);
        self
    }

    pub fn value_type(mut self, value_type: WorkflowTypeSpec) -> Self {
        self.value_type = value_type;
        self
    }

    pub fn suggested_adapters<I, S>(mut self, suggested_adapters: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.suggested_adapters = suggested_adapters.into_iter().map(Into::into).collect();
        self
    }

    pub fn runtime_validation(mut self, mode: WorkflowRuntimeValidation) -> Self {
        self.runtime_validation = Some(mode);
        self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowCategory {
    App,
    Ingest,
    Youtube,
    Video,
    Audio,
    Text,
    Song,
    Collection,
    Output,
    Data,
    Logic,
    Network,
    Ui,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeSpec {
    pub id: String,
    pub title: String,
    pub package_name: String,
    pub category: WorkflowCategory,
    #[serde(default)]
    pub ui_owned: bool,
    #[serde(default)]
    pub inputs: Vec<PortSpec>,
    #[serde(default)]
    pub outputs: Vec<PortSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowCatalog {
    pub version: u32,
    pub port_kinds: Vec<PortKind>,
    pub nodes: Vec<NodeSpec>,
    pub compatibility: Vec<PortCompatibility>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PortCompatibility {
    pub source: PortKind,
    pub target: PortKind,
}

pub fn default_workflow_catalog() -> WorkflowCatalog {
    let nodes = vec![
        node(
            "react-page",
            "React Page",
            "@video-analysis-studio/ui",
            WorkflowCategory::App,
            true,
        )
        .outputs(vec![
            port("run", "Run", PortKind::RunTrigger),
            port("config", "Config", PortKind::RunConfig)
                .exposable_input(WorkflowInputSurface::Config)
                .runtime_validation(WorkflowRuntimeValidation::ExternalInput),
        ]),
        node(
            "source",
            "Source",
            "@video-analysis-studio/ui",
            WorkflowCategory::Ingest,
            true,
        )
        .outputs(vec![
            port("url", "YouTube URL", PortKind::YoutubeUrl)
                .exposable_input(WorkflowInputSurface::SourceUrl)
                .runtime_validation(WorkflowRuntimeValidation::ExternalInput),
            port(
                "collection",
                "Collection URL",
                PortKind::YoutubeCollectionUrl,
            )
            .exposable_input(WorkflowInputSurface::CollectionUrl)
            .runtime_validation(WorkflowRuntimeValidation::ExternalInput),
            port("file", "Video File", PortKind::VideoFile)
                .exposable_input(WorkflowInputSurface::File)
                .runtime_validation(WorkflowRuntimeValidation::ExternalInput),
            port("image", "Image File", PortKind::ImageFile)
                .exposable_input(WorkflowInputSurface::File)
                .runtime_validation(WorkflowRuntimeValidation::ExternalInput),
        ]),
        node(
            "vite-api",
            "Studio API",
            "studio-core",
            WorkflowCategory::App,
            false,
        )
        .inputs(vec![
            port("run", "Run", PortKind::RunTrigger),
            port("config", "Config", PortKind::RunConfig),
        ])
        .outputs(vec![port("args", "Arguments", PortKind::RunConfig)]),
        node(
            "youtube-workflow",
            "YouTube Workflow",
            "video-analysis-use-cases",
            WorkflowCategory::Youtube,
            false,
        )
        .inputs(vec![
            port("url", "URL", PortKind::YoutubeUrl)
                .optional()
                .exposable_input(WorkflowInputSurface::SourceUrl)
                .runtime_validation(WorkflowRuntimeValidation::ExternalInput),
            port("collection", "Collection", PortKind::YoutubeCollectionUrl)
                .optional()
                .exposable_input(WorkflowInputSurface::CollectionUrl)
                .runtime_validation(WorkflowRuntimeValidation::ExternalInput),
            port("file", "File", PortKind::VideoFile)
                .optional()
                .exposable_input(WorkflowInputSurface::File)
                .runtime_validation(WorkflowRuntimeValidation::ExternalInput),
            port("args", "Args", PortKind::RunConfig)
                .exposable_input(WorkflowInputSurface::Config)
                .runtime_validation(WorkflowRuntimeValidation::ExternalInput),
        ])
        .outputs(vec![
            port("file", "Video", PortKind::VideoFile),
            port("report", "Report", PortKind::JsonReport),
        ]),
        node(
            "video-red-cars-workflow",
            "Video Red Cars Workflow",
            "video-analysis-use-cases",
            WorkflowCategory::Video,
            false,
        )
        .inputs(vec![
            port("file", "File", PortKind::VideoFile)
                .exposable_input(WorkflowInputSurface::File)
                .runtime_validation(WorkflowRuntimeValidation::ExternalInput),
            port("args", "Args", PortKind::RunConfig)
                .exposable_input(WorkflowInputSurface::Config)
                .runtime_validation(WorkflowRuntimeValidation::ExternalInput),
        ])
        .outputs(vec![
            port("file", "Video", PortKind::VideoFile),
            port("report", "Report", PortKind::JsonReport),
        ]),
        node(
            "audio-voice-analysis-workflow",
            "Audio Voice Analysis Workflow",
            "video-analysis-use-cases",
            WorkflowCategory::Audio,
            false,
        )
        .inputs(vec![
            port("file", "Audio", PortKind::AudioFile)
                .exposable_input(WorkflowInputSurface::File)
                .runtime_validation(WorkflowRuntimeValidation::ExternalInput),
            port("args", "Args", PortKind::RunConfig)
                .exposable_input(WorkflowInputSurface::Config)
                .runtime_validation(WorkflowRuntimeValidation::ExternalInput),
        ])
        .outputs(vec![
            port("audio", "Audio", PortKind::AudioFile),
            port("report", "Report", PortKind::JsonReport),
        ]),
        node(
            "image-person-edit-workflow",
            "Image Person Edit Workflow",
            "video-analysis-use-cases",
            WorkflowCategory::Ui,
            false,
        )
        .inputs(vec![
            port("image", "Image", PortKind::ImageFile)
                .exposable_input(WorkflowInputSurface::File)
                .runtime_validation(WorkflowRuntimeValidation::ExternalInput),
            port("args", "Args", PortKind::RunConfig)
                .exposable_input(WorkflowInputSurface::Config)
                .runtime_validation(WorkflowRuntimeValidation::ExternalInput),
        ])
        .outputs(vec![
            port("image", "Image", PortKind::ImageFile),
            port("report", "Report", PortKind::JsonReport),
        ]),
        node(
            "song-workflow",
            "Song Workflow",
            "video-analysis-use-cases",
            WorkflowCategory::Song,
            false,
        )
        .inputs(vec![
            port("url", "URL", PortKind::YoutubeUrl)
                .optional()
                .exposable_input(WorkflowInputSurface::SourceUrl)
                .runtime_validation(WorkflowRuntimeValidation::ExternalInput),
            port("file", "File", PortKind::MediaFile)
                .optional()
                .exposable_input(WorkflowInputSurface::File)
                .runtime_validation(WorkflowRuntimeValidation::ExternalInput),
            port("args", "Args", PortKind::RunConfig)
                .exposable_input(WorkflowInputSurface::Config)
                .runtime_validation(WorkflowRuntimeValidation::ExternalInput),
        ])
        .outputs(vec![
            port("file", "Media", PortKind::MediaFile),
            port("report", "Report", PortKind::JsonReport),
        ]),
        node(
            "subtitle-workflow",
            "Subtitle Workflow",
            "text-analysis-transcription",
            WorkflowCategory::Text,
            false,
        )
        .inputs(vec![
            port("url", "URL", PortKind::YoutubeUrl)
                .optional()
                .exposable_input(WorkflowInputSurface::SourceUrl)
                .runtime_validation(WorkflowRuntimeValidation::ExternalInput),
            port("file", "File", PortKind::VideoFile)
                .optional()
                .exposable_input(WorkflowInputSurface::File)
                .runtime_validation(WorkflowRuntimeValidation::ExternalInput),
            port("args", "Args", PortKind::RunConfig)
                .exposable_input(WorkflowInputSurface::Config)
                .runtime_validation(WorkflowRuntimeValidation::ExternalInput),
        ])
        .outputs(vec![
            port("subtitle", "Subtitle", PortKind::SubtitleFile),
            port("report", "Report", PortKind::JsonReport),
        ]),
        node(
            "collection-manifest",
            "Collection Manifest",
            "video-analysis-use-cases",
            WorkflowCategory::Collection,
            false,
        )
        .inputs(vec![port(
            "collection",
            "Collection",
            PortKind::YoutubeCollectionUrl,
        )
        .exposable_input(WorkflowInputSurface::CollectionUrl)
        .runtime_validation(WorkflowRuntimeValidation::ExternalInput)])
        .outputs(vec![port(
            "manifest",
            "Manifest",
            PortKind::CollectionManifest,
        )]),
        node(
            "collection-download-loop",
            "Collection Download Loop",
            "video-analysis-use-cases",
            WorkflowCategory::Collection,
            false,
        )
        .inputs(vec![port(
            "manifest",
            "Manifest",
            PortKind::CollectionManifest,
        )])
        .outputs(vec![port("items", "Items", PortKind::CollectionItems)]),
        node(
            "collection-report",
            "Collection Report",
            "video-analysis-use-cases",
            WorkflowCategory::Collection,
            false,
        )
        .inputs(vec![port("items", "Items", PortKind::CollectionItems)])
        .outputs(vec![port("report", "Report", PortKind::JsonReport)]),
        node(
            "download",
            "YouTube Download",
            "video-analysis-use-cases",
            WorkflowCategory::Youtube,
            false,
        )
        .inputs(vec![port("url", "URL", PortKind::YoutubeUrl)
            .exposable_input(WorkflowInputSurface::SourceUrl)
            .runtime_validation(WorkflowRuntimeValidation::ExternalInput)])
        .outputs(vec![port("file", "Video", PortKind::VideoFile)]),
        node(
            "ffmpeg",
            "FFmpeg Decode",
            "video-analysis-ffmpeg",
            WorkflowCategory::Ingest,
            false,
        )
        .inputs(vec![port("file", "File", PortKind::MediaFile)
            .exposable_input(WorkflowInputSurface::File)
            .runtime_validation(WorkflowRuntimeValidation::ExternalInput)])
        .outputs(vec![
            port("video", "Video Frames", PortKind::VideoFile).stream(),
            port("audio", "Audio Frames", PortKind::AudioWaveform).stream(),
        ]),
        node(
            "video-pipeline",
            "Video Pipeline",
            "video-analysis-core",
            WorkflowCategory::Video,
            false,
        )
        .inputs(vec![port("video", "Frames", PortKind::VideoFile).stream()])
        .outputs(vec![
            port("scenes", "Scenes", PortKind::SceneList),
            port("observations", "Observations", PortKind::ObservationList),
        ]),
        node(
            "content-detector",
            "Content Detector",
            "video-analysis-detectors",
            WorkflowCategory::Video,
            false,
        )
        .inputs(vec![port("video", "Frames", PortKind::VideoFile).stream()])
        .outputs(vec![port("scenes", "Scenes", PortKind::SceneList)]),
        node(
            "model-sampler",
            "Model Sampler",
            "video-analysis-core",
            WorkflowCategory::Video,
            false,
        )
        .inputs(vec![port("video", "Frames", PortKind::VideoFile).stream()])
        .outputs(vec![
            port("sampled", "Sampled Frames", PortKind::VideoFile).stream()
        ]),
        node(
            "external-models",
            "External Models",
            "video-analysis-models",
            WorkflowCategory::Video,
            false,
        )
        .inputs(vec![port("video", "Frames", PortKind::VideoFile).stream()])
        .outputs(vec![port(
            "observations",
            "Observations",
            PortKind::ObservationList,
        )]),
        node(
            "audio-pipeline",
            "Audio Pipeline",
            "audio-analysis-processing",
            WorkflowCategory::Audio,
            false,
        )
        .inputs(vec![
            port("audio", "Audio", PortKind::AudioWaveform).stream()
        ])
        .outputs(vec![port("events", "Events", PortKind::AudioEvents)]),
        node(
            "transcriber",
            "Transcriber",
            "text-analysis-transcription",
            WorkflowCategory::Text,
            false,
        )
        .inputs(vec![port("audio", "Audio", PortKind::AudioWaveform)])
        .outputs(vec![port("transcript", "Transcript", PortKind::Transcript)]),
        node(
            "text-pipeline",
            "Text Pipeline",
            "text-analysis-features",
            WorkflowCategory::Text,
            false,
        )
        .inputs(vec![port("transcript", "Transcript", PortKind::Transcript)])
        .outputs(vec![port("events", "Events", PortKind::TextEvents)]),
        node(
            "subtitle-writer",
            "Subtitle Writer",
            "text-analysis-transcription",
            WorkflowCategory::Text,
            false,
        )
        .inputs(vec![port("transcript", "Transcript", PortKind::Transcript)])
        .outputs(vec![port("subtitle", "Subtitle", PortKind::SubtitleFile)]),
        node(
            "buckets",
            "Data Buckets",
            "video-analysis-data",
            WorkflowCategory::Data,
            false,
        )
        .inputs(vec![
            port("video", "Video", PortKind::VideoFile).stream(),
            port("audio", "Audio", PortKind::AudioWaveform).stream(),
            port("text", "Text", PortKind::Transcript).stream(),
        ])
        .outputs(vec![port("buckets", "Buckets", PortKind::DataBuckets)]),
        node(
            "report-writer",
            "Report Writer",
            "video-analysis-use-cases",
            WorkflowCategory::Output,
            false,
        )
        .inputs(vec![
            port("scenes", "Scenes", PortKind::SceneList).optional(),
            port("observations", "Observations", PortKind::ObservationList).optional(),
            port("audio", "Audio Events", PortKind::AudioEvents).optional(),
            port("text", "Text Events", PortKind::TextEvents).optional(),
            port("buckets", "Buckets", PortKind::DataBuckets).optional(),
        ])
        .outputs(vec![port("report", "Report", PortKind::JsonReport)]),
        node(
            "song-fingerprinter",
            "Song Fingerprinter",
            "audio-analysis-recognition",
            WorkflowCategory::Song,
            false,
        )
        .inputs(vec![port("file", "Media", PortKind::MediaFile)
            .exposable_input(WorkflowInputSurface::File)
            .runtime_validation(WorkflowRuntimeValidation::ExternalInput)])
        .outputs(vec![port(
            "fingerprint",
            "Fingerprint",
            PortKind::SongFingerprint,
        )]),
        node(
            "song-catalog-matcher",
            "Song Catalog Matcher",
            "video-analysis-use-cases",
            WorkflowCategory::Song,
            false,
        )
        .inputs(vec![
            port("fingerprint", "Fingerprint", PortKind::SongFingerprint),
            port("catalog", "Catalog", PortKind::SongCatalog)
                .exposable_input(WorkflowInputSurface::Generic)
                .runtime_validation(WorkflowRuntimeValidation::ExternalInput),
        ])
        .outputs(vec![port("matches", "Matches", PortKind::SongMatches)]),
        node(
            "song-music-analyzer",
            "Music Analyzer",
            "video-analysis-use-cases",
            WorkflowCategory::Song,
            false,
        )
        .inputs(vec![port("file", "Media", PortKind::MediaFile)
            .exposable_input(WorkflowInputSurface::File)
            .runtime_validation(WorkflowRuntimeValidation::ExternalInput)])
        .outputs(vec![port("features", "Features", PortKind::MusicFeatures)]),
        node(
            "song-report-writer",
            "Song Report Writer",
            "video-analysis-use-cases",
            WorkflowCategory::Song,
            false,
        )
        .inputs(vec![
            port("matches", "Matches", PortKind::SongMatches).optional(),
            port("features", "Features", PortKind::MusicFeatures).optional(),
            port("transcript", "Lyrics", PortKind::Transcript).optional(),
        ])
        .outputs(vec![port("report", "Report", PortKind::JsonReport)]),
    ];

    WorkflowCatalog {
        version: 5,
        port_kinds: all_port_kinds(),
        compatibility: compatibility_pairs(),
        nodes,
    }
}

pub fn ports_compatible(source: PortKind, target: PortKind) -> bool {
    workflow_types_assignable(
        &workflow_type_for_port_kind(source),
        &workflow_type_for_port_kind(target),
    )
}

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

fn node(
    id: impl Into<String>,
    title: impl Into<String>,
    package_name: impl Into<String>,
    category: WorkflowCategory,
    ui_owned: bool,
) -> NodeBuilder {
    NodeBuilder(NodeSpec {
        id: id.into(),
        title: title.into(),
        package_name: package_name.into(),
        category,
        ui_owned,
        inputs: Vec::new(),
        outputs: Vec::new(),
    })
}

fn port(id: impl Into<String>, label: impl Into<String>, kind: PortKind) -> PortSpec {
    PortSpec::new(id, label, kind)
}

struct NodeBuilder(NodeSpec);

impl NodeBuilder {
    fn inputs(mut self, inputs: Vec<PortSpec>) -> Self {
        self.0.inputs = inputs;
        self
    }

    fn outputs(mut self, outputs: Vec<PortSpec>) -> NodeSpec {
        self.0.outputs = outputs;
        self.0
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
