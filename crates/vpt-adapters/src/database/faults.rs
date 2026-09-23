use super::DatabaseError;

#[cfg(not(test))]
#[derive(Default)]
pub(super) struct FaultState;

#[cfg(not(test))]
impl FaultState {
    pub(super) fn check(&self, _: &'static str) -> Result<(), DatabaseError> {
        Ok(())
    }
    pub(super) fn write_prefix(&self) -> Result<Option<usize>, DatabaseError> {
        Ok(None)
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Fault {
    Write,
    FileSync,
    DirectorySync,
    Truncate,
}

#[cfg(test)]
#[derive(Clone, Copy)]
enum Injection {
    Failure(Fault),
    Short(usize),
}

#[cfg(test)]
#[derive(Default)]
pub(super) struct FaultState {
    next: std::cell::Cell<Option<Injection>>,
    pub(super) consumed: std::rc::Rc<std::cell::Cell<bool>>,
}

#[cfg(test)]
impl FaultState {
    pub(super) fn arm(&self, fault: Fault) {
        self.next.set(Some(Injection::Failure(fault)));
        self.consumed.set(false);
    }
    pub(super) fn short_write(&self, bytes: usize) {
        self.next.set(Some(Injection::Short(bytes)));
        self.consumed.set(false);
    }
    pub(super) fn check(&self, operation: &'static str) -> Result<(), DatabaseError> {
        let Some(Injection::Failure(fault)) = self.next.get() else {
            return Ok(());
        };
        let expected = match fault {
            Fault::Write => "write",
            Fault::FileSync => "file sync",
            Fault::DirectorySync => "directory sync",
            Fault::Truncate => "truncate",
        };
        if operation != expected {
            return Ok(());
        }
        self.next.set(None);
        self.consumed.set(true);
        Err(super::io_error(
            operation,
            std::io::Error::from_raw_os_error(libc::EIO),
        ))
    }
    pub(super) fn write_prefix(&self) -> Result<Option<usize>, DatabaseError> {
        self.check("write")?;
        if let Some(Injection::Short(bytes)) = self.next.get() {
            self.next.set(None);
            self.consumed.set(true);
            Ok(Some(bytes))
        } else {
            Ok(None)
        }
    }
}
