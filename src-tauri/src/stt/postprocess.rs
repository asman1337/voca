//! Post-processing for raw Whisper transcripts.
//!
//! Cleans up common artifacts, trims whitespace, and optionally
//! capitalizes the first letter of the transcript.

/// Known Whisper hallucination / noise artifacts to strip.
static ARTIFACTS: &[&str] = &[
    "[BLANK_AUDIO]",
    "(music)",
    "(Music)",
    "[Music]",
    "[MUSIC]",
    "(crowd noise)",
    "(background noise)",
    "(silence)",
    "[inaudible]",
    "[Inaudible]",
];

/// Clean up a raw Whisper transcript.
///
/// # Steps
/// 1. Strip surrounding whitespace.
/// 2. Remove known hallucination artifacts.
/// 3. If `capitalize` is true, uppercase the first letter.
pub fn clean(raw: &str, capitalize: bool) -> String {
    let mut text = raw.trim().to_string();

    for artifact in ARTIFACTS {
        text = text.replace(artifact, "");
    }

    // Re-trim after artifact removal
    text = text.trim().to_string();

    // Collapse multiple spaces
    while text.contains("  ") {
        text = text.replace("  ", " ");
    }

    if capitalize && !text.is_empty() {
        let mut chars = text.chars();
        if let Some(first) = chars.next() {
            text = first.to_uppercase().to_string() + chars.as_str();
        }
    }

    text
}

#[cfg(test)]
mod tests {
    use super::clean;

    #[test]
    fn strips_artifact() {
        let raw = "  [BLANK_AUDIO] hello world  ";
        assert_eq!(clean(raw, false), "hello world");
    }

    #[test]
    fn capitalizes_first_letter() {
        assert_eq!(clean("hello there", true), "Hello there");
    }

    #[test]
    fn empty_string_stays_empty() {
        assert_eq!(clean("", true), "");
    }

    #[test]
    fn strips_artifact_only_becomes_empty() {
        assert_eq!(clean("[BLANK_AUDIO]", false), "");
    }
}
