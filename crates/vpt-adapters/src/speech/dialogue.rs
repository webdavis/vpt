use super::{Clip, SpeechError, clip, failed, private_directory, run, text_of};

const SAMPLE_RATE: u32 = 16_000;

/// Speaks each line in its own voice and joins the lines, in order, into one clip.
///
/// `say` speaks one voice per file, so the lines are joined as samples and macOS `afconvert`
/// encodes the result.
pub fn converse(lines: &[(&str, &str)], maximum: u64) -> Result<Clip, SpeechError> {
    let directory = private_directory()?;
    let mut samples = Vec::new();
    for (index, &(voice, text)) in lines.iter().enumerate() {
        let line = directory.path().join(format!("line-{index}.wav"));
        let format = format!("--data-format=LEI16@{SAMPLE_RATE}");
        let arguments = ["-v", voice, "--file-format=WAVE", format.as_str(), "-o"];
        run(
            "say",
            &[&arguments[..], &[text_of(&line)?, text]].concat(),
            &line,
        )?;
        let bytes = std::fs::read(&line).map_err(failed)?;
        samples.extend_from_slice(pcm(&bytes).ok_or_else(|| {
            SpeechError::Failed("say", "wrote a WAVE file without samples".into())
        })?);
    }
    let joined = directory.path().join("dialogue.wav");
    std::fs::write(&joined, wave(&samples)).map_err(failed)?;
    let path = directory.path().join("clip.m4a");
    let arguments = [
        "-f",
        "m4af",
        "-d",
        "aac",
        text_of(&joined)?,
        text_of(&path)?,
    ];
    run("afconvert", &arguments, &path)?;
    clip("afconvert", directory, path, maximum)
}

fn pcm(bytes: &[u8]) -> Option<&[u8]> {
    let mut rest = bytes.get(12..)?;
    loop {
        let size = usize::try_from(u32::from_le_bytes(rest.get(4..8)?.try_into().ok()?)).ok()?;
        let body = rest.get(8..8 + size)?;
        if &rest[..4] == b"data" {
            return Some(body);
        }
        rest = rest.get(8 + size + size % 2..)?;
    }
}

fn wave(samples: &[u8]) -> Vec<u8> {
    let length = |count: usize| u32::try_from(count).unwrap_or(u32::MAX).to_le_bytes();
    let mut bytes = Vec::with_capacity(44 + samples.len());
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&length(36 + samples.len()));
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    bytes.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&length(samples.len()));
    bytes.extend_from_slice(samples);
    bytes
}
