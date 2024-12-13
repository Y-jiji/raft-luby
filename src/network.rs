use std::{cmp::Reverse, collections::{BinaryHeap, HashMap, VecDeque}, fmt::Debug, marker::PhantomData, sync::{Arc, Mutex}};
use crate::*;

// Network Adaptor Module
pub trait Adaptor<Msg> {
    fn send(&self, to: RaftId, msg: Msg);
    fn receive(&self) -> Option<Msg>;
}

// Mock Adaptor
pub struct MockAdaptor<Msg, Net: MockNetwork<Msg>> {
    id: RaftId,
    ph: PhantomData<Msg>,
    net: Net
}

impl<Msg, Net: MockNetwork<Msg>> MockAdaptor<Msg, Net> {
    pub fn new(id: RaftId, net: Net) -> Self {
        Self { id, ph: PhantomData, net }
    }
}

// Implementation of Simulated Adaptor
impl<Net: MockNetwork<Msg>, Msg> Adaptor<Msg> for MockAdaptor<Msg, Net> {
    fn send(&self, to: RaftId, msg: Msg) {
        self.net.send(self.id, to, msg);
    }
    fn receive(&self) -> Option<Msg> {
        self.net.receive(self.id)
    }
}

// Network Simulator Trait
pub trait MockNetwork<Msg> {
    fn receive(&self, receiver: RaftId) -> Option<Msg>;
    fn send(&self, sender: RaftId, to: RaftId, msg: Msg);
}

// Network Environment with Burst Connection
pub struct MockBrustNetwork<Msg> {
    // channel state
    state: HashMap<(RaftId, RaftId), bool>,
    // message queue for each server
    queue: HashMap<RaftId, BinaryHeap<(Reverse<usize>, Msg)>>,
    // the upper and lower bound of rate
    rate_upper: f32,
    rate_lower: f32,
    // the flip rate for upper and lower state
    flip_upper: f32,
    flip_lower: f32,
    // message delay and timestamp
    delay: usize,
    timestamp: usize,
    timedelta_probability: f32
}

impl<Msg: Ord> MockBrustNetwork<Msg> {
    pub fn new(
        peers: usize,
        rate_upper: f32, 
        rate_lower: f32, 
        flip_upper: f32, 
        flip_lower: f32,
        delay: usize
    ) -> Self {
        let queue = HashMap::from_iter(
            (0..peers).map(|i| (RaftId(i as u64), BinaryHeap::new())));
        let state = HashMap::from_iter(
            (0..peers).flat_map(|i| (0..peers)
                .filter_map(move |j| 
                    if i == j {None} 
                    else {Some(((RaftId(i as u64), RaftId(j as u64)), false))}
                )
            )
        );
        Self {
            queue, state,
            rate_lower, rate_upper,
            flip_lower, flip_upper,
            delay, timestamp: 0, timedelta_probability: 1.0
        }
    }
}

// Simulator of Network Environment
impl<Msg: Ord + Debug> MockNetwork<Msg> for Arc<Mutex<MockBrustNetwork<Msg>>> {
    fn receive(&self, receiver: RaftId) -> Option<Msg> {
        let mut lock = self.lock().unwrap();
        // if rand::random::<f32>() <= lock.timedelta_probability { lock.timestamp += 1; }
        // let early = lock.queue[&receiver].peek().map(|(Reverse(x), _)| x < &lock.timestamp);
        // if matches!(early, Some(true) | None) {
        //     println!("NETWORK :: {receiver:?} :: NO MESSAGE");
        //     // println!("NETWORK :: {:?}", lock.queue[&receiver]);
        //     None
        // } else {
            let msg = lock.queue.get_mut(&receiver).unwrap().pop().map(|(_, y)| y);
            println!("NETWORK :: {receiver:?} :: GET {msg:?}"); msg
        // }
    }
    fn send(&self, sender: RaftId, to: RaftId, msg: Msg) {
        let mut lock = self.lock().unwrap();
        println!("NETWORK :: {sender:?} -> {to:?} :: {msg:?}");
        if rand::random::<f32>() <= lock.timedelta_probability { lock.timestamp += 1; }
        let erase = 
            if lock.state[&(sender,to)] { rand::random::<f32>() < lock.rate_upper }
            else { rand::random::<f32>() <= lock.rate_lower };
        if erase {
            println!("NETWORK :: {sender:?} -> {to:?} :: ERASE MESSAGE");
        }
        let arrive = lock.timestamp;
        if !erase { lock.queue.get_mut(&to).unwrap().push((Reverse(arrive), msg.try_into().unwrap())); }
        let flip =
            if lock.state[&(sender,to)] { rand::random::<f32>() < lock.flip_upper }
            else { rand::random::<f32>() <= lock.flip_lower };
        if flip { *lock.state.get_mut(&(sender,to)).unwrap() ^= true; }
    }
}


// Network Environment with Simple Connection
pub struct MockFIFONetwork<Msg> {
    // message queue for each server
    queue: HashMap<RaftId, VecDeque<Msg>>,
}

impl<Proposal: Ord> MockFIFONetwork<Proposal> {
    pub fn new(
        peers: usize,
    ) -> Self {
        let queue = HashMap::from_iter(
            (0..peers).map(|i| (RaftId(i as u64), VecDeque::new())));
        Self { queue }
    }
}

// Simulator of Network Environment
impl<Msg: Ord + Debug> MockNetwork<Msg> for Arc<Mutex<MockFIFONetwork<Msg>>> {
    fn receive(&self, receiver: RaftId) -> Option<Msg> {
        let mut lock = self.lock().unwrap();
        let msg = lock.queue.get_mut(&receiver).unwrap().pop_front();
        println!("NETWORK :: {receiver:?} :: GET {msg:?}"); msg
    }
    fn send(&self, sender: RaftId, to: RaftId, msg: Msg) {
        let mut lock = self.lock().unwrap();
        println!("NETWORK :: {sender:?} -> {to:?} :: {msg:?}");
        lock.queue.get_mut(&to).unwrap().push_back(msg);
    }
}