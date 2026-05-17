#![doc = include_str!("../README.md")]

pub use audio_analysis_core as audio_core;
pub use audio_analysis_fourier as audio_fourier;
pub use audio_analysis_io as audio_io;
pub use audio_analysis_pitch as audio_pitch;
pub use audio_analysis_processing as audio_processing;
pub use audio_analysis_recognition as audio_recognition;
pub use audio_analysis_rhythm as audio_rhythm;
pub use audio_analysis_separation as audio_separation;
pub use audio_analysis_synthesis as audio_synthesis;
pub use comfyui_data;
pub use comfyui_latents;
pub use comfyui_models;
pub use data_inversion_core as inversion;
pub use dense_data as dense;
pub use graph_analysis_core as graph_core;
pub use image_analysis_comfyui as image_comfyui;
pub use image_analysis_core as image_core;
pub use image_analysis_detection as image_detection;
pub use image_analysis_io as image_io;
pub use image_analysis_models as image_models;
#[cfg(feature = "onnx-backend")]
pub use image_analysis_onnx as image_onnx;
pub use image_analysis_processing as image_processing;
pub use image_analysis_segmentation as image_segmentation;
pub use image_analysis_synthesis as image_synthesis;
pub use math_geometry_2d as geometry2d;
pub use math_linear as linear;
pub use math_signal_core as signal;
pub use math_sparse_data as sparse;
pub use math_statistics as stats;
pub use numbers_core as numbers;
pub use tensor_data;
pub use text_analysis_core as text_core;
pub use text_analysis_corpus as text_corpus;
pub use text_analysis_features as text_features;
pub use text_analysis_linguistics as text_linguistics;
pub use text_analysis_models as text_models;
pub use text_analysis_prediction as text_prediction;
pub use text_analysis_retrieval as text_retrieval;
pub use text_analysis_retrieval_storage as text_retrieval_storage;
pub use text_analysis_semantics as text_semantics;
pub use text_analysis_synthesis as text_synthesis;
pub use text_analysis_transcription as text_transcription;
pub use three_d_processing_core as three_d_core;
pub use three_d_processing_io as three_d_io;
pub use three_d_processing_mesh as three_d_mesh;
pub use vector_analysis_core as vector_core;
pub use vector_analysis_index as vector_index;
pub use video_analysis_core::*;
pub use video_analysis_data as data;
pub use video_analysis_dataset as dataset_records;
pub use video_analysis_detectors::*;
pub use video_analysis_editing as editing;
pub use video_analysis_features as features;
pub use video_analysis_ffmpeg as ffmpeg;
pub use video_analysis_gaussian_splatting as gaussian_splatting;
pub use video_analysis_ingest as ingest;
pub use video_analysis_models as models;
#[cfg(feature = "onnx-backend")]
pub use video_analysis_onnx as onnx;
pub use video_analysis_output as output;
pub use video_analysis_posture as posture;
pub use video_analysis_posture_io as posture_io;
pub use video_analysis_radiance_fields as radiance_fields;
pub use video_analysis_radiance_io as radiance_io;
pub use video_analysis_recognition as recognition;
pub use video_analysis_reconstruction as reconstruction;
pub use video_analysis_segmentation as video_segmentation;
pub use video_analysis_split as split;
pub use video_analysis_storage as storage;
pub use video_analysis_synthesis as synthesis;
pub use video_analysis_tracking as tracking;
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
