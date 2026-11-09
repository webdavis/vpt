use crate::{
    brief::{BriefError, Candidate},
    relations::{Kind, State, Target},
};
use std::collections::BTreeMap;

#[derive(Default)]
struct Components<'a> {
    names: BTreeMap<&'a str, usize>,
    parents: Vec<usize>,
    sizes: Vec<usize>,
}
impl<'a> Components<'a> {
    fn node(&mut self, name: &'a str) -> Result<usize, BriefError> {
        if let Some(id) = self.names.get(name) {
            return Ok(*id);
        }
        if self.parents.len() == 65_536 {
            return Err(BriefError::CapacityExceeded);
        }
        let id = self.parents.len();
        self.parents.push(id);
        self.sizes.push(1);
        self.names.insert(name, id);
        Ok(id)
    }
    fn root(&mut self, mut node: usize) -> usize {
        while self.parents[node] != node {
            self.parents[node] = self.parents[self.parents[node]];
            node = self.parents[node];
        }
        node
    }
    fn join(&mut self, first: usize, second: usize) {
        let mut first = self.root(first);
        let mut second = self.root(second);
        if first == second {
            return;
        }
        if self.sizes[first] < self.sizes[second] {
            std::mem::swap(&mut first, &mut second);
        }
        self.parents[second] = first;
        self.sizes[first] += self.sizes[second];
    }
}

pub(super) fn groups(rows: &[Candidate], indices: &[usize]) -> Result<Vec<usize>, BriefError> {
    let mut graph = Components::default();
    for index in indices {
        let row = &rows[*index];
        let first = graph.node(row.recording.as_str())?;
        for relation in &row.relations {
            if relation.kind == Kind::Continues
                && relation.state == State::Confirmed
                && let Target::Recording(id) = &relation.target
            {
                let second = graph.node(id.as_str())?;
                graph.join(first, second);
            }
        }
    }
    let mut result = vec![0; rows.len()];
    for index in indices {
        let node = graph.names[rows[*index].recording.as_str()];
        result[*index] = graph.root(node);
    }
    Ok(result)
}
