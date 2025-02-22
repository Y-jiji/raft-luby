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

#[derive(Debug)]
pub enum RaftErr {
    // Proposal Failed: 
    // 1. The proposer doesn't know who is the leader of current term. 
    // 2. A log entry that contains the proposal is overwritten. 
    ProposalFailed { id: ProposalId },
}
