# Raft Implementation

If I receive a message, the sender informs me that it has a larger term: 

In the following table, use `L=Leader`, `C=Candidate`, `F=Follower`. 

| Sender | Receiver  | Action |
| -----  | --------  | ------ |
| L      | L         | Step down to follower |
| L      | C         | Step down to follower |
| L      | F         | Follow leader; Update term |
| C      | L         | Vote for Candidate;  |
| C      | C         | Vote for Candidate; |
| C      | F         |  |
| F      | L         | Step down to follower |
| F      | C         | Step down to follower |
| F      | F         | Step down to follower |