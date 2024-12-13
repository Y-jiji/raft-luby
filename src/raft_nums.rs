//! Various 'numbers' used as id,term,log entry
use std::ops::Add;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Term(pub(crate) u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RaftId(pub(crate) u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProposalId(pub(crate) u64);

impl Add<u64> for Term {
    type Output = Self;
    fn add(self, rhs: u64) -> Self::Output {
        Term(self.0 + rhs)
    }
}