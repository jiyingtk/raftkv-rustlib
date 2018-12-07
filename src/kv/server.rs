use std::thread;
use std::sync::Arc;
use std::time::Duration;
use std::collections::HashMap;

use futures::Future;
use crossbeam_channel::{self, Sender};

use raft::eraftpb::{ConfChange, ConfChangeType};

use grpcio::{RpcContext, UnarySink};
use grpcio::{ChannelBuilder, EnvBuilder};

use super::kvservice::{ConfChgReq, Request, Reply};
use super::kvservice_grpc::{KvService, KvServiceClient};
use super::super::mraft::{self, RNode, Msg};

#[derive(Clone)]
pub struct KVServer {
    // rf: Arc<Mutex<RNode>>,
    id : u64,
    kv_addr: String,
    prop_send : Sender<Msg>,
    leader_addr: Option<String>,
    raft_addr: String,
}

impl KVServer {
    pub fn new(
        id: u64,
        leader_addr: Option<String>,
//        maxraftstate: u64,
        ) -> KVServer {

        let (s1, r1) = crossbeam_channel::unbounded();
        let mut rn1 = RNode::new(id, s1.clone());
        let raft_addr = rn1.get_addr();
        thread::spawn(move || {
            mraft::run_raft(&mut rn1, r1);
        });


        let kv = KVServer {
            id,
            kv_addr: String::new(),
            prop_send : s1,
            leader_addr,
            raft_addr,
        };

        // Self::register_callback(&kv, reply_sender, req_recv);
        // thread::spawn(move || { Self::run(kv, r); });
        kv
    }

    fn add_to_cluster(&mut self, addr: String) -> HashMap<u64, String> {
        let raft_addr = self.raft_addr.clone();
        let mut leader_addr = addr;
        loop {
            let env = Arc::new(EnvBuilder::new().build());
            let ch = ChannelBuilder::new(env).connect(&leader_addr);
            let client = KvServiceClient::new(ch);

            let mut cc = ConfChgReq::new();
            cc.set_id(self.id);
            cc.set_change_type(1);
            cc.set_node_id(self.id);
            cc.set_node_addr(raft_addr.clone());
            cc.set_node_kv_addr(self.kv_addr.clone());


            let mut reply = client.conf_change(&cc).expect("rpc");
            if reply.get_ok() == false {
                leader_addr = reply.take_value();
                // println!("retry, leader addr is {:?}", &leader_addr);
            }
            else {
                return reply.take_addrs();
            }
        }
    }

    pub fn notify_leader_addr(&mut self, addr: String) {
        let mut addrs = HashMap::new();
        self.kv_addr = addr.clone();
        addrs.insert(self.id, addr);
        self.prop_send.send(Msg::Addrs{addrs, node_id:self.id}).unwrap();

        let mut addr = String::new();
        let is_exists = match &self.leader_addr {
            Some(addrr) => {
                addr = addrr.to_string();
                true
            },
            None => false
        };
        if is_exists {
            let addrs = self.add_to_cluster(addr);
            // println!("recv addr from leader: {:?}", &addrs);
            self.prop_send.send(Msg::Addrs{addrs, node_id:0}).unwrap();
        }
    }
}

impl KvService for KVServer {
    fn operate(&mut self, ctx: RpcContext, req: Request, sink: UnarySink<Reply>) {        
        let (c, r) = crossbeam_channel::unbounded();
        self.prop_send.send(Msg::Propose {
            request:req.clone(),
            cb: Box::new(move |resp| {
                c.send(resp).unwrap();
            }),
        }).unwrap();

        
        let mut reply = Reply::new();
        if let Ok(r) = r.recv_timeout(Duration::from_secs(3)) {
            reply.set_ok(r.ok);
            reply.set_value(r.value.clone());
            reply.set_addrs(r.addrs);
        }
        else {
            reply.set_ok(false);
            reply.set_value(String::new());
        }        

        let f = sink
            .success(reply)
            .map_err(move |e| println!("failed to reply {:?}: {:?}", req, e));
        ctx.spawn(f)
    }

    fn conf_change(&mut self, ctx: RpcContext, req: ConfChgReq, sink: UnarySink<Reply>) {
        let (c, r) = crossbeam_channel::unbounded();

        let mut conf_change_req = ConfChange::new();
        conf_change_req.set_id(req.get_id());
        conf_change_req.set_node_id(req.get_node_id());
        conf_change_req.set_change_type(ConfChangeType::AddNode);

        self.prop_send.send(Msg::Conf{
            conf: conf_change_req,
            node_addr: req.get_node_addr().to_string(),
            node_kv_addr: req.get_node_kv_addr().to_string(),
            cb: Box::new(move |resp| {
                c.send(resp).unwrap();
            }),
        }).unwrap();

        
        let mut reply = Reply::new();
        if let Ok(r) = r.recv_timeout(Duration::from_secs(3)) {
            reply.set_ok(r.ok);
            reply.set_value(r.value);
            reply.set_addrs(r.addrs);
        }
        else {
            reply.set_ok(false);
            reply.set_value(String::new());
        }

        let f = sink
            .success(reply)
            .map_err(move |e| println!("failed to reply {:?}: {:?}", req, e));
        ctx.spawn(f)
    }

//     pub fn get(mu: Arc<Mutex<KVServer>>, args: &ReqArgs) -> GetReply {
//         let args = serialize(args).unwrap();
//         let (err, value) = Self::start(mu, &args);
//         GetReply{err, value}
//     }

//     pub fn put_append(mu: Arc<Mutex<KVServer>>, args: &ReqArgs) -> PutAppendReply {
//         let args = serialize(args).unwrap();
//         let (err, _) = Self::start(mu, &args);
//         PutAppendReply{err: err}
//     }

//     fn notify_if_present(&mut self, index: usize, reply: NotifyArgs) {
//         if let Some(sch) = self.notify_ch_map.get(&index) {
//             sch.send(reply).unwrap();
//         }
//         self.notify_ch_map.remove(&index);
//     }

//     fn start(mu: Arc<Mutex<KVServer>>, command: &Vec<u8>) -> (RespErr, String) {
//         let notify_ch: Receiver<NotifyArgs>;
//         let index;
//         let term;
//         {
//             let mut kv = mu.lock().unwrap();
//             let (i, t, ok) = Raft::start(kv.rf.clone(), command);
//             index = i;
//             term = t;
//             if ok == false {
//                 return (RespErr::ErrWrongLeader, String::from(""));
//             }
//             let (sh, rh) = mpsc::sync_channel(0);
//             notify_ch = rh;
//             kv.notify_ch_map.insert(index, sh);
//         }
//         let d = Duration::from_millis(START_TIMEOUT_INTERVAL);
//         match notify_ch.recv_timeout(d) {
//             Ok(result) => {
//                 if result.term != term {
//                     return (RespErr::ErrWrongLeader,  String::from(""));
//                 }
//                 return (result.err, result.value);
//             }
//             Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => {
//                 println!("---------------------start timeout---------------------");
//                 let mut kv = mu.lock().unwrap();
//                 kv.notify_ch_map.remove(&index);
//                 return (RespErr::ErrWrongLeader,  String::from(""));
//             }
//         }
//     }

//     fn apply(&mut self, msg :&ApplyMsg) {
// //        println!("---------------apply");
//         let mut result = NotifyArgs{
//             term: msg.term,
//             value: String::from(""),
//             err: RespErr::OK
//         };
//         let args: ReqArgs = deserialize(&msg.command).unwrap();
//         if args.request_type == 0 {
//             let m_data = self.data.lock().unwrap();
//             let value = m_data.get(&args.key);
//             match value {
//                 Some(v) => result.value = v.clone(),
//                 None => result.value = String::from(""),
//             }
//         } else if args.request_type == 1 {
//             let seq = self.cache.get(&args.cliend_id);
//             let mut flag = true;
//             match seq {
//                 Some(n) => {
//                     if *n >= args.request_seq {
//                         flag = false;
//                     }
//                 },
//                 None => (),
//             }
//             if flag {
//                 if args.op == "Put" {
//                     let mut m_data = self.data.lock().unwrap();
//                     m_data.insert(args.key, args.value);
//                 } else {
//                     let m_data = self.data.lock().unwrap();
//                     let value = m_data.get(&args.key);
//                     match value {
//                         Some(v) => {
//                             let mut m_data = self.data.lock().unwrap();

//                             m_data.insert(args.key, format!("{}{}", v, args.value))
//                         },
//                         None => {
//                             let mut m_data = self.data.lock().unwrap();
//                             m_data.insert(args.key, args.value)
//                         },
//                     };
//                 }
//             }
//         } else {
//             result.err = RespErr::ErrWrongLeader;
//         }
//         self.notify_if_present(msg.index, result);
//     }

//     fn run(mu: Arc<Mutex<KVServer>>, apply_ch: Receiver<ApplyMsg>) {
//         loop {
//             let msg = apply_ch.recv();
// //            println!("--------receive message");
//             match msg {
//                 Ok(m) => {
//                     let mut kv = mu.lock().unwrap();
//                     if m.valid {
//                         kv.apply(&m);
//                     }
//                 },
//                 Err(_) => continue,
//             }
//         }
//     }

//     fn register_callback(
//         kv: &Arc<Mutex<KVServer>>,
//         mut reply_sender: Vec<SyncSender<(Vec<u8>, bool)>>,
//         mut req_recv: Vec<Receiver<Vec<u8>>>
//     ) {
//         let kv1 = kv.clone();
//         let get_req = req_recv.remove(0);
//         let get_reply = reply_sender.remove(0);
//         thread::spawn(move || { //RequestVote
//             loop {
//                 let args = get_req.recv().unwrap();

//                 let req : ReqArgs = deserialize(&args[..]).unwrap();
//                 let reply = Self::get(kv1.clone(), &req);
//                 let reply = serialize(&reply).unwrap();
//                 get_reply.send((reply, true)).unwrap();
//             }
//         });

//         let kv2 = kv.clone();
//         let put_req = req_recv.remove(0);
//         let put_reply = reply_sender.remove(0);
//         thread::spawn(move || { //RequestVote
//             loop {
//                 let args = put_req.recv().unwrap();

//                 let req : ReqArgs = deserialize(&args[..]).unwrap();
//                 let reply = Self::put_append(kv2.clone(), &req);
//                 let reply = serialize(&reply).unwrap();
//                 put_reply.send((reply, true)).unwrap();
//             }
//         });
//     }
}
