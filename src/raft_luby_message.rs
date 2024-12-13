use std::ops::BitXor;

use crate::raft_nums::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RaftLubyMsg<Proposal> where
    Proposal: BitXor<Proposal, Output = Proposal>
{
    // Proposal request
    ProposalReq { proposal: Proposal, id: ProposalId },
    // Replicate a segment of log items
    // Leader should send out this to all
    ReplicateReq {
        commit: usize,
        leader: (Term, RaftId),
        prefix: (Option<Term>, usize),
        patch: Vec<(Proposal, ProposalId, Term)>,
    },
    // Acknowledge replication
    ReplicateAck { from: RaftId, sync: usize, tail: usize },
    // Reject replication
    ReplicateRej { from: RaftId, term: Term, at: usize },
    // Vote request
    VoteReq {
        candidate: (Term, RaftId),
        last: (Term, usize),
    },
    // Vote acknowledged
    VoteAck { term: Term },
    // Vote rejected
    VoteRej { term: Term },
}

#[derive(Debug)]
pub enum RaftErr {
    // Proposal Failed: 
    // 1. The proposer doesn't know who is the leader of current term. 
    // 2. A log entry that contains the proposal is overwritten. 
    ProposalFailed { id: ProposalId },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Codeword<Proposal> where
    Proposal: BitXor<Proposal, Output = Proposal>
{
    data: Proposal,
    symb: Vec<usize>
}