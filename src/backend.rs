// use std::io::Stderr;
use std::vec;

use cosmwasm_std::{
    debug_print, to_binary, Api, Env, Extern, HandleResponse, HumanAddr, Querier, StdError,
    StdResult, Storage,
};
use cosmwasm_storage::{bucket, bucket_read};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::messaging::{ Message, create_empty_collection, append_message, collection_exist, send_message };
use crate::msg::{FileResponse, HandleAnswer, WalletInfoResponse };
use crate::nodes::write_claim;
use crate::ordered_set::OrderedSet;
use crate::state::{load, write_viewing_key, State, CONFIG_KEY};
use crate::viewing_key::ViewingKey;

// Bucket namespace list:
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

    let namespace = get_namespace(&deps.storage, &adr).unwrap_or(String::from("namespace does not exist!"));
    let already_init = file_exists(&mut deps.storage, &path, &namespace);

    match already_init {
        false => {
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

            create_file(deps, adr.to_string(), path.clone(), contents);

            // Messaging
            
            //Check to see if the collection already exists. If true, it means that sometime before this address called InitAddress, another user
            //had sent a message to them. This message would have prompted the creation of an append_store list, even though the recipient
            //did not have a Storage account at the time. However, If collection_exists == false, we make an empty append_store list for them. 
            let collection_exists = collection_exist(deps, &ha);
            if collection_exists == false { 
                let dummy_message = Message::new(String::from("Placeholder contents created by you"), String::from(env.message.sender.as_str()));
                let _storage_space = create_empty_collection(deps, &ha);
                let _appending_message = append_message(deps, &dummy_message, &ha);   
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

pub fn return_wallet(x: Option<WalletInfo>) -> WalletInfo {
    match x {
        Some(i) => i,//if exists, their wallet init could be false or true, and their namespace is present,
        //If none, it means the user has never called init before, so we return a wallet info that can be altered and saved right away
        None => WalletInfo { init: false, namespace: "empty".to_string(), counter: 0, message_list_counter: 0 },

    }
}

pub fn get_namespace<'a, S: Storage>(store: &'a S, sender: &String) -> StdResult<String> {
    let loaded_wallet: Result<WalletInfo, StdError> = bucket_read(WALLET_INFO_LOCATION, store).load(sender.as_bytes());
    let unwrapped_wallet = loaded_wallet?;
    Ok(unwrapped_wallet.namespace)
}

pub fn get_counter<'a, S: Storage>(store: &'a S, sender: &String) -> StdResult<i32> {
    let loaded_wallet: Result<WalletInfo, StdError> = bucket_read(WALLET_INFO_LOCATION, store).load(sender.as_bytes());
    let unwrapped_wallet = loaded_wallet?;
    Ok(unwrapped_wallet.counter)
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
    let new_counter = wallet_info.counter + 1;
    wallet_info.counter = new_counter;
    let new_namespace = format!("{}{}", adr, new_counter);
    wallet_info.namespace = new_namespace;

    bucket(WALLET_INFO_LOCATION, &mut deps.storage)
        .save(ha.as_str().as_bytes(), &wallet_info)
        .map_err(|err| println!("{:?}", err))
        .ok();

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
            namespace: wallet_info.namespace,
            counter: wallet_info.counter,
            message_list_counter: wallet_info.message_list_counter
        }),
        Err(_e) => Ok(WalletInfoResponse {
            init: false,
            namespace: String::from("empty"),
            counter: 0,
            message_list_counter: 0
        })
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

pub fn try_allow_write<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    path: String,
    message: String,
    address_list: Vec<String>,
) -> StdResult<HandleResponse> {
    let signer = deps
        .api
        .human_address(&deps.api.canonical_address(&env.message.sender)?)?;

    let namespace = get_namespace_from_path(deps, path.clone()).unwrap_or(String::from("namespace does not exist!"));
    let mut f = bucket_load_file(&mut deps.storage, &path, &namespace)?;

    if !f.can_write(signer.to_string()) {
        return Err(StdError::generic_err("Unauthorized to allow write"));
    }

    for i in 0..address_list.len() {
        let address = &address_list[i];
        f.allow_write(address.to_string());

        let recipient = HumanAddr::from(String::from(address));
        let sent_message = send_message(deps, &env, recipient , &message);

        match sent_message{
            Ok(_) => (),
            Err(_) => return Err(StdError::NotFound { kind: String::from("recipient does not exist"), backtrace: None }),
        }

        bucket_save_file(&mut deps.storage, &path, f.clone(), &namespace);
    }

    Ok(HandleResponse::default())
}

pub fn try_disallow_write<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    path: String,
    message: String,
    notify: bool,
    address_list: Vec<String>,
) -> StdResult<HandleResponse> {
    let signer = deps
        .api
        .human_address(&deps.api.canonical_address(&env.message.sender)?)?;

    let namespace = get_namespace_from_path(deps, path.clone()).unwrap_or(String::from("namespace does not exist!"));
    let mut f = bucket_load_file(&mut deps.storage, &path, &namespace)?;

    if !f.can_write(signer.to_string()) {
        return Err(StdError::generic_err("Unauthorized to disallow write"));
    }
    for i in 0..address_list.len() {
        let address = &address_list[i];
        f.disallow_write(address.to_string());

        if notify == true {
            let recipient = HumanAddr::from(String::from(address));
            let sent_message = send_message(deps, &env, recipient , &message);
    
            match sent_message{
                Ok(_) => (),
                Err(_) => return Err(StdError::NotFound { kind: String::from("recipient does not exist"), backtrace: None }),
            }
        }

        bucket_save_file(&mut deps.storage, &path, f.clone(), &namespace);
    }
    Ok(HandleResponse::default())
}

pub fn try_reset_write<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    path: String,
    message: String,
    notify: bool
) -> StdResult<HandleResponse> {
    let signer = deps
    .api
    .human_address(&deps.api.canonical_address(&env.message.sender)?)?;

    let namespace = get_namespace_from_path(deps, path.clone()).unwrap_or(String::from("namespace does not exist!"));
    let mut f = bucket_load_file(&mut deps.storage, &path, &namespace)?;

    if !f.can_write(signer.to_string()) {
        return Err(StdError::generic_err("Unauthorized to reset write list"));
    }

    if notify == true {
        let address_list = f.allow_write_list.to_vec();
        for i in 0..address_list.len() {
            let address = &address_list[i];
            let recipient = HumanAddr::from(String::from(address));
            let sent_message = send_message(deps, &env, recipient , &message);
    
            match sent_message{
                Ok(_) => (),
                Err(_) => return Err(StdError::NotFound { kind: String::from("recipient does not exist"), backtrace: None }),
            }
        }
    }
    
    f.allow_write_list = OrderedSet::new();
    bucket_save_file(&mut deps.storage, &path, f, &namespace);
    Ok(HandleResponse::default())
}

pub fn try_allow_read<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    path: String,
    message: String,
    address_list: Vec<String>,
) -> StdResult<HandleResponse> {
    let signer = deps
    .api
    .human_address(&deps.api.canonical_address(&env.message.sender)?)?;

    let namespace = get_namespace_from_path(deps, path.clone()).unwrap_or(String::from("namespace does not exist!"));
    let mut f = bucket_load_file(&mut deps.storage, &path, &namespace)?; //maybe load read only?

    if !f.can_write(signer.to_string()) {
        return Err(StdError::generic_err("Unauthorized to allow read"));
    }

    for i in 0..address_list.len() {
        let address = &address_list[i];
        f.allow_read(address.to_string());

        let recipient = HumanAddr::from(String::from(address));
        let sent_message = send_message(deps, &env, recipient , &message);

        match sent_message{
            Ok(_) => (),
            Err(_) => return Err(StdError::NotFound { kind: String::from("recipient does not exist"), backtrace: None }),
        }

        bucket_save_file(&mut deps.storage, &path, f.clone(), &namespace);
    }
    Ok(HandleResponse::default())

}

pub fn try_disallow_read<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    path: String,
    message: String,
    notify: bool,
    address_list: Vec<String>,
) -> StdResult<HandleResponse> {
    let signer = deps
        .api
        .human_address(&deps.api.canonical_address(&env.message.sender)?)?;

    let namespace = get_namespace_from_path(deps, path.clone()).unwrap_or(String::from("namespace does not exist!"));
    let mut f = bucket_load_file(&mut deps.storage, &path, &namespace)?;

    if !f.can_write(signer.to_string()) {
        return Err(StdError::generic_err("Unauthorized to disallow read"));
    }

    for i in 0..address_list.len() {
        let address = &address_list[i];
        f.disallow_read(address.to_string());

        if notify == true {
            let recipient = HumanAddr::from(String::from(address));
            let sent_message = send_message(deps, &env, recipient , &message);
    
            match sent_message{
                Ok(_) => (),
                Err(_) => return Err(StdError::NotFound { kind: String::from("recipient does not exist"), backtrace: None }),
            }
        }

        bucket_save_file(&mut deps.storage, &path, f.clone(), &namespace);
    }
    Ok(HandleResponse::default())
}

pub fn try_reset_read<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    path: String,
    message: String,
    notify: bool
) -> StdResult<HandleResponse> {
    let signer = deps
    .api
    .human_address(&deps.api.canonical_address(&env.message.sender)?)?;

    let namespace = get_namespace_from_path(deps, path.clone()).unwrap_or(String::from("namespace does not exist!"));
    let mut f = bucket_load_file(&mut deps.storage, &path, &namespace)?;

    if !f.can_write(signer.to_string()) {
        return Err(StdError::generic_err("Unauthorized to reset read list"));
    }

    if notify == true {
        let address_list = f.allow_read_list.to_vec();
        for i in 0..address_list.len() {
            let address = &address_list[i];
            let recipient = HumanAddr::from(String::from(address));
            let sent_message = send_message(deps, &env, recipient , &message);
    
            match sent_message{
                Ok(_) => (),
                Err(_) => return Err(StdError::NotFound { kind: String::from("recipient does not exist"), backtrace: None }),
            }
        }
    }

    f.allow_read_list = OrderedSet::new();
    bucket_save_file(&mut deps.storage, &path, f, &namespace);
    Ok(HandleResponse::default())
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Debug, Clone)]
pub struct WalletInfo {
    pub init: bool,
    pub namespace: String,
    pub counter: i32,
    pub message_list_counter: i32
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

    let namespace = get_namespace_from_path(&deps, old_path.clone()).unwrap_or(String::from("namespace not found!"));

    //only the owner of a file should be able to move it
    //if we only need to read from a file, we should utilize bucket_read because it's more gas efficient than bucket_load
    let file = bucket_load_readonly_file(&mut deps.storage, &old_path, &namespace);
    let file_res = match file {
        Ok(f) => f,
        Err(_) => return Err(StdError::NotFound { kind: String::from("File move unsuccessful. This file does not exist. Check path is correct"), backtrace: None })
    };

    if env.message.sender.to_string() != file_res.owner {
        return Err(StdError::GenericErr { msg: "You do not own this file and cannot move it".to_string(), backtrace: None })
    }

    let duplicated_contents = file_res.contents;

    //this was previously try_create_file
    let new_file = do_create_file(
        deps,
        env.message.sender.to_string(),
        duplicated_contents,
        new_path,
        String::from(""),//Nug/Marston: do we need to put something here?
        String::from(""),//Nug/Marston: do we need to put something here?
    );

    match new_file {
        Ok(handle_response) => handle_response,
        Err(e) => match e {
            StdError::GenericErr { msg:_, backtrace:None } =>
            return Err(StdError::GenericErr { msg: "File move unsuccessful. Not permitted to write to destination folder".to_string(), backtrace: None }),

            StdError::NotFound { kind:_, backtrace:None } =>
            return Err(StdError::NotFound { kind: "File move unsuccessful. Destination folder does not exist".to_string(), backtrace: None }),
            _ =>
            return Err(StdError::GenericErr { msg: "It's impossible to reach this default branch".to_string(), backtrace: None }),
        }
    };

    let removed = try_remove_file(deps, env, old_path);
    //if we were able to get contents of old_path above, then try_remove_file should always succeed, but I want to keep this here just incase
    match removed {
        Ok(handle_response) => handle_response,
        Err(_e) => return Err(StdError::GenericErr {
            msg: "Unable to remove file in old path".to_string(), backtrace: None })
    };

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

        let _res = try_move_file(
            deps,
            env.clone(),
            old_path.to_string(),
            new_path.to_string(),
        )?;
    }

    //match statement not needed here because errors
    //already properly handled at try_move_file

    Ok(HandleResponse::default())
}

pub fn try_remove_file<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    path: String,
) -> StdResult<HandleResponse> {

    let namespace = get_namespace_from_path(&deps, path.clone()).unwrap_or(String::from("namespace does not exist!"));
    //I think getting namespace from path is needed because we could have a situation in which
    //ownership of a file or folder is changed to another user, and then that user should be able to now
    //delete that folder if they wish, because it belongs to them. If they do so, the env.sender will no longer
    //be able to retrieve the correct namespace

    //bucket.remove() in bucket.rs returns the unit type (), which kind of means return nothing, EVEN IF you were to pass in a key that didn't exist.
    //This means that if you passed in a path that didn't exist or made a typo during testing, you will still get an Ok(HandleResponse)--which is bad
    //because you should be getting an error response. We can't re write bucket.remove() to return an error, so we handle it as such:

    let res = bucket_load_readonly_file(&deps.storage, &path, &namespace);
    match res {
        Ok(f) => {
            if f.owner == env.message.sender.to_string() {
                bucket_remove_file(&mut deps.storage, &path, &namespace);
                return Ok(HandleResponse::default());
            }
            Err(StdError::GenericErr { msg: "Sorry. You are not authorized to remove this file".to_string(), backtrace: None })
        }
        Err(_e) => {
            Err(StdError::NotFound { kind: "This path does not exist. Cannot remove.".to_string(), backtrace: None })
        }
    }
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

    let namespace = get_namespace_from_path(&deps, path.clone()).unwrap_or(String::from("namespace does not exist!"));
    let res = bucket_load_readonly_file(&deps.storage, &par_path, &namespace);

    match res {
        Ok(f) => {
            if f.can_write(ha.to_string()) {
                // Add new file to bucket
                create_file(
                    deps,
                    ha.to_string(),
                    path.to_string(),
                    contents,
                );

                let adr = String::from(&ha);
                let mut acl = adr;
                acl.push_str(&pkey);

                write_claim(&mut deps.storage, acl, skey);

                return Ok(HandleResponse::default());
            }
            Err(StdError::GenericErr { msg: "Sorry. You are unauthorized to create a file in this folder.".to_string(), backtrace: None })
        }
        Err(_e) => {
            Err(StdError::NotFound { kind: format!("File creation unsuccessful. Parent path: '{}' doesn't exist.", &par_path), backtrace: None })
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

        let _res = do_create_file(
            deps,
            ha.to_string(),
            file_contents,
            path,
            pkey.to_string(),
            skey.to_string(),
        )?;
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
        let _res = try_remove_file(deps, env.clone(), path)?;
    }

    Ok(HandleResponse::default())
}

pub fn create_file<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>, //used to just be <'a, S: Storage>(store: &'a mut S),
    owner: String,
    path: String,
    contents: String,
) {
    let file = make_file(&owner, &contents);

    //below allows user to create a file in anyone else's folder, if they had write permissions.
    //They can also move a file that they owned into anyone else's folder, if they had write permissions.
    //The file they owned could be given to them by anyone
    //If we just used get_namespace(), based on the functions that call create_file, the user could only create files within their own root directory,
    //and move files within and to their own root directory

    let namespace = get_namespace_from_path(deps, path.clone()).unwrap_or(String::from("namespace does not exist!"));
    bucket_save_file(&mut deps.storage, &path, file, &namespace);
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

pub fn bucket_save_file<'a, S: Storage>(store: &'a mut S, path: &String, folder: File, namespace: &String) {
    let bucket_response = bucket(namespace.as_bytes(), store).save(path.as_bytes(), &folder);
    match bucket_response {
        Ok(bucket_response) => bucket_response,
        Err(e) => panic!("Bucket Save Error: {}", e),
    }
}

pub fn bucket_remove_file<'a, S: Storage>(store: &'a mut S, path: &String, namespace: &String) {
    bucket::<S, File>(namespace.as_bytes(), store).remove(path.as_bytes());
}
//need to make file_exists use bucket read
pub fn file_exists<'a, S: Storage>(store: &'a mut S, path: &String, namespace: &String) -> bool {
    let f: Result<File, StdError> = bucket(namespace.as_bytes(), store).load(path.as_bytes());

    match f {
        Ok(_file) => true,
        Err(_error) => false,
    }
}

pub fn bucket_load_file<'a, S: Storage>(store: &'a mut S, path: &String, namespace: &String) -> StdResult<File> {
    let f: Result<File, StdError> = bucket(namespace.as_bytes(), store).load(path.as_bytes());
    match f {
        Ok(file) => Ok(file),
        Err(_error) => Err(StdError::NotFound { kind: String::from("No file found at this path."), backtrace: None })
    }
}

pub fn bucket_load_readonly_file<'a, S: Storage>(
    store: &'a S,
    path: &String,
    namespace: &String
) -> Result<File, StdError> {
    bucket_read(namespace.as_bytes(), store).load(path.as_bytes())
}

// QueryMsg
pub fn query_file<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    path: String,
    behalf: &HumanAddr,
) -> StdResult<FileResponse> {

    let full_namespace = get_namespace_from_path(&deps, path.clone()).unwrap_or(String::from("namespace not found!"));

    let f = bucket_load_readonly_file(&deps.storage, &path, &full_namespace); //take in a namespace

    match f {
        Ok(f1) => {
            if f1.can_read(String::from(behalf.as_str())) {
                return Ok(FileResponse { file: f1 });
            }
            Err(StdError::GenericErr { msg: "Sorry bud! Unauthorized to read file.".to_string(), backtrace: None })
        }

        Err(_err) => {
            Err(StdError::NotFound { kind: "File not found. Incorrect path or root directory.".to_string(), backtrace: None })
        }
    }
}

//This previously returned a wallet with init = false and namespace = "empty", but this is illogical so we will just return a NotFound error.
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
            namespace: wallet_info.namespace,
            counter: wallet_info.counter,
            message_list_counter: wallet_info.message_list_counter
        }),
        Err(_e) => Err(StdError::NotFound { kind: String::from("Wallet not found."), backtrace: None })
    }
}

pub fn try_change_owner<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    path: String,
    message: String,
    new_owner: String,
) -> StdResult<HandleResponse> {
    let signer = deps
        .api
        .human_address(&deps.api.canonical_address(&env.message.sender)?)?;

    //if alice now wants to give ownership of the file back to anyone, she would have to pass in the namespace of anyone
    //the only way to get the namespace of the file owner, is from the passed in path
    let full_namespace = get_namespace_from_path(deps, path.clone()).unwrap_or(String::from("namespace not found!"));

    let mut f = bucket_load_file(&mut deps.storage, &path, &full_namespace)?;

    if f.can_write(signer.to_string()){

        f.change_owner(new_owner.to_string());
        let recipient = HumanAddr::from(String::from(new_owner));
        let sent_message = send_message(deps, &env, recipient , &message);

        match sent_message{
            Ok(_) => (),
            Err(_) => return Err(StdError::NotFound { kind: String::from("recipient does not exist"), backtrace: None }),
        }
        
        bucket_save_file(&mut deps.storage, &path, f, &full_namespace);
    }
    else {
        return Err(StdError::GenericErr { msg: "Unauthorized to change owner".to_string(), backtrace: None });
    }

    Ok(HandleResponse::default())
}

pub fn get_namespace_from_path<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    path: String,
) -> StdResult<String> {

    let split = path.split('/');
    let vec = split.collect::<Vec<&str>>();
    let namespace_owner = vec[0].to_string();
    let counter = get_counter(&deps.storage, &namespace_owner)?.to_string();
    let full_namespace = format!("{}{}", namespace_owner, counter);
    Ok(full_namespace)

}

