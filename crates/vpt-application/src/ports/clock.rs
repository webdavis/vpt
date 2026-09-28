use vpt_domain::time::{UtcInstant, UtcOffset};

pub trait Clock {
    fn now(&self) -> UtcInstant;
    fn offset_at(&self, at: UtcInstant) -> UtcOffset;
}
