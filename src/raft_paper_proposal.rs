use crate::*;
use serde::{Serialize, Deserialize};

// Lifecycle of a proposal: 
// - A proposal is submitted to the leader. 
// - The leader tries to replicate this proposal. 
//    (1) A majority of servers accepted this proposal. 
//    (1.a) The new leader will eventually apply it, so it is safe to reply to the client. 
//    (2) The leader crashed / disconnected before a majority of servers accepted this proposal. 
//    (2.a) The new leader will eventually discard or apply it. 
// - When the item is committed, reply to the client. 
// - When the item is discarded, reply to the client. 
impl<Proposal> RaftPaperImpl<Proposal> where
    Proposal: Serialize + for<'de> Deserialize<'de> + Clone
{
    // try replicate based on current knowledge
    pub(crate) fn replicate(&mut self, adaptor: &impl Adaptor<RaftPaperMsg<Proposal>>, disk: &mut impl Persistor<Proposal>) {
        let PaperRole::Leader { guessed, .. } = &self.role else { return };
        self.timeout_heart = 0;
        println!("RAFT :: {:?} replicate", self.id);
        for id in self.peers.iter().copied() {
            if id == self.id { continue }
            let last_index = guessed[&id].min(disk.last().1);
            let last_term = last_index.checked_add_signed(-1).map(|x| disk.term(x).unwrap_or(Term(0)));
            adaptor.send(id, RaftPaperMsg::ReplicateReq {
                patch: disk.slice(last_index..last_index+self.batch), 
                leader: (self.term, self.id), 
                commit: self.commitable,
                prefix: (last_term, last_index)
            });
        }
    }
    // validate and append delta
    pub(crate) fn handle_replicate(&mut self,
        (leader_term, leader_id): (Term, RaftId),
        (prefix_term, prefix_index): (Option<Term>, usize),
        patch: Vec<(Proposal, ProposalId, Term)>,
        commit: usize,
        adaptor: &impl Adaptor<RaftPaperMsg<Proposal>>,
        disk: &mut impl Persistor<Proposal>
    ) {
        // if term is outdated or log doesn't match, reply append failed
        let reject = RaftPaperMsg::ReplicateRej {
            from: self.id, 
            term: self.term,
            at: prefix_index
        };
        if leader_term < self.term {
            println!("RAFT :: {:?} :: reject replication because current term is bigger", self.id);
            adaptor.send(leader_id, reject.clone());
            return;
        }
        // if currently i'm not a follower in this term, convert to follower
        self.role = PaperRole::Follower { leader: leader_id };
        self.timeout_elect = rand::random::<u64>() % self.bound_elect;
        if self.term < leader_term {
            self.term = leader_term;
            self.vote = None;
            disk.persist(self.term, self.vote);
        }
        if prefix_term.is_some() && prefix_term != disk.term(prefix_index.checked_add_signed(-1).unwrap_or(0)) {
            println!("RAFT :: {:?} :: reject replication because prefix doesn't match", self.id);
            adaptor.send(leader_id, reject);
            return;
        }
        // modify or update replicated entries
        // get the last synchronized entry
        let sync = disk.append(prefix_index, patch);
        // update commitable index
        if commit >= self.commitable {
            self.commitable = commit.min(disk.last().1);
            disk.commit(commit.min(disk.last().1));
        }
        adaptor.send(leader_id, RaftPaperMsg::ReplicateAck { from: self.id, tail: disk.last().1, sync });
    }
    // handle follower/candidate acknowledge
    pub(crate) fn handle_replicate_ack(&mut self,
        from: RaftId,
        sync: usize,
        tail: usize,
        disk: &mut impl Persistor<Proposal>
    ) {
        let PaperRole::Leader { matched, guessed } = &mut self.role else { return };
        *guessed.get_mut(&from).expect("every peer should be logged") = sync;
        *matched.get_mut(&from).expect("every peer should be logged") = sync;
        let mut matches = matched.values().copied().collect::<Vec<_>>();
        matches.sort();
        self.commitable = self.commitable.max(matches[self.peers.len() / 2 - 1]);
        disk.commit(self.commitable);
    }
    // handle follower/candidate rejection
    pub(crate) fn handle_replicate_rej(&mut self,
        from: RaftId,
        term: Term,
        at: usize, disk: &mut impl Persistor<Proposal>
    ) {
        let PaperRole::Leader { guessed, .. } = &mut self.role else { return };
        if term <= self.term {
            *guessed.get_mut(&from).expect("every peer should be logged") = at / 2;
        } else {
            self.role = PaperRole::Candidate { count: 0 };
            self.term = term;
            self.vote = None;
            disk.persist(self.term, self.vote);
        }
    }
}