use super::{EventLog, LogError, LogInspection, verify_file};
use crate::contained::private_state::effective_user;
use std::io::ErrorKind;
use std::io::Write;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TailRepair {
    pub original_end: u64,
    pub retained_end: u64,
    pub synced: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AppendSync {
    #[default]
    None,
    File,
    FileAndDirectory,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AppendEffects {
    pub inspection: Option<LogInspection>,
    pub repair: Option<TailRepair>,
    pub written_bytes: u64,
    pub synced: AppendSync,
}

pub(super) fn synchronize(
    file: &std::fs::File,
    step: AppendStep,
    before: &mut impl FnMut(AppendStep) -> Result<(), LogError>,
) -> Result<(), LogError> {
    before(step)?;
    file.sync_all().map_err(|error| LogError::Io(error.kind()))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppendFailure {
    pub error: LogError,
    pub effects: AppendEffects,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AppendReceipt {
    pub device: u64,
    pub inode: u64,
    pub end_offset: u64,
    pub repair: Option<TailRepair>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AppendStep {
    Repair,
    RepairSync,
    Write,
    FileSync,
    DirectorySync,
    Confirm,
}

impl EventLog {
    pub fn append_line(&mut self, line: &str) -> Result<AppendReceipt, Box<AppendFailure>> {
        self.append_with(line, |_| Ok(()))
    }

    pub(super) fn append_with(
        &mut self,
        line: &str,
        mut before: impl FnMut(AppendStep) -> Result<(), LogError>,
    ) -> Result<AppendReceipt, Box<AppendFailure>> {
        let mut effects = AppendEffects::default();
        self.append_inner(line, &mut before, &mut effects)
            .map_err(|error| Box::new(AppendFailure { error, effects }))
    }

    fn append_inner(
        &mut self,
        line: &str,
        before: &mut impl FnMut(AppendStep) -> Result<(), LogError>,
        effects: &mut AppendEffects,
    ) -> Result<AppendReceipt, LogError> {
        if line.is_empty() || line.contains(['\r', '\n']) {
            return Err(LogError::InvalidLine);
        }
        let inspection = self.inspect()?;
        effects.inspection = Some(inspection);
        let line_length =
            u64::try_from(line.len()).map_err(|_| LogError::Io(ErrorKind::InvalidInput))?;
        let end_offset = inspection
            .complete_end_offset
            .checked_add(line_length)
            .and_then(|end| end.checked_add(1))
            .ok_or(LogError::Io(ErrorKind::InvalidInput))?;
        if inspection.complete_end_offset != inspection.physical_end_offset {
            before(AppendStep::Repair)?;
            verify_file(&self.root, &self.file, effective_user())?;
            self.require_length(inspection.physical_end_offset)?;
            self.file
                .set_len(inspection.complete_end_offset)
                .map_err(|error| LogError::Io(error.kind()))?;
            effects.repair = Some(TailRepair {
                original_end: inspection.physical_end_offset,
                retained_end: inspection.complete_end_offset,
                synced: false,
            });
            synchronize(&self.file, AppendStep::RepairSync, before)?;
            if let Some(repair) = &mut effects.repair {
                repair.synced = true;
            }
        }
        self.write_bytes(
            line.as_bytes(),
            inspection.complete_end_offset,
            effects,
            before,
        )?;
        self.write_bytes(b"\n", inspection.complete_end_offset, effects, before)?;
        synchronize(&self.file, AppendStep::FileSync, before)?;
        effects.synced = AppendSync::File;
        synchronize(&self.root.file, AppendStep::DirectorySync, before)?;
        effects.synced = AppendSync::FileAndDirectory;
        before(AppendStep::Confirm)?;
        verify_file(&self.root, &self.file, effective_user())?;
        self.require_length(end_offset)?;
        Ok(AppendReceipt {
            device: inspection.device,
            inode: inspection.inode,
            end_offset,
            repair: effects.repair,
        })
    }

    fn write_bytes(
        &mut self,
        mut bytes: &[u8],
        start: u64,
        effects: &mut AppendEffects,
        before: &mut impl FnMut(AppendStep) -> Result<(), LogError>,
    ) -> Result<(), LogError> {
        while !bytes.is_empty() {
            before(AppendStep::Write)?;
            verify_file(&self.root, &self.file, effective_user())?;
            self.require_length(
                start
                    .checked_add(effects.written_bytes)
                    .ok_or(LogError::Io(ErrorKind::InvalidData))?,
            )?;
            match self.file.write(bytes) {
                Ok(0) => return Err(LogError::Io(ErrorKind::WriteZero)),
                Ok(written) => {
                    let count =
                        u64::try_from(written).map_err(|_| LogError::Io(ErrorKind::InvalidData))?;
                    effects.written_bytes = effects
                        .written_bytes
                        .checked_add(count)
                        .ok_or(LogError::Io(ErrorKind::InvalidData))?;
                    bytes = &bytes[written..];
                }
                Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                Err(error) => return Err(LogError::Io(error.kind())),
            }
        }
        Ok(())
    }

    fn require_length(&self, expected: u64) -> Result<(), LogError> {
        let current = self
            .file
            .metadata()
            .map_err(|error| LogError::Io(error.kind()))?;
        if current.len() != expected {
            return Err(LogError::LengthChanged);
        }
        Ok(())
    }
}
