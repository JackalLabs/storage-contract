//! This integration test tries to run and call the generated wasm.
//! It depends on a Wasm build being available, which you can create with `cargo wasm`.
//! Then running `cargo integration-test` will validate we can properly call into that generated Wasm.
//!
//! You can easily convert unit tests to integration tests.
//! 1. First copy them over verbatum,
//! 2. Then change
//!      let mut deps = mock_instance(WASM, &[]);
//!    to
//!      let mut deps = mock_instance(WASM, &[]);
//! 3. If you access raw storage, where ever you see something like:
//!      deps.storage.get(CONFIG_KEY).expect("no data stored");
//!    replace it with:
//!      deps.with_storage(|store| {
//!          let data = store.get(CONFIG_KEY).expect("no data stored");
//!          //...
//!      });
//! 4. Anywhere you see query(&deps, ...) you must replace it with query(&mut deps, ...)



use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    debug_print, to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier,
    StdError, StdResult, Storage, HumanAddr
};

use jackal::msg::{FileResponse, HandleMsg, InitMsg, QueryMsg};

use jackal::state::{config, config_read, State};
use jackal::backend::{write_folder, read_folder, traverse_to_file, traverse_folders, make_folder, make_file, build_file, Folder, File};
use jackal::contract::{ query, handle, init};

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary, StdError};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_instance(WASM, &[]);

        let msg = InitMsg {};
        let env = mock_env("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

    }

    #[test]
    fn init_test() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg {};
        let env = mock_env("creator", &coins(2, "token"));
        let _res = init(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::InitAddress { seed_phrase: String::from("JACKAL IS ALIVE")};
        let _res = handle(&mut deps, env, msg).unwrap();

    }

    #[test]
    fn make_file_test() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg {};
        let env = mock_env("creator", &coins(2, "token"));
        let _res = init(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::InitAddress { seed_phrase: String::from("JACKAL IS ALIVE")};
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFile { name: String::from("test.txt"), contents: String::from("Hello World!"), path: String::from("") };
        let _res = handle(&mut deps, env, msg).unwrap();
    }

    #[test]
    fn get_file_test() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg {};
        let env = mock_env("creator", &coins(2, "token"));
        let _res = init(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::InitAddress { seed_phrase: String::from("JACKAL IS ALIVE")};
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFile { name: String::from("test.txt"), contents: String::from("Hello World!"), path: String::from("") };
        let _res = handle(&mut deps, env, msg).unwrap();



        // should increase counter by 1
        let res = query(&mut deps, QueryMsg::GetFile { address: String::from("anyone"), path: String::from("test.txt") }).unwrap();
        let value: FileResponse = from_binary(&res).unwrap();
        assert_eq!(make_file("test.txt", "anyone", "Hello World!"), value.file);
    }

}
