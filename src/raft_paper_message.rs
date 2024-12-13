use crate::raft_nums::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RaftPaperMsg<Proposal> {
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