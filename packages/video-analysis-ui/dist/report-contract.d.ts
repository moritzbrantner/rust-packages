export declare const youtubeVideoReportContractFixture: {
    use_case: "youtube-video";
    source: {
        url: null;
        local_video: string;
    };
    assets: {
        work_dir: string;
        report_path: string;
        audio_wav: null;
    };
    capabilities: {
        completed: string[];
        skipped: string[];
    };
    video: {
        width: number;
        height: number;
        frame_rate: string;
        duration_seconds: number;
        frames_processed: number;
        scenes: {
            index: number;
            start_frame: number;
            end_frame: number;
            start_seconds: number;
            end_seconds: number;
            observations: never[];
        }[];
        observations: never[];
    };
    transcription: {
        status: string;
        text: null;
        segments: never[];
        message: string;
    };
    audio: {
        status: string;
        frames_processed: number;
        events: never[];
        message: string;
    };
    text: {
        status: string;
        segments_processed: number;
        events: never[];
        message: string;
    };
    data_buckets: {
        bucket_index: number;
        records: number;
        estimated_bytes: number;
        streams: {
            stream_id: string;
            records: number;
            estimated_bytes: number;
            payload_counts: {
                video: number;
            };
            video_frames: number;
            audio_frames: number;
            text_segments: number;
        }[];
    }[];
};
//# sourceMappingURL=report-contract.d.ts.map