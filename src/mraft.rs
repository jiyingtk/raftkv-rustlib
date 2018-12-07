// Copyright 2018 PingCAP, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// See the License for the specific language governing permissions and
// limitations under the License.

use std::io;
use std::io::prelude::*;
use std::thread;
use std::boxed::FnBox;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::Future;
use futures::sync::oneshot;

use crossbeam_channel::{self, Sender, Receiver, RecvTimeoutError};

use protobuf::Message;
use grpcio::{Environment, RpcContext, ServerBuilder, UnarySink};
use grpcio::{ChannelBuilder, EnvBuilder};

use raft::prelude::*;
use raft::storage::MemStorage;
use raft::eraftpb::{Message as RaftMessage, ConfChange, ConfChangeType};

use rocksdb::{DB, Writable};

use super::kv::kvservice::{MessageWrap, Reply};
use super::kv::kvservice_grpc::{self, RaftService, RaftServiceClient};



use super::kv::kvservice::Request;
type ProposeCallback = Box<FnBox(Response) + Send>;


#[derive(Serialize, Deserialize, Default)]
pub struct Response {
    pub id: u64,
    pub ok: bool,
    pub op: u32,
    pub value: String,
    pub addrs: HashMap<u64, String>,
}

pub enum Msg {
    Propose {
        request: Request,
        cb: ProposeCallback,
    },
    Conf {
        conf: ConfChange,
        node_addr: String,
        node_kv_addr: String,
        cb: ProposeCallback,
    },
    // Here we don't use Raft Message, so use dead_code to
    // avoid the compiler warning.
    Raft{
        from_leader: bool,
        leader_id: u64,
        leader_kv_addr: String,
        addrs: HashMap<u64, String>,
        kv_addrs: HashMap<u64, String>,
        msg: RaftMessage,
    },
    Addrs {
        addrs : HashMap<u64, String>,
        node_id : u64,
    },
}

#[derive(Clone)]
struct RaftServiceImpl {
    sender : Sender<Msg>,
}

impl RaftService for RaftServiceImpl {
    fn send_msg(&mut self, ctx: RpcContext, requ: MessageWrap, sink: UnarySink<Reply>) {
        let mut req = requ;
        self.sender.send(Msg::Raft{
            msg: req.take_msg(),
            from_leader: req.from_leader,
            leader_id: req.leader_id,
            leader_kv_addr: req.leader_kv_addr,
            addrs: req.addrs,
            kv_addrs: req.kv_addrs,
        }).unwrap();

        let mut reply = Reply::new();
        reply.set_ok(true);

        let f = sink
            .success(reply)
            .map_err(move |e| println!("failed to reply: {:?}", e));
        ctx.spawn(f)
    }

}

pub struct RNode {
    id : u64,
    first_leader : bool,
    r : RawNode<MemStorage>,
    db : DB,
    kv_addr : String,
    leader_kv_addr : String,
    kv_addrs : HashMap<u64, String>,
    addrs : HashMap<u64, String>,
}

impl RNode {
        // A simple example about how to use the Raft library in Rust.
    pub fn new(id : u64, sender : Sender<Msg>) -> RNode {
        // Create a storage for Raft, and here we just use a simple memory storage.
        // You need to build your own persistent storage in your production.
        // Please check the Storage trait in src/storage.rs to see how to implement one.
        let storage = MemStorage::new();

        // Create the configuration for the Raft node.
        let cfg = Config {
            // The unique ID for the Raft node.
            id,
            // The Raft node list.
            // Mostly, the peers need to be saved in the storage
            // and we can get them from the Storage::initial_state function, so here
            // you need to set it empty.
            peers: vec![id],
            // Election tick is for how long the follower may campaign again after
            // it doesn't receive any message from the leader.
            election_tick: 10,
            // Heartbeat tick is for how long the leader needs to send
            // a heartbeat to keep alive.
            heartbeat_tick: 3,
            // The max size limits the max size of each appended message. Mostly, 1 MB is enough.
            max_size_per_msg: 1 * 1024 * 1024,
            // Max inflight msgs that the leader sends messages to follower without
            // receiving ACKs.
            max_inflight_msgs: 256,
            // The Raft applied index.
            // You need to save your applied index when you apply the committed Raft logs.
            applied: 0,
            // Just for log
            tag: format!("[{}]", 1),
            ..Default::default()
        };



        let (sch, rch) = crossbeam_channel::unbounded();

        thread::spawn(move || {
            let raft_server = RaftServiceImpl {
                sender : sender,
            };
            let env = Arc::new(Environment::new(1));
            let service = kvservice_grpc::create_raft_service(raft_server);
            let mut server = ServerBuilder::new(env)
                .register_service(service)
                .bind("127.0.0.1", 0)
                .build()
                .unwrap();
            server.start();

            let mut addr = String::from("127.0.0.1:0");
            for &(ref host, port) in server.bind_addrs() {
                addr = format!("{}:{}", host, port);
            }

            sch.send(addr).unwrap();

            let (tx, rx) = oneshot::channel();
            thread::spawn(move || {
                // println!("Press ENTER to exit...");
                let _ = io::stdin().read(&mut [0]).unwrap();
                tx.send(())
            });
            let _ = rx.wait();
            let _ = server.shutdown().wait();
        });

        let addr = rch.recv().unwrap();
        // println!("raft bind addr {:?}", &addr);
        let mut addrs = HashMap::new();
        addrs.insert(id, addr);

        let r = RawNode::new(&cfg, storage, vec![]).unwrap();
        let db = DB::open_default(&format!("rocksdb{}", id)).unwrap();

        let r_node = RNode {
            id,
            first_leader: false,
            r,
            db,
            addrs,
            kv_addrs: HashMap::new(),
            kv_addr: String::new(),
            leader_kv_addr: String::new(),
        };

        r_node
    }

    pub fn get_addr(&self) -> String {
        self.addrs.get(&self.id).unwrap().to_string()
    }

    fn send_msg(&mut self, rmsg: RaftMessage) {
        let id_to = rmsg.get_to();
        // println!("id {} send msg to id {:?}", self.r.raft.id, id_to);
        let mut addr_to = String::new();
        let is_exists = match self.addrs.get(&id_to) {
            Some(addr) => {
                addr_to = addr.to_string();
                true
            },
            None => false,
        };
        if is_exists {
            let env = Arc::new(EnvBuilder::new().build());
            let ch = ChannelBuilder::new(env).connect(&addr_to);
            let client = RaftServiceClient::new(ch);

            let is_leader = self.r.raft.leader_id == self.r.raft.id;
            let leader_kv_addr = if is_leader {
                self.kv_addr.clone()
            } else {
                String::new()
            };
            let mut mr = MessageWrap::new();
            mr.set_msg(rmsg);
            mr.set_from_leader(is_leader);
            mr.set_leader_id(self.r.raft.leader_id);
            mr.set_leader_kv_addr(leader_kv_addr);
            mr.set_addrs(self.addrs.clone());
            mr.set_kv_addrs(self.kv_addrs.clone());

            if let Ok(_) = client.send_msg(&mr) {
                
            }
            else {

                let mut conf_change_req = ConfChange::new();
                conf_change_req.set_id(id_to + 1);
                conf_change_req.set_node_id(id_to);
                conf_change_req.set_change_type(ConfChangeType::RemoveNode);
                
                self.addrs.remove(&id_to);
                self.kv_addrs.remove(&id_to);
                let is_leader = self.r.raft.leader_id == self.r.raft.id;

                if is_leader {
                    self.r.propose_conf_change(vec![], conf_change_req.clone()).unwrap();
                }
                
                // println!("id {} connect to id {:?} error, remove it", self.r.raft.id, id_to);
                self.r.apply_conf_change(&conf_change_req);
            }
        }
        else {
            // println!("send not exists id {:?}", id_to);
            let mut conf_change_req = ConfChange::new();
            conf_change_req.set_id(id_to + 1);
            conf_change_req.set_node_id(id_to);
            conf_change_req.set_change_type(ConfChangeType::RemoveNode);
            
            self.addrs.remove(&id_to);
            self.kv_addrs.remove(&id_to);
            let is_leader = self.r.raft.leader_id == self.r.raft.id;

            if is_leader {
                self.r.propose_conf_change(vec![], conf_change_req.clone()).unwrap();
            }
            
            // println!("id {} connect to id {:?} error, remove it", self.r.raft.id, id_to);
            self.r.apply_conf_change(&conf_change_req);
        }
    }

    fn on_ready(&mut self, cbs: &mut HashMap<u64, ProposeCallback>) {
        if !self.r.has_ready() {
            return;
        }

        // The Raft is ready, we can do something now.
        let mut ready = self.r.ready();

        let is_leader = self.r.raft.leader_id == self.r.raft.id;
        if is_leader {
            let first_leader = self.first_leader;
            if first_leader == false {
                println!("id {:?} is leader", self.r.raft.id);
                self.first_leader = true;
            }
            // If the peer is leader, the leader can send messages to other followers ASAP.
            let msgs = ready.messages.drain(..);
            for msg in msgs {
                self.send_msg(msg);
                // Here we only have one peer, so can ignore this.
            }
        }

        if !raft::is_empty_snap(ready.snapshot()) {
            // This is a snapshot, we need to apply the snapshot at first.
            self.r.mut_store()
                .wl()
                .apply_snapshot(ready.snapshot().clone())
                .unwrap();
        }

        if !ready.entries().is_empty() {
            // Append entries to the Raft log
            self.r.mut_store().wl().append(ready.entries()).unwrap();
        }

        if let Some(hs) = ready.hs() {
            // Raft HardState changed, and we need to persist it.
            self.r.mut_store().wl().set_hardstate(hs.clone());
        }

        if !is_leader {
            // If not leader, the follower needs to reply the messages to
            // the leader after appending Raft entries.
            let msgs = ready.messages.drain(..);
            for msg in msgs {
                self.send_msg(msg);
                // Send messages to other peers.
            }
        }

        if let Some(committed_entries) = ready.committed_entries.take() {
            let mut _last_apply_index = 0;
            for entry in committed_entries {
                // Mostly, you need to save the last apply index to resume applying
                // after restart. Here we just ignore this because we use a Memory storage.
                _last_apply_index = entry.get_index();

                if entry.get_data().is_empty() {
                    // Emtpy entry, when the peer becomes Leader it will send an empty entry.
                    continue;
                }

                if entry.get_entry_type() == EntryType::EntryNormal {
                    let mut request = Request::new();
                    request.merge_from_bytes(entry.get_data()).unwrap();
                    // println!("apply request: {:?}", &request);
                    let resp = self.do_request(&request);
                    if let Some(cb) = cbs.remove(&request.get_request_seq()) {
                        cb(resp);
                    }
                } 
                else if entry.get_entry_type() == EntryType::EntryConfChange {
                    let mut cc = ConfChange::new();
                    cc.merge_from_bytes(entry.get_data()).unwrap();
                    // println!("recv EntryConfChange, type: {:?}", cc.get_change_type());
                    self.r.apply_conf_change(&cc);
                }

                // TODO: handle EntryConfChange
            }
        }

        // Advance the Raft
        self.r.advance(ready);
    }

    fn do_request(&mut self, request : &Request) -> Response {
        match request.request_type {
            0 => {  //Get
                match self.db.get(request.get_key().as_bytes()) {
                    Ok(Some(value)) => {
                        return Response {
                            id : 0,
                            ok : true,
                            op : 0,
                            value : value.to_utf8().unwrap().to_string(),
                            addrs : self.kv_addrs.clone(),
                            ..Default::default()
                        };
                    },
                    Ok(None) => {
                        return Response {
                            id : 0,
                            ok : true,
                            op : 0,
                            value : String::from(""),
                            addrs : self.kv_addrs.clone(),
                            ..Default::default()
                        };
                    },
                    Err(_e) => {
                        return Response {
                            id : 0,
                            ok : false,
                            op : 0,
                            value : String::from(""),
                            addrs : self.kv_addrs.clone(),
                            ..Default::default()
                        };
                    }
                }
            },
            1 => {
                self.db.put(request.get_key().as_bytes(), request.get_value().as_bytes()).unwrap();
                return Response {
                    ok : true,
                    op : 1,
                    ..Default::default()
                }
            },
            _ => {
                println!("unknown request_type: {:?}", request.request_type);
                return Response {
                    ..Default::default()
                };
            },
        }
    }
}

pub fn run_raft(r_node : &mut RNode, receiver : Receiver<Msg>) {
    // Loop forever to drive the Raft.
    let mut t = Instant::now();
    let mut timeout = Duration::from_millis(100);

    // Use a HashMap to hold the `propose` callbacks.
    let mut cbs = HashMap::new();

    loop {
        match receiver.recv_timeout(timeout) {
            Ok(Msg::Propose { request, cb }) => {
                if r_node.r.raft.leader_id == r_node.r.raft.id { //Leader
                    let id = request.get_request_seq();
                    cbs.insert(id, cb);
                    r_node.r.propose(vec![], request.write_to_bytes().unwrap()).unwrap();
                }
                else {
                    // println!("not leader");
                    cb(Response {
                        addrs : r_node.kv_addrs.clone(),
                        ..Default::default()
                    });
                }
            },
            Ok(Msg::Conf{conf, node_addr, node_kv_addr, cb}) => {
                if r_node.r.raft.leader_id == r_node.r.raft.id { //Leader
                    if conf.get_change_type() != ConfChangeType::RemoveNode {
                        r_node.addrs.insert(conf.get_node_id(), node_addr);
                        r_node.kv_addrs.insert(conf.get_node_id(), node_kv_addr);
                    }

                    r_node.r.propose_conf_change(vec![], conf).unwrap();
                    cb(Response {
                        ok : true,
                        addrs : r_node.addrs.clone(),
                        ..Default::default()
                    });
                }
                else {
                    cb(Response {   //response leader addr
                        ok : false,
                        value : r_node.leader_kv_addr.clone(),
                        ..Default::default()
                    });
                }
            },
            Ok(Msg::Addrs{addrs, node_id}) => {
                if node_id != 0 {
                    r_node.kv_addr = addrs.get(&node_id).unwrap().to_string();
                    r_node.kv_addrs.insert(node_id, r_node.kv_addr.clone());
                    // println!("recv kv-addr {:?}", &r_node.kv_addr);
                } else {
                    // println!("recv addrs {:?}", addrs);
                    for (key, value) in &addrs {
                        r_node.addrs.insert(*key, value.to_string());
                    }
                }
                continue;
            },
            Ok(Msg::Raft{msg, from_leader, leader_id:_, leader_kv_addr, kv_addrs, addrs}) => {
                if r_node.r.raft.leader_id != r_node.r.raft.id && from_leader {
                    r_node.leader_kv_addr = leader_kv_addr;
                    // for (key, value) in &addrs {
                    //     r_node.addrs.insert(*key, value.to_string());
                    // }
                    r_node.addrs = addrs;
                    r_node.kv_addrs = kv_addrs;
                    // println!("recv leader_kv_addr from leader, {:?}, kv_addrs:{:?}", &r_node.leader_kv_addr, &r_node.kv_addrs);
                }
                r_node.r.step(msg).unwrap();
            },
            Err(RecvTimeoutError::Timeout) => (),
            Err(RecvTimeoutError::Disconnected) => return,
        }

        let d = t.elapsed();
        if d >= timeout {
            t = Instant::now();
            timeout = Duration::from_millis(100);
            // We drive Raft every 100ms.
            r_node.r.tick();
        } else {
            timeout -= d;
        }

        r_node.on_ready(&mut cbs);
    }
}


pub fn send_propose(sender: Sender<Msg>) {
    thread::spawn(move || {
        // Wait some time and send the request to the Raft.
        thread::sleep(Duration::from_secs(3));

        let (s1, r1) = crossbeam_channel::unbounded();

        println!("propose a request");

        // Send a command to the Raft, wait for the Raft to apply it
        // and get the result.
        sender
            .send(Msg::Propose {
                request: Request::new(),
                cb: Box::new(move |_| {
                    s1.send(0).unwrap();
                }),
            }).unwrap();

        let n = r1.recv().unwrap();
        assert_eq!(n, 0);

        println!("receive the propose callback");
    });
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raft() {
        // let server_num = 3;
        // let mut base_port = 7810;
        // let mut addrs = Vec::new();
        // for i in (0..server_num+1) {
        //     addrs.push(format!("127.0.0.1:{}", base_port));
        //     base_port += 1;
        // }

        // let (s1, r1) = crossbeam_channel::unbounded();
        // let mut rn1 = RNode::new(1, addrs.clone(), s1.clone());
        // thread::spawn(move || {
        //     run_raft(&mut rn1, r1);
        // });

        // // send_propose(s1);
        // // thread::sleep(Duration::from_secs(60));

        // let (s2, r2) = crossbeam_channel::unbounded();
        // let mut rn2 = RNode::new(2, addrs.clone(), s2.clone());
        // thread::spawn(move || {
        //     run_raft(&mut rn2, r2);
        // });

        // let (s3, r3) = crossbeam_channel::unbounded();
        // let mut rn3 = RNode::new(3, addrs.clone(), s3.clone());
        // thread::spawn(move || {
        //     run_raft(&mut rn3, r3);
        // });
        // send_propose(s1);

        // thread::sleep(Duration::from_secs(10));
    }

}