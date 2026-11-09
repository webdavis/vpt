use vpt_application::ports::EngineFailure;

const CODES: &str = "af am ar as az ba be bg bn bo br bs ca cs cy da de el en es et eu fa fi fo fr gl gu ha haw he hi hr ht hu hy id is it ja jw ka kk km kn ko la lb ln lo lt lv mg mi mk ml mn mr ms mt my ne nl nn no oc pa pl ps pt ro ru sa sd si sk sl sn so sq sr su sv sw ta te tg th tk tl tr tt uk ur uz vi yi yo zh yue";

pub(super) fn base(language: &str, model: &str) -> Result<&'static str, EngineFailure> {
    if language
        .split('-')
        .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_alphanumeric()))
    {
        return Err(EngineFailure::UnsupportedLanguage);
    }
    let base = language
        .split('-')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let base = if base == "jv" { "jw" } else { &base };
    let code = CODES
        .split_ascii_whitespace()
        .find(|candidate| *candidate == base)
        .ok_or(EngineFailure::UnsupportedLanguage)?;
    if model.ends_with(".en") && code != "en" {
        return Err(EngineFailure::UnsupportedLanguage);
    }
    Ok(code)
}
