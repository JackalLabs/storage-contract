use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{debug_print,Env, Api, Querier, ReadonlyStorage, Storage, StdResult, StdError, Extern, HandleResponse, HumanAddr};
use cosmwasm_storage::{ bucket, bucket_read, Bucket, ReadonlyBucket};


static NODE_LOCATION: &[u8] = b"NODES";

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct NodeData {
    score: f32,
    secret_address: String,
} 


pub fn save_node<'a, S: Storage>( store: &'a mut S, ipaddress: String, node_data: NodeData ) {
    let bucket_response = bucket(NODE_LOCATION, store).save(&ipaddress.as_bytes(), &node_data);
    match bucket_response {
        Ok(bucket_response) => bucket_response,
        Err(e) => panic!("Bucket Error: {}", e)
    }

}

