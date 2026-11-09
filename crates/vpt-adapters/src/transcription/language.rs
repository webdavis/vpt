use super::InvocationError;

// Tokenizer language codes shared by MLX Whisper and Faster Whisper.
// https://github.com/ml-explore/mlx-examples/blob/main/whisper/mlx_whisper/tokenizer.py
// https://github.com/SYSTRAN/faster-whisper/blob/master/faster_whisper/tokenizer.py
const WHISPER_CODES: &str = "en zh de es ru ko fr ja pt tr pl ca nl ar sv it id hi fi vi he uk \
    el ms cs ro da hu ta no th ur hr bg lt la mi ml cy sk te fa lv bn sr az sl kn et mk br eu is \
    hy ne mn bs kk sq sw gl mr pa si km sn yo so af oc ka be tg sd gu am yi lo uz fo ht ps tk nn \
    mt sa lb my bo tl mg as tt haw ln ha ba jw su yue";

pub(super) fn validate(tag: &str) -> Result<(), InvocationError> {
    let mut parts = tag.split('-');
    let base = parts.next().unwrap_or("");
    if tag.len() > 255
        || !(2..=8).contains(&base.len())
        || !base.bytes().all(|byte| byte.is_ascii_alphabetic())
        || parts.any(|part| {
            !(1..=8).contains(&part.len()) || !part.bytes().all(|byte| byte.is_ascii_alphanumeric())
        })
    {
        return Err(InvocationError::InvalidLanguage);
    }
    Ok(())
}

pub(super) fn whisply(tag: &str, model: &str) -> Result<String, InvocationError> {
    let base = tag.split('-').next().unwrap_or("").to_ascii_lowercase();
    if !WHISPER_CODES.split_whitespace().any(|code| code == base)
        || (model.ends_with(".en") && base != "en")
    {
        return Err(InvocationError::UnsupportedLanguage);
    }
    Ok(base)
}
