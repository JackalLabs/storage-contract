// use schemars::JsonSchema;
// use serde::{Deserialize, Serialize};

// use std::ptr::null;

use cosmwasm_std::{debug_print, to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier, StdResult, Storage, QueryResult, StdError};
use secret_toolkit::crypto::sha_256;

use crate::msg::{HandleMsg, InitMsg, QueryMsg, HandleAnswer};
use crate::state::{ State, CONFIG_KEY, save, load, write_viewing_key, read_viewing_key};
use crate::backend::{try_allow_read, try_disallow_read, query_file, query_folder_contents, try_create_folder, try_create_file, try_init, try_remove_folder, try_remove_file, try_move_folder, try_move_file, query_big_tree};
use crate::viewing_key::{ViewingKey, VIEWING_KEY_SIZE};
use crate::nodes::{push_node, get_node, get_node_size, set_node_size};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {

    let ha = deps.api.human_address(&deps.api.canonical_address(&env.message.sender)?)?;

    let config = State {
        owner: ha.clone(),
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
        HandleMsg::InitAddress { } => try_init(deps, env),
        HandleMsg::CreateFile { name, contents, path } => try_create_file(deps, env, name, contents, path),
        HandleMsg::CreateFolder { name, path } => try_create_folder(deps, env, name, path),
        HandleMsg::RemoveFolder { name, path } => try_remove_folder(deps, env, name, path),
        HandleMsg::RemoveFile { name, path } => try_remove_file(deps, env, name, path),
        HandleMsg::MoveFolder { name, old_path, new_path } => try_move_folder(deps, env, name, old_path, new_path),
        HandleMsg::MoveFile { name, old_path, new_path } => try_move_file(deps, env, name, old_path, new_path),
        HandleMsg::CreateViewingKey { entropy, .. } => try_create_viewing_key(deps, env, entropy),
        HandleMsg::AllowRead { path, address } => try_allow_read(deps, env, path, address),
        HandleMsg::DisallowRead { path, address } => try_disallow_read(deps, env, path, address),
        HandleMsg::InitNode {ip, address} => try_init_node(deps, ip, address),

    }
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {

        QueryMsg::GetNodeIP {index} => to_binary(&try_get_ip(deps, index)?),
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
                QueryMsg::GetFile { path, address, behalf, .. } => to_binary(&query_file(deps, address, path, &behalf)?),
                QueryMsg::GetFolderContents { path, behalf, address, .. } => to_binary(&query_folder_contents(deps, &address, path, &behalf)?),
                QueryMsg::GetBigTree { address, key, .. } =>to_binary(&query_big_tree(deps, address, key)?),
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

fn try_get_node_list_size<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<HandleResponse> {

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&get_node_size(&deps.storage))?),
    })
}

fn try_create_viewing_key<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    entropy: String,
) -> StdResult<HandleResponse> {
    let config: State = load(&mut deps.storage, CONFIG_KEY)?;
    let prng_seed = config.prng_seed;

    let key = ViewingKey::new(&env, &prng_seed, (&entropy).as_ref());

    let message_sender = deps.api.canonical_address(&env.message.sender)?;

    write_viewing_key(&mut deps.storage, &message_sender, &key);

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::CreateViewingKey { 
            key,
        })?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary, HumanAddr};
    use crate::backend::make_file;
    use crate::msg::{FolderContentsResponse, FileResponse, BigTreeResponse};

    fn init_for_test<S: Storage, A: Api, Q: Querier> (
        deps: &mut Extern<S, A, Q>,
        address:String,
    ) -> ViewingKey {

        // Init Contract
        let msg = InitMsg {prng_seed:String::from("lets init bro")};
        let env = mock_env("creator", &[]);
        let _res = init(deps, env, msg).unwrap();

        // Init Address
        let env = mock_env(String::from(&address), &[]);
        let msg = HandleMsg::InitAddress { };
        let _res = handle(deps, env, msg).unwrap();

        // Create Viewingkey
        let env = mock_env(String::from(&address), &[]);
        let create_vk_msg = HandleMsg::CreateViewingKey {
            entropy: "supbro".to_string(),
            padding: None,
        };
        let handle_response = handle(deps, env, create_vk_msg).unwrap();
        let vk = match from_binary(&handle_response.data.unwrap()).unwrap() {
            HandleAnswer::CreateViewingKey { key } => {
                // println!("viewing key here: {}",key);
                key
            },
            _ => panic!("Unexpected result from handle"),
        };
        vk
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
    fn test_big_tree() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));
        let vk = init_for_test(&mut deps, String::from("anyone"));

        // Create Folders & Files
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateFolder { name: String::from("a"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateFolder { name: String::from("empty_folder"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateFolder { name: String::from("b"), path: String::from("/a/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateFolder { name: String::from("d"), path: String::from("/a/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateFile { name: String::from("file_in_a"), path: String::from("/a/"), contents: String::from("<content here>") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateFolder { name: String::from("c"), path: String::from("/a/b/") };
        let _res = handle(&mut deps, env, msg).unwrap();
        
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateFile { name: String::from("file_in_b"), path: String::from("/a/b/"), contents: String::from("<content here>") };
        let _res = handle(&mut deps, env, msg).unwrap();
        
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateFolder { name: String::from("f"), path: String::from("/a/b/c/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateFolder { name: String::from("e"), path: String::from("/a/b/c/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateFile { name: String::from("file_in_c"), path: String::from("/a/b/c/"), contents: String::from("<content here>") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateFile { name: String::from("file2_in_c"), path: String::from("/a/b/c/"), contents: String::from("<content here>") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateFolder { name: String::from("f"), path: String::from("/a/b/c/e/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateFile { name: String::from("file_in_e"), path: String::from("/a/b/c/e/"), contents: String::from("<content here>") };
        let _res = handle(&mut deps, env, msg).unwrap();

        //Query Big Tree
        let query_res = query(&deps, QueryMsg::GetBigTree { address: HumanAddr("anyone".to_string()), path: String::from("/"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() }).unwrap();
        let result:BigTreeResponse = from_binary(&query_res).unwrap();
        println!("{:#?}", &result);

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
        
        let _vk = match from_binary(&handle_response.data.unwrap()).unwrap() {
            HandleAnswer::CreateViewingKey { key } => {
                // println!("viewing key here: {}",key);
                key
            },
            _ => panic!("Unexpected result from handle"),
        };

    }
    
    #[test]
    fn make_file_with_vk_test() {
        let mut deps = mock_dependencies(20, &[]);
        let vk = init_for_test(&mut deps, String::from("anyone"));

        // Create File
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateFile { name: String::from("pepe.jpeg"), contents: String::from("I'm sad"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Get File with viewing key
        let query_res = query(&deps, QueryMsg::GetFile { address: HumanAddr("anyone".to_string()), path: String::from("/pepe.jpeg"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() }).unwrap();
        let value: FileResponse = from_binary(&query_res).unwrap();
        println!("{:#?}", value);

    }

    #[test]
    fn permission_test() {
        let mut deps = mock_dependencies(20, &[]);
        let _vk = init_for_test(&mut deps, String::from("nugget"));
        let vk_alice = init_for_test(&mut deps, String::from("alice"));

        // Create Folder and File
        let env = mock_env("nugget", &[]);
        let msg = HandleMsg::CreateFolder { name: String::from("a"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("nugget", &[]);
        let msg = HandleMsg::CreateFile { name: String::from("big_nuggz.txt"), path: String::from("/a/"), contents: String::from("shrimp") };
        let _res = handle(&mut deps, env, msg).unwrap();

        // AllowRead File
        let env = mock_env("nugget", &[]);
        let msg = HandleMsg::AllowRead { path: String::from("/a/big_nuggz.txt"), address: String::from("alice") };
        let _res = handle(&mut deps, env, msg).unwrap();
        
        // AllowRead Folder
        let env = mock_env("nugget", &[]);
        let msg = HandleMsg::AllowRead { path: String::from("/a/"), address: String::from("alice") };
        let _res = handle(&mut deps, env, msg).unwrap();
        
        // DisallowRead File
        let env = mock_env("nugget", &[]);
        let msg = HandleMsg::DisallowRead { path: String::from("/a/big_nuggz.txt"), address: String::from("alice") };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Query
        let query_res = query(&deps, QueryMsg::GetFile { address: HumanAddr("nugget".to_string()), path: String::from("/a/big_nuggz.txt"), behalf: HumanAddr("alice".to_string()), key: vk_alice.to_string() });
        assert!(query_res.is_err() == true);


    }

    #[test]
    fn make_folder_with_vk_test() {
        let mut deps = mock_dependencies(20, &[]);

        let vk = init_for_test(&mut deps, String::from("anyone"));


        let vk_alice = init_for_test(&mut deps, String::from("alice"));

        // Create Folders
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateFolder { name: String::from("a"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateFolder { name: String::from("new_folder"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateFolder { name: String::from("b"), path: String::from("/a/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateFolder { name: String::from("c"), path: String::from("/a/b/") };
        let _res = handle(&mut deps, env, msg).unwrap();
        
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateFolder { name: String::from("d"), path: String::from("/a/b/c/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Get Folder with viewing key
        let query_res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&query_res).unwrap();
        println!("Query anyone's root by anyone: {:#?}", value);

        let query_res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/a/"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&query_res).unwrap();
        println!("Query anyone's /a/ by anyone: {:#?}", value);

        let query_res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/a/b/"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&query_res).unwrap();
        println!("Query anyone's /b/ by anyone: {:#?}", value);

        let query_res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/a/b/c/"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&query_res).unwrap();
        println!("Query anyone's /c/ by anyone: {:#?}", value);

        let query_res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/a/b/c/"), behalf: HumanAddr("alice".to_string()), key: vk_alice.to_string() });
        assert!(query_res.is_err() == true);

        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::AllowRead { path: String::from("/a/b/c/"), address: String::from("alice") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("alice", &[]);
        let msg = HandleMsg::CreateFolder { name: String::from("yeeet"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let query_res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("alice".to_string()), path: String::from("/"), behalf: HumanAddr("alice".to_string()), key: vk_alice.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&query_res).unwrap();
        println!("Query alice's root by alice: {:#?}", value);

        let query_res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/a/b/c/"), behalf: HumanAddr("alice".to_string()), key: vk_alice.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&query_res).unwrap();
        println!("Query anyone's /c/ by alice: {:#?}", value);

    }

    #[test]
    fn move_file_test() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));
        let vk = init_for_test(&mut deps, String::from("anyone"));
        
        // Create Folders
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFolder { name: String::from("meme_storage"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFolder { name: String::from("sad_meme"), path: String::from("/meme_storage/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Create File
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFile { name: String::from("pepe.jpeg"), contents: String::from("I'm sad"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();
        

        let res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        println!("Folder Content BEFORE: {:?}", &value);

        // Move File
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::MoveFile { name: String::from("pepe.jpeg"), old_path: String::from("/"), new_path: String::from("/meme_storage/sad_meme/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        assert_eq!(value.folders, vec!["anyone/meme_storage/"]);
        println!("Folder Content AFTER: {:?}", &value);

        let res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/meme_storage/sad_meme/"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        assert_eq!(value.files, vec!["anyone/meme_storage/sad_meme/pepe.jpeg"]);
        println!("--> pepe.jpeg should be HERE: {:#?}", &value);

        let res = query(&deps, QueryMsg::GetFile { address: HumanAddr("anyone".to_string()), path: String::from("/meme_storage/sad_meme/pepe.jpeg"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() }).unwrap();
        let value: FileResponse = from_binary(&res).unwrap();
        println!("contents HERE: {:?}", &value.file);
        assert_eq!(make_file("pepe.jpeg", "anyone", "I'm sad"), value.file);

    }

    #[test]
    fn move_folder_test() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));
        let vk = init_for_test(&mut deps, String::from("anyone"));

        // Create Folders
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFolder { name: String::from("meme_storage"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFolder { name: String::from("sad_meme"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        println!("Folder Content BEFORE: {:?}", &value);

        // Move Folder
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::MoveFolder { name: String::from("sad_meme"), old_path: String::from("/"), new_path: String::from("/meme_storage/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        assert_eq!(value.folders, vec!["anyone/meme_storage/"]);
        println!("Folder Content AFTER: {:?}", &value);

        let res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/meme_storage/"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        assert_eq!(value.folders, vec!["anyone/meme_storage/sad_meme/"]);
        println!("sad_meme folder should be HERE: {:?}", &value);

    }

    #[test]
    fn remove_folder_test() {
        let mut deps = mock_dependencies(20, &[]);
        let vk = init_for_test(&mut deps, String::from("anyone"));

        // Create Folder
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFolder { name: String::from("layer_1"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFolder { name: String::from("layer_2"), path: String::from("/layer_1/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFolder { name: String::from("second_layer_2"), path: String::from("/layer_1/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFolder { name: String::from("layer_3"), path: String::from("/layer_1/layer_2/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Create 2 File
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateFile { name: String::from("pepe.jpeg"), contents: String::from("I'm sad"), path: String::from("/layer_1/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateFile { name: String::from("nuggie.jpeg"), contents: String::from("I'm nuggie"), path: String::from("/layer_1/layer_2/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Query Before
        let res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        println!("Root Folder content before removal: {:#?}", &value);

        let res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/layer_1/layer_2/"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        println!("Layer_2 content before removal: {:#?}", &value);

        // Remove Folder
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::RemoveFolder { name: String::from("layer_1"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Query After
        let res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/layer_1/layer_2/"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() });
        assert!(res.is_err() == true);

        let res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        println!("Root folder Content after removal: {:?}", &value);

        // Query File that should fail
        let res = query(&deps, QueryMsg::GetFile { address: HumanAddr("anyone".to_string()), path: String::from("/layer_1/pepe.jpeg"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() });
        assert!(res.is_err() == true);
        // println!("Get pepe.jpeg: {:#?}", res);

        let res = query(&deps, QueryMsg::GetFile { address: HumanAddr("anyone".to_string()), path: String::from("/layer_1/layer_2/nuggie.jpeg"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() });
        assert!(res.is_err() == true);
        // println!("Get nuggie.jpeg: {:#?}", res);
    }

    #[test]
    fn remove_file_test() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));
        let vk = init_for_test(&mut deps, String::from("anyone"));

        // Create Folder
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFolder { name: String::from("new_folder"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Create File in root
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFile { name: String::from("test.txt"), contents: String::from("Hello World!"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Create File in new_folder
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFile { name: String::from("very_nice.txt"), contents: String::from("OK!"), path: String::from("/new_folder/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Remove File in root
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::RemoveFile { name: String::from("test.txt"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Remove File in new_folder
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::RemoveFile { name: String::from("very_nice.txt"), path: String::from("/new_folder/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        println!("Files after removal in root: {:?}", &value.files);
        assert_eq!(value.files, Vec::<String>::new());


        let res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/new_folder/"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        println!("Files after removal in new_folder: {:?}", &value.files);
        assert_eq!(value.files, Vec::<String>::new());

    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg { prng_seed:String::from("lets init bro")};
        let env = mock_env("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

    }

    #[test]
    fn init_address_test() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg {prng_seed:String::from("lets init bro")};
        let env = mock_env("creator", &coins(2, "token"));
        let _res = init(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::InitAddress { };
        let _res = handle(&mut deps, env, msg).unwrap();

        // This should fail to prevent init again
        // let env = mock_env("anyone", &coins(2, "token"));
        // let msg = HandleMsg::InitAddress { };
        // let _res = handle(&mut deps, env, msg).unwrap();
    }

}
