use std::thread;
use std::sync::Arc;
use std::time::Duration;
use std::collections::HashMap;
use grpcio::{ChannelBuilder, EnvBuilder};

use super::common::*;
use super::kvservice::Request;
use super::kvservice_grpc::KvServiceClient;

pub struct Clerk {
    kv_addrs: HashMap<u64, String>,
    client: KvServiceClient,
    client_id: u64,
    request_seq: u64,
    leader_id: u64,
}

impl Clerk {
    pub fn new(server_addr: String, client_id: u64) -> Clerk {
        let env = Arc::new(EnvBuilder::new().build());
        let ch = ChannelBuilder::new(env).connect(&server_addr);
        let client = KvServiceClient::new(ch);

        Clerk {
            client,
            client_id,
            kv_addrs: HashMap::new(),
            request_seq: 0,
            leader_id: 0,
        }
    }

    fn reset_client(&mut self, addr: &str) {
        let env = Arc::new(EnvBuilder::new().build());
        let ch = ChannelBuilder::new(env).connect(addr);
        self.client = KvServiceClient::new(ch);
    }
    
    fn parse_request(req : ReqArgs) -> Request {
        let mut r = Request::new();
        r.set_request_type(req.request_type);
        r.set_request_seq(req.request_seq);
        r.set_client_id(req.client_id);
        r.set_key(req.key);
        r.set_value(req.value);
        r.set_op(req.op);
        r
    }

    pub fn get(&mut self, key: &String) -> String {
        let args = ReqArgs{
            request_type: 0,
            request_seq: self.request_seq,
            client_id: self.client_id,
            key: key.clone(),
            value: String::new(),
            op: String::from("Get"),
        };
        self.request_seq += 1;
        let req = Self::parse_request(args);
        

        loop {
           // println!("--------send get rpc to {}", self.leader_id);
            if let Ok(reply) = self.client.operate(&req) {
                self.kv_addrs = reply.addrs;
                // println!("recv kv_addrs {:?}", &self.kv_addrs);
                if reply.ok {
                    return reply.value;
                }
            } else {
                // println!("error in client rpc call");
            }

            self.leader_id = (self.leader_id + 1) % (self.kv_addrs.len() as u64);

            let mut count = 0;
            let kv_addrs = self.kv_addrs.clone();
            for (_, v) in &kv_addrs {
                if count == self.leader_id {
                    self.reset_client(v);
                    break;
                }
                count += 1;       
            }

            thread::sleep(Duration::from_millis(100));
        }
    }

    pub fn put(&mut self, key: &String, value: &String) {
        let op = String::from("Put");
        self.put_append(key, value, &op);
    }

    pub fn append(&mut self, key: &String, value: &String) {
        let op = String::from("Append");
        self.put_append(key, value, &op);
    }

    fn put_append(&mut self, key: &String, value: &String, op: &String) {
        let args = ReqArgs {
            request_type: 1,
            request_seq: self.request_seq,
            client_id: self.client_id,
            key: key.clone(),
            value: value.clone(),
            op: op.clone(),
        };
        self.request_seq += 1;
        let req = Self::parse_request(args);
        loop {
           // println!("--------send put rpc to {}", self.leader_id);
            if let Ok(reply) = self.client.operate(&req) {
                if reply.addrs.len() != 0 {
                    self.kv_addrs = reply.addrs;
                }
                    
                // println!("recv kv_addrs {:?}", &self.kv_addrs);
                if reply.ok {
                    return;
                }
            } else {
                // println!("error in client rpc call");
            }

            self.leader_id = (self.leader_id + 1) % (self.kv_addrs.len() as u64);

            let mut count = 0;
            let kv_addrs = self.kv_addrs.clone();
            for (_, v) in &kv_addrs {
                if count == self.leader_id {
                    self.reset_client(v);
                    break;
                }
                count += 1;       
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
}
