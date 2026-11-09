use super::*;
pub(super) struct Session<F, N, I> {
    pub deadline: Instant,
    pub send: F,
    pub now: N,
    pub interrupted: I,
}
impl<F, N, I> Session<F, N, I>
where
    F: FnMut(Request<'_>, Duration) -> Result<Vec<u8>, Failure>,
    N: FnMut() -> Instant,
    I: FnMut() -> bool,
{
    pub fn check(&mut self) -> Result<(), Failure> {
        if (self.interrupted)() {
            return Err(Failure::Interrupted);
        }
        if (self.now)() >= self.deadline {
            return Err(Failure::Deadline);
        }
        Ok(())
    }
    pub fn fetch(&mut self, request: Request<'_>) -> Result<Vec<u8>, Failure> {
        self.check()?;
        let remaining = self
            .deadline
            .checked_duration_since((self.now)())
            .filter(|d| !d.is_zero())
            .ok_or(Failure::Deadline)?;
        let response = (self.send)(request, remaining);
        if matches!(&response, Err(Failure::Interrupted)) {
            return Err(Failure::Interrupted);
        }
        self.check()?;
        let bytes = response?;
        if bytes.len() > 65_536 {
            return Err(Failure::Capacity);
        }
        Ok(bytes)
    }
}
