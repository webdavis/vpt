use super::recorder_error;
use crate::voice_memos::TitleCopy;
use crate::{ContainedError, RootDir};
use std::cell::RefCell;
use std::path::PathBuf;
use vpt_application::ports::{RecorderError, TitleLookup};

pub(super) struct Titles {
    state_dir: PathBuf,
    state: Result<RootDir, ContainedError>,
    refresh: bool,
    copy: RefCell<Option<TitleCopy>>,
}

impl Titles {
    pub(super) fn from_path(state_dir: PathBuf, refresh: bool) -> Self {
        let state = RootDir::open(&state_dir);
        Self {
            state_dir,
            state,
            refresh,
            copy: RefCell::new(None),
        }
    }

    pub(super) fn from_root(state: &RootDir, refresh: bool) -> Self {
        Self {
            state_dir: state.path().into(),
            state: state.try_clone(),
            refresh,
            copy: RefCell::new(None),
        }
    }

    pub(super) fn title(&self, file_name: &str) -> TitleLookup {
        self.copy
            .borrow()
            .as_ref()
            .map_or(TitleLookup::Unavailable, |copy| copy.title(file_name))
    }

    pub(super) fn refresh(&self, recordings: &RootDir) -> Result<(), RecorderError> {
        if !self.refresh {
            return Ok(());
        }
        drop(self.copy.borrow_mut().take());
        let state = match &self.state {
            Ok(state) => state,
            Err(
                ContainedError::Escape { .. }
                | ContainedError::NotRegular(_)
                | ContainedError::NotADirectory(_),
            ) => {
                return Err(RecorderError::Escape(self.state_dir.clone()));
            }
            Err(_) => return Ok(()),
        };
        recordings
            .revalidate()
            .map_err(|error| recorder_error(recordings.path(), error))?;
        let container = recordings.path().parent().unwrap_or(recordings.path());
        let copy = TitleCopy::refresh_at(container, state)
            .map_err(|_| RecorderError::Escape(self.state_dir.clone()))?;
        *self.copy.borrow_mut() = Some(copy);
        Ok(())
    }
}
