use std::{collections::HashMap, marker::PhantomData};
use serde::{Deserialize, Serialize};

use crate::*;

pub struct RaftLubyImpl<Proposal> where
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
    pub(crate) role: LubyRole,
    pub(crate) commitable: usize,
    pub(crate) timeout_elect: u64,
    pub(crate) timeout_heart: u64,
    pub(crate) bound_elect: u64,
    pub(crate) bound_heart: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LubyRole {
    Leader {
        matched: HashMap<RaftId, usize>, 
        guessed: HashMap<RaftId, usize> 
    },
    Follower { leader: RaftId },
    Candidate { count: usize },
}