use super::{Finding, GroundedClaim, Grounder, GroundingError, evidence, spend};
use crate::proposal::Due;

const TIME_WORDS: &str = "\
    monday tuesday wednesday thursday friday saturday sunday mon tue wed thu fri sat sun \
    january february march april may june july august september october november december \
    jan feb mar apr jun jul aug sep oct nov dec one two three four five six seven eight nine \
    ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty \
    twenty-one twenty-two twenty-three twenty-four twenty-five twenty-six twenty-seven \
    twenty-eight twenty-nine thirty thirty-one first second third fourth fifth sixth seventh \
    eighth ninth tenth eleventh twelfth thirteenth fourteenth fifteenth sixteenth \
    seventeenth eighteenth nineteenth twentieth twenty-first twenty-second twenty-third \
    twenty-fourth twenty-fifth twenty-sixth twenty-seventh twenty-eighth twenty-ninth \
    thirtieth thirty-first today tonight tomorrow yesterday morning afternoon evening noon \
    midnight week weeks weekend month months year quarter day days hour hours minute minutes \
    eod eow deadline";

impl Grounder<'_> {
    pub fn due(&mut self, due: &Due) -> Result<GroundedClaim, GroundingError> {
        if due.source_references.len() > 256 {
            return Err(GroundingError::CapacityExceeded);
        }
        spend(&mut self.remaining, due.date.len() + 1)?;
        let mut grounded = self.sources(&due.source_references)?;
        if grounded.sources.is_empty() {
            return Ok(grounded);
        }
        let evidence =
            evidence::collect(&self.evidence, &grounded.sources, 0, &mut self.remaining)?;
        if !evidence.iter().any(|word| says_time(word)) {
            grounded.finding = Some(Finding::Unsupported(due.date.clone()));
        }
        Ok(grounded)
    }
}

fn says_time(word: &str) -> bool {
    let stem = word.split(['\'', '’']).next().unwrap_or(word);
    word.chars().any(char::is_numeric) || TIME_WORDS.split_whitespace().any(|time| time == stem)
}
