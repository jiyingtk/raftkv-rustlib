extern crate kv_rustlib;
extern crate grpcio;
extern crate futures;
extern crate rand;

use std::env;
use std::thread;
// use std::time::Duration;
use grpcio::{Environment, ServerBuilder};
use std::sync::Arc;

use futures::Future;
use futures::sync::oneshot;
use std::io::{self,Read};

use kv_rustlib::kv::server;
use kv_rustlib::kv::client;
use kv_rustlib::kv::kvservice_grpc;

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut is_client = false;
    let mut leader_port = 0;
    if args.len() > 1 {
        is_client = if args[1].parse::<u64>().unwrap() == 0 {
            true
        } else {
            false
        }
    }
    if args.len() > 2 {
        leader_port = args[2].parse::<u64>().unwrap();
    }

    let node_id = args[1].parse::<u64>().unwrap();

    let leader_addr = if leader_port != 0 {
        Some(format!("127.0.0.1:{}", leader_port))
    } else {
        None
    };
    if is_client == false {
        let kv_server = server::KVServer::new(node_id, leader_addr);
        let mut kv_server2 = kv_server.clone();
        let env = Arc::new(Environment::new(1));
        let service = kvservice_grpc::create_kv_service(kv_server);
        let mut server = ServerBuilder::new(env)
            .register_service(service)
            .bind("127.0.0.1", 0)
            .build()
            .unwrap();
        server.start();
        for &(ref host, port) in server.bind_addrs() {
            println!("listening on {}:{}", host, port);
            kv_server2.notify_leader_addr(format!("{}:{}", host, port));
        }

        let (tx, rx) = oneshot::channel();
        thread::spawn(move || {
            println!("Press ENTER to exit...");
            let _ = io::stdin().read(&mut [0]).unwrap();
            tx.send(())
        });
        let _ = rx.wait();
        let _ = server.shutdown().wait();
    } else {
        // client
        
        let mut clerk = client::Clerk::new(format!("127.0.0.1:{}", leader_port), 0);

        for i in 0..50000 {
            clerk.put(&String::from(format!("key {}",i)), &String::from(format!("value {}",i)));
            let v = clerk.get(&String::from(format!("key {}",i)));
            println!("get {}", v);
        }

        for i in (0..5000).rev() {
            let v = clerk.get(&String::from(format!("key {}",i)));
            println!("get {}", v);
        }
    }

    // thread::sleep(Duration::from_secs(6000));
}