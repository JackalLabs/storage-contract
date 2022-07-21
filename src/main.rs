pub mod contract;
pub mod msg;
pub mod state;
pub mod backend;
pub mod ordered_set;
pub mod nodes;
pub mod more_tests;
mod viewing_key;
mod utils;
mod messaging;

//use super::cosmwasm_std::testing::{mock_dependencies, mock_env};

use cosmwasm_std::{
    debug_print, to_binary, Api, Env, Extern, HandleResponse, HumanAddr, Querier, StdError,
    StdResult, Storage, coins
};
use cosmwasm_storage::{bucket, bucket_read};
use jackal::backend::{WalletInfo, bucket_save_file, bucket_load_file, query_file, try_forget_me, create_file, try_init};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::messaging::{ Message, create_empty_collection, append_message, collection_exist, send_message };
use crate::msg::{FileResponse, HandleAnswer, WalletInfoResponse };
use crate::nodes::write_claim;
use crate::ordered_set::OrderedSet;
use crate::state::{load, write_viewing_key, State, CONFIG_KEY};
use crate::viewing_key::ViewingKey;

// Bucket namespace list:
static WALLET_INFO_LOCATION: &[u8] = b"WALLET_INFO";


fn main() {

    use cosmwasm_std::{
        Api, Extern, Querier, Storage,
    };

    use crate::msg::{HandleMsg, InitMsg, QueryMsg};

    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary, HumanAddr};

    use crate::backend::File;
    use crate::contract::{init, handle, query};
    use crate::messaging::Message;
    use crate::msg::{FileResponse, HandleAnswer, MessageResponse};
    use crate::viewing_key::ViewingKey;

    //use super::*;

    let mut deps = mock_dependencies(20, &coins(2, "token"));
    let env = mock_env(String::from("BiPhan"), &[]); 
    let borrowed_sender = &env.message.sender.to_string(); 

    try_init(&mut deps, env, String::from("Rainbows"), String::from("yeet"));

    create_file(&mut deps, String::from("BiPhan"), &String::from("BiPhan/memes/"),&String::from("Rainbows"));


    let file = query_file(&deps, String::from("BiPhan/memes/"), &HumanAddr(String::from("BiPhan")));

    println!("The file is {:#?}", file);

    


}

