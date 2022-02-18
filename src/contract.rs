// use schemars::JsonSchema;
// use serde::{Deserialize, Serialize};

// use std::ptr::null;

use cosmwasm_std::{debug_print, to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier, StdResult, Storage, QueryResult, StdError};
use secret_toolkit::crypto::sha_256;
use std::cmp;

use crate::msg::{HandleMsg, InitMsg, QueryMsg, HandleAnswer};
use crate::state::{ State, CONFIG_KEY, save, load, write_viewing_key, read_viewing_key};
use crate::merged_back::{try_allow_read, try_disallow_read, query_file, try_create_file, try_init, try_remove_file, try_move_file, try_create_multi_files};
use crate::viewing_key::{ViewingKey, VIEWING_KEY_SIZE};
use crate::nodes::{pub_query_coins, claim, push_node, get_node, get_node_size, set_node_size};

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
        HandleMsg::InitAddress { contents } => try_init(deps, env, contents),
        HandleMsg::Create { contents, path , pkey, skey} => try_create_file(deps, env, contents, path, pkey, skey),
        HandleMsg::CreateMulti { contents_list, path_list , pkeys, skeys} => try_create_multi_files(deps, env, contents_list, path_list, pkeys, skeys),
        HandleMsg::Remove {  path } => try_remove_file(deps, env, path),
        HandleMsg::Move { old_path, new_path } => try_move_file(deps, env, old_path, new_path),
        HandleMsg::CreateViewingKey { entropy, .. } => try_create_viewing_key(deps, env, entropy),
        HandleMsg::AllowRead { path, address } => try_allow_read(deps, env, path, address),
        HandleMsg::DisallowRead { path, address } => try_disallow_read(deps, env, path, address),
        HandleMsg::InitNode {ip, address} => try_init_node(deps, ip, address),
        HandleMsg::ClaimReward {path, key, address} => claim(deps, path, key, address),
    }
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {

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

    return Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&nodes)?),
    });
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
    use std::vec;
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary, HumanAddr};
    use crate::merged_back::make_file;
    use crate::msg::{FileResponse, BigTreeResponse};

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
        let msg = HandleMsg::InitAddress { contents: String::from("{}") };
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
    fn test_create_file() {
        let mut deps = mock_dependencies(20, &[]);
        let vk = init_for_test(&mut deps, String::from("anyone"));

        let vk2 = init_for_test(&mut deps, String::from("alice"));

        // Create File
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Create { contents: String::from("I'm sad"), path: String::from("anyone/pepe.jpg") , pkey: String::from("test"), skey: String::from("test")};
        let _res = handle(&mut deps, env, msg).unwrap();

        // Get File with viewing key
        let query_res = query(&deps, QueryMsg::GetContents { path: String::from("anyone/pepe.jpg"), behalf: HumanAddr("alice".to_string()), key: vk2.to_string() });
        assert_eq!(query_res.is_err(), true);

        let query_res = query(&deps, QueryMsg::GetContents { path: String::from("anyone/pepe.jpg"), behalf: HumanAddr("anyone".to_string()), key: vk.to_string() }).unwrap();
        let value: FileResponse = from_binary(&query_res).unwrap();
        println!("{:#?}", value);

    }
}
