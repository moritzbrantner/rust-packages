#![doc = include_str!("../README.md")]

/// Re-exports the audio core API.
pub use audio_analysis_core as audio_core;
/// Re-exports the MIDI-like audio generation API.
pub use audio_generation_midi as audio_midi;
/// Re-exports the audio fourier API.
pub use audio_analysis_fourier as audio_fourier;
/// Re-exports the audio I/O API.
pub use audio_analysis_io as audio_io;
/// Re-exports the audio pitch API.
pub use audio_analysis_pitch as audio_pitch;
/// Re-exports the audio processing API.
pub use audio_analysis_processing as audio_processing;
/// Re-exports the audio recognition API.
pub use audio_analysis_recognition as audio_recognition;
/// Re-exports the audio rhythm API.
pub use audio_analysis_rhythm as audio_rhythm;
/// Re-exports the audio separation API.
pub use audio_analysis_separation as audio_separation;
/// Re-exports the audio speaker analysis API.
pub use audio_analysis_speakers as audio_speakers;
/// Re-exports the audio synthesis API.
pub use audio_analysis_synthesis as audio_synthesis;
/// Re-exports the ComfyUI data API.
pub use comfyui_data;
/// Re-exports the ComfyUI latents API.
pub use comfyui_latents;
/// Re-exports the ComfyUI models API.
pub use comfyui_models;
/// Re-exports the inversion API.
pub use data_inversion_core as inversion;
/// Re-exports the dense API.
pub use dense_data as dense;
/// Re-exports the finance statistics API.
pub use finance_statistics as finance;
/// Re-exports the graph core API.
pub use graph_analysis_core as graph_core;
/// Re-exports the image ComfyUI API.
pub use image_analysis_comfyui as image_comfyui;
/// Re-exports the image core API.
pub use image_analysis_core as image_core;
/// Re-exports the image detection API.
pub use image_analysis_detection as image_detection;
/// Re-exports the image I/O API.
pub use image_analysis_io as image_io;
/// Re-exports the image models API.
pub use image_analysis_models as image_models;
#[cfg(feature = "onnx-backend")]
/// Re-exports the image ONNX API.
pub use image_analysis_onnx as image_onnx;
/// Re-exports the image processing API.
pub use image_analysis_processing as image_processing;
/// Re-exports the image segmentation API.
pub use image_analysis_segmentation as image_segmentation;
/// Re-exports the image synthesis API.
pub use image_analysis_synthesis as image_synthesis;
/// Re-exports the geometry2d API.
pub use math_geometry_2d as geometry2d;
/// Re-exports the linear API.
pub use math_linear as linear;
/// Re-exports the signal API.
pub use math_signal_core as signal;
/// Re-exports the sparse API.
pub use math_sparse_data as sparse;
/// Re-exports the stats API.
pub use math_statistics as stats;
/// Re-exports the numbers API.
pub use numbers_core as numbers;
/// Re-exports the tensor data API.
pub use tensor_data;
/// Re-exports the text core API.
pub use text_analysis_core as text_core;
/// Re-exports the text corpus API.
pub use text_analysis_corpus as text_corpus;
/// Re-exports the text features API.
pub use text_analysis_features as text_features;
/// Re-exports the text linguistics API.
pub use text_analysis_linguistics as text_linguistics;
/// Re-exports the text models API.
pub use text_analysis_models as text_models;
/// Re-exports the text prediction API.
pub use text_analysis_prediction as text_prediction;
/// Re-exports the text retrieval API.
pub use text_analysis_retrieval as text_retrieval;
/// Re-exports the text retrieval storage API.
pub use text_analysis_retrieval_storage as text_retrieval_storage;
/// Re-exports the text search API.
pub use text_analysis_search as text_search;
/// Re-exports the text semantics API.
pub use text_analysis_semantics as text_semantics;
/// Re-exports the text synthesis API.
pub use text_analysis_synthesis as text_synthesis;
/// Re-exports the text transcription API.
pub use text_analysis_transcription as text_transcription;
/// Re-exports the three d core API.
pub use three_d_processing_core as three_d_core;
/// Re-exports the three d I/O API.
pub use three_d_processing_io as three_d_io;
/// Re-exports the three d mesh API.
pub use three_d_processing_mesh as three_d_mesh;
/// Re-exports the vector core API.
pub use vector_analysis_core as vector_core;
/// Re-exports the vector index API.
pub use vector_analysis_index as vector_index;
/// Re-exports the COLMAP compatibility backend API.
pub use video_analysis_colmap_backend as colmap_backend;
/// Re-exports the * API.
pub use video_analysis_core::*;
/// Re-exports the data API.
pub use video_analysis_data as data;
/// Re-exports the dataset records API.
pub use video_analysis_dataset as dataset_records;
/// Re-exports the * API.
pub use video_analysis_detectors::*;
/// Re-exports the editing API.
pub use video_analysis_editing as editing;
/// Re-exports the features API.
pub use video_analysis_features as features;
/// Re-exports the FFmpeg API.
pub use video_analysis_ffmpeg as ffmpeg;
/// Re-exports the gaussian splatting API.
pub use video_analysis_gaussian_splatting as gaussian_splatting;
/// Re-exports the ingest API.
pub use video_analysis_ingest as ingest;
/// Re-exports the models API.
pub use video_analysis_models as models;
/// Re-exports the MVS API.
pub use video_analysis_mvs as mvs;
#[cfg(feature = "onnx-backend")]
/// Re-exports the ONNX API.
pub use video_analysis_onnx as onnx;
/// Re-exports the OpenCV backend API.
pub use video_analysis_opencv_backend as opencv_backend;
/// Re-exports the output API.
pub use video_analysis_output as output;
/// Re-exports the posture API.
pub use video_analysis_posture as posture;
/// Re-exports the posture I/O API.
pub use video_analysis_posture_io as posture_io;
/// Re-exports the radiance fields API.
pub use video_analysis_radiance_fields as radiance_fields;
/// Re-exports the radiance I/O API.
pub use video_analysis_radiance_io as radiance_io;
/// Re-exports the recognition API.
pub use video_analysis_recognition as recognition;
/// Re-exports the reconstruction API.
pub use video_analysis_reconstruction as reconstruction;
/// Re-exports the video segmentation API.
pub use video_analysis_segmentation as video_segmentation;
/// Re-exports the SfM API.
pub use video_analysis_sfm as sfm;
/// Re-exports the Rust-native SfM backend API.
pub use video_analysis_sfm_rust_backend as sfm_rust_backend;
/// Re-exports the split API.
pub use video_analysis_split as split;
/// Re-exports the storage API.
pub use video_analysis_storage as storage;
/// Re-exports the synthesis API.
pub use video_analysis_synthesis as synthesis;
/// Re-exports the tracking API.
pub use video_analysis_tracking as tracking;
/// Re-exports the transform API.
pub use video_analysis_transform as transform;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facade_reexports_foundational_library_types() {
        let timestamp = Timestamp::new(3, Timebase::new(1, 2));
        assert_eq!(timestamp.seconds(), 1.5);

        let tensor = tensor_data::F32Tensor::from_dims([1, 2], vec![0.0, 1.0]).unwrap();
        assert_eq!(tensor.shape().dimensions(), &[1, 2]);

        let vector = vector_core::DenseVector::new([3.0, 4.0]).unwrap();
        assert_eq!(vector_core::l2_norm(vector.as_slice()).unwrap(), 5.0);
    }

    #[test]
    fn facade_reexports_domain_library_modules() {
        let image = image_synthesis::solid_image(
            image_synthesis::RgbColor::new(1, 2, 3),
            image_synthesis::ImageSynthesisConfig {
                width: 2,
                height: 2,
                pixel_format: image_core::ImagePixelFormat::Rgb24,
            },
        )
        .unwrap();
        assert_eq!(image.value.width, 2);

        let summary = text_features::summarize_text("facade unit tests cover package exports", 4);
        assert_eq!(summary.stats.words, 6);

        let graph = graph_core::Graph::directed();
        assert_eq!(graph.node_count(), 0);
    }
}
