#![feature(fnbox)]

extern crate futures;
extern crate grpcio;
extern crate grpcio_proto;
extern crate protobuf;
extern crate crossbeam_channel;
#[macro_use]
extern crate serde_derive;
extern crate bincode;
extern crate rocksdb;
extern crate raft;
extern crate rand;

pub mod mraft;
pub mod kv;
