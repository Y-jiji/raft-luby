use std::path;

use crate::*;

pub trait Persistor<Proposal> {
    /// persist raft state
    /// this must be synchronous
    fn persist(&mut self, term: Term, vote: Option<RaftId>);
    /// load persisted state
    fn load(&mut self) -> (Term, Option<RaftId>);
    /// push a proposal to local log
    fn push(&mut self, proposal: Proposal, id: ProposalId, term: Term);
    /// access last log item
    fn last(&self) -> (Term, usize);
    /// get term at a given position
    fn term(&self, at: usize) -> Option<Term>;
    /// append / overwrite from a start position
    fn append(&mut self, at: usize, patch: Vec<(Proposal, ProposalId, Term)>) -> usize;
    /// mark entries 0..=at as commitable
    /// return last applied index
    fn commit(&mut self, at: usize);
    /// get last commitable
    fn commitable(&self) -> usize;
    /// copy a slice of range
    fn slice(&mut self, range: std::ops::Range<usize>) -> Vec<(Proposal, ProposalId, Term)>;
}

#[derive(Debug, Clone)]
pub struct MockPersistor<Proposal> {
    commit: usize,
    log: Vec<(Proposal, ProposalId, Term)>,
    vote: Option<RaftId>,
    term: Term
}

impl<Proposal: Clone> MockPersistor<Proposal> {
    pub fn new() -> Self {
        Self { commit: 0, log: vec![], vote: None, term: Term(0) }
    }
}

impl<Proposal: Clone> Persistor<Proposal> for MockPersistor<Proposal> {
    fn persist(&mut self, term: Term, vote: Option<RaftId>) {
        self.term = term;
        self.vote = vote;
    }
    fn load(&mut self) -> (Term, Option<RaftId>) {
        (self.term, self.vote)
    }
    fn push(&mut self, proposal: Proposal, id: ProposalId, term: Term) {
        self.log.push((proposal, id, term));
    }
    fn last(&self) -> (Term, usize) {
        self.log.last().map(|(_, _, term)| (*term, self.log.len())).unwrap_or((Term(0), 0))
    }
    fn term(&self, at: usize) -> Option<Term> {
        self.log.get(at).map(|(_, _, term)| *term)
    }
    fn append(&mut self, at: usize, patch: Vec<(Proposal, ProposalId, Term)>) -> usize {
        let mut end = at;
        for (delta, (proposal, id, term)) in patch.into_iter().enumerate() {
            if let Some(entry) = self.log.get(at + delta) {
                if entry.2 != term { self.log.resize_with(at + delta, || panic!()); }
                else { end = at + delta + 1; }
            }
            if at + delta == self.log.len() {
                self.log.push((proposal, id, term));
                end = at + delta + 1;
            }
        }
        return end;
    }
    fn commit(&mut self, at: usize) {
        self.commit = self.commit.max(at);
    }
    fn commitable(&self) -> usize {
        self.commit
    }
    fn slice(&mut self, mut range: std::ops::Range<usize>) -> Vec<(Proposal, ProposalId, Term)> {
        range.end = range.end.min(self.log.len());
        range.start = range.start.min(range.end);
        self.log[range].to_vec()
    }
}