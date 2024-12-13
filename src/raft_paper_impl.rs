use std::{collections::HashMap, fmt::Debug, marker::PhantomData};
use serde::{Deserialize, Serialize};

use crate::*;

pub struct RaftPaperImpl<Proposal> where
    Proposal: Serialize + for<'de> Deserialize<'de>
{
    // constant parameters
    pub(crate) id: RaftId,
    pub(crate) batch: usize,
    pub(crate) peers: Vec<RaftId>,
    pub(crate) phantom: PhantomData<Proposal>,
    // non-volatile states
    pub(crate) term: Term,
    pub(crate) vote: Option<RaftId>,
    // volatile states
    pub(crate) role: PaperRole,
    pub(crate) commitable: usize,
    pub(crate) timeout_elect: u64,
    pub(crate) timeout_heart: u64,
    pub(crate) bound_elect: u64,
    pub(crate) bound_heart: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaperRole {
    Leader { 
        matched: HashMap<RaftId, usize>, 
        guessed: HashMap<RaftId, usize> 
    },
    Follower { leader: RaftId },
    Candidate { count: usize },
}

impl<Proposal> RaftPaperImpl<Proposal> where 
    Proposal: Serialize + for<'de> Deserialize<'de> + Clone + Debug
{
    pub fn new(
        id: RaftId, batch: usize, 
        peers: Vec<RaftId>, 
        bound_elect: u64,
        bound_heart: u64,
        disk: &mut impl Persistor<Proposal>
    ) -> Self {
        let (term, vote) = disk.load();
        Self {
            role: PaperRole::Candidate { count: 0 },
            commitable: disk.commitable(),
            id, batch, peers, term, vote, phantom: PhantomData, 
            bound_elect, timeout_elect: rand::random::<u64>() % bound_elect,
            bound_heart, timeout_heart: 0
        }
    }
    pub fn handle(&mut self, adaptor: &impl Adaptor<RaftMsg<Proposal>>, disk: &mut impl Persistor<Proposal>) -> bool {
        let Some(msg) = adaptor.receive() else { return false };
        match msg {
            RaftMsg::ProposalReq { proposal, id } 
                => {let _ = self.propose(proposal, id, adaptor, disk);},
            RaftMsg::ReplicateReq { leader, prefix, patch, commit } 
                => self.handle_replicate(leader, prefix, patch, commit, adaptor, disk),
            RaftMsg::ReplicateAck { from, sync, tail }
                => self.handle_replicate_ack(from, sync, tail, disk),
            RaftMsg::ReplicateRej { from, term, at }
                => self.handle_replicate_rej(from, term, at, disk),
            RaftMsg::VoteReq { candidate, last }
                => self.handle_vote_req(candidate, last, adaptor, disk),
            RaftMsg::VoteAck { term }
                => self.handle_vote_ack(term, disk),
            RaftMsg::VoteRej { term }
                => self.handle_vote_rej(term, disk),
        } true
    }
    // submit a proposal to a server
    // - the proposal id must be distinct, even if it is a resubmission
    pub fn propose(&mut self, proposal: Proposal, id: ProposalId, adaptor: &impl Adaptor<RaftMsg<Proposal>>, disk: &mut impl Persistor<Proposal>) -> Result<(), RaftErr> {
        match self.role {
            // a follower cannot handle request itself, but can redirect it to leader
            PaperRole::Follower { leader } => {
                adaptor.send(leader, RaftMsg::ProposalReq { proposal, id });
                Ok(())
            }
            // a candidate cannot effectively handle this
            PaperRole::Candidate { .. } => Err(RaftErr::ProposalFailed { id }),
            // a leader can locally 
            PaperRole::Leader { .. } => {
                // push a new log item to current log
                disk.push(proposal, id, self.term);
                // try to replicate once
                self.replicate(adaptor, disk);
                Ok(())
            }
        }
    }
    // tick timeout, do what is needed
    pub fn tick(&mut self, adaptor: &impl Adaptor<RaftMsg<Proposal>>, disk: &mut impl Persistor<Proposal>) {
        self.timeout_elect += 1;
        self.timeout_heart += 1;
        if self.timeout_elect >= self.bound_elect {
            self.coup_détat(adaptor, disk);
        }
        if self.timeout_heart >= self.bound_heart {
            self.replicate(adaptor, disk);
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::{Arc, Mutex};
    use super::*;

    #[test]
    fn mock_fifo() {
        type P = usize;
        type M = RaftMsg<P>;
        let peers = (0..5).map(|i| RaftId(i)).collect::<Vec<_>>();
        let network = Arc::new(Mutex::new(MockFIFONetwork::<M>::new(5)));
        let mut adaptors = (0..5).map(|i| MockAdaptor::<M, _>::new(RaftId(i), network.clone())).collect::<Vec<_>>();
        let mut disks = vec![MockPersistor::<P>::new(); 5];
        let mut nodes = (0..5).map(|i| 
            RaftPaperImpl::new(RaftId(i), 10, 
        {let mut peer = peers.clone(); peer.remove(i as usize); peer}, 
        100, 2, &mut disks[i as usize]
            )
        ).collect::<Vec<_>>();
        for p in 0..2000 {
            for i in 0..5 {
                while nodes[i].handle(&mut adaptors[i], &mut disks[i]) {}
                nodes[i].tick(&mut adaptors[i], &mut disks[i]);
                let _ = nodes[i].propose(p, ProposalId((p * 5 + i) as u64), &mut adaptors[i],&mut disks[i]);
            }
        }
    }

    #[test]
    fn mock_burst() {
        type P = usize;
        type M = RaftMsg<P>;
        let peers = (0..5).map(|i| RaftId(i)).collect::<Vec<_>>();
        let network = Arc::new(Mutex::new(MockBrustNetwork::<M>::new(5, 0.9, 0.01, 0.5, 0.1, 1)));
        let mut adaptors = (0..5).map(|i| MockAdaptor::<M, _>::new(RaftId(i), network.clone())).collect::<Vec<_>>();
        let mut disks = vec![MockPersistor::<P>::new(); 5];
        let mut nodes = (0..5).map(|i| 
            RaftPaperImpl::new(RaftId(i), 10, 
        {let mut peer = peers.clone(); peer.remove(i as usize); peer}, 
        100, 2, &mut disks[i as usize]
            )
        ).collect::<Vec<_>>();
        for p in 0..2000 {
            for i in 0..5 {
                while nodes[i].handle(&mut adaptors[i], &mut disks[i]) {}
                nodes[i].tick(&mut adaptors[i], &mut disks[i]);
                let _ = nodes[i].propose(p * 5 + i, ProposalId((p * 5 + i) as u64), &mut adaptors[i],&mut disks[i]);
            }
        }
    }
}