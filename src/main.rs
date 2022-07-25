use std::{io, string};
use std::collections::BTreeMap;
use cosmwasm_std::testing::{mock_dependencies, mock_env};
use cosmwasm_std::{coins, from_binary};

use cosmwasm_std::{
    debug_print, to_binary, Api, Env, Extern, HandleResponse, HumanAddr, Querier, StdError,
    StdResult, Storage, testing::{MockStorage, self}, coin, 
};
use cosmwasm_storage::{singleton, bucket, bucket_read};
use jackal::backend::{query_file, create_file, get_namespace_from_path, WalletInfo, return_wallet, get_namespace, file_exists};
use schemars::JsonSchema;
use secret_toolkit::storage;
use serde::{Serialize, Deserialize};

static WALLET_INFO_LOCATION: &[u8] = b"WALLET_INFO";

fn main() {
    let mut deps = mock_dependencies(20, &coins(2, "token"));
    let env = mock_env(String::from("BiPhan"), &[]); 

    let ha = deps
    .api
    .human_address(&deps.api.canonical_address(&env.message.sender).unwrap()).unwrap();
    let adr = String::from(ha.as_str());
    let mut path = adr.to_string();
    path.push('/');
    println!("The root is: {:#?}", path);

    //Register Wallet info:
    //   One of two things could happen:
    //a) They already have a wallet info saved, so we just pull it out and set init to true.
    //b) They don't have a wallet info saved, so may_load will return None, which prompts a return of a default walletinfo that can be altered and saved asap.
    let loaded_wallet: Result<Option<WalletInfo>, StdError> = bucket(WALLET_INFO_LOCATION, &mut deps.storage).may_load(adr.as_bytes());
    let unwrapped_wallet = loaded_wallet.expect("Wallet not found."); //Option will always be unwrapped, but providing error message for clarity.
    let mut returned_wallet = return_wallet(unwrapped_wallet);

    if returned_wallet.namespace == "empty".to_string() {
        returned_wallet.init = true;
        let new_namespace = format!("{}{}", adr, 0);
        returned_wallet.namespace = new_namespace;
    } else {
        returned_wallet.init = true;
    }

    let bucket_response =
        bucket(WALLET_INFO_LOCATION, &mut deps.storage).save(adr.as_bytes(), &returned_wallet);
    match bucket_response {
        Ok(bucket_response) => bucket_response,
        Err(e) => panic!("Bucket Error: {}", e),
    }
    create_file(&mut deps, adr.to_string(), &path, &String::from("root contents"));

    let Contents_list = vec![String::from("contents 1"), String::from("contents 2"), String::from("contents 3")];
    let path_list = vec![String::from("movies/"), String::from("memes/"), String::from("work/")];

    for i in 0..path_list.len() {
        let sub_folder = format!("{}{}", path, path_list[i]);
        println!("The subfolder is: {:#?}", sub_folder);
        create_file(&mut deps, adr.to_string(), &sub_folder, &Contents_list[i]);

    }
    
    //let sf1 = format!("{}", String::from(sub_folder_1));
    let full_namespace = get_namespace_from_path(&deps, &String::from("BiPhan/")).unwrap_or(String::from("namespace not found!"));

    println!("{:#?}", full_namespace);
    //when querying a file, the path String would come from the FE...so no need to borrow or clone the sub_folder_1...etc...from above


    let file_1 = query_file(&deps, String::from("BiPhan/"), &env.message.sender);
    let file_2 = query_file(&deps, String::from("BiPhan/movies/"), &env.message.sender);
    let file_3 = query_file(&deps, String::from("BiPhan/memes/"), &env.message.sender);
    let file_4 = query_file(&deps, String::from("BiPhan/work/"), &env.message.sender);

    println!("{:#?}", file_1);
    println!("{:#?}", file_2);
    println!("{:#?}", file_3);
    println!("{:#?}", file_4);
    
}

