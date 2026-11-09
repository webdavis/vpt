use super::*;
use std::fs;
use vpt_domain::{
    transcript::LanguageSource,
    uncertainty::{Class, Diagnostic},
};

#[test]
fn whisply_defaults_and_cpu_options_reach_the_adapter_and_preserve_confidence_provenance() {
    for (device, model) in [("mlx", "large-v3-turbo"), ("cpu", "medium")] {
        let (sandbox, row) = configured();
        let archive = fs::read(&row.audio_path).unwrap();
        let script = sandbox.path().join("whisply fixture.sh");
        let source = sandbox.path().join("backend.json");
        let args = sandbox.path().join("backend.args");
        let input = sandbox.path().join("backend.input");
        let mut word = serde_json::json!({"word":"bonjour","start":0.0,"end":1.5});
        if device == "cpu" {
            word["score"] = 0.2.into();
        }
        let document = serde_json::json!({"device":device,"model":model,"transcription":{"fr":{
            "text":"bonjour","chunks":[{"timestamp":[0.0,1.5],"text":"bonjour","words":[word]}]}}});
        fs::write(&source, document.to_string()).unwrap();
        fs::write(&script, "#!/bin/sh\nset -eu\n[ \"$1\" = 'prefix with spaces' ]\nsource=$2\nargs=$3\ninput=$4\nshift 4\nprintf '%s\\n' \"$@\" > \"$args\"\n/bin/cat \"$3\" > \"$input\"\n/bin/mkdir -p \"$5/audio\"\n/bin/cat \"$source\" > \"$5/audio/audio.json\"\n/bin/rm \"$3\"\nprintf 'Private backend progress is not a document'\n").unwrap();
        let options = if device == "cpu" {
            "device='cpu'\nmodel='medium'\n"
        } else {
            ""
        };
        fixture::write_config(
            &sandbox,
            &format!(
                "[recording]\nlanguage='fr-CA'\n[engines]\nmain='whisply'\n[engines.whisply]\ncommand=['/bin/sh', {script:?}, 'prefix with spaces', {source:?}, {args:?}, {input:?}]\n{options}"
            ),
        );
        let output = run(&sandbox, row.id.as_str(), &[]);
        json(&output);
        let argv = fs::read_to_string(args).unwrap();
        let argv: Vec<_> = argv.lines().collect();
        assert_eq!(argv.len(), 13);
        assert_eq!(&argv[..2], ["run", "-f"]);
        assert_eq!(argv[3], "-o");
        assert_eq!(
            &argv[5..],
            ["-d", device, "-m", model, "-l", "fr", "-e", "json"]
        );
        assert_ne!(argv[2], row.audio_path.to_str().unwrap());
        assert!(!std::path::Path::new(argv[2]).exists());
        assert!(!std::path::Path::new(argv[4]).exists());
        assert_eq!(fs::read(input).unwrap(), archive);
        assert_eq!(fs::read(&row.audio_path).unwrap(), archive);
        let accepted = ledger(&sandbox)
            .accepted_transcript(&row.id)
            .unwrap()
            .unwrap();
        assert_eq!(accepted.requested_language, "fr-CA");
        assert_eq!(accepted.transcript.language, "fr-CA");
        assert_eq!(
            accepted.transcript.language_source,
            LanguageSource::Requested
        );
        assert_eq!(accepted.transcript.engine.model, model);
        assert_eq!(accepted.transcript.engine.family, "whisper");
        assert!(accepted.transcript.local);
        assert_eq!(accepted.transcript.segments[0].words[0].end, 1.5);
        assert_eq!(
            accepted.transcript.segments[0].words[0].confidence,
            if device == "cpu" { Some(0.2) } else { None }
        );
        assert_eq!(accepted.uncertainty.diagnostics, [Diagnostic::SingleEngine]);
        assert_eq!(
            accepted.uncertainty.lexical.len(),
            usize::from(device == "cpu")
        );
        if device == "cpu" {
            assert_eq!(accepted.uncertainty.lexical[0].class, Class::LowConfidence);
        }
        let path = sandbox
            .path()
            .join(format!("home/.vpt/engine-outputs/{}.whisply.json", row.id));
        let saved: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved["schema"], "vpt.engine/1");
        assert_eq!(saved["engine"]["model"], model);
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert!(!stdout(&output).contains("bonjour"));
        assert!(!stdout(&output).contains("Private"));
        assert!(!stderr(&output).contains("Private"));
    }
}
