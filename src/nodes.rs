use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_storage::{ bucket, bucket_read};
use cosmwasm_std::{to_binary, Api, Querier, Storage, StdResult, StdError, Extern, HandleResponse};


static NODE_LOCATION: &[u8] = b"NODES";
static NODE_LOC_LOCATION: &[u8] = b"NODE_LOC";
static NODE_MAP_DATA: &[u8] = b"NODE_MAP";

static NODE_CLAIM_CODES: &[u8] = b"CLAIM_CODES";

static COIN_COUNT: &[u8] = b"TOKEN_COUNT";

pub fn pub_query_coins<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    address: String,
) -> StdResult<HandleResponse> {

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&query_coins(&deps.storage, address))?),
    })
}

fn query_coins<'a, S: Storage>(store: &'a S, address: String) -> u32{
    let r = bucket_read(COIN_COUNT, store).load(&address.as_bytes());

    match r {
        Ok(c) => {
            c
        },
        Err(_e) => {
            0
        }
    }
}

pub fn claim<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    claim_path: String, 
    claim_code: String, 
    address: String
)-> StdResult<HandleResponse> {

    
    let resp:String = bucket_read(NODE_CLAIM_CODES, &deps.storage).load(&claim_path.as_bytes()).unwrap();
    

    let count_resp:Result<u32, StdError> = bucket_read(COIN_COUNT, &deps.storage).load(&address.as_bytes());

    let mut old_count:u32 = 0;
    match count_resp {
        Ok(count) => {

            old_count = count;
        },
        Err(_e) => {}
    }

    old_count += 1;

    if claim_code.eq(&resp)  {

        let _bucket_response = bucket(COIN_COUNT, &mut deps.storage).save(&address.as_bytes(), &old_count);

        bucket::<S, String>(NODE_CLAIM_CODES, &mut deps.storage).remove(&claim_path.as_bytes());

        
    }

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary("OK")?),
    })

}

pub fn write_claim<'a, S: Storage>(store: &'a mut S, claim_path: String, claim_code: String) {

    let bucket_response = bucket(NODE_CLAIM_CODES, store).save(&claim_code.as_bytes(), &claim_path);
    match bucket_response {
        Ok(bucket_response) => bucket_response,
        Err(e) => panic!("Bucket Error: {}", e)
    }
}







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

