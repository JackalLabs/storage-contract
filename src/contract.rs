use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fs::read_to_string;

use cosmwasm_std::{
    debug_print, to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier,
    StdError, StdResult, Storage, HumanAddr
};



use crate::msg::{FolderContentsResponse, FileResponse, HandleMsg, InitMsg, QueryMsg};

use crate::state::{config, config_read, State};
use crate::backend::{create_file, create_folder, save_folder, load_folder, load_readonly_folder, save_file, load_file, load_readonly_file, write_folder, read_folder, make_folder, make_file, Folder, File};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {

    let ha = deps.api.human_address(&deps.api.canonical_address(&env.message.sender)?)?;

    let state = State {
        owner: ha.clone(),
    };

    config(&mut deps.storage).save(&state)?;
       

    debug_print!("Contract was initialized by {}", env.message.sender);
    debug_print!("Contract was initialized by {}", env.message.sender);

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

    }
}

pub fn try_init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    seed_phrase: String,
) -> StdResult<HandleResponse> {

    let ha = deps.api.human_address(&deps.api.canonical_address(&env.message.sender)?)?;
    let mut adr = String::from(ha.clone().as_str());

    let folder = make_folder(&adr, &adr);

    adr.push_str("/");

    save_folder(&mut deps.storage, adr, folder);

    Ok(HandleResponse::default())
}

pub fn try_create_file<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    name: String,
    contents: String,
    path: String,
) -> StdResult<HandleResponse> {

    let ha = deps.api.human_address(&deps.api.canonical_address(&env.message.sender)?)?;
    debug_print!("Attempting to create folder for account: {}", ha.clone());

    let mut adr = String::from(ha.clone().as_str());


    let mut p = adr.clone();
    p.push_str(&path);

    let mut l = load_folder(&mut deps.storage, p.clone());


    create_file(&mut deps.storage, &mut l, p.clone(), name, contents);

    save_folder(&mut deps.storage, p.clone(), l);


    debug_print!("create file success");


    Ok(HandleResponse::default())
}

pub fn try_create_folder<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    name: String,
    path: String,
) -> StdResult<HandleResponse> {

    let ha = deps.api.human_address(&deps.api.canonical_address(&env.message.sender)?)?;
    debug_print!("Attempting to create folder for account: {}", ha.clone());

    let mut adr = String::from(ha.clone().as_str());


    let mut p = adr.clone();
    p.push_str(&path);


    let mut l = load_folder(&mut deps.storage, p.clone());


    create_folder(&mut deps.storage, &mut l, p.clone(), name);

    save_folder(&mut deps.storage, p.clone(), l);


    debug_print!("create file success");


    Ok(HandleResponse::default())
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetFile { path, address } => to_binary(&query_file(deps, address, path)?),
        QueryMsg::GetFolderContents { path, address } => to_binary(&query_folder_contents(deps, address, path)?),
    }
}

fn query_file<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, address: String, path: String) -> StdResult<FileResponse> {

    let mut adr = address.clone();

    adr.push_str(&path);

    let f = load_readonly_file(&deps.storage, adr);


    Ok(FileResponse { file: f })
}

fn query_folder_contents<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, address: String, path: String) -> StdResult<FolderContentsResponse> {

    let mut adr = address.clone();

    adr.push_str(&path);

    let f = load_readonly_folder(&deps.storage, adr);



    Ok(FolderContentsResponse { folders: f.list_folders(), files: f.list_files() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary, StdError};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

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
        let msg = HandleMsg::CreateFile { name: String::from("test.txt"), contents: String::from("Hello World!"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();
    }

    #[test]
    fn make_folder_test() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg {};
        let env = mock_env("creator", &coins(2, "token"));
        let _res = init(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::InitAddress { seed_phrase: String::from("JACKAL IS ALIVE")};
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFolder { name: String::from("new_folder"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFile { name: String::from("test.txt"), contents: String::from("Hello World!"), path: String::from("/new_folder/") };
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
        let msg = HandleMsg::CreateFile { name: String::from("test.txt"), contents: String::from("Hello World!"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let res = query(&deps, QueryMsg::GetFile { address: String::from("anyone"), path: String::from("/test.txt") }).unwrap();
        let value: FileResponse = from_binary(&res).unwrap();
        assert_eq!(make_file("test.txt", "anyone", "Hello World!"), value.file);
    }

    #[test]
    fn get_folder_test() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg {};
        let env = mock_env("creator", &coins(2, "token"));
        let _res = init(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::InitAddress { seed_phrase: String::from("JACKAL IS ALIVE")};
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFolder { name: String::from("new_folder"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFile { name: String::from("test.txt"), contents: String::from("Hello World!"), path: String::from("/new_folder/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFile { name: String::from("test2.txt"), contents: String::from("Hello World!"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let res = query(&deps, QueryMsg::GetFolderContents { address: String::from("anyone"), path: String::from("/") }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        assert_eq!(value.files, vec!["anyone/test2.txt"]);
        assert_eq!(value.folders, vec!["anyone/new_folder/"]);

        let res = query(&deps, QueryMsg::GetFolderContents { address: String::from("anyone"), path: String::from("/new_folder/") }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        assert_eq!(value.files, vec!["anyone/new_folder/test.txt"]);
        assert_eq!(value.folders, Vec::<String>::new());
    }

    #[test]
    fn big_files_test() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg {};
        let env = mock_env("creator", &coins(2, "token"));
        let _res = init(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::InitAddress { seed_phrase: String::from("JACKAL IS ALIVE")};
        let _res = handle(&mut deps, env, msg).unwrap();

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
        

        let res = query(&deps, QueryMsg::GetFile { address: String::from("anyone"), path: String::from("/99.png") }).unwrap();
        let value: FileResponse = from_binary(&res).unwrap();
        assert_eq!(value.file.get_contents(), fcont.clone());



        let res = query(&deps, QueryMsg::GetFolderContents { address: String::from("anyone"), path: String::from("/") }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();

        println!("{:?}", value.files);


        assert_eq!(value.folders, vec!["anyone/new_folder/"]);

        let res = query(&deps, QueryMsg::GetFolderContents { address: String::from("anyone"), path: String::from("/new_folder/") }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        assert_eq!(value.files, vec!["anyone/new_folder/test.txt"]);
        assert_eq!(value.folders, Vec::<String>::new());
    }

}
