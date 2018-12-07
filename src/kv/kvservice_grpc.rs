// This file is generated. Do not edit
// @generated

// https://github.com/Manishearth/rust-clippy/issues/702
#![allow(unknown_lints)]
#![allow(clippy)]

#![cfg_attr(rustfmt, rustfmt_skip)]

#![allow(box_pointers)]
#![allow(dead_code)]
#![allow(missing_docs)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(trivial_casts)]
#![allow(unsafe_code)]
#![allow(unused_imports)]
#![allow(unused_results)]

const METHOD_KV_SERVICE_OPERATE: ::grpcio::Method<super::kvservice::Request, super::kvservice::Reply> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/kv.KVService/Operate",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_KV_SERVICE_CONF_CHANGE: ::grpcio::Method<super::kvservice::ConfChgReq, super::kvservice::Reply> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/kv.KVService/ConfChange",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

#[derive(Clone)]
pub struct KvServiceClient {
    client: ::grpcio::Client,
}

impl KvServiceClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        KvServiceClient {
            client: ::grpcio::Client::new(channel),
        }
    }

    pub fn operate_opt(&self, req: &super::kvservice::Request, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::kvservice::Reply> {
        self.client.unary_call(&METHOD_KV_SERVICE_OPERATE, req, opt)
    }

    pub fn operate(&self, req: &super::kvservice::Request) -> ::grpcio::Result<super::kvservice::Reply> {
        self.operate_opt(req, ::grpcio::CallOption::default())
    }

    pub fn operate_async_opt(&self, req: &super::kvservice::Request, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::kvservice::Reply>> {
        self.client.unary_call_async(&METHOD_KV_SERVICE_OPERATE, req, opt)
    }

    pub fn operate_async(&self, req: &super::kvservice::Request) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::kvservice::Reply>> {
        self.operate_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn conf_change_opt(&self, req: &super::kvservice::ConfChgReq, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::kvservice::Reply> {
        self.client.unary_call(&METHOD_KV_SERVICE_CONF_CHANGE, req, opt)
    }

    pub fn conf_change(&self, req: &super::kvservice::ConfChgReq) -> ::grpcio::Result<super::kvservice::Reply> {
        self.conf_change_opt(req, ::grpcio::CallOption::default())
    }

    pub fn conf_change_async_opt(&self, req: &super::kvservice::ConfChgReq, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::kvservice::Reply>> {
        self.client.unary_call_async(&METHOD_KV_SERVICE_CONF_CHANGE, req, opt)
    }

    pub fn conf_change_async(&self, req: &super::kvservice::ConfChgReq) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::kvservice::Reply>> {
        self.conf_change_async_opt(req, ::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F) where F: ::futures::Future<Item = (), Error = ()> + Send + 'static {
        self.client.spawn(f)
    }
}

pub trait KvService {
    fn operate(&mut self, ctx: ::grpcio::RpcContext, req: super::kvservice::Request, sink: ::grpcio::UnarySink<super::kvservice::Reply>);
    fn conf_change(&mut self, ctx: ::grpcio::RpcContext, req: super::kvservice::ConfChgReq, sink: ::grpcio::UnarySink<super::kvservice::Reply>);
}

pub fn create_kv_service<S: KvService + Send + Clone + 'static>(s: S) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_KV_SERVICE_OPERATE, move |ctx, req, resp| {
        instance.operate(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_KV_SERVICE_CONF_CHANGE, move |ctx, req, resp| {
        instance.conf_change(ctx, req, resp)
    });
    builder.build()
}

const METHOD_RAFT_SERVICE_SEND_MSG: ::grpcio::Method<super::kvservice::MessageWrap, super::kvservice::Reply> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/kv.RaftService/sendMsg",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

#[derive(Clone)]
pub struct RaftServiceClient {
    client: ::grpcio::Client,
}

impl RaftServiceClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        RaftServiceClient {
            client: ::grpcio::Client::new(channel),
        }
    }

    pub fn send_msg_opt(&self, req: &super::kvservice::MessageWrap, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::kvservice::Reply> {
        self.client.unary_call(&METHOD_RAFT_SERVICE_SEND_MSG, req, opt)
    }

    pub fn send_msg(&self, req: &super::kvservice::MessageWrap) -> ::grpcio::Result<super::kvservice::Reply> {
        self.send_msg_opt(req, ::grpcio::CallOption::default())
    }

    pub fn send_msg_async_opt(&self, req: &super::kvservice::MessageWrap, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::kvservice::Reply>> {
        self.client.unary_call_async(&METHOD_RAFT_SERVICE_SEND_MSG, req, opt)
    }

    pub fn send_msg_async(&self, req: &super::kvservice::MessageWrap) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::kvservice::Reply>> {
        self.send_msg_async_opt(req, ::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F) where F: ::futures::Future<Item = (), Error = ()> + Send + 'static {
        self.client.spawn(f)
    }
}

pub trait RaftService {
    fn send_msg(&mut self, ctx: ::grpcio::RpcContext, req: super::kvservice::MessageWrap, sink: ::grpcio::UnarySink<super::kvservice::Reply>);
}

pub fn create_raft_service<S: RaftService + Send + Clone + 'static>(s: S) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_RAFT_SERVICE_SEND_MSG, move |ctx, req, resp| {
        instance.send_msg(ctx, req, resp)
    });
    builder.build()
}
