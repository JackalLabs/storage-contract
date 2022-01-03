// use schemars::JsonSchema;
// use serde::{Deserialize, Serialize};

// use std::ptr::null;

use cosmwasm_std::{
    debug_print, to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier,
     StdResult, Storage
};

use crate::msg::{HandleMsg, InitMsg, QueryMsg};

use crate::state::{config, State};
use crate::backend::{query_file, query_folder_contents, try_create_folder, try_create_file, try_init, try_remove_folder, try_remove_file, try_move_folder, try_move_file};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    _msg: InitMsg,
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
        HandleMsg::RemoveFolder { name, path } => try_remove_folder(deps, env, name, path),
        HandleMsg::RemoveFile { name, path } => try_remove_file(deps, env, name, path),
        HandleMsg::MoveFolder { name, old_path, new_path } => try_move_folder(deps, env, name, old_path, new_path),
        HandleMsg::MoveFile { name, old_path, new_path } => try_move_file(deps, env, name, old_path, new_path),

    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary};
    use std::fs::read_to_string;
    use crate::backend::{make_file};
    use crate::msg::{FolderContentsResponse, FileResponse};

    #[test]
    fn move_file_test() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg {};
        let env = mock_env("creator", &coins(2, "token"));
        let _res = init(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::InitAddress { seed_phrase: String::from("JACKAL IS ALIVE")};
        let _res = handle(&mut deps, env, msg).unwrap();

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

        let res = query(&deps, QueryMsg::GetFolderContents { address: String::from("anyone"), path: String::from("/") }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        println!("Folder Content BEFORE: {:?}", &value);

        // Move File
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::MoveFile { name: String::from("pepe.jpeg"), old_path: String::from("/"), new_path: String::from("/meme_storage/sad_meme/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let res = query(&deps, QueryMsg::GetFolderContents { address: String::from("anyone"), path: String::from("/") }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        assert_eq!(value.folders, vec!["anyone/meme_storage/"]);
        println!("Folder Content AFTER: {:?}", &value);

        let res = query(&deps, QueryMsg::GetFolderContents { address: String::from("anyone"), path: String::from("/meme_storage/sad_meme/") }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        assert_eq!(value.files, vec!["anyone/meme_storage/sad_meme/pepe.jpeg"]);
        println!("pepe.jpeg should be HERE: {:?}", &value);

        let res = query(&deps, QueryMsg::GetFile { address: String::from("anyone"), path: String::from("/meme_storage/sad_meme/pepe.jpeg") }).unwrap();
        let value: FileResponse = from_binary(&res).unwrap();
        println!("contents HERE: {:?}", &value.file);
        assert_eq!(make_file("pepe.jpeg", "anyone", "I'm sad"), value.file);

    }

    #[test]
    fn move_folder_test() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg {};
        let env = mock_env("creator", &coins(2, "token"));
        let _res = init(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::InitAddress { seed_phrase: String::from("JACKAL IS ALIVE")};
        let _res = handle(&mut deps, env, msg).unwrap();

        // Create Folders
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFolder { name: String::from("meme_storage"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFolder { name: String::from("sad_meme"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let res = query(&deps, QueryMsg::GetFolderContents { address: String::from("anyone"), path: String::from("/") }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        println!("Folder Content BEFORE: {:?}", &value);

        // Move Folder
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::MoveFolder { name: String::from("sad_meme"), old_path: String::from("/"), new_path: String::from("/meme_storage/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let res = query(&deps, QueryMsg::GetFolderContents { address: String::from("anyone"), path: String::from("/") }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        assert_eq!(value.folders, vec!["anyone/meme_storage/"]);
        println!("Folder Content AFTER: {:?}", &value);

        let res = query(&deps, QueryMsg::GetFolderContents { address: String::from("anyone"), path: String::from("/meme_storage/") }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        assert_eq!(value.folders, vec!["anyone/meme_storage/sad_meme/"]);
        println!("sad_meme should be HERE: {:?}", &value);

    }
    #[test]
    fn remove_folder_test() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg {};
        let env = mock_env("creator", &coins(2, "token"));
        let _res = init(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::InitAddress { seed_phrase: String::from("JACKAL IS ALIVE")};
        let _res = handle(&mut deps, env, msg).unwrap();

        // Create Folder
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFolder { name: String::from("new_folder"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::CreateFolder { name: String::from("u_cant_see_this_folder"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let res = query(&deps, QueryMsg::GetFolderContents { address: String::from("anyone"), path: String::from("/") }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        println!("Folder Content before removal: {:?}", &value);

        // Remove Folder
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::RemoveFolder { name: String::from("u_cant_see_this_folder"), path: String::from("/") };
        let _res = handle(&mut deps, env, msg).unwrap();

        let res = query(&deps, QueryMsg::GetFolderContents { address: String::from("anyone"), path: String::from("/") }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        assert_eq!(value.folders, vec!["anyone/new_folder/"]);

        println!("Folder Content after removal: {:?}", &value);

    }

    #[test]
    fn remove_file_test() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg {};
        let env = mock_env("creator", &coins(2, "token"));
        let _res = init(&mut deps, env, msg).unwrap();

        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::InitAddress { seed_phrase: String::from("JACKAL IS ALIVE")};
        let _res = handle(&mut deps, env, msg).unwrap();

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
        // let env = mock_env("anyone", &coins(2, "token"));
        // let msg = HandleMsg::RemoveFile { name: String::from("very_nice.txt"), path: String::from("/new_folder/") };
        // let _res = handle(&mut deps, env, msg).unwrap();

        let res = query(&deps, QueryMsg::GetFolderContents { address: String::from("anyone"), path: String::from("/") }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        println!("Files after removal in root: {:?}", &value.files);

        let res = query(&deps, QueryMsg::GetFolderContents { address: String::from("anyone"), path: String::from("/new_folder/") }).unwrap();
        let value: FolderContentsResponse = from_binary(&res).unwrap();
        println!("Files after removal in new_folder: {:?}", &value.files);

    }

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

        // This should fail to prevent init again
        // let env = mock_env("anyone", &coins(2, "token"));
        // let msg = HandleMsg::InitAddress { seed_phrase: String::from("JACKAL IS ALIVE")};
        // let _res = handle(&mut deps, env, msg).unwrap();
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
