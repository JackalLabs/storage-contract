// use std::io::Stderr;
use std::vec;

use cosmwasm_std::{
    debug_print, to_binary, Api, Env, Extern, HandleResponse, HumanAddr, Querier, StdError,
    StdResult, Storage,
};
use cosmwasm_storage::{bucket, bucket_read};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::{FileResponse, HandleAnswer, WalletInfoResponse};
use crate::nodes::write_claim;
use crate::ordered_set::OrderedSet;
use crate::state::{load, write_viewing_key, State, CONFIG_KEY};
use crate::viewing_key::ViewingKey;

// Bucket namespace list:
static FILE_LOCATION: &[u8] = b"FILES";
static WALLET_INFO_LOCATION: &[u8] = b"WALLET_INFO";

// HandleMsg::InitAddress
pub fn try_init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    contents: String,
    entropy: String,
) -> StdResult<HandleResponse> {
    let ha = deps
        .api
        .human_address(&deps.api.canonical_address(&env.message.sender)?)?;
    let adr = String::from(ha.as_str());
    let mut path = adr.to_string();
    path.push('/');

    let already_init = file_exists(&mut deps.storage, &path);

    match already_init {
        false => {
            create_file(&mut deps.storage, adr.to_string(), path.clone(), contents);

            //Register Wallet info
            let wallet_info = WalletInfo {
                init: true,
                all_paths: vec![path],
            };
            let bucket_response =
                bucket(WALLET_INFO_LOCATION, &mut deps.storage).save(adr.as_bytes(), &wallet_info);
            match bucket_response {
                Ok(bucket_response) => bucket_response,
                Err(e) => panic!("Bucket Error: {}", e),
            }

            // Let's create viewing key
            let config: State = load(&mut deps.storage, CONFIG_KEY)?;
            let prng_seed = config.prng_seed;
            let key = ViewingKey::new(&env, &prng_seed, (&entropy).as_ref());
            let message_sender = deps.api.canonical_address(&env.message.sender)?;
            write_viewing_key(&mut deps.storage, &message_sender, &key);

            Ok(HandleResponse {
                messages: vec![],
                log: vec![],
                data: Some(to_binary(&HandleAnswer::CreateViewingKey { key })?),
            })
        }
        true => {
            let error_message = format!("User has already been initiated");
            Err(StdError::generic_err(error_message))
        }
    }
}

pub fn try_forget_me<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    let ha = deps
        .api
        .human_address(&deps.api.canonical_address(&env.message.sender)?)?;
    let adr = String::from(ha.as_str());

    let load_bucket: Result<WalletInfo, StdError> =
        bucket_read(WALLET_INFO_LOCATION, &deps.storage).load(adr.as_bytes());
    let mut wallet_info = load_bucket?;

    wallet_info.init = false;
    bucket(WALLET_INFO_LOCATION, &mut deps.storage)
        .save(ha.as_str().as_bytes(), &wallet_info)
        .map_err(|err| println!("{:?}", err))
        .ok();

    let all_paths = &wallet_info.all_paths;
    for i in 0..all_paths.len() {
        let path = all_paths[i].to_string();

        let res = try_remove_file(&mut *deps, env.clone(), path);

        match res {
            Ok(_r) => {}
            Err(e) => {
                return Err(e);
            }
        }
    }

    Ok(HandleResponse::default())
}

pub fn try_you_up_bro<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    address: String,
) -> StdResult<WalletInfoResponse> {
    let load_bucket: Result<WalletInfo, StdError> =
        bucket_read(WALLET_INFO_LOCATION, &deps.storage).load(address.as_bytes());

    match load_bucket {
        Ok(wallet_info) => Ok(WalletInfoResponse {
            init: wallet_info.init,
            all_paths: vec![" private stuff here ;) ".to_string()],
        }),
        Err(_e) => Ok(WalletInfoResponse {
            init: false,
            all_paths: vec![],
        }),
    }
}

pub fn try_create_viewing_key<S: Storage, A: Api, Q: Querier>(
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
        data: Some(to_binary(&HandleAnswer::CreateViewingKey { key })?),
    })
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
enum PermType {
    READ,
    WRITE,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct PermissionBlock {
    address: String,
    permission_type: PermType,
}

//This will match all (Read & Write) permissions of ALL children inside parent_path including grandchildrens too
pub fn try_clone_parent_permission<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    parent_path: String,
) -> StdResult<HandleResponse> {
    let signer = deps
        .api
        .human_address(&deps.api.canonical_address(&env.message.sender)?)?;

    let par = bucket_load_file(&mut deps.storage, &parent_path);
    if !par.can_write(signer.to_string()) {
        return Err(StdError::generic_err("Unauthorized to mess with this"));
    }
    let read_list = par.allow_read_list;
    let write_list = par.allow_write_list;

    let address = signer.as_str();
    let wallet_info: WalletInfo = bucket_read(WALLET_INFO_LOCATION, &deps.storage).load(address.as_bytes())?;
    let all_paths = wallet_info.all_paths;
    
    for path in all_paths.iter(){
        if path.contains(&parent_path){
            if path != &parent_path{
                let copy_data = |mayd: Option<File>| -> StdResult<File> {
                    let mut d = mayd.ok_or(StdError::not_found("Data"))?;
                    d.allow_read_list = read_list.clone();
                    d.allow_write_list = write_list.clone();
                    Ok(d)
                };
                bucket(FILE_LOCATION, &mut deps.storage)
                    .update(&path.as_bytes(), copy_data)
                    .unwrap();
            }
        }
    }
    
    Ok(HandleResponse::default())
}

pub fn try_allow_write<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    path: String,
    address_list: Vec<String>,
) -> StdResult<HandleResponse> {
    let signer = deps
        .api
        .human_address(&deps.api.canonical_address(&env.message.sender)?)?;

    let mut f = bucket_load_file(&mut deps.storage, &path);
    
    if !f.can_write(signer.to_string()) {
        return Err(StdError::generic_err("Unauthorized to allow write"));
    }

    for i in 0..address_list.len() {
        let address = &address_list[i];
        f.allow_write(address.to_string());
        bucket_save_file(&mut deps.storage, &path, f.clone());
    }

    Ok(HandleResponse::default())
}

pub fn try_disallow_write<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    path: String,
    address_list: Vec<String>,
) -> StdResult<HandleResponse> {
    let signer = deps
        .api
        .human_address(&deps.api.canonical_address(&env.message.sender)?)?;

    let mut f = bucket_load_file(&mut deps.storage, &path);

    if !f.can_write(signer.to_string()) {
        return Err(StdError::generic_err("Unauthorized to disallow write"));
    }
    for i in 0..address_list.len() {
        let address = &address_list[i];
        f.disallow_write(address.to_string());
        bucket_save_file(&mut deps.storage, &path, f.clone());
    }
    Ok(HandleResponse::default())
}

pub fn try_reset_write<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    path: String,
) -> StdResult<HandleResponse> {
    let signer = deps
        .api
        .human_address(&deps.api.canonical_address(&env.message.sender)?)?;

    let mut f = bucket_load_file(&mut deps.storage, &path);

    if !f.can_write(signer.to_string()) {
        return Err(StdError::generic_err("Unauthorized to reset write list"));
    }

    f.allow_write_list = OrderedSet::new();
    bucket_save_file(&mut deps.storage, &path, f);
    Ok(HandleResponse::default())
}

pub fn try_allow_read<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    path: String,
    address_list: Vec<String>,
) -> StdResult<HandleResponse> {
    let signer = deps
        .api
        .human_address(&deps.api.canonical_address(&env.message.sender)?)?;

    let mut f = bucket_load_file(&mut deps.storage, &path);
    
    if !f.can_write(signer.to_string()) {
        return Err(StdError::generic_err("Unauthorized to allow write"));
    }

    for i in 0..address_list.len() {
        let address = &address_list[i];
        f.allow_read(address.to_string());
        bucket_save_file(&mut deps.storage, &path, f.clone());
    }
    Ok(HandleResponse::default())
}

pub fn try_disallow_read<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    path: String,
    address_list: Vec<String>,
) -> StdResult<HandleResponse> {
    let signer = deps
        .api
        .human_address(&deps.api.canonical_address(&env.message.sender)?)?;

    let mut f = bucket_load_file(&mut deps.storage, &path);

    if !f.can_write(signer.to_string()) {
        return Err(StdError::generic_err("Unauthorized to disallow read"));
    }

    for i in 0..address_list.len() {
        let address = &address_list[i];
        f.disallow_read(address.to_string());
        bucket_save_file(&mut deps.storage, &path, f.clone());
    }
    Ok(HandleResponse::default())
}

pub fn try_reset_read<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    path: String,
) -> StdResult<HandleResponse> {
    let signer = deps
        .api
        .human_address(&deps.api.canonical_address(&env.message.sender)?)?;

    let mut f = bucket_load_file(&mut deps.storage, &path);

    if !f.can_write(signer.to_string()) {
        return Err(StdError::generic_err("Unauthorized to reset read list"));
    }

    f.allow_read_list = OrderedSet::new();
    bucket_save_file(&mut deps.storage, &path, f);
    Ok(HandleResponse::default())
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Debug, Clone)]
pub struct WalletInfo {
    init: bool,
    pub all_paths: Vec<String>,
}

// HandleMsg FILE
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Debug, Clone)]
pub struct File {
    contents: String,
    owner: String,
    public: bool,
    allow_read_list: OrderedSet<String>,
    allow_write_list: OrderedSet<String>,
}

impl File {
    pub fn get_contents(&self) -> &str {
        &self.contents
    }

    /**
      Please call these before doing anything to files. If you are adding a newly
      created file to a folder, please check that you can write to the folder. If
      the file exists, just check the file permission since they overwrite the
      folder.
    */
    pub fn can_read(&self, address: String) -> bool {
        if self.owner.eq(&address) {
            return true;
        }
        if self.public {
            return true;
        }
        for i in 0..self.allow_read_list.len() {
            if String::from(self.allow_read_list.get(i).unwrap()).eq(&address) {
                return true;
            }
        }
        for i in 0..self.allow_write_list.len() {
            if String::from(self.allow_write_list.get(i).unwrap()).eq(&address) {
                return true;
            }
        }

        false
    }

    pub fn can_write(&self, address: String) -> bool {
        if self.owner.eq(&address) {
            return true;
        }
        for i in 0..self.allow_write_list.len() {
            if String::from(self.allow_write_list.get(i).unwrap()).eq(&address) {
                return true;
            }
        }
        false
    }

    pub fn allow_read(&mut self, address: String) -> bool {
        if self.owner.eq(&address) {
            return false;
        }

        self.allow_read_list.push(address);

        true
    }

    pub fn allow_write(&mut self, address: String) -> bool {
        if self.owner.eq(&address) {
            return false;
        }

        self.allow_write_list.push(address);

        true
    }

    pub fn disallow_read(&mut self, address: String) -> bool {
        if self.owner.eq(&address) {
            return false;
        }

        self.allow_read_list.remove(address);

        true
    }

    pub fn disallow_write(&mut self, address: String) -> bool {
        if self.owner.eq(&address) {
            return false;
        }

        self.allow_write_list.remove(address);

        true
    }

    pub fn make_public(&mut self) -> bool {
        self.public = true;
        true
    }

    pub fn make_private(&mut self) -> bool {
        self.public = false;
        true
    }

    pub fn is_public(&self) -> bool {
        self.public
    }

    pub fn change_owner(&mut self, new_owner: String) {
        self.owner = new_owner;
    }

}

pub fn try_move_file<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    old_path: String,
    new_path: String,
) -> StdResult<HandleResponse> {
    debug_print!(
        "Attempting to move file from `{}` to `{}`",
        old_path.clone(),
        new_path
    );

    let duplicated_contents = bucket_load_file(&mut deps.storage, &old_path).contents;

    try_create_file(
        deps,
        env.clone(),
        duplicated_contents,
        new_path,
        String::from(""),
        String::from(""),
    )?;
    try_remove_file(deps, env, old_path)?;

    Ok(HandleResponse::default())
}

pub fn try_move_multi_files<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    old_path_list: Vec<String>,
    new_path_list: Vec<String>,
) -> StdResult<HandleResponse> {
    debug_print!("Attempting to move multiple files");

    for i in 0..old_path_list.len() {
        let old_path = &old_path_list[i];
        let new_path = &new_path_list[i];

        let res = try_move_file(
            deps,
            env.clone(),
            old_path.to_string(),
            new_path.to_string(),
        );

        match res {
            Ok(_r) => {}
            Err(e) => {
                return Err(e);
            }
        }
    }
    Ok(HandleResponse::default())
}

pub fn try_remove_file<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    path: String,
) -> StdResult<HandleResponse> {
    let ha = deps
        .api
        .human_address(&deps.api.canonical_address(&env.message.sender)?)?;

    // Remove path from file bucket
    bucket_remove_file(&mut deps.storage, &path);

    // Remove path from Wallet info bucket
    let new_data = |mayd: Option<WalletInfo>| -> StdResult<WalletInfo> {
        let mut d = mayd.ok_or(StdError::not_found("Data"))?;
        let index = d.all_paths.iter().position(|r| r == &path).unwrap();
        d.all_paths.remove(index);
        Ok(d)
    };
    bucket(WALLET_INFO_LOCATION, &mut deps.storage)
        .update(ha.as_str().as_bytes(), new_data)
        .unwrap();

    Ok(HandleResponse::default())
}

fn do_create_file<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    ha: String,
    contents: String,
    path: String,
    pkey: String,
    skey: String,
) -> StdResult<HandleResponse> {
    let par_path = parent_path(path.to_string());

    let res = bucket_load_readonly_file(&deps.storage, &par_path);

    match res {
        Ok(f) => {
            if f.can_write(ha.to_string()) {
                // Add new file to bucket
                create_file(
                    &mut deps.storage,
                    ha.to_string(),
                    path.to_string(),
                    contents,
                );

                let adr = String::from(&ha);
                let mut acl = adr;
                acl.push_str(&pkey);

                write_claim(&mut deps.storage, acl, skey);

                // Add new path to Wallet info bucket
                let new_data = |mayd: Option<WalletInfo>| -> StdResult<WalletInfo> {
                    let mut d = mayd.ok_or(StdError::not_found("Data"))?;
                    d.all_paths.push(path);
                    Ok(d)
                };
                bucket(WALLET_INFO_LOCATION, &mut deps.storage)
                    .update(ha.as_bytes(), new_data)
                    .unwrap();

                return Ok(HandleResponse::default());
            }
            let error_message = String::from("Not authorized to create file");
            Err(StdError::generic_err(error_message))
        }
        Err(_e) => {
            let error_message = format!("{} doesn't exist", &par_path);
            Err(StdError::generic_err(error_message))
        }
    }
}

fn parent_path(mut path: String) -> String {
    if path.ends_with('/') {
        path.pop();
    }
    let split = path.split('/');
    let vec = split.collect::<Vec<&str>>();

    let mut par_path = String::new();

    let mut i = 0;
    while i < vec.len() - 1 {
        let s = vec[i];
        par_path.push_str(s);
        par_path.push('/');
        i += 1;
    }

    par_path
}

pub fn try_create_file<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    contents: String,
    path: String,
    pkey: String,
    skey: String,
) -> StdResult<HandleResponse> {
    let ha = deps
        .api
        .human_address(&deps.api.canonical_address(&env.message.sender)?)?;

    do_create_file(deps, ha.to_string(), contents, path, pkey, skey)
}
pub fn try_create_multi_files<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    contents_list: Vec<String>,
    paths: Vec<String>,
    pkeys: Vec<String>,
    skeys: Vec<String>,
) -> StdResult<HandleResponse> {
    let ha = deps
        .api
        .human_address(&deps.api.canonical_address(&env.message.sender)?)?;
    debug_print!("Attempting to create multiple files for account: {}", ha);

    for i in 0..contents_list.len() {
        let file_contents = contents_list[i].clone();
        let path = paths[i].to_string();
        let pkey = &pkeys[i];
        let skey = &skeys[i];

        let res = do_create_file(
            deps,
            ha.to_string(),
            file_contents,
            path,
            pkey.to_string(),
            skey.to_string(),
        );

        match res {
            Ok(_r) => {}
            Err(e) => {
                return Err(e);
            }
        }
    }

    Ok(HandleResponse::default())
}

pub fn try_remove_multi_files<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    path_list: Vec<String>,
) -> StdResult<HandleResponse> {
    let ha = deps
        .api
        .human_address(&deps.api.canonical_address(&env.message.sender)?)?;
    debug_print!("Attempting to remove multiple files for account: {}", ha);

    for i in 0..path_list.len() {
        let path = path_list[i].to_string();

        let res = try_remove_file(deps, env.clone(), path);

        match res {
            Ok(_r) => {}
            Err(e) => {
                return Err(e);
            }
        }
    }

    Ok(HandleResponse::default())
}

pub fn create_file<'a, S: Storage>(
    store: &'a mut S,
    owner: String,
    path: String,
    contents: String,
) {
    let file = make_file(&owner, &contents);

    bucket_save_file(store, &path, file);
}

pub fn make_file(owner: &str, contents: &str) -> File {
    File {
        contents: String::from(contents),
        owner: String::from(owner),
        public: false,
        allow_read_list: OrderedSet::<String>::new(),
        allow_write_list: OrderedSet::<String>::new(),
    }
}

pub fn bucket_save_file<'a, S: Storage>(store: &'a mut S, path: &String, folder: File) {
    let bucket_response = bucket(FILE_LOCATION, store).save(path.as_bytes(), &folder);
    match bucket_response {
        Ok(bucket_response) => bucket_response,
        Err(e) => panic!("Bucket Error: {}", e),
    }
}

pub fn bucket_remove_file<'a, S: Storage>(store: &'a mut S, path: &String) {
    bucket::<S, File>(FILE_LOCATION, store).remove(path.as_bytes());
}

pub fn file_exists<'a, S: Storage>(store: &'a mut S, path: &String) -> bool {
    let f: Result<File, StdError> = bucket(FILE_LOCATION, store).load(path.as_bytes());

    match f {
        Ok(_v) => true,
        Err(_e) => false,
    }
}

pub fn bucket_load_file<'a, S: Storage>(store: &'a mut S, path: &String) -> File {
    bucket(FILE_LOCATION, store).load(path.as_bytes()).unwrap()
}

pub fn bucket_load_readonly_file<'a, S: Storage>(
    store: &'a S,
    path: &String,
) -> Result<File, StdError> {
    bucket_read(FILE_LOCATION, store).load(path.as_bytes())
}

// QueryMsg
pub fn query_file<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    path: String,
    behalf: &HumanAddr,
) -> StdResult<FileResponse> {
    let f = bucket_load_readonly_file(&deps.storage, &path);

    match f {
        Ok(f1) => {
            if f1.can_read(String::from(behalf.as_str())) {
                return Ok(FileResponse { file: f1 });
            }

            let error_message = String::from("Sorry bud! Unauthorized to read file.");
            Err(StdError::generic_err(error_message))
        }

        Err(_err) => {
            let error_message = String::from("Error querying file.");
            Err(StdError::generic_err(error_message))
        }
    }
}

pub fn query_wallet_info<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    behalf: &HumanAddr,
) -> StdResult<WalletInfoResponse> {
    let address = behalf.as_str();
    let load_bucket: Result<WalletInfo, StdError> =
        bucket_read(WALLET_INFO_LOCATION, &deps.storage).load(address.as_bytes());

    match load_bucket {
        Ok(wallet_info) => Ok(WalletInfoResponse {
            init: wallet_info.init,
            all_paths: wallet_info.all_paths,
        }),
        Err(_e) => Ok(WalletInfoResponse {
            init: false,
            all_paths: vec![],
        }),
    }
}

pub fn try_change_owner<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    path: String,
    new_owner: String,
) -> StdResult<HandleResponse> {
    let signer = deps
        .api
        .human_address(&deps.api.canonical_address(&env.message.sender)?)?;

    let mut f = bucket_load_file(&mut deps.storage, &path);

    if f.can_write(signer.to_string()){
        f.change_owner(new_owner.to_string());
        bucket_save_file(&mut deps.storage, &path, f);
    }
    else {
        return Err(StdError::generic_err("Unauthorized to change owner"));
    }

    Ok(HandleResponse::default())
}
