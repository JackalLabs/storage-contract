// use schemars::JsonSchema;
// use serde::{Deserialize, Serialize};

// use std::ptr::null;

use cosmwasm_std::{debug_print, to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier, StdResult, Storage, QueryResult, StdError};
use secret_toolkit::crypto::sha_256;

use crate::msg::{HandleMsg, InitMsg, QueryMsg, HandleAnswer};
use crate::state::{ State, CONFIG_KEY, save, load, write_viewing_key, read_viewing_key};
use crate::backend::{query_file, query_folder_contents, try_create_folder, try_create_file, try_init, try_remove_folder, try_remove_file, try_move_folder, try_move_file, query_big_tree};
use crate::viewing_key::{ViewingKey, VIEWING_KEY_SIZE};

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
        HandleMsg::InitAddress { seed_phrase } => try_init(deps, env, seed_phrase),
        HandleMsg::CreateFile { name, contents, path } => try_create_file(deps, env, name, contents, path),
        HandleMsg::CreateFolder { name, path } => try_create_folder(deps, env, name, path),
        HandleMsg::RemoveFolder { name, path } => try_remove_folder(deps, env, name, path),
        HandleMsg::RemoveFile { name, path } => try_remove_file(deps, env, name, path),
        HandleMsg::MoveFolder { name, old_path, new_path } => try_move_folder(deps, env, name, old_path, new_path),
        HandleMsg::MoveFile { name, old_path, new_path } => try_move_file(deps, env, name, old_path, new_path),
        HandleMsg::CreateViewingKey { entropy, .. } => try_create_viewing_key(deps, env, entropy),
    }
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
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
                QueryMsg::GetFile { path, address, .. } => to_binary(&query_file(deps, address, path)?),
                QueryMsg::GetFolderContents { path, address, .. } => to_binary(&query_folder_contents(deps, &address, path)?),
                QueryMsg::GetBigTree { address, key, .. } =>to_binary(&query_big_tree(deps, address, key)?),
            };
        }
    }

    Err(StdError::unauthorized())
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
    use std::fs::read_to_string;
    use crate::backend::make_file;
    use crate::msg::{FolderContentsResponse, FileResponse, BigTreeResponse};

    fn init_for_test<S: Storage, A: Api, Q: Querier> (
        deps: &mut Extern<S, A, Q>
    ) -> ViewingKey {

        // Init Contract
        let msg = InitMsg {prng_seed:String::from("lets init bro")};
        let env = mock_env("creator", &[]);
        let _res = init(deps, env, msg).unwrap();

        // Init Address
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::InitAddress { seed_phrase: String::from("JACKAL IS ALIVE")};
        let _res = handle(deps, env, msg).unwrap();

        // Create Viewingkey
        let env = mock_env("anyone", &[]);
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
    fn test_big_tree() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));
        let vk = init_for_test(&mut deps);

        // Create Folders
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
        let msg = HandleMsg::CreateFolder { name: String::from("c"), path: String::from("/a/b/") };
        let _res = handle(&mut deps, env, msg).unwrap();
        
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateFolder { name: String::from("f"), path: String::from("/a/b/c/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateFolder { name: String::from("e"), path: String::from("/a/b/c/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateFolder { name: String::from("f"), path: String::from("/a/b/c/e/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        //Query Big Tree
        let query_res = query(&deps, QueryMsg::GetBigTree { address: HumanAddr("anyone".to_string()), path: String::from("/"), key: vk.to_string() }).unwrap();
        let result:BigTreeResponse = from_binary(&query_res).unwrap();
        println!("{:#?}", &result.data);

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
        let vk = init_for_test(&mut deps);

        // Create File
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateFile { name: String::from("pepe.jpeg"), contents: String::from("I'm sad"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Get File with viewing key
        let query_res = query(&deps, QueryMsg::GetFile { address: HumanAddr("anyone".to_string()), path: String::from("/pepe.jpeg"), key: vk.to_string() }).unwrap();
        let value: FileResponse = from_binary(&query_res).unwrap();
        println!("{:#?}", value);

    }

    #[test]
    fn make_folder_with_vk_test() {
        let mut deps = mock_dependencies(20, &[]);
        let vk = init_for_test(&mut deps);

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
        let query_res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/"), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&query_res).unwrap();
        println!("From /: {:#?}", value);

        let query_res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/a/"), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&query_res).unwrap();
        println!("From /a/: {:#?}", value);

        let query_res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/a/b/"), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&query_res).unwrap();
        println!("From /b/: {:#?}", value);

        let query_res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/a/b/c/"), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&query_res).unwrap();
        println!("From /c/: {:#?}", value);

    }

    #[test]
    fn move_file_test() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));
        let vk = init_for_test(&mut deps);
        
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
        

        let res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/"), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        println!("Folder Content BEFORE: {:?}", &value);

        // Move File
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::MoveFile { name: String::from("pepe.jpeg"), old_path: String::from("/"), new_path: String::from("/meme_storage/sad_meme/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/"), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        assert_eq!(value.folders, vec!["anyone/meme_storage/"]);
        println!("Folder Content AFTER: {:?}", &value);

        let res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/meme_storage/sad_meme/"), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        assert_eq!(value.files, vec!["anyone/meme_storage/sad_meme/pepe.jpeg"]);
        println!("--> pepe.jpeg should be HERE: {:#?}", &value);

        let res = query(&deps, QueryMsg::GetFile { address: HumanAddr("anyone".to_string()), path: String::from("/meme_storage/sad_meme/pepe.jpeg"), key: vk.to_string() }).unwrap();
        let value: FileResponse = from_binary(&res).unwrap();
        println!("contents HERE: {:?}", &value.file);
        assert_eq!(make_file("pepe.jpeg", "anyone", "I'm sad"), value.file);

    }

    #[test]
    fn move_folder_test() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));
        let vk = init_for_test(&mut deps);

        // Create Folders
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFolder { name: String::from("meme_storage"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFolder { name: String::from("sad_meme"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/"), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        println!("Folder Content BEFORE: {:?}", &value);

        // Move Folder
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::MoveFolder { name: String::from("sad_meme"), old_path: String::from("/"), new_path: String::from("/meme_storage/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/"), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        assert_eq!(value.folders, vec!["anyone/meme_storage/"]);
        println!("Folder Content AFTER: {:?}", &value);

        let res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/meme_storage/"), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        assert_eq!(value.folders, vec!["anyone/meme_storage/sad_meme/"]);
        println!("sad_meme folder should be HERE: {:?}", &value);

    }

    #[test]
    fn remove_folder_test() {
        let mut deps = mock_dependencies(20, &[]);
        let vk = init_for_test(&mut deps);

        // Create Folder
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFolder { name: String::from("new_folder"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFolder { name: String::from("u_cant_see_this_folder"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/"), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        println!("Folder Content before removal: {:?}", &value);

        // Remove Folder
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::RemoveFolder { name: String::from("u_cant_see_this_folder"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/"), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        assert_eq!(value.folders, vec!["anyone/new_folder/"]);

        println!("Folder Content after removal: {:?}", &value);

    }

    #[test]
    fn remove_file_test() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));
        let vk = init_for_test(&mut deps);

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

        let res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/"), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        println!("Files after removal in root: {:?}", &value.files);
        assert_eq!(value.files, Vec::<String>::new());


        let res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/new_folder/"), key: vk.to_string() }).unwrap();
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
        let msg = HandleMsg::InitAddress { seed_phrase: String::from("JACKAL IS ALIVE")};
        let _res = handle(&mut deps, env, msg).unwrap();

        // This should fail to prevent init again
        // let env = mock_env("anyone", &coins(2, "token"));
        // let msg = HandleMsg::InitAddress { seed_phrase: String::from("JACKAL IS ALIVE")};
        // let _res = handle(&mut deps, env, msg).unwrap();
    }

    #[test]
    fn big_files_test() {
        let mut deps = mock_dependencies(20, &[]);
        let vk = init_for_test(&mut deps);

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFolder { name: String::from("new_folder"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFile { name: String::from("test.txt"), contents: String::from("Hello World!"), path: String::from("/new_folder/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFile { name: String::from("test2.txt"), contents: String::from("Hello World!"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let fcont : String = read_to_string("eth.txt").unwrap();


        for i in 0..100 {
            let mut nm: String = i.to_string();
            nm.push_str(".png");

            let env = mock_env("anyone", &coins(2, "token"));
            let msg = HandleMsg::CreateFile { name: String::from(nm), contents: fcont.clone(), path: String::from("/") };
            let _res = handle(&mut deps, env, msg).unwrap();
        }
        
        let res = query(&deps, QueryMsg::GetFile { address: HumanAddr("anyone".to_string()), path: String::from("/99.png"), key: vk.to_string() }).unwrap();
        let value: FileResponse = from_binary(&res).unwrap();
        assert_eq!(value.file.get_contents(), fcont.clone());

        let res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/"), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();

        println!("-->Folder Contents: {:#?}", value.files);
        assert_eq!(value.folders, vec!["anyone/new_folder/"]);

        let res = query(&deps, QueryMsg::GetFolderContents { address: HumanAddr("anyone".to_string()), path: String::from("/new_folder/"), key: vk.to_string() }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        assert_eq!(value.files, vec!["anyone/new_folder/test.txt"]);
        assert_eq!(value.folders, Vec::<String>::new());
    }

}
