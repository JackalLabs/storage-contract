use std::io::Stderr;
use std::vec;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{debug_print,Env, Api, Querier, ReadonlyStorage, Storage, StdResult, StdError, Extern, HandleResponse, HumanAddr};
use cosmwasm_storage::{ bucket, bucket_read, Bucket, ReadonlyBucket};


use crate::ordered_set::{OrderedSet};
use crate::msg::{FileResponse};
use crate::nodes::{ write_claim };

static FOLDER_LOCATION: &[u8] = b"FOLDERS";
static FILE_LOCATION: &[u8] = b"FILES";


// HandleMsg::InitAddress
pub fn try_init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    contents: String
) -> StdResult<HandleResponse> {

    let ha = deps.api.human_address(&deps.api.canonical_address(&env.message.sender)?)?;
    let adr = String::from(ha.clone().as_str());

    let mut path = adr.to_string();
    path.push_str("/");

    create_file(&mut deps.storage, adr.to_string(), path, contents);

    Ok(HandleResponse::default())
}


#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
enum PermType {
    READ,
    WRITE,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct PermissionBlock{
    address: String,
    permission_type: PermType,
}

pub fn try_allow_write<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    path: String,
    address: String,
) -> StdResult<HandleResponse> {

    let signer = deps.api.human_address(&deps.api.canonical_address(&env.message.sender)?)?;

    // let par_path = parent_path(path.to_string());
    // let par = bucket_load_file(&mut deps.storage, &par_path);
    let mut f = bucket_load_file(&mut deps.storage, &path);

    if !f.can_write(signer.to_string()) {
        return Err(StdError::generic_err("Unauthorized to allow write"));
    }

    f.allow_write(address);
    bucket_save_file(&mut deps.storage, path, f);
    Ok(HandleResponse::default())
    
}

pub fn try_disallow_write<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    path: String,
    address: String,
) -> StdResult<HandleResponse> {

    let signer = deps.api.human_address(&deps.api.canonical_address(&env.message.sender)?)?;

    // let par_path = parent_path(path.to_string());
    // let par = bucket_load_file(&mut deps.storage, &par_path);
    let mut f = bucket_load_file(&mut deps.storage, &path);

    if !f.can_write(signer.to_string()) {
        return Err(StdError::generic_err("Unauthorized to disallow write"));
    }

    f.disallow_write(address);
    bucket_save_file(&mut deps.storage, path, f);
    Ok(HandleResponse::default())
    
}

pub fn try_reset_write<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    path: String,
) -> StdResult<HandleResponse> {

    let signer = deps.api.human_address(&deps.api.canonical_address(&env.message.sender)?)?;
    let mop = bucket_load_file(&mut deps.storage, &path); //this should load path not par_path

    if !mop.can_write(signer.to_string()) {
        return Err(StdError::generic_err("Unauthorized to reset write list"));
    } else {
        let mut f = bucket_load_file(&mut deps.storage, &path);
        f.allow_write_list = OrderedSet::new();
        bucket_save_file(&mut deps.storage, path, f);
        Ok(HandleResponse::default())
    }
    
}

pub fn try_allow_read<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    path: String,
    address: String,
) -> StdResult<HandleResponse> {

    let signer = deps.api.human_address(&deps.api.canonical_address(&env.message.sender)?)?;

    // let par_path = parent_path(path.to_string());
    // let par = bucket_load_file(&mut deps.storage, &par_path);
    let mut f = bucket_load_file(&mut deps.storage, &path);

    if !f.can_write(signer.to_string()) {
        return Err(StdError::generic_err("Unathorized to allow read"));
    }

    f.allow_read(address);
    bucket_save_file(&mut deps.storage, path, f);
    Ok(HandleResponse::default())
    
}

pub fn try_disallow_read<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    path: String,
    address: String,
) -> StdResult<HandleResponse> {

    let signer = deps.api.human_address(&deps.api.canonical_address(&env.message.sender)?)?;

    // let par_path = parent_path(path.to_string());
    // let par = bucket_load_file(&mut deps.storage, &par_path);
    // Shouldn't we check permission with the path instead of parent_path?
    let mut f = bucket_load_file(&mut deps.storage, &path);

    if !f.can_write(signer.to_string()) {
        return Err(StdError::generic_err("Unauthorized to disallow read"));
    }

    f.disallow_read(address);
    bucket_save_file(&mut deps.storage, path, f);
    Ok(HandleResponse::default())
    
}

pub fn try_reset_read<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    path: String,
) -> StdResult<HandleResponse> {

    let signer = deps.api.human_address(&deps.api.canonical_address(&env.message.sender)?)?;
    let mop = bucket_load_file(&mut deps.storage, &path);

    if !mop.can_write(signer.to_string()) {
        return Err(StdError::generic_err("Unauthorized to reset read list"));
    }

    let mut f = bucket_load_file(&mut deps.storage, &path);
    f.allow_read_list = OrderedSet::new();
    bucket_save_file(&mut deps.storage, path, f);
    Ok(HandleResponse::default())
    
}

// HandleMsg FILE
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Debug, Clone)]
pub struct File{
    contents: String,
    owner: String,
    public: bool,
    allow_read_list: OrderedSet<String>,
    allow_write_list: OrderedSet<String>
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
    pub fn can_read(&self, address:String) -> bool{
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

        return false;
    }

    pub fn can_write(&self, address:String) -> bool{
        if self.owner.eq(&address) {
            return true;
        } 
            for i in 0..self.allow_write_list.len() {
                if String::from(self.allow_write_list.get(i).unwrap()).eq(&address) {
                    return true;
                }
            }
            return false;
    }

    pub fn allow_read(&mut self, address:String) -> bool {
        if self.owner.eq(&address) {
            return false;
        }

        self.allow_read_list.push(address);

        return true;
    }

    pub fn allow_write(&mut self, address:String) -> bool {
        if self.owner.eq(&address) {
            return false;
        }

        self.allow_write_list.push(address);

        true
    }

    pub fn disallow_read(&mut self, address:String) -> bool {
        if self.owner.eq(&address) {
            return false;
        }

        self.allow_read_list.remove(address);

        return true;
    }

    pub fn disallow_write(&mut self, address:String) -> bool {
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
}

pub fn try_move_file<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    old_path: String,
    new_path: String,
) -> StdResult<HandleResponse> {


    debug_print!("Attempting to move file from `{}` to `{}`", old_path.clone() , new_path.clone());

    let duplicated_contents = bucket_load_file(&mut deps.storage, &old_path).contents;

    try_create_file(deps, env.clone(), duplicated_contents, new_path, String::from(""), String::from(""))?;
    try_remove_file(deps, env, old_path)?;

    Ok(HandleResponse::default())
}

pub fn try_remove_file<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    path: String,
) -> StdResult<HandleResponse> {

    bucket_remove_file(&mut deps.storage, path);

    Ok(HandleResponse::default())
}

fn do_create_file<S: Storage, A: Api, Q: Querier>(deps: &mut Extern<S, A, Q>, ha: String, contents: String, path: String, pkey: String, skey: String) -> StdResult<HandleResponse> {

    let par_path = parent_path(path.to_string());

    let res = bucket_load_readonly_file(&deps.storage, par_path);

    let error_message = String::from("Error Creating File");

    match res {
        Ok(f) => {
            if f.can_write(ha.to_string()) {
                create_file(&mut deps.storage, ha.to_string(), path.to_string(), contents);

                let adr = String::from(ha);
                let mut acl = adr.clone();
                acl.push_str(&pkey);
            
                write_claim(&mut deps.storage, acl, skey);
                return Ok(HandleResponse::default());

            }
            let error_message = String::from("Not authorized to create file");
            return Err(StdError::generic_err(error_message));
        },
        Err(_e) => {
            return Err(StdError::generic_err(error_message));
        }
    }

    


}

fn parent_path(mut path: String) -> String{

    if path.ends_with('/') {
        path.pop();
    }
    let split = path.split("/");
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
    skey: String
) -> StdResult<HandleResponse> {
    
    let ha = deps.api.human_address(&deps.api.canonical_address(&env.message.sender)?)?;

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

    let ha = deps.api.human_address(&deps.api.canonical_address(&env.message.sender)?)?;
    debug_print!("Attempting to create multiple files for account: {}", ha.clone());

    for i in 0..contents_list.len() {

        let file_contents = contents_list[i].clone();
        let path = paths[i].to_string();
        let pkey = &pkeys[i];
        let skey = &skeys[i];

        let res = do_create_file(deps, ha.to_string(), file_contents, path, pkey.to_string(), skey.to_string());

        match res {
            Ok(_r) => {

            },
            Err(e) => {
                return Err(e);
            }
        }
    }

    Ok(HandleResponse::default())
}

pub fn create_file<'a, S: Storage>(store: &'a mut S, owner: String, path: String, contents: String) {

    let file = make_file(&owner, &contents);

    bucket_save_file(store, path, file); 

}

pub fn make_file(owner: &str, contents: &str) -> File{
    File {
        contents: String::from(contents),
        owner: String::from(owner),
        public: false,
        allow_read_list: OrderedSet::<String>::new(),
        allow_write_list: OrderedSet::<String>::new()
    }
}

pub fn bucket_save_file<'a, S: Storage>( store: &'a mut S, path: String, folder: File ) {
    let bucket_response = bucket(FILE_LOCATION, store).save(&path.as_bytes(), &folder);
    match bucket_response {
        Ok(bucket_response) => bucket_response,
        Err(e) => panic!("Bucket Error: {}", e)
    }
}

pub fn bucket_remove_file<'a, S: Storage>( store: &'a mut S, path: String) {
    bucket::<S, File>(FILE_LOCATION, store).remove(&path.as_bytes());
}

pub fn file_exists<'a, S: Storage>( store: &'a mut S, path: String) -> bool{
    let f : Result<File, StdError> = bucket(FILE_LOCATION, store).load(&path.as_bytes());

    match f {
        Ok(_v) => {return true},
        Err(_e) => return false,
    };
}

pub fn bucket_load_file<'a, S: Storage>( store: &'a mut S, path: &String) -> File{
    bucket(FILE_LOCATION, store).load(&path.as_bytes()).unwrap()
}

pub fn bucket_load_readonly_file<'a, S: Storage>( store: &'a S, path: String ) -> Result<File, StdError>{
    bucket_read(FILE_LOCATION, store).load(&path.as_bytes())
}

// QueryMsg
pub fn query_file<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, path: String, behalf: &HumanAddr) -> StdResult<FileResponse> {


    let f = bucket_load_readonly_file(&deps.storage, path);

    match f {
        Ok(f1) => {

            if f1.can_read(String::from(behalf.as_str())) {
                return Ok(FileResponse { file: f1 });
            }

            let error_message = String::from("Sorry bud! Unauthorized to read file.");
            return Err(StdError::generic_err(error_message))
        },

        Err(_err) => {
            let error_message = String::from("Error querying file.");
            return Err(StdError::generic_err(error_message))
        }
    }

    
}

