use super::{Edit, Token};

pub(super) struct Frontier {
    distance: isize,
    positions: Vec<isize>,
}

impl Frontier {
    pub(super) fn new(distance: usize) -> Self {
        Self {
            distance: distance as isize,
            positions: vec![-1; distance + 1],
        }
    }

    fn get(&self, diagonal: isize) -> isize {
        if diagonal.abs() > self.distance {
            return -1;
        }
        self.positions[((diagonal + self.distance) / 2) as usize]
    }

    pub(super) fn set(&mut self, diagonal: isize, x: isize) {
        self.positions[((diagonal + self.distance) / 2) as usize] = x;
    }

    fn predecessor(&self, diagonal: isize) -> isize {
        if diagonal == -self.distance - 1
            || (diagonal != self.distance + 1 && self.get(diagonal - 1) < self.get(diagonal + 1))
        {
            diagonal + 1
        } else {
            diagonal - 1
        }
    }

    pub(super) fn start(&self, previous: &Self, diagonal: isize) -> (isize, isize) {
        let predecessor = previous.predecessor(diagonal);
        let prior_x = previous.get(predecessor);
        if prior_x < 0 {
            return (-1, -1);
        }
        let x = prior_x + isize::from(predecessor < diagonal);
        (x, x - diagonal)
    }
}

pub(super) fn traceback(trace: &[Frontier], main: &[Token], checker: &[Token]) -> Vec<Edit> {
    let (mut x, mut y) = (main.len(), checker.len());
    let mut edits = Vec::with_capacity(x + y);
    for previous in trace.iter().rev() {
        let diagonal = x as isize - y as isize;
        let predecessor = previous.predecessor(diagonal);
        let prior_x = previous.get(predecessor) as usize;
        let prior_y = (prior_x as isize - predecessor) as usize;
        while x > prior_x && y > prior_y {
            x -= 1;
            y -= 1;
            edits.push(Edit::Equal {
                main: main[x].original_index,
                checker: checker[y].original_index,
            });
        }
        if x == prior_x {
            y -= 1;
            edits.push(Edit::Insert {
                checker: checker[y].original_index,
            });
        } else {
            x -= 1;
            edits.push(Edit::Delete {
                main: main[x].original_index,
            });
        }
    }
    while x > 0 && y > 0 {
        x -= 1;
        y -= 1;
        edits.push(Edit::Equal {
            main: main[x].original_index,
            checker: checker[y].original_index,
        });
    }
    edits.reverse();
    edits
}
