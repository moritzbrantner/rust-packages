use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PortKind {
    RunTrigger,
    RunConfig,
    YoutubeUrl,
    YoutubeCollectionUrl,
    VideoFile,
    AudioFile,
    MediaFile,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PortSpec {
    pub id: String,
    pub label: String,
    pub kind: PortKind,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub stream: bool,
}

impl PortSpec {
    pub fn new(id: impl Into<String>, label: impl Into<String>, kind: PortKind) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind,
            optional: false,
            stream: false,
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
            port("config", "Config", PortKind::RunConfig),
        ]),
        node(
            "source",
            "Source",
            "@video-analysis-studio/ui",
            WorkflowCategory::Ingest,
            true,
        )
        .outputs(vec![
            port("url", "YouTube URL", PortKind::YoutubeUrl),
            port(
                "collection",
                "Collection URL",
                PortKind::YoutubeCollectionUrl,
            ),
            port("file", "Video File", PortKind::VideoFile),
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
            port("url", "URL", PortKind::YoutubeUrl).optional(),
            port("collection", "Collection", PortKind::YoutubeCollectionUrl).optional(),
            port("file", "File", PortKind::VideoFile).optional(),
            port("args", "Args", PortKind::RunConfig),
        ])
        .outputs(vec![
            port("file", "Video", PortKind::VideoFile),
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
            port("url", "URL", PortKind::YoutubeUrl).optional(),
            port("file", "File", PortKind::MediaFile).optional(),
            port("args", "Args", PortKind::RunConfig),
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
            port("url", "URL", PortKind::YoutubeUrl).optional(),
            port("file", "File", PortKind::VideoFile).optional(),
            port("args", "Args", PortKind::RunConfig),
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
        )])
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
        .inputs(vec![port("url", "URL", PortKind::YoutubeUrl)])
        .outputs(vec![port("file", "Video", PortKind::VideoFile)]),
        node(
            "ffmpeg",
            "FFmpeg Decode",
            "video-analysis-ffmpeg",
            WorkflowCategory::Ingest,
            false,
        )
        .inputs(vec![port("file", "File", PortKind::MediaFile)])
        .outputs(vec![
            port("video", "Video Frames", PortKind::VideoFile).stream(),
            port("audio", "Audio Frames", PortKind::AudioFile).stream(),
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
        .inputs(vec![port("audio", "Audio", PortKind::AudioFile).stream()])
        .outputs(vec![port("events", "Events", PortKind::AudioEvents)]),
        node(
            "transcriber",
            "Transcriber",
            "text-analysis-transcription",
            WorkflowCategory::Text,
            false,
        )
        .inputs(vec![port("audio", "Audio", PortKind::AudioFile)])
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
            port("audio", "Audio", PortKind::AudioFile).stream(),
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
        .inputs(vec![port("file", "Media", PortKind::MediaFile)])
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
            port("catalog", "Catalog", PortKind::SongCatalog),
        ])
        .outputs(vec![port("matches", "Matches", PortKind::SongMatches)]),
        node(
            "song-music-analyzer",
            "Music Analyzer",
            "video-analysis-use-cases",
            WorkflowCategory::Song,
            false,
        )
        .inputs(vec![port("file", "Media", PortKind::MediaFile)])
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
        version: 1,
        port_kinds: all_port_kinds(),
        nodes,
        compatibility: compatibility_pairs(),
    }
}

pub fn ports_compatible(source: PortKind, target: PortKind) -> bool {
    source == target
        || compatibility_pairs()
            .into_iter()
            .any(|pair| pair.source == source && pair.target == target)
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
        AudioFile,
        MediaFile,
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
    use PortKind::*;
    [
        (VideoFile, MediaFile),
        (AudioFile, MediaFile),
        (String, TemplateString),
        (TemplateString, String),
        (Object, JsonReport),
        (JsonReport, Object),
        (YoutubeUrl, String),
        (YoutubeCollectionUrl, String),
    ]
    .into_iter()
    .map(|(source, target)| PortCompatibility { source, target })
    .collect()
}
