use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{debug_print,Env, Api, Querier, ReadonlyStorage, Storage, StdResult, StdError, Extern, HandleResponse, HumanAddr};
use cosmwasm_storage::{ bucket, bucket_read, Bucket, ReadonlyBucket};


static NODE_LOCATION: &[u8] = b"NODES";
static NODE_LOC_LOCATION: &[u8] = b"NODE_LOC";
static NODE_MAP_DATA: &[u8] = b"NODE_MAP";

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct NodeData {
    score: u32,
    secret_address: String,
} 

pub fn get_node<'a, S: Storage>(store: &'a S, index: u64) -> String {
    let size = get_node_size(store);

    if index >= size {
        return String::from("null");
    }

    load_node_loc(store, index.to_string())
}

pub fn push_node<'a, S: Storage>(store: &'a mut S, ip: String, address: String) {

    let size = get_node_size(store);


    save_node_loc(store, size.to_string(), ip.clone());

    let node = NodeData {
        score: 500,
        secret_address: address
    };

    save_node_data(store, ip, node);

    let size = size + 1;

    set_node_size(store, size);

}

pub fn set_node_size<'a, S: Storage>( store: &'a mut S, size: u64 ) {
    let bucket_response = bucket(NODE_MAP_DATA, store).save(&"list_size".as_bytes(), &size);
    match bucket_response {
        Ok(bucket_response) => bucket_response,
        Err(e) => panic!("Bucket Error: {}", e)
    }
}

pub fn get_node_size<'a, S: Storage>( store: &'a S) -> u64{
    bucket_read(NODE_MAP_DATA, store).load(&"list_size".as_bytes()).unwrap()
}

pub fn save_node_loc<'a, S: Storage>( store: &'a mut S, loc: String, ipaddress: String ) {
    let bucket_response = bucket(NODE_LOC_LOCATION, store).save(&loc.as_bytes(), &ipaddress);
    match bucket_response {
        Ok(bucket_response) => bucket_response,
        Err(e) => panic!("Bucket Error: {}", e)
    }
}

pub fn load_node_loc<'a, S: Storage>( store: &'a S, loc: String) -> String{
    bucket_read(NODE_LOC_LOCATION, store).load(&loc.as_bytes()).unwrap()
}


pub fn save_node_data<'a, S: Storage>( store: &'a mut S, ipaddress: String, node_data: NodeData ) {



    let bucket_response = bucket(NODE_LOCATION, store).save(&ipaddress.as_bytes(), &node_data);


    match bucket_response {
        Ok(bucket_response) => bucket_response,
        Err(e) => panic!("Bucket Error: {}", e)
    }


}

pub fn load_node_data<'a, S: Storage>( store: &'a S, ipaddress: String) -> NodeData{
    bucket_read(NODE_LOCATION, store).load(&ipaddress.as_bytes()).unwrap()
}

