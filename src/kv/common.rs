#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum RespErr {
    OK,
    ErrWrongLeader,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct ReqArgs {
    pub request_type: u32,
    pub client_id: u64,
    pub request_seq: u64,
    pub key: String,
    pub value: String,
    pub op: String,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct PutAppendReply {
    pub err: RespErr,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct GetReply {
    pub err: RespErr,
    pub value: String,
}
