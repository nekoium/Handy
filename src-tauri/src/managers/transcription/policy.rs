//! How audio is decoded: the one place the model's traits and the settings
//! choose between streaming, long-form chunking and a single run.

use crate::engine_supervisor::LoadedInfo;
use crate::settings::OverlayStyle;

/// What a model can do that decides how its audio is decoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelTraits {
    /// Decodes audio incrementally as it is recorded.
    pub streaming: bool,
    /// Keeps its accuracy on audio longer than its native window in one run
    /// (whisper today).
    pub long_form: bool,
}

impl ModelTraits {
    /// The loaded model's own traits: the authority once a model is loaded.
    pub fn of(info: &LoadedInfo) -> Self {
        Self {
            streaming: info.capabilities.supports_streaming,
            long_form: info.supports_long_form,
        }
    }
}

/// How a recording is decoded while it is still going.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiveDecode {
    /// The model decodes the audio as it arrives.
    Stream,
    /// Experimental display-only Qwen batch snapshots alongside normal final decoding.
    Preview,
    /// Long audio is cut at pauses and each chunk decoded once its cut is
    /// certain.
    Chunked,
    /// Nothing until the recording stops; then it is decoded in batch.
    None,
}

/// How a whole clip is decoded: a recording without a live result, a
/// re-transcribed history entry, or a file from the CLI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatchDecode {
    /// Cut at pauses into chunks of about 30 s, each decoded in turn; a clip
    /// of 40 s or less is one chunk.
    Chunked,
    /// One run.
    Whole,
}

/// Process-local opt-in; never changes persisted settings.
pub fn qwen_preview_requested() -> bool {
    std::env::var("HANDY_QWEN_LIVE_PREVIEW").is_ok_and(|value| value == "1")
}

/// Unknown models are bounded preview candidates until loaded architecture resolves them.
/// This grants no authority to use preview text as a final result.
pub fn qwen_preview_enabled(requested: bool, overlay: OverlayStyle, arch: Option<&str>) -> bool {
    requested && overlay == OverlayStyle::Live && arch.is_none_or(|arch| arch == "qwen3_asr")
}

/// Streaming wins: a streaming model handles long audio itself. Otherwise
/// long audio is chunked unless the model keeps its accuracy on it.
pub fn live_decode(model: ModelTraits, long_form_chunking: bool) -> LiveDecode {
    if model.streaming {
        LiveDecode::Stream
    } else if long_form_chunking && !model.long_form {
        LiveDecode::Chunked
    } else {
        LiveDecode::None
    }
}

/// As live, except that a streaming model takes a whole clip in one run, as
/// it did before long-form chunking existed.
pub fn batch_decode(model: ModelTraits, long_form_chunking: bool) -> BatchDecode {
    if long_form_chunking && !model.long_form && !model.streaming {
        BatchDecode::Chunked
    } else {
        BatchDecode::Whole
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use transcribe_cpp::{Capabilities, TimestampKind};

    fn loaded(arch: &str) -> LoadedInfo {
        LoadedInfo {
            arch: arch.into(),
            variant: String::new(),
            backend: "cpu".into(),
            device: "cpu".into(),
            on_gpu: false,
            capabilities: Capabilities {
                native_sample_rate: 16000,
                languages: vec![],
                translate_target_languages: vec![],
                max_timestamp_kind: TimestampKind::None,
                supports_language_detect: true,
                supports_translate: false,
                supports_streaming: arch != "qwen3_asr",
                supports_spec_decode: false,
                max_audio_ms: 0,
            },
            supports_initial_prompt: false,
            supports_long_form: false,
            supports_vocabulary: false,
        }
    }

    #[test]
    fn preview_selection_table_uses_architecture_and_bounds_unknown_candidates() {
        for flag in [None, Some("0"), Some("1")] {
            for overlay in [
                OverlayStyle::Live,
                OverlayStyle::Minimal,
                OverlayStyle::None,
            ] {
                for (name, before, after) in [
                    ("catalog Qwen3", None, "qwen3_asr"),
                    ("renamed model", None, "qwen3_asr"),
                    ("misleading Qwen3 filename", None, "whisper"),
                    (
                        "other stream",
                        Some("moonshine_streaming"),
                        "moonshine_streaming",
                    ),
                    ("loaded Qwen", Some("qwen3_asr"), "qwen3_asr"),
                    ("model switched during load", Some("whisper"), "qwen3_asr"),
                ] {
                    let requested = flag == Some("1");
                    let bounded = qwen_preview_enabled(requested, overlay, before);
                    let selected = bounded && qwen_preview_enabled(requested, overlay, Some(after));
                    assert_eq!(
                        bounded,
                        requested
                            && overlay == OverlayStyle::Live
                            && before.is_none_or(|arch| arch == "qwen3_asr"),
                        "{name}"
                    );
                    assert_eq!(selected, bounded && after == "qwen3_asr", "{name}");
                    for long_form in [false, true] {
                        let traits = ModelTraits::of(&loaded(after));
                        let final_policy = batch_decode(traits, long_form);
                        assert_eq!(
                            final_policy,
                            if after == "qwen3_asr" && long_form {
                                BatchDecode::Chunked
                            } else {
                                BatchDecode::Whole
                            }
                        );
                        if after == "moonshine_streaming" {
                            assert_eq!(live_decode(traits, long_form), LiveDecode::Stream);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn pristine_qwen_keeps_default_and_final_batch_policy() {
        let model = ModelTraits::of(&loaded("qwen3_asr"));
        assert_eq!(live_decode(model, true), LiveDecode::Chunked);
        assert_eq!(live_decode(model, false), LiveDecode::None);
        assert_eq!(batch_decode(model, true), BatchDecode::Chunked);
        assert_eq!(batch_decode(model, false), BatchDecode::Whole);
    }

    #[test]
    fn other_streaming_models_keep_existing_selection() {
        let model = ModelTraits::of(&loaded("moonshine_streaming"));
        assert_eq!(live_decode(model, true), LiveDecode::Stream);
        assert_eq!(batch_decode(model, true), BatchDecode::Whole);
    }
}
