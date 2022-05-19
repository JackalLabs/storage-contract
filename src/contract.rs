// use schemars::JsonSchema;
// use serde::{Deserialize, Serialize};

// use std::ptr::null;

use cosmwasm_std::{debug_print, to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier, StdResult, Storage, QueryResult, StdError};
use secret_toolkit::crypto::sha_256;
use std::cmp;

use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{ State, CONFIG_KEY, save, read_viewing_key};
use crate::backend::{try_create_viewing_key, try_allow_write, try_disallow_write, try_allow_read, try_disallow_read, query_file, try_create_file, try_init, try_remove_multi_files, try_remove_file, try_move_file, try_create_multi_files, try_reset_read, try_reset_write, try_you_up_bro, query_wallet_info, try_forget_me, try_move_multi_files, try_clone_parent_permission};
use crate::viewing_key::VIEWING_KEY_SIZE;
use crate::nodes::{pub_query_coins, claim, push_node, get_node, get_node_size, set_node_size};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {

    let ha = deps.api.human_address(&deps.api.canonical_address(&env.message.sender)?)?;

    let config = State {
        owner: ha,
        prng_seed: sha_256(base64::encode(msg.prng_seed).as_bytes()).to_vec(), 
    };

    set_node_size(&mut deps.storage, 0);

    debug_print!("Contract was initialized by {}", env.message.sender);

    save(&mut deps.storage, CONFIG_KEY, &config)?;
    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::InitAddress { contents, entropy } => try_init(deps, env, contents, entropy),
        HandleMsg::Create { contents, path , pkey, skey} => try_create_file(deps, env, contents, path, pkey, skey),
        HandleMsg::CreateMulti { contents_list, path_list , pkey_list, skey_list} => try_create_multi_files(deps, env, contents_list, path_list, pkey_list, skey_list),
        HandleMsg::Remove {  path } => try_remove_file(deps, env, path),
        HandleMsg::RemoveMulti {  path_list } => try_remove_multi_files(deps, env, path_list),
        HandleMsg::MoveMulti { old_path_list, new_path_list } => try_move_multi_files(deps, env, old_path_list, new_path_list),
        HandleMsg::Move { old_path, new_path } => try_move_file(deps, env, old_path, new_path),
        HandleMsg::CreateViewingKey { entropy, .. } => try_create_viewing_key(deps, env, entropy),
        HandleMsg::AllowRead { path, address_list } => try_allow_read(deps, env, path, address_list),
        HandleMsg::DisallowRead { path, address_list } => try_disallow_read(deps, env, path, address_list),
        HandleMsg::ResetRead { path } => try_reset_read(deps, env, path),
        HandleMsg::AllowWrite { path, address_list } => try_allow_write(deps, env, path, address_list),
        HandleMsg::DisallowWrite { path, address_list } => try_disallow_write(deps, env, path, address_list),
        HandleMsg::ResetWrite { path } => try_reset_write(deps, env, path),
        HandleMsg::CloneParentPermission { path } => try_clone_parent_permission(deps, env, path),
        HandleMsg::InitNode {ip, address} => try_init_node(deps, ip, address),
        HandleMsg::ClaimReward {path, key, address} => claim(deps, path, key, address),
        HandleMsg::ForgetMe { .. } => try_forget_me(deps, env),
    }
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::YouUpBro {address} => to_binary(&try_you_up_bro(deps, address)?),
        QueryMsg::GetNodeCoins {address} => to_binary(&pub_query_coins(deps, address)?),
        QueryMsg::GetNodeIP {index} => to_binary(&try_get_ip(deps, index)?),
        QueryMsg::GetNodeList {size} => to_binary(&try_get_top_x(deps, size)?),
        QueryMsg::GetNodeListSize {} => to_binary(&try_get_node_list_size(deps)?),
        _ => authenticated_queries(deps, msg),
    }
}

fn authenticated_queries<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> QueryResult {
    let (addresses, key) = msg.get_validation_params();

    for address in addresses {
        let canonical_addr = deps.api.canonical_address(address)?;

        let expected_key = read_viewing_key(&deps.storage, &canonical_addr);

        if expected_key.is_none() {
            // Checking the key will take significant time. We don't want to exit immediately if it isn't set
            // in a way which will allow to time the command and determine if a viewing key doesn't exist
            key.check_viewing_key(&[0u8; VIEWING_KEY_SIZE]);
        } else if key.check_viewing_key(expected_key.unwrap().as_slice()) {

            return match msg {
                QueryMsg::GetContents { path, behalf, .. } => to_binary(&query_file(deps, path, &behalf)?),
                QueryMsg::GetWalletInfo { behalf, .. } => to_binary(&query_wallet_info(deps, &behalf)?),
                _ => panic!("How did this even get to this stage. It should have been processed.")
            };
        }
    }

    Err(StdError::unauthorized())
}

fn try_init_node<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    ip: String,
    address: String,
) -> StdResult<HandleResponse> {

    push_node(&mut deps.storage, ip, address);

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary("OK")?),
    })
}

fn try_get_ip<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    index: u64,
) -> StdResult<HandleResponse> {

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&get_node(&deps.storage, index))?),
    })
}

fn try_get_top_x<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    size: u64,
) -> StdResult<HandleResponse> {

    let size = cmp::min(size, get_node_size(&deps.storage));

    let index_node = &get_node(&deps.storage, 0);

    let mut nodes = vec!(index_node.clone());

    if size <= 1  {
        return Ok(HandleResponse {
            messages: vec![],
            log: vec![],
            data: Some(to_binary(&nodes)?),
        });
    }


    let mut x = 1;
    while x < size {
        let new_node = &get_node(&deps.storage, x);
        nodes.push(new_node.clone());
        x += 1;
    } 

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&nodes)?),
    })
}

fn try_get_node_list_size<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<HandleResponse> {

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&get_node_size(&deps.storage))?),
    })
}

#[cfg(test)]
mod tests {
    // use std::vec;
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary, HumanAddr};
    
    use crate::msg::{FileResponse, HandleAnswer, WalletInfoResponse};
    use crate::viewing_key::ViewingKey;
    use crate::backend::make_file;

    fn init_for_test<S: Storage, A: Api, Q: Querier> (
        deps: &mut Extern<S, A, Q>,
        address:String,
    ) -> ViewingKey {

        // Init Contract
        let msg = InitMsg {prng_seed:String::from("lets init bro")};
        let env = mock_env("creator", &[]);
        let _res = init(deps, env, msg).unwrap();

        // Init Address and Create ViewingKey
        let env = mock_env(String::from(&address), &[]);
        let msg = HandleMsg::InitAddress { contents: String::from("{}"), entropy: String::from("Entropygoeshereboi") };
        let handle_response = handle(deps, env, msg).unwrap();
        
        match from_binary(&handle_response.data.unwrap()).unwrap() {
            HandleAnswer::CreateViewingKey { key } => {
                key
            },
            _ => panic!("Unexpected result from handle"),
        }
    }

    #[test]
    fn double_init_address_test(){
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        // Init Contract
        let msg = InitMsg {prng_seed:String::from("lets init bro")};
        let env = mock_env("creator", &[]);
        let _res = init(&mut deps, env, msg).unwrap();

        // Init Address
        let env = mock_env(String::from("anyone"), &[]);
        let msg = HandleMsg::InitAddress { contents: String::from("{}"), entropy: String::from("Entropygoeshereboi") };
        let handle_response = handle(&mut deps, env, msg).unwrap();
        let vk = match from_binary(&handle_response.data.unwrap()).unwrap() {
            HandleAnswer::CreateViewingKey { key } => {
                key
            },
            _ => panic!("Unexpected result from handle"),
        };
        println!("{:?}", &vk);

        // // Init Address Again
        let env = mock_env(String::from("anyone"), &[]);
        let msg = HandleMsg::InitAddress { contents: String::from("{}"), entropy: String::from("Entropygoeshereboi") };
        let handle_response = handle(&mut deps, env, msg);
        assert!(handle_response.is_err());
    }

    #[test]
    fn test_node_setup() {

        let mut deps = mock_dependencies(20, &coins(2, "token"));
        let _vk = init_for_test(&mut deps, String::from("anyone"));

        let query_res: Binary = query(&deps, QueryMsg::GetNodeListSize {  }).unwrap();
        let result:HandleResponse = from_binary(&query_res).unwrap();
        let size: u64 = from_binary(&result.data.unwrap()).unwrap();
        println!("{:#?}", &size);

        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::InitNode { ip: String::from("192.168.0.1"), address: String::from("secret123456789") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let query_res: Binary = query(&deps, QueryMsg::GetNodeListSize {  }).unwrap();
        let result:HandleResponse = from_binary(&query_res).unwrap();
        let size: u64 = from_binary(&result.data.unwrap()).unwrap();
        println!("{:#?}", &size);


        let s = size - 1;

        let query_res: Binary = query(&deps, QueryMsg::GetNodeIP { index: (s) }).unwrap();
        let result:HandleResponse = from_binary(&query_res).unwrap();
        let ip:String = from_binary(&result.data.unwrap()).unwrap();
        println!("{:#?}", &ip);

    }


    #[test]
    fn test_create_viewing_key() {
        let mut deps = mock_dependencies(20, &[]);

        // init
        let msg = InitMsg {prng_seed:String::from("lets init bro")};
        let env = mock_env("anyone", &[]);
        let _res = init(&mut deps, env, msg).unwrap();
        
        // create viewingkey
        let env = mock_env("anyone", &[]);
        let create_vk_msg = HandleMsg::CreateViewingKey {
            entropy: "supbro".to_string(),
            padding: None,
        };
        let handle_response = handle(&mut deps, env, create_vk_msg).unwrap();
        
        let vk = match from_binary(&handle_response.data.unwrap()).unwrap() {
            HandleAnswer::CreateViewingKey { key } => {
                key
            },
            _ => panic!("Unexpected result from handle"),
        };
        let test_key = ViewingKey("anubis_key_u25NSWPI5+wpGW7WP6eXtcBpA4RmyZ1CrJRvYFWDNQM=".to_string());
        assert_eq!(vk, test_key);
    }

    #[test]
    fn test_create_file() {
        let mut deps = mock_dependencies(20, &[]);
        let vk = init_for_test(&mut deps, String::from("anyone"));

        let vk2 = init_for_test(&mut deps, String::from("alice"));

        // Create File
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Create { contents: String::from("I'm sad"), path: String::from("anyone/test/") , pkey: String::from("test"), skey: String::from("test")};
        let _res = handle(&mut deps, env, msg).unwrap();

        // Create File
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Create { contents: String::from("I'm sad"), path: String::from("anyone/pepe.jpg") , pkey: String::from("test"), skey: String::from("test")};
        let _res = handle(&mut deps, env, msg).unwrap();
        
        
        // Get File with viewing key
        let query_res = query(&deps, QueryMsg::GetContents { path: String::from("anyone/pepe.jpg"), behalf: HumanAddr("alice".to_string()), key: vk2.to_string() });
        assert_eq!(query_res.is_err(), true);
        
        // Allow Read Alice
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::AllowRead { path: String::from("anyone/pepe.jpg"), address_list: vec!(String::from("alice"), String::from("bob")) };
        let _res = handle(&mut deps, env, msg).unwrap();
        
        // Query File
        let query_res = query(&deps, QueryMsg::GetContents { path: String::from("anyone/pepe.jpg"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() }).unwrap();
        let value: FileResponse = from_binary(&query_res).unwrap();
        println!("Before Reset --> {:#?}", value.file);

        // Reset Read
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::ResetRead { path: String::from("anyone/pepe.jpg") };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Query File
        let query_res = query(&deps, QueryMsg::GetContents { path: String::from("anyone/pepe.jpg"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() }).unwrap();
        let value: FileResponse = from_binary(&query_res).unwrap();
        println!("After Reset --> {:#?}", value.file);

        //Query File as Alice
        let query_res = query(&deps, QueryMsg::GetContents { path: String::from("anyone/pepe.jpg"), behalf: HumanAddr("alice".to_string()), key: vk2.to_string() });
        assert!(query_res.is_err());

        // let env = mock_env("alice", &[]);
        // let msg = HandleMsg::Create { contents: String::from("I'm not sad"), path: String::from("anyone/pepe.jpg") , pkey: String::from("test"), skey: String::from("test")};
        // let res = handle(&mut deps, env, msg);
        // assert_eq!(res.is_err(), true);

    }

    #[test]
    fn test_multi_file() {
        let mut deps = mock_dependencies(20, &[]);
        
        let vk = init_for_test(&mut deps, String::from("anyone"));

        // Create folder test/
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Create { contents: String::from("<content inside test/ folder>"), path: String::from("anyone/test/") , pkey: String::from("test"), skey: String::from("test")};
        let _res = handle(&mut deps, env, msg).unwrap();

        // Create File
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateMulti { contents_list: vec!(String::from("I'm sad"), String::from("I'm sad2")), path_list: vec!(String::from("anyone/test/pepe.jpg"), String::from("anyone/test/pepe2.jpg")) , pkey_list: vec!(String::from("test"), String::from("test")), skey_list: vec!(String::from("test"), String::from("test"))};
        let _res = handle(&mut deps, env, msg).unwrap();

        // Remove Multi File
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::RemoveMulti { path_list: vec!(String::from("anyone/test/pepe.jpg"), String::from("anyone/test/pepe2.jpg"))};
        let _res = handle(&mut deps, env, msg).unwrap();

        // Get File
        let query_res = query(&deps, QueryMsg::GetContents { path: String::from("anyone/test/pepe.jpg"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() });
        assert!(query_res.is_err());

        // Get WalletInfo with viewing key
        let query_res = query(&deps, QueryMsg::GetWalletInfo { behalf: HumanAddr("anyone".to_string()), key: vk.to_string() }).unwrap();
        let value:WalletInfoResponse = from_binary(&query_res).unwrap(); 
        let arr : Vec<String> = vec!["anyone/".to_string(), "anyone/test/".to_string()];
        assert_eq!(value.all_paths, arr);
    }

    #[test]
    fn test_you_up_bro() {
        let mut deps = mock_dependencies(20, &[]);
        let _vk = init_for_test(&mut deps, String::from("anyone"));

        let msg = QueryMsg::YouUpBro {address: String::from("anyone")};
        let query_res = query(&deps, msg).unwrap();
        let value:WalletInfoResponse = from_binary(&query_res).unwrap();
        assert_eq!(value.init, true);

        let msg = QueryMsg::YouUpBro {address: String::from("yeet")};
        let query_res = query(&deps, msg).unwrap();
        let value:WalletInfoResponse = from_binary(&query_res).unwrap();
        assert_eq!(value.init, false);
    }
    
    #[test]
    fn forget_me_test() {
        let mut deps = mock_dependencies(20, &[]);
        let vk = init_for_test(&mut deps, String::from("anyone"));

        // Create File
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Create { contents: String::from("content of meme/ folder "), path: String::from("anyone/meme/") , pkey: String::from("test"), skey: String::from("test")};
        let _res = handle(&mut deps, env, msg).unwrap();
        
        // Create Multi File
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateMulti { contents_list: vec!(String::from("I'm sad"), String::from("I'm sad2")), path_list: vec!(String::from("anyone/meme/pepe.jpg"), String::from("anyone/meme/pepe2.jpg")) , pkey_list: vec!(String::from("test"), String::from("test")), skey_list: vec!(String::from("test"), String::from("test"))};
        let _res = handle(&mut deps, env, msg).unwrap();
        
        // Get WalletInfo with viewing key
        let query_res = query(&deps, QueryMsg::GetWalletInfo { behalf: HumanAddr("anyone".to_string()), key: vk.to_string() }).unwrap();
        let value:WalletInfoResponse = from_binary(&query_res).unwrap(); 
        let arr : Vec<&str> = vec!["anyone/", "anyone/meme/", "anyone/meme/pepe.jpg", "anyone/meme/pepe2.jpg"];
        assert_eq!(value.all_paths, arr);
        
        // Forget Abt Me! It's not you, It's me 
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::ForgetMe {  };
        let _res = handle(&mut deps, env, msg).unwrap();
        
        // Get File with viewing key
        let query_res = query(&deps, QueryMsg::GetContents { path: String::from("anyone/meme/"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() });
        assert!(query_res.is_err());

        // Get WalletInfo with viewing key
        let query_res = query(&deps, QueryMsg::GetWalletInfo { behalf: HumanAddr("anyone".to_string()), key: vk.to_string() }).unwrap();
        let value:WalletInfoResponse = from_binary(&query_res).unwrap(); 
        let empty : Vec<String> = vec![];
        assert_eq!(value.all_paths, empty);
        assert_eq!(value.init, false);
        
    }

    #[test]
    fn move_file_test() {
        let mut deps = mock_dependencies(20, &[]);
        let vk = init_for_test(&mut deps, String::from("anyone"));

        // Create 3 folders (test/ meme_folder/ pepe/)
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateMulti { 
                contents_list: vec!(String::from("<content inside test/>"), String::from("<content inside meme_folder/>"), String::from("<content inside pepe/>")),  
                path_list: vec!(String::from("anyone/test/"), String::from("anyone/meme_folder/"), String::from("anyone/pepe/")), 
                pkey_list: vec!(String::from("test"), String::from("test"), String::from("test")), 
                skey_list: vec!(String::from("test"), String::from("test"), String::from("test"))
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Create 2 Files phrog1.png and phrog2.png
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateMulti { contents_list: vec!(String::from("content 1"), String::from("content 2")), path_list: vec!(String::from("anyone/test/phrog1.png"), String::from("anyone/test/phrog2.png")) , pkey_list: vec!(String::from("test"), String::from("test")), skey_list: vec!(String::from("test"), String::from("test"))};
        let _res = handle(&mut deps, env, msg).unwrap();

        // Move phrog1.png from /test/ to /meme_folder/
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Move {old_path: String::from("anyone/test/phrog1.png") ,new_path: String::from("anyone/meme_folder/phrog1.png") };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Move phrog2.png from /test/ to /doesnt_exist/
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Move {old_path: String::from("anyone/test/phrog2.png") ,new_path: String::from("anyone/doesnt_exist/phrog2.png") };
        let res = handle(&mut deps, env, msg);
        assert!(res.is_err());

        // Create 2 Files pepe1.png and pepe2.png
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateMulti { contents_list: vec!(String::from("content 1"), String::from("content 2")), path_list: vec!(String::from("anyone/test/pepe1.png"), String::from("anyone/test/pepe2.png")) , pkey_list: vec!(String::from("test"), String::from("test")), skey_list: vec!(String::from("test"), String::from("test"))};
        let _res = handle(&mut deps, env, msg).unwrap();

        // Move pepe1.png and pepe2.png from /test/ to /pepe/
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::MoveMulti { 
                old_path_list: vec!(String::from("anyone/test/pepe1.png"), String::from("anyone/test/pepe2.png")), 
                new_path_list: vec!(String::from("anyone/pepe/pepe1.png"), String::from("anyone/pepe/pepe2.png")) 
        };
        let _res = handle(&mut deps, env, msg).unwrap();


        // Get WalletInfo with viewing key
        let query_res = query(&deps, QueryMsg::GetWalletInfo { behalf: HumanAddr("anyone".to_string()), key: vk.to_string() }).unwrap();
        let value:WalletInfoResponse = from_binary(&query_res).unwrap();
        let arr = vec!["anyone/", "anyone/test/", "anyone/meme_folder/", "anyone/pepe/", "anyone/test/phrog2.png", "anyone/meme_folder/phrog1.png", "anyone/pepe/pepe1.png", "anyone/pepe/pepe2.png"];
        assert_eq!(value.all_paths, arr);
    }

    #[test]
    fn permission_test() {
        let mut deps = mock_dependencies(20, &[]);
        let vk = init_for_test(&mut deps, String::from("anyone"));
        let vk2 = init_for_test(&mut deps, String::from("alice"));

        // Create Folder Test
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Create { contents: String::from("<content of test/ folder>"), path: String::from("anyone/test/") , pkey: String::from("test"), skey: String::from("test")};
        let _res = handle(&mut deps, env, msg).unwrap();

        // Create File
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Create { contents: String::from("I'm sad"), path: String::from("anyone/pepe.jpg") , pkey: String::from("test"), skey: String::from("test")};
        let _res = handle(&mut deps, env, msg).unwrap();
        
        // Allow WRITE for Alice, Bob and Charlie
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::AllowWrite { path: String::from("anyone/test/"), address_list: vec!(String::from("alice"), String::from("bob"), String::from("charlie")) };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Allow READ for Alice, Bob and Charlie
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::AllowRead { path: String::from("anyone/test/"), address_list: vec!(String::from("alice"), String::from("bob"), String::from("charlie")) };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Get File with Alice's viewing key
        let query_res = query(&deps, QueryMsg::GetContents { path: String::from("anyone/test/"), behalf: HumanAddr("alice".to_string()), key: vk2.to_string() });
        assert!(query_res.is_ok());
        
        // DISAllow WRITE for Alice, Bob and Charlie
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::DisallowWrite { path: String::from("anyone/test/"), address_list: vec!(String::from("alice"), String::from("bob"), String::from("charlie")) };
        let _res = handle(&mut deps, env, msg).unwrap();

        // DISAllow WRITE for Alice, Bob and Charlie
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::DisallowRead { path: String::from("anyone/test/"), address_list: vec!(String::from("alice"), String::from("bob"), String::from("charlie")) };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Get File with Anyone's viewing key
        let query_res = query(&deps, QueryMsg::GetContents { path: String::from("anyone/test/"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() });
        let value: FileResponse = from_binary(&query_res.unwrap()).unwrap();
        let test = make_file("anyone", "<content of test/ folder>");
        assert_eq!(test, value.file);
    }

    #[test]
    fn clone_permission_test() {
        let mut deps = mock_dependencies(20, &[]);
        let vk = init_for_test(&mut deps, String::from("anyone"));


        // Create Folder Test
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Create { contents: String::from("<content of layer1 folder>"), path: String::from("anyone/layer1/") , pkey: String::from("pkey"), skey: String::from("skey")};
        let _res = handle(&mut deps, env, msg).unwrap();
        // Create Folder Test
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Create { contents: String::from("<content of layer2 folder>"), path: String::from("anyone/layer1/layer2/") , pkey: String::from("pkey"), skey: String::from("skey")};
        let _res = handle(&mut deps, env, msg).unwrap();    
        // Create Folder Test
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Create { contents: String::from("<content of layer3 folder>"), path: String::from("anyone/layer1/layer2/layer3/") , pkey: String::from("pkey"), skey: String::from("skey")};
        let _res = handle(&mut deps, env, msg).unwrap();
        // Create Folder Test
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Create { contents: String::from("<content of layer4 folder>"), path: String::from("anyone/layer1/layer2/layer3/layer4/") , pkey: String::from("pkey"), skey: String::from("skey")};
        let _res = handle(&mut deps, env, msg).unwrap();

        // Allow WRITE for Alice, Bob and Charlie
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::AllowWrite { path: String::from("anyone/layer1/layer2/"), address_list: vec!(String::from("alice"), String::from("bob"), String::from("charlie")) };
        let _res = handle(&mut deps, env, msg).unwrap();
        // Allow READ for Pepe, Satoshi and Nugget
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::AllowRead { path: String::from("anyone/layer1/layer2/"), address_list: vec!(String::from("pepe"), String::from("satoshi"), String::from("nugget")) };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Clone Permission of all layer2's children
        let env = mock_env("anyone", &[]);
        let _msg = handle(&mut deps, env, HandleMsg::CloneParentPermission {path: String::from("anyone/layer1/layer2/") });

        // Now we query those children to double check the permisions
        // Get Folder layer3/
        let query_res = query(&deps, QueryMsg::GetContents { path: String::from("anyone/layer1/layer2/layer3/"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() });
        let value: FileResponse = from_binary(&query_res.unwrap()).unwrap();
        println!("{:#?}", value);
        // Get Folder layer4/
        let query_res = query(&deps, QueryMsg::GetContents { path: String::from("anyone/layer1/layer2/layer3/layer4/"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() });
        let value: FileResponse = from_binary(&query_res.unwrap()).unwrap();
        println!("{:#?}", value);
    }
}
