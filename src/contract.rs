// use schemars::JsonSchema;
// use serde::{Deserialize, Serialize};

// use std::ptr::null;

use cosmwasm_std::{
    debug_print, to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier,
    QueryResult, StdError, StdResult, Storage, 
};
use secret_toolkit::crypto::sha_256;
use std::cmp;

use crate::backend::{
    query_file, query_wallet_info, try_allow_read, try_allow_write, try_change_owner,
    try_create_file, try_create_multi_files, try_create_viewing_key, try_disallow_read,
    try_disallow_write, try_forget_me, try_init, try_move_file, try_move_multi_files,
    try_remove_file, try_remove_multi_files, try_reset_read, try_reset_write, try_you_up_bro,
};
use crate::messaging::{query_messages, send_message, clear_all_messages};
use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::nodes::{claim, get_node, get_node_size, pub_query_coins, push_node, set_node_size};
use crate::state::{read_viewing_key, save, State, CONFIG_KEY};
use crate::viewing_key::VIEWING_KEY_SIZE;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let ha = deps
        .api
        .human_address(&deps.api.canonical_address(&env.message.sender)?)?;

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
        HandleMsg::Create {
            contents,
            path,
            pkey,
            skey,
        } => try_create_file(deps, &env, &contents, path, pkey, skey),
        HandleMsg::CreateMulti {
            contents_list,
            path_list,
            pkey_list,
            skey_list,
        } => try_create_multi_files(deps, env, contents_list, path_list, pkey_list, skey_list),
        HandleMsg::Remove { path } => try_remove_file(deps, &env, path),
        HandleMsg::RemoveMulti { path_list } => try_remove_multi_files(deps, env, path_list),
        HandleMsg::MoveMulti {
            old_path_list,
            new_path_list,
        } => try_move_multi_files(deps, env, old_path_list, new_path_list),
        HandleMsg::Move { old_path, new_path } => try_move_file(deps, &env, old_path, new_path),
        HandleMsg::CreateViewingKey { entropy, .. } => try_create_viewing_key(deps, env, entropy),
        HandleMsg::AllowRead { path, message, address_list } => {
            try_allow_read(deps, env, path, message, address_list)
        }
        HandleMsg::DisallowRead { path, message, notify, address_list } => {
            try_disallow_read(deps, env, path, message, notify, address_list)
        }
        HandleMsg::ResetRead { path, message, notify } => try_reset_read(deps, env, path, message, notify),
        HandleMsg::AllowWrite { path, message, address_list } => {
            try_allow_write(deps, env, path, message, address_list)
        }
        HandleMsg::DisallowWrite { path, message, notify, address_list } => {
            try_disallow_write(deps, env, path, message, notify, address_list)
        }
        HandleMsg::ResetWrite { path, message, notify } => try_reset_write(deps, env, path, message, notify),
        HandleMsg::InitNode { ip, address } => try_init_node(deps, ip, address),
        HandleMsg::ClaimReward { path, key, address } => claim(deps, path, key, address),
        HandleMsg::ForgetMe { .. } => try_forget_me(deps, env),
        HandleMsg::ChangeOwner { path, message, new_owner } => try_change_owner(deps, env, path, message, new_owner),
        // Messaging
        HandleMsg::SendMessage { to, contents } => send_message(deps, &env, to, &contents),
        HandleMsg::DeleteAllMessages {} => clear_all_messages(deps, env),
    }
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::YouUpBro { address } => to_binary(&try_you_up_bro(deps, address)?),
        QueryMsg::GetNodeCoins { address } => to_binary(&pub_query_coins(deps, address)?),
        QueryMsg::GetNodeIP { index } => to_binary(&try_get_ip(deps, index)?),
        QueryMsg::GetNodeList { size } => to_binary(&try_get_top_x(deps, size)?),
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
                QueryMsg::GetContents { path, behalf, .. } => {
                    to_binary(&query_file(deps, path, &behalf)?)
                }
                QueryMsg::GetWalletInfo { behalf, .. } => {
                    to_binary(&query_wallet_info(deps, &behalf)?)
                }
                QueryMsg::GetMessages { behalf, .. } => to_binary(&query_messages(deps, &behalf)?),
                _ => panic!("How did this even get to this stage. It should have been processed."),
            };
        }
    }

    Err(StdError::NotFound {
        kind: String::from("Your viewing key does not match 'behalf' address."),
        backtrace: None,
    })
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

    let mut nodes = vec![index_node.clone()];

    if size <= 1 {
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

    fn init_for_test<S: Storage, A: Api, Q: Querier>(
        deps: &mut Extern<S, A, Q>,
        address: String,
    ) -> ViewingKey {
        // Init Contract
        let msg = InitMsg {
            prng_seed: String::from("lets init bro"),
        };
        let env = mock_env("creator", &[]);
        let _res = init(deps, env, msg).unwrap();

        // Init Address and Create ViewingKey
        let env = mock_env(String::from(&address), &[]);
        let msg = HandleMsg::InitAddress {
            contents: String::from("{}"),
            entropy: String::from("Entropygoeshereboi"),
        };
        let handle_response = handle(deps, env, msg).unwrap();

        match from_binary(&handle_response.data.unwrap()).unwrap() {
            HandleAnswer::CreateViewingKey { key } => key,
            _ => panic!("Unexpected result from handle"),
        }
    }

    #[test]
    fn double_init_address_test() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        // Init Contract
        let msg = InitMsg {
            prng_seed: String::from("lets init bro"),
        };
        let env = mock_env("creator", &[]);
        let _res = init(&mut deps, env, msg).unwrap();

        // Init Address
        let env = mock_env(String::from("anyone"), &[]);
        let msg = HandleMsg::InitAddress {
            contents: String::from("{}"),
            entropy: String::from("Entropygoeshereboi"),
        };
        let handle_response = handle(&mut deps, env, msg).unwrap();
        let vk = match from_binary(&handle_response.data.unwrap()).unwrap() {
            HandleAnswer::CreateViewingKey { key } => key,
            _ => panic!("Unexpected result from handle"),
        };
        println!("{:?}", &vk);

        // // Init Address Again
        let env = mock_env(String::from("anyone"), &[]);
        let msg = HandleMsg::InitAddress {
            contents: String::from("{}"),
            entropy: String::from("Entropygoeshereboi"),
        };
        let handle_response = handle(&mut deps, env, msg);
        assert!(handle_response.is_err());
        println!("{:#?}", handle_response);
    }

    #[test]
    fn test_node_setup() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));
        let _vk = init_for_test(&mut deps, String::from("anyone"));

        let query_res: Binary = query(&deps, QueryMsg::GetNodeListSize {}).unwrap();
        let result: HandleResponse = from_binary(&query_res).unwrap();
        let size: u64 = from_binary(&result.data.unwrap()).unwrap();
        println!("{:#?}", &size);

        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::InitNode {
            ip: String::from("192.168.0.1"),
            address: String::from("secret123456789"),
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        let query_res: Binary = query(&deps, QueryMsg::GetNodeListSize {}).unwrap();
        let result: HandleResponse = from_binary(&query_res).unwrap();
        let size: u64 = from_binary(&result.data.unwrap()).unwrap();
        println!("{:#?}", &size);

        let s = size - 1;

        let query_res: Binary = query(&deps, QueryMsg::GetNodeIP { index: (s) }).unwrap();
        let result: HandleResponse = from_binary(&query_res).unwrap();
        let ip: String = from_binary(&result.data.unwrap()).unwrap();
        println!("{:#?}", &ip);
    }

    #[test]
    fn test_create_viewing_key() {
        let mut deps = mock_dependencies(20, &[]);

        // init
        let msg = InitMsg {
            prng_seed: String::from("lets init bro"),
        };
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
            HandleAnswer::CreateViewingKey { key } => key,
            _ => panic!("Unexpected result from handle"),
        };
        let test_key =
            ViewingKey("anubis_key_u25NSWPI5+wpGW7WP6eXtcBpA4RmyZ1CrJRvYFWDNQM=".to_string());
        assert_eq!(vk, test_key);
    }

    #[test]
    fn test_create_file() {
        let mut deps = mock_dependencies(20, &[]);
        let vk = init_for_test(&mut deps, String::from("anyone"));
        let vk2 = init_for_test(&mut deps, String::from("alice"));

        // Create File - Nug: We don't actually use "anyone/test" throughout this test. Should we just delete this paragraph? - Bi
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Create {
            contents: String::from("I'm sad"),
            path: String::from("anyone/test/"),
            pkey: String::from("test"),
            skey: String::from("test"),
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Create File
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Create {
            contents: String::from("I'm lonely"),
            path: String::from("anyone/pepe.jpg"),
            pkey: String::from("test"),
            skey: String::from("test"),
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // anyone attempts to create file in a folder that doesn't exist. Will fail. Print error and run: 'cargo test test_create_file -- --nocapture' to see error message
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Create {
            contents: String::from("Abdul"),
            path: String::from("Stacy/crazy_man.jpg"),
            pkey: String::from("test"),
            skey: String::from("test"),
        };
        let res = handle(&mut deps, env, msg);
        assert!(res.is_err());
        println!("{:#?}", res);

        // Dave attempts to create file in anyone's directory. Will fail. Print error and run: 'cargo test test_create_file -- --nocapture' to see error message
        let env = mock_env("Dave", &[]);
        let msg = HandleMsg::Create {
            contents: String::from("Hasbullah"),
            path: String::from("anyone/silly_man.jpg"),
            pkey: String::from("test"),
            skey: String::from("test"),
        };
        let res = handle(&mut deps, env, msg);
        assert!(res.is_err());
        println!("{:#?}", res);

        //anyone queries their own file with their viewing key. Will succeed.
        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("anyone/pepe.jpg"),
                behalf: HumanAddr("anyone".to_string()),
                key: vk.to_string(),
            },
        )
        .unwrap();
        let value: FileResponse = from_binary(&query_res).unwrap();
        println!(" anyone/pepe.jpg:\n {:#?}", value.file);

        //anyone tries to query their file with the wrong viewing key. Error will say: Your viewing key does not match "behalf" address. Before it just said "unauthorized", which is not clear
        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("anyone/pepe.jpg"),
                behalf: HumanAddr("anyone".to_string()),
                key: "wrong_key".to_string(),
            },
        );
        assert!(query_res.is_err());
        println!("{:#?}", query_res);

        // alice attempts to use her viewing key to query anyone's file. Will fail because alice does not have read permission for the file
        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("anyone/pepe.jpg"),
                behalf: HumanAddr("alice".to_string()),
                key: vk2.to_string(),
            },
        );
        assert!(query_res.is_err());
        println!("{:#?}", query_res);

        // Add alice and bob to file's allow read permissions
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::AllowRead {
            path: String::from("anyone/pepe.jpg"),
            message: String::from("anyone has given you read access to [ anyone/pepe.jpg ]"),
            address_list: vec![String::from("alice"), String::from("bob")],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        //alice's query will now succeed
        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("anyone/pepe.jpg"),
                behalf: HumanAddr("alice".to_string()),
                key: vk2.to_string(),
            },
        )
        .unwrap();
        let value: FileResponse = from_binary(&query_res).unwrap();
        println!("alice successfully queries the file:\n{:#?}", value.file);

        // Query File to show read permissions before resetting
        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("anyone/pepe.jpg"),
                behalf: HumanAddr("anyone".to_string()),
                key: vk.to_string(),
            },
        )
        .unwrap();
        let value: FileResponse = from_binary(&query_res).unwrap();
        println!("Before Reset --> {:#?}", value.file);

        // Reset Read
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::ResetRead {
            path: String::from("anyone/pepe.jpg"),
            message: String::from("anyone has reset read access to anyone/pepe.jpg"),
            notify: true
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Query File
        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("anyone/pepe.jpg"),
                behalf: HumanAddr("anyone".to_string()),
                key: vk.to_string(),
            },
        )
        .unwrap();
        let value: FileResponse = from_binary(&query_res).unwrap();
        println!("After Reset --> {:#?}", value.file);

        //querying file as alice will now fail
        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("anyone/pepe.jpg"),
                behalf: HumanAddr("alice".to_string()),
                key: vk2.to_string(),
            },
        );
        assert!(query_res.is_err());
        println!(
            "alice's query will now fail because she was removed from allow read list:\n{:#?}",
            query_res
        );
    }

    #[test]
    fn test_remove_file() {
        let mut deps = mock_dependencies(20, &[]);
        let vk = init_for_test(&mut deps, String::from("anyone"));
        let vk2 = init_for_test(&mut deps, String::from("alice"));
        //let vk2 = init_for_test(&mut deps, String::from("alice"));

        // Create File No. 1
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Create {
            contents: String::from("I'm sad"),
            path: String::from("anyone/pepe.jpg"),
            pkey: String::from("test"),
            skey: String::from("test"),
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Create File No. 2
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Create {
            contents: String::from("I'm lonely"),
            path: String::from("anyone/hasbullah.jpg"),
            pkey: String::from("test"),
            skey: String::from("test"),
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Create File No. 3
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Create {
            contents: String::from("I'm happy now :)"),
            path: String::from("anyone/sunshine.jpg"),
            pkey: String::from("test"),
            skey: String::from("test"),
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // anyone queries their own file with their viewing key. Will succeed.
        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("anyone/pepe.jpg"),
                behalf: HumanAddr("anyone".to_string()),
                key: vk.to_string(),
            },
        )
        .unwrap();
        let value: FileResponse = from_binary(&query_res).unwrap();
        println!(" anyone/pepe.jpg:\n {:#?}", value.file);

        // Some random user tries to remove anyone's file no. 1. Will fail.
        let env = mock_env("random user", &[]);
        let msg = HandleMsg::Remove {
            path: String::from("anyone/pepe.jpg"),
        };
        let res = handle(&mut deps, env, msg);
        assert!(res.is_err());
        println!("random user failed to remove file:\n{:#?}", res);

        // anyone tries to remove a file that doesn't exist. Will fail.
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Remove {
            path: String::from("anyone/DoesNotExist"),
        };
        let res = handle(&mut deps, env, msg);
        assert!(res.is_err());
        println!("Failed to remove a file that doesn't exist:\n{:#?}", res);

        // Remove file no. 1. Will succeed.
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Remove {
            path: String::from("anyone/pepe.jpg"),
        };
        let res = handle(&mut deps, env, msg).unwrap();
        println!("Removed file:\n{:#?}", res);

        //anyone tries to query the deleted file. Will fail.
        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("anyone/pepe.jpg"),
                behalf: HumanAddr("anyone".to_string()),
                key: vk.to_string(),
            },
        );
        assert!(query_res.is_err());
        println!(" Querying deleted file failed:\n {:#?}", query_res);

        //remove files no. 2 and 3
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::RemoveMulti {
            path_list: vec![
                String::from("anyone/hasbullah.jpg"),
                String::from("anyone/sunshine.jpg"),
            ],
        };
        let res = handle(&mut deps, env, msg).unwrap();
        println!("Successfully removed 2 files:\n{:#?}", res);

        // Create file to be given to new owner and then that owner can delete
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Create {
            contents: String::from("King pepe"),
            path: String::from("anyone/King_pepe.jpg"),
            pkey: String::from("test"),
            skey: String::from("test"),
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::ChangeOwner {
            path: String::from("anyone/King_pepe.jpg"),
            message: String::from("anyone has given you ownership of anyone/King_pepe.jpg"),
            new_owner: String::from("alice"),
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // anyone tries to remove a file that doesn't belong to them anymore. Will fail
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Remove {
            path: String::from("anyone/King_pepe.jpg"),
        };
        let res = handle(&mut deps, env, msg);
        assert!(res.is_err());
        println!(
            "anyone failed to remove a file that doesn't belong to them anymore:\n{:#?}",
            res
        );

        // alice can remove the file now because it belongs to her
        let env = mock_env("alice", &[]);
        let msg = HandleMsg::Remove {
            path: String::from("anyone/King_pepe.jpg"),
        };
        let res = handle(&mut deps, env, msg).unwrap();
        println!(
            "Alice successfully removed the file that she just received from anyone:\n{:#?}",
            res
        );

        //alice tries to query the deleted file. Will fail.
        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("anyone/King_pepe.jpg"),
                behalf: HumanAddr("alice".to_string()),
                key: vk2.to_string(),
            },
        );
        assert!(query_res.is_err());
        println!(
            " Querying the file that alice just deleted will fail:\n {:#?}",
            query_res
        );
    }

    #[test]
    fn test_multi_file() {
        let mut deps = mock_dependencies(20, &[]);

        let vk = init_for_test(&mut deps, String::from("anyone"));

        // Create folder test/
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Create {
            contents: String::from("<content inside test/ folder>"),
            path: String::from("anyone/test/"),
            pkey: String::from("test"),
            skey: String::from("test"),
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Create File
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateMulti {
            contents_list: vec![String::from("I'm sad"), String::from("I'm sad2")],
            path_list: vec![
                String::from("anyone/test/pepe.jpg"),
                String::from("anyone/test/pepe2.jpg"),
            ],
            pkey_list: vec![String::from("test"), String::from("test")],
            skey_list: vec![String::from("test"), String::from("test")],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Remove Multi File
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::RemoveMulti {
            path_list: vec![
                String::from("anyone/test/pepe.jpg"),
                String::from("anyone/test/pepe2.jpg"),
            ],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Get File
        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("anyone/test/pepe.jpg"),
                behalf: HumanAddr("anyone".to_string()),
                key: vk.to_string(),
            },
        );
        assert!(query_res.is_err());

        // Get WalletInfo with viewing key
        let query_res = query(
            &deps,
            QueryMsg::GetWalletInfo {
                behalf: HumanAddr("anyone".to_string()),
                key: vk.to_string(),
            },
        )
        .unwrap();
        let value: WalletInfoResponse = from_binary(&query_res).unwrap();
        // let arr : Vec<String> = vec!["anyone/".to_string(), "anyone/test/".to_string()];
        // assert_eq!(value.all_paths, arr);
        println!("{:#?}", value);
    }

    #[test]
    fn test_you_up_bro() {
        let mut deps = mock_dependencies(20, &[]);
        let _vk = init_for_test(&mut deps, String::from("anyone"));

        let msg = QueryMsg::YouUpBro {
            address: String::from("anyone"),
        };
        let query_res = query(&deps, msg).unwrap();
        let value: WalletInfoResponse = from_binary(&query_res).unwrap();
        assert_eq!(value.init, true);

        let msg = QueryMsg::YouUpBro {
            address: String::from("yeet"),
        };
        let query_res = query(&deps, msg).unwrap();
        let value: WalletInfoResponse = from_binary(&query_res).unwrap();
        assert_eq!(value.init, false);
    }

    #[test]
    fn forget_me_test() {
        let mut deps = mock_dependencies(20, &[]);
        let vk = init_for_test(&mut deps, String::from("anyone"));

        // Create File
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Create {
            contents: String::from("content of meme/ folder "),
            path: String::from("anyone/meme/"),
            pkey: String::from("public key"),
            skey: String::from("secret key"),
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Create Multi File
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateMulti {
            contents_list: vec![String::from("I'm sad"), String::from("I'm sad2")],
            path_list: vec![
                String::from("anyone/meme/pepe.jpg"),
                String::from("anyone/meme/pepe2.jpg"),
            ],
            pkey_list: vec![String::from("test"), String::from("test")],
            skey_list: vec![String::from("test"), String::from("test")],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Get WalletInfo with viewing key
        let query_res = query(
            &deps,
            QueryMsg::GetWalletInfo {
                behalf: HumanAddr("anyone".to_string()),
                key: vk.to_string(),
            },
        )
        .unwrap();
        let wallet: WalletInfoResponse = from_binary(&query_res).unwrap();
        println!("{:#?}", wallet);

        // Get File with viewing key
        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("anyone/meme/"),
                behalf: HumanAddr("anyone".to_string()),
                key: vk.to_string(),
            },
        )
        .unwrap();
        let file: FileResponse = from_binary(&query_res).unwrap();
        println!("{:#?}", file);

        // Forget Abt Me! It's not you, It's me
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::ForgetMe {};
        let _res = handle(&mut deps, env, msg).unwrap();

        // Try and get the file with viewing key again
        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("anyone/meme/"),
                behalf: HumanAddr("anyone".to_string()),
                key: vk.to_string(),
            },
        );
        assert!(query_res.is_err());
        println!("{:#?}", query_res);

        // Get WalletInfo with viewing key
        let query_res = query(
            &deps,
            QueryMsg::GetWalletInfo {
                behalf: HumanAddr("anyone".to_string()),
                key: vk.to_string(),
            },
        )
        .unwrap();
        let value: WalletInfoResponse = from_binary(&query_res).unwrap();
        assert_eq!(value.init, false);
        assert_eq!(value.namespace, "anyone1".to_string());
        assert_eq!(value.counter, 1);
        println!("{:#?}", value);
    }

    #[test]
    fn move_file_test() {
        let mut deps = mock_dependencies(20, &[]);
        let vk = init_for_test(&mut deps, String::from("anyone"));

        // Create 3 folders (test/ meme_folder/ pepe/)
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateMulti {
            contents_list: vec![
                String::from("<content inside test/>"),
                String::from("<content inside meme_folder/>"),
                String::from("<content inside pepe/>"),
            ],
            path_list: vec![
                String::from("anyone/test/"),
                String::from("anyone/meme_folder/"),
                String::from("anyone/pepe/"),
            ],
            pkey_list: vec![
                String::from("test"),
                String::from("test"),
                String::from("test"),
            ],
            skey_list: vec![
                String::from("test"),
                String::from("test"),
                String::from("test"),
            ],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Create 2 Files phrog1.png and phrog2.png
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateMulti {
            contents_list: vec![String::from("content 1"), String::from("content 2")],
            path_list: vec![
                String::from("anyone/test/phrog1.png"),
                String::from("anyone/test/phrog2.png"),
            ],
            pkey_list: vec![String::from("test"), String::from("test")],
            skey_list: vec![String::from("test"), String::from("test")],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Move phrog1.png from /test/ to /meme_folder/
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Move {
            old_path: String::from("anyone/test/phrog1.png"),
            new_path: String::from("anyone/meme_folder/phrog1.png"),
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Try to query "anyone/test/phrog1.png" to ensure it's no longer there
        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("anyone/test/phrog1.png"),
                behalf: HumanAddr("anyone".to_string()),
                key: vk.to_string(),
            },
        );
        assert!(query_res.is_err());
        println!(
            "Confirming that 'anyone/test/phrog1.png' no longer contains a file:\n{:#?}",
            query_res
        );

        // Dave, who doesn't own the file or has write permission in meme_folder/, tries to Move phrog1.png from meme_folder/ back to /test/ - will fail with clear error message
        let env = mock_env("Dave", &[]);
        let msg = HandleMsg::Move {
            old_path: String::from("anyone/meme_folder/phrog1.png"),
            new_path: String::from("anyone/test/phrog1.png"),
        };
        let res = handle(&mut deps, env, msg);
        assert!(res.is_err());
        println!("Dave fails to move phrog1.png:\n{:#?}", res);

        // Move phrog2.png from /test/ to /doesnt_exist/
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Move {
            old_path: String::from("anyone/test/phrog2.png"),
            new_path: String::from("anyone/doesnt_exist/phrog2.png"),
        };
        let res = handle(&mut deps, env, msg);
        assert!(res.is_err());
        println!(
            "Trying to move phrog2.png to a folder that doesn't exist:\n{:#?}",
            res
        );
        // Do I need to use the two files below?
        // Create 2 Files pepe1.png and pepe2.png
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateMulti {
            contents_list: vec![String::from("content 1"), String::from("content 2")],
            path_list: vec![
                String::from("anyone/test/pepe1.png"),
                String::from("anyone/test/pepe2.png"),
            ],
            pkey_list: vec![String::from("test"), String::from("test")],
            skey_list: vec![String::from("test"), String::from("test")],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Move pepe1.png and pepe2.png from /test/ to /pepe/
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::MoveMulti {
            old_path_list: vec![
                String::from("anyone/test/pepe1.png"),
                String::from("anyone/test/pepe2.png"),
            ],
            new_path_list: vec![
                String::from("anyone/pepe/pepe1.png"),
                String::from("anyone/pepe/pepe2.png"),
            ],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Get WalletInfo with viewing key
        let query_res = query(
            &deps,
            QueryMsg::GetWalletInfo {
                behalf: HumanAddr("anyone".to_string()),
                key: vk.to_string(),
            },
        )
        .unwrap();
        let value: WalletInfoResponse = from_binary(&query_res).unwrap();
        println!("{:#?}", value);
    }

}