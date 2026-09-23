use super::*;
use crate::ports::PromptError;
use std::cell::RefCell;

struct Answer(Result<usize, PromptError>);
impl Prompt for Answer {
    fn choose(
        &mut self,
        _: &str,
        options: &[&str],
        preselected: usize,
    ) -> Result<usize, PromptError> {
        assert_eq!(options, ["apple", "whisply"]);
        assert_eq!(preselected, 0);
        std::mem::replace(&mut self.0, Err(PromptError::Closed))
    }
}
#[derive(Default)]
struct Writer {
    exists: bool,
    fail_write: bool,
    fail_dir: bool,
    writes: RefCell<Vec<(String, bool)>>,
    dirs: RefCell<Vec<PathBuf>>,
}
impl SetupWriter for Writer {
    fn config_path(&self) -> PathBuf {
        "/config.toml".into()
    }
    fn config_exists(&self) -> bool {
        self.exists
    }
    fn write_config(&self, text: &str, force: bool) -> Result<PathBuf, SetupError> {
        if self.fail_write {
            return Err(SetupError::Io("write failed".into()));
        }
        self.writes.borrow_mut().push((text.into(), force));
        Ok(self.config_path())
    }
    fn create_private_dir(&self, path: &Path) -> Result<(), SetupError> {
        if self.fail_dir {
            return Err(SetupError::Io("directory failed".into()));
        }
        self.dirs.borrow_mut().push(path.into());
        Ok(())
    }
}
fn setup() -> Setup<impl Fn(&str) -> String> {
    Setup {
        engines: vec!["apple".into(), "whisply".into()],
        render: |engine| format!("main={engine}"),
        directories: vec!["/home".into(), "/state".into(), "/home/audio".into()],
    }
}
#[test]
fn selected_engine_is_rendered_and_all_directories_are_created() {
    let writer = Writer::default();
    let result = setup()
        .run(&mut Answer(Ok(1)), &writer, false)
        .expect("setup");
    assert_eq!(result.main_engine, "whisply");
    assert_eq!(result.written, Path::new("/config.toml"));
    assert_eq!(*writer.writes.borrow(), [("main=whisply".into(), false)]);
    assert_eq!(
        *writer.dirs.borrow(),
        [
            PathBuf::from("/home"),
            "/state".into(),
            "/home/audio".into()
        ]
    );
}
#[test]
fn apple_is_preselected_and_force_reaches_the_writer() {
    let writer = Writer {
        exists: true,
        ..Writer::default()
    };
    let result = setup()
        .run(&mut Answer(Ok(0)), &writer, true)
        .expect("forced");
    assert_eq!(result.main_engine, "apple");
    assert_eq!(*writer.writes.borrow(), [("main=apple".into(), true)]);
}
#[test]
fn existing_configuration_is_refused_before_prompting_or_writing() {
    let writer = Writer {
        exists: true,
        ..Writer::default()
    };
    assert_eq!(
        setup()
            .run(&mut Answer(Err(PromptError::Closed)), &writer, false)
            .unwrap_err(),
        SetupError::Exists("/config.toml".into())
    );
    assert!(writer.writes.borrow().is_empty());
    assert!(writer.dirs.borrow().is_empty());
}
#[test]
fn terminal_failure_writes_nothing() {
    for error in [PromptError::NoTerminal, PromptError::Closed] {
        let writer = Writer::default();
        let expected = if error == PromptError::NoTerminal {
            SetupError::NoTerminal
        } else {
            SetupError::Io("the terminal closed before an answer".into())
        };
        assert_eq!(
            setup()
                .run(&mut Answer(Err(error)), &writer, false)
                .unwrap_err(),
            expected
        );
        assert!(writer.writes.borrow().is_empty());
        assert!(writer.dirs.borrow().is_empty());
    }
}
#[test]
fn write_failure_stops_before_directory_creation() {
    let writer = Writer {
        fail_write: true,
        ..Writer::default()
    };
    assert_eq!(
        setup().run(&mut Answer(Ok(0)), &writer, false).unwrap_err(),
        SetupError::Io("write failed".into())
    );
    assert!(writer.dirs.borrow().is_empty());
}
#[test]
fn directory_failure_is_reported() {
    let writer = Writer {
        fail_dir: true,
        ..Writer::default()
    };
    assert_eq!(
        setup().run(&mut Answer(Ok(0)), &writer, false).unwrap_err(),
        SetupError::Io("directory failed".into())
    );
}
#[test]
fn an_invalid_choice_is_refused_without_a_write() {
    let writer = Writer::default();
    assert!(matches!(
        setup().run(&mut Answer(Ok(2)), &writer, false),
        Err(SetupError::Io(_))
    ));
    assert!(writer.writes.borrow().is_empty());
}
