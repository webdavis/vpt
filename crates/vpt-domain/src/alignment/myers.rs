use super::{AlignmentError, Edit, Outcome};
use crate::normalize::Token;

mod trace;
use trace::{Frontier, traceback};

const MAX_TRACE_CELLS: usize = 1_048_576;
const MAX_COMPARISONS: usize = 8_388_608;

pub(super) fn diff(
    main: &[Token],
    checker: &[Token],
    limit: usize,
) -> Result<Outcome, AlignmentError> {
    if main.len().abs_diff(checker.len()) > limit {
        return Ok(Outcome::Divergent);
    }
    let mut trace = Vec::new();
    let mut cells = 0;
    let mut comparisons = 0;
    for distance in 0..=limit {
        cells += distance + 1;
        if cells > MAX_TRACE_CELLS {
            return Err(AlignmentError::CapacityExceeded);
        }
        let mut frontier = Frontier::new(distance);
        for diagonal in (-(distance as isize)..=distance as isize).step_by(2) {
            let (mut x, mut y) = if let Some(previous) = trace.last() {
                frontier.start(previous, diagonal)
            } else {
                (0, 0)
            };
            if x < 0 || y < 0 || x > main.len() as isize || y > checker.len() as isize {
                continue;
            }
            while x < main.len() as isize && y < checker.len() as isize {
                comparisons += 1;
                if comparisons > MAX_COMPARISONS {
                    return Err(AlignmentError::CapacityExceeded);
                }
                if main[x as usize].text != checker[y as usize].text {
                    break;
                }
                x += 1;
                y += 1;
            }
            frontier.set(diagonal, x);
            if x == main.len() as isize && y == checker.len() as isize {
                let edits = traceback(&trace, main, checker);
                return Ok(Outcome::Aligned { distance, edits });
            }
        }
        trace.push(frontier);
    }
    Ok(Outcome::Divergent)
}
