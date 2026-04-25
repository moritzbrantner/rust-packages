#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use std::ffi::{c_char, c_float, c_int, c_void};

pub enum whisper_context {}
pub enum whisper_state {}

pub type whisper_token = c_int;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum whisper_alignment_heads_preset {
    WHISPER_AHEADS_NONE = -1,
    WHISPER_AHEADS_N_TOP_MOST = 0,
    WHISPER_AHEADS_CUSTOM = 1,
    WHISPER_AHEADS_TINY_EN = 2,
    WHISPER_AHEADS_TINY = 3,
    WHISPER_AHEADS_BASE_EN = 4,
    WHISPER_AHEADS_BASE = 5,
    WHISPER_AHEADS_SMALL_EN = 6,
    WHISPER_AHEADS_SMALL = 7,
    WHISPER_AHEADS_MEDIUM_EN = 8,
    WHISPER_AHEADS_MEDIUM = 9,
    WHISPER_AHEADS_LARGE_V1 = 10,
    WHISPER_AHEADS_LARGE_V2 = 11,
    WHISPER_AHEADS_LARGE_V3 = 12,
    WHISPER_AHEADS_LARGE_V3_TURBO = 13,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct whisper_ahead {
    pub n_text_layer: c_int,
    pub n_head: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct whisper_aheads {
    pub n_heads: usize,
    pub heads: *const whisper_ahead,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct whisper_context_params {
    pub use_gpu: bool,
    pub flash_attn: bool,
    pub gpu_device: c_int,
    pub dtw_token_timestamps: bool,
    pub dtw_aheads_preset: whisper_alignment_heads_preset,
    pub dtw_n_top: c_int,
    pub dtw_aheads: whisper_aheads,
    pub dtw_mem_size: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct whisper_token_data {
    pub id: whisper_token,
    pub tid: whisper_token,
    pub p: c_float,
    pub plog: c_float,
    pub pt: c_float,
    pub ptsum: c_float,
    pub t0: i64,
    pub t1: i64,
    pub t_dtw: i64,
    pub vlen: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum whisper_gretype {
    WHISPER_GRETYPE_END = 0,
    WHISPER_GRETYPE_ALT = 1,
    WHISPER_GRETYPE_RULE_REF = 2,
    WHISPER_GRETYPE_CHAR = 3,
    WHISPER_GRETYPE_CHAR_NOT = 4,
    WHISPER_GRETYPE_CHAR_RNG_UPPER = 5,
    WHISPER_GRETYPE_CHAR_ALT = 6,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct whisper_grammar_element {
    pub type_: whisper_gretype,
    pub value: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct whisper_vad_params {
    pub threshold: c_float,
    pub min_speech_duration_ms: c_int,
    pub min_silence_duration_ms: c_int,
    pub max_speech_duration_s: c_float,
    pub speech_pad_ms: c_int,
    pub samples_overlap: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum whisper_sampling_strategy {
    WHISPER_SAMPLING_GREEDY = 0,
    WHISPER_SAMPLING_BEAM_SEARCH = 1,
}

pub type whisper_new_segment_callback =
    Option<unsafe extern "C" fn(*mut whisper_context, *mut whisper_state, c_int, *mut c_void)>;
pub type whisper_progress_callback =
    Option<unsafe extern "C" fn(*mut whisper_context, *mut whisper_state, c_int, *mut c_void)>;
pub type whisper_encoder_begin_callback =
    Option<unsafe extern "C" fn(*mut whisper_context, *mut whisper_state, *mut c_void) -> bool>;
pub type ggml_abort_callback = Option<unsafe extern "C" fn(*mut c_void) -> bool>;
pub type whisper_logits_filter_callback = Option<
    unsafe extern "C" fn(
        *mut whisper_context,
        *mut whisper_state,
        *const whisper_token_data,
        c_int,
        *mut c_float,
        *mut c_void,
    ),
>;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct whisper_full_params_greedy {
    pub best_of: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct whisper_full_params_beam_search {
    pub beam_size: c_int,
    pub patience: c_float,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct whisper_full_params {
    pub strategy: whisper_sampling_strategy,
    pub n_threads: c_int,
    pub n_max_text_ctx: c_int,
    pub offset_ms: c_int,
    pub duration_ms: c_int,
    pub translate: bool,
    pub no_context: bool,
    pub no_timestamps: bool,
    pub single_segment: bool,
    pub print_special: bool,
    pub print_progress: bool,
    pub print_realtime: bool,
    pub print_timestamps: bool,
    pub token_timestamps: bool,
    pub thold_pt: c_float,
    pub thold_ptsum: c_float,
    pub max_len: c_int,
    pub split_on_word: bool,
    pub max_tokens: c_int,
    pub debug_mode: bool,
    pub audio_ctx: c_int,
    pub tdrz_enable: bool,
    pub suppress_regex: *const c_char,
    pub initial_prompt: *const c_char,
    pub carry_initial_prompt: bool,
    pub prompt_tokens: *const whisper_token,
    pub prompt_n_tokens: c_int,
    pub language: *const c_char,
    pub detect_language: bool,
    pub suppress_blank: bool,
    pub suppress_nst: bool,
    pub temperature: c_float,
    pub max_initial_ts: c_float,
    pub length_penalty: c_float,
    pub temperature_inc: c_float,
    pub entropy_thold: c_float,
    pub logprob_thold: c_float,
    pub no_speech_thold: c_float,
    pub greedy: whisper_full_params_greedy,
    pub beam_search: whisper_full_params_beam_search,
    pub new_segment_callback: whisper_new_segment_callback,
    pub new_segment_callback_user_data: *mut c_void,
    pub progress_callback: whisper_progress_callback,
    pub progress_callback_user_data: *mut c_void,
    pub encoder_begin_callback: whisper_encoder_begin_callback,
    pub encoder_begin_callback_user_data: *mut c_void,
    pub abort_callback: ggml_abort_callback,
    pub abort_callback_user_data: *mut c_void,
    pub logits_filter_callback: whisper_logits_filter_callback,
    pub logits_filter_callback_user_data: *mut c_void,
    pub grammar_rules: *const *const whisper_grammar_element,
    pub n_grammar_rules: usize,
    pub i_start_rule: usize,
    pub grammar_penalty: c_float,
    pub vad: bool,
    pub vad_model_path: *const c_char,
    pub vad_params: whisper_vad_params,
}

unsafe extern "C" {
    pub fn whisper_context_default_params() -> whisper_context_params;
    pub fn whisper_full_default_params(
        strategy: whisper_sampling_strategy,
    ) -> whisper_full_params;
    pub fn whisper_init_from_file_with_params(
        path_model: *const c_char,
        params: whisper_context_params,
    ) -> *mut whisper_context;
    pub fn whisper_free(ctx: *mut whisper_context);
    pub fn whisper_full(
        ctx: *mut whisper_context,
        params: whisper_full_params,
        samples: *const c_float,
        n_samples: c_int,
    ) -> c_int;
    pub fn whisper_full_n_segments(ctx: *mut whisper_context) -> c_int;
    pub fn whisper_full_lang_id(ctx: *mut whisper_context) -> c_int;
    pub fn whisper_full_get_segment_t0(ctx: *mut whisper_context, i_segment: c_int) -> i64;
    pub fn whisper_full_get_segment_t1(ctx: *mut whisper_context, i_segment: c_int) -> i64;
    pub fn whisper_full_get_segment_text(
        ctx: *mut whisper_context,
        i_segment: c_int,
    ) -> *const c_char;
    pub fn whisper_full_n_tokens(ctx: *mut whisper_context, i_segment: c_int) -> c_int;
    pub fn whisper_full_get_token_p(
        ctx: *mut whisper_context,
        i_segment: c_int,
        i_token: c_int,
    ) -> c_float;
    pub fn whisper_lang_id(lang: *const c_char) -> c_int;
    pub fn whisper_lang_str(id: c_int) -> *const c_char;
    pub fn whisper_print_system_info() -> *const c_char;
}
