use crate::*;
use serde::{Serialize, Deserialize};
use std::{collections::HashMap, fmt::Debug};

// Leader election in a term: 
// - To get elected, the candidate's term must be 'up-to-date' to a majority of servers.
// - To get elected, the candidate must have a more 'up-to-date' log than a majority of servers. 
impl<Proposal> RaftPaperImpl<Proposal> where
    Proposal: Serialize + for<'de> Deserialize<'de> + Debug
{
    // become a candidate and request vote from all peers
    // - the server is not currently a leader
    // - start a new term and vote for itself
    // - reset election timeout
    // - send request
    pub fn coup_détat(&mut self, adaptor: &impl Adaptor<RaftPaperMsg<Proposal>>, disk: &mut impl Persistor<Proposal>) {
        // normally the leader will not start a coup d'état
        // (unless you are the president of south korea in 2025)
        if matches!(self.role, PaperRole::Leader { .. }) { return }
        println!("RAFT :: {:?} coup_détat", self.id);
        // increase current term
        // in this term, vote for self
        self.term = self.term + 1;
        self.role = PaperRole::Candidate { count: 1 };
        self.vote = Some(self.id);
        disk.persist(self.term, self.vote);
        self.timeout_elect = rand::random::<u64>() % self.bound_elect;
        // ask for vote from all other servers
        for id in self.peers.iter().copied() {
            if id == self.id { continue }
            adaptor.send(id, RaftPaperMsg::VoteReq { last: disk.last(), candidate: (self.term, self.id)});
        }
    }
    // handle vote request
    // - reject vote if self.term > candidate.term
    // - reject vote if current server has already voted in (self.term, candidate.term)
    //   - for the same candidate, it only asks for vote once in each term
    //     - we can infer that the candidate must increased its term
    //   - for different candidates, a server cannot vote twice in the same term
    // - reject vote if current log is considered more 'up-to-date'
    //   - first compare term, larger wins
    //   - if term is equal, compare log length. larger wins
    pub fn handle_vote_req(&mut self, 
        (cand_term, cand_id): (Term, RaftId),
        (last_term, last_index): (Term, usize),
        adaptor: &impl Adaptor<RaftPaperMsg<Proposal>>, disk: &mut impl Persistor<Proposal>
    ) {
        let reject = self.term > cand_term && {println!("RAFT :: reject vote because current term is larger"); true};
        let reject = reject || (
            !self.vote.is_none() && 
            self.vote != Some(cand_id) && {println!("RAFT :: reject vote, already voted for {:?}", self.vote.unwrap()); true});
        let reject = reject || (
            disk.last() > (last_term, last_index)
            && {println!("RAFT :: reject vote, log not up-to-date"); true});
        let msg = if reject {
            RaftPaperMsg::VoteRej { term: self.term }
        } else {
            self.vote = Some(cand_id);
            RaftPaperMsg::VoteAck { term: cand_term }
        };
        disk.persist(self.term, self.vote);
        println!("RAFT :: vote {:?} -> {cand_id:?} :: {msg:?}", self.id);
        adaptor.send(cand_id, msg);
    }
    // handle vote acknowledge
    // - vote is valid if and only if:
    //   - current the server is still a candidate
    //   - the vote is actually a vote for current term
    // - become a leader if vote count exceeds majority
    pub fn handle_vote_ack(&mut self, term: Term, disk: &mut impl Persistor<Proposal>) {
        // if current server is not a candidate, do nothing
        let PaperRole::Candidate { count } = &self.role else { return };
        // if vote is for previous terms, do nothing
        if term < self.term { return };
        let count = *count + 1;
        self.role = if 2 * count < self.peers.len() {
            println!("RAFT :: {:?} vote count {count:?} ", self.id);
            // update vote count only
            PaperRole::Candidate { count }
        } else {
            println!("RAFT :: {:?} become leader", self.id);
            // update role if enough vote is collected
            PaperRole::Leader {
                matched: HashMap::from_iter(self.peers.iter().map(|x| (*x, 0))),
                guessed: HashMap::from_iter(self.peers.iter().map(|x| (*x, disk.last().1)))
            }
        }
    }
    // handle vote rejection
    // - the candidate will use the term to update itself
    pub fn handle_vote_rej(&mut self, term: Term, disk: &mut impl Persistor<Proposal>) {
        if term <= self.term { return }
        self.term = term;
        self.role = PaperRole::Candidate { count: 0 };
        self.vote = None;
        disk.persist(self.term, self.vote);
    }
}