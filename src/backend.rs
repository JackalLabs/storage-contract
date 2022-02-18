use std::vec;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{debug_print,Env, Api, Querier, ReadonlyStorage, Storage, StdResult, StdError, Extern, HandleResponse, HumanAddr};
use cosmwasm_storage::{ bucket, bucket_read, Bucket, ReadonlyBucket};


use crate::ordered_set::{OrderedSet};
use crate::msg::{FileResponse, FolderContentsResponse, BigTreeResponse};
use crate::nodes::{write_claim};

// use crate::viewing_key::ViewingKey;

static FOLDER_LOCATION: &[u8] = b"FOLDERS";
static FILE_LOCATION: &[u8] = b"FILES";

// KEEP IN MIND!!!
// Root Folder is user wallet address ex: "secret420rand0mwall3t6969/" is a root folder
// FOLDER always ends with '/' ex: "secret420rand0mwall3t6969/meme_folder/"
// FILE does NOT ends with '/' ex: "secret420rand0mwall3t6969/meme_folder/beautiful_pepe.jpeg"


// HandleMsg::InitAddress
pub fn try_init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {

    let ha = deps.api.human_address(&deps.api.canonical_address(&env.message.sender)?)?;
    let mut adr = String::from(ha.clone().as_str());

    let folder = make_folder(&adr, &adr, &adr);

    adr.push_str("/");

    let already_init = folder_exists(&mut deps.storage, adr.to_string());
    
    match already_init{
        false => {
            bucket_save_folder(&mut deps.storage, adr, folder);
            debug_print!("init root folder success");
            Ok(HandleResponse::default())
        }
        true => {
            let error_message = format!("User has already been initiated");
            Err(StdError::generic_err(error_message))
        }
    }

}

// HandleMsg FOLDER 
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct Folder{
    parent: String,
    child_folder_names: OrderedSet<String>,
    files: OrderedSet<String>,
    name: String,
    owner: String,
    public: bool,
    allow_read_list: OrderedSet<String>,
    allow_write_list: OrderedSet<String>
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

impl Folder {
    pub fn list_files(&self) -> Vec<String>{
        let mut files: Vec<String> = Vec::new();

        for i in 0..self.files.len() {
            files.push(String::from(self.files.get(i).unwrap()));
        }

        return files;
    }

    pub fn list_folders(&self) -> Vec<String>{
        let mut folders: Vec<String> = Vec::new();

        for i in 0..self.child_folder_names.len() {
            folders.push(String::from(self.child_folder_names.get(i).unwrap()));
        }

        return folders;
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

impl PartialEq<Folder> for Folder {
    fn eq(&self, other: &Folder) -> bool {
        self.name == other.name
    }
}

pub fn try_move_folder<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    name: String,
    old_path: String,
    new_path: String,
) -> StdResult<HandleResponse> {

    let ha = deps.api.human_address(&deps.api.canonical_address(&env.message.sender)?)?;

    debug_print!("Attempting to move folder from `{}` to `{}` for account: {}", old_path.clone() , new_path.clone() , ha.clone());

    try_create_folder(deps, env.clone(), name.clone(), new_path)?;
    try_remove_folder(deps, env, name, old_path)?;

    Ok(HandleResponse::default())
}

pub fn try_remove_folder<S: Storage, A: Api, Q: Querier>( // TODO: change this to accept people who have write permissions for the folder not just the account.
    deps: &mut Extern<S, A, Q>,
    env: Env,
    name: String,
    path: String,
) -> StdResult<HandleResponse> {

    let ha = deps.api.human_address(&deps.api.canonical_address(&env.message.sender)?)?;


    let adr = String::from(ha.clone().as_str());

    let parent_path = format!("{}{}", adr, path);

    let child_path = format!("{}{}{}/",adr, path, name); // "anyone/layer_1/"
    

    // Load PARENT FOLDER from bucket
    let mut load_from_bucket = bucket_load_folder(&mut deps.storage, parent_path.clone());
    // println!("Load from bucket: {:?}", load_from_bucket);

    // Remove CHILD FOLDER from PARENT FOLDER
    load_from_bucket.child_folder_names.remove(child_path.clone());
    // println!("here --- {:?}", load_from_bucket);

    // SAVE new ver of PARENT FOLDER to bucket
    bucket_save_folder(&mut deps.storage, parent_path, load_from_bucket);


    // REMOVE CHILD FOLDER from bucket
    remove_children_from_folder(&mut deps.storage, child_path);

    Ok(HandleResponse::default())
}

/** 
 This function will remove ALL folders and files within the target path
*/
fn remove_children_from_folder<'a, S: Storage>(store: &'a mut S, path: String) {

    let mop = bucket_load_readonly_folder(store, path.clone());
    
    match mop {
        Ok(top) => {
            let folders_found = top.child_folder_names.to_vec();
            let files_found = top.files.to_vec();
            
            // Remove folders loop
            if folders_found.len() > 0 {
                let iter = folders_found.iter();
                
                for val in iter {
                    let k = val.to_string(); 
                    remove_children_from_folder(store, k) 
                }
            }
            
            // Remove files within this folder
            if files_found.len() > 0 {
                let iter = files_found.iter();
                
                for val in iter {
                    let file_path = val.to_string(); 
                    bucket_remove_file(store, file_path);
                }
            }
        
            remove_folder_by_path(store, path);
        },

        Err(_err) => {
            return;
        }
    }

}

fn remove_folder_by_path<'a, S: Storage>(store: &'a mut S, path: String) {
    bucket_remove_folder(store, path);
}

pub fn try_allow_read<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    path: String,
    address: String,
) -> StdResult<HandleResponse> {
    let owner_address = deps.api.human_address(&deps.api.canonical_address(&env.message.sender)?)?.to_string();

    let mut proper_path = owner_address.clone();
    proper_path.push_str(&path);

    let last_char = &proper_path.chars().last().unwrap();

    if *last_char == '/'{
        let mut f = bucket_load_folder(&mut deps.storage, String::from(&proper_path));
    
        f.allow_read(address);
    
        bucket_save_folder(&mut deps.storage, String::from(&proper_path), f);

        Ok(HandleResponse::default())

    } else {
        let mut f = bucket_load_file(&mut deps.storage, &proper_path);

        f.allow_read(address);

        bucket_save_file(&mut deps.storage, proper_path, f);
        Ok(HandleResponse::default())
    }
}

pub fn try_disallow_read<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    path: String,
    address: String,
) -> StdResult<HandleResponse> {
    let owner_address = deps.api.human_address(&deps.api.canonical_address(&env.message.sender)?)?.to_string();

    let mut proper_path = owner_address.clone();
    proper_path.push_str(&path);

    let last_char = &proper_path.chars().last().unwrap();

    if *last_char == '/'{
        let mut f = bucket_load_folder(&mut deps.storage, String::from(&proper_path));
    
        f.disallow_read(address);
    
        bucket_save_folder(&mut deps.storage, String::from(&proper_path), f);

        Ok(HandleResponse::default())

    } else {
        let mut f = bucket_load_file(&mut deps.storage, &proper_path);

        f.disallow_read(address);

        bucket_save_file(&mut deps.storage, proper_path, f);
        Ok(HandleResponse::default())
    }
}


pub fn try_create_folder<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    name: String,
    path: String,
) -> StdResult<HandleResponse> {

    let ha = deps.api.human_address(&deps.api.canonical_address(&env.message.sender)?)?;
    debug_print!("Attempting to create folder for account: {}", ha.clone());

    let adr = String::from(ha.clone().as_str());

    let mut p = adr.clone();
    p.push_str(&path);

    // println!("{:}", p.clone());

    let mut l = bucket_load_folder(&mut deps.storage, p.clone());
    // println!("LOAD BUCKET before creating file: {:?}", l);

    let path_to_compare = &mut p.clone();
    path_to_compare.push_str(&name);
    path_to_compare.push('/');

    let folder_name_taken = folder_exists(&mut deps.storage, path_to_compare.to_string());

    match folder_name_taken{
        false => {
            create_folder(&mut deps.storage, &mut l, p.clone(), name);
            bucket_save_folder(&mut deps.storage, p.clone(), l);
            debug_print!("create file success");
            Ok(HandleResponse::default())
        }
        true => {
            let error_message = format!("Folder name '{}' has already been taken", name);
            Err(StdError::generic_err(error_message))
        }
    }

}

pub fn create_folder<'a, S: Storage>(store: &'a mut S, root: &mut Folder, path: String, name: String) {
    let folder = make_folder(&name, &name, "");

    add_folder(store, root, path, folder);

}

pub fn make_folder(parent: &str,name: &str, owner: &str) -> Folder{
    Folder {
        parent: String::from(parent),
        child_folder_names: OrderedSet::<String>::new(),
        files: OrderedSet::<String>::new(),
        name: String::from(name),
        owner: String::from(owner),
        public: false,
        allow_read_list: OrderedSet::<String>::new(),
        allow_write_list: OrderedSet::<String>::new()
    }
}

pub fn add_folder<'a, S: Storage>(store: &'a mut S, parent : &mut Folder, path: String, mut child: Folder){
    child.owner = parent.owner.clone();

    // let mut child_path = path.clone();
    // child_path.push_str(&child.name);
    // child_path.push('/');
    let child_path = format!("{}{}/",path,child.name);

    parent.child_folder_names.push(child_path.clone());

    bucket_save_folder(store, child_path.clone(), child);
}

pub fn bucket_save_folder<'a, S: Storage>( store: &'a mut S, path: String, folder: Folder ) {
    let bucket_response = bucket(FOLDER_LOCATION, store).save(&path.as_bytes(), &folder);
    match bucket_response {
        Ok(bucket_response) => bucket_response,
        Err(e) => panic!("Bucket Error: {}", e)
    }

}

pub fn bucket_remove_folder<'a, S: Storage>( store: &'a mut S, path: String) {
    bucket::<S, Folder>(FOLDER_LOCATION, store).remove(&path.as_bytes());
}

pub fn folder_exists<'a, S: Storage>( store: &'a mut S, path: String) -> bool{
    let f : Result<Folder, StdError> = bucket(FOLDER_LOCATION, store).load(&path.as_bytes());

    match f {
        Ok(_v) => {return true},
        Err(_e) => return false,
    };
}

pub fn bucket_load_folder<'a, S: Storage>( store: &'a mut S, path: String) -> Folder{
    bucket(FOLDER_LOCATION, store).load(&path.as_bytes()).unwrap()
}

pub fn bucket_load_readonly_folder<'a, S: Storage>( store: &'a S, path: String) -> Result<Folder, StdError>{
    bucket_read(FOLDER_LOCATION, store).load(&path.as_bytes())

}

pub fn write_folder<'a, S: Storage>(
    store: &'a mut S,
) -> Bucket<'a, S, Folder> {
    bucket(FOLDER_LOCATION, store)
}

pub fn read_folder<'a, S: ReadonlyStorage>(
    store: &'a S,
) -> ReadonlyBucket<'a, S, Folder> {
    bucket_read(FOLDER_LOCATION, store)
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

pub fn try_create_file<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    contents: String,
    path: String,
    pkey: String,
    skey: String
) -> StdResult<HandleResponse> {

  
    let file_name_taken = file_exists(&mut deps.storage, path.to_string());

    match file_name_taken{
        false => {
            create_file(&mut deps.storage, path.to_string(), contents);

            let ha = deps.api.human_address(&deps.api.canonical_address(&env.message.sender)?)?;
            let adr = String::from(ha.clone().as_str());
            let mut acl = adr.clone();
            acl.push_str(&pkey);

            write_claim(&mut deps.storage, acl, skey);
            
            debug_print!("create file success");
            Ok(HandleResponse::default())
        }
        true => {
            let error_message = format!("File name '{}' has already been taken", path);
            Err(StdError::generic_err(error_message))
        },
    }

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

    let adr = String::from(ha.clone().as_str());

    for i in 0..contents_list.len() {

        let file_contents = contents_list[i].clone();


        let path = paths[i];
    
        
        let file_name_taken = file_exists(&mut deps.storage, path.to_string());

        let pkey = &pkeys[i];
        let skey = &skeys[i];

        match file_name_taken{
            false => {
                create_file(&mut deps.storage, path.to_string(), file_contents);
                debug_print!("create file success");

                let mut acl = adr.clone();
                acl.push_str(&pkey);
                write_claim(&mut deps.storage, acl, skey.to_string());
                // Ok(HandleResponse::default())
            }
            true => {
                let _error_message = format!("File name '{}' has already been taken", path);
                // Err(StdError::generic_err(error_message))
            },
        }
    }

    Ok(HandleResponse::default())
}

pub fn create_file<'a, S: Storage>(store: &'a mut S, path: String, contents: String) {

    let file = make_file("", &contents);

    add_file(store, path, file); 

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

pub fn add_file<'a, S: Storage>(store: &'a mut S, path: String, mut child: File){
    bucket_save_file(store, path, child);
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

// pub fn bucket_load_readonly_file<'a, S: Storage>( store: &'a S, path: String ) -> Option<File>{
//     let load = bucket_read(FILE_LOCATION, store).load(&path.as_bytes());
//     let load = match load {
//         Ok(File) => {
//             return load.unwrap()
//         },
//         Err(E) => panic!("Can't find file in bucket")
//     };
// }

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

pub fn query_folder_contents<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, address: &HumanAddr, path: String, behalf: &HumanAddr) -> StdResult<FolderContentsResponse> {

    let adr = address.as_str();
    let query_path = format!("{}{}",adr,&path);

    let f = bucket_load_readonly_folder(&deps.storage, query_path);

    match f {
        Ok(f1) => {
            if f1.can_read(String::from(behalf.as_str())) {
                let parent = &f1.parent;
        
                return Ok(FolderContentsResponse { parent: parent.to_string(), folders: f1.list_folders(), files: f1.list_files() });
            }

            let error_message = String::from("Sorry bud! Unauthorized to read folder.");
            return Err(StdError::generic_err(error_message))
        },

        Err(_err) => {
            let error_message = String::from("Error querying folder.");
            return Err(StdError::generic_err(error_message))
        }
    }
    
}

pub fn query_big_tree<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    address: HumanAddr,
    key: String
) 
-> StdResult<BigTreeResponse>
{
    let value = query_folder_contents(deps, &address, String::from("/"), &address).unwrap();
    let folders_from_root = &value.folders;
    let big_vec = scan_folders(&deps, &address, folders_from_root.to_vec(), key);

    Ok(BigTreeResponse{ folders: big_vec.0, files: big_vec.1 })
}


fn scan_folders<S: Storage, A: Api, Q: Querier> (
    deps: &Extern<S, A, Q>,
    address: &HumanAddr,
    folders_to_scan: Vec<String>,
    vk: String,
) -> (Vec<String>, Vec<String>) {
    let mut temp_vec = vec![];
    let mut files_vec = vec![]; 
    for folder in folders_to_scan {

        // get proper path 
        let owner_address = address.to_string();
        let proper_path = folder.replace(&owner_address, "");

        // calling query to get folders
        let value = query_folder_contents(deps, &HumanAddr(owner_address.clone()), proper_path.to_string(),  &HumanAddr(owner_address.clone())).unwrap();
        let folders_to_scan_next = &value.folders;
        let folder_is_empty = folders_to_scan_next.is_empty();
        
        // Getting Files
        let files_in_here = value.files;
        for each in files_in_here {
            files_vec.push(each);
        }

        let hashie = format!("{}:{:?}",folder, folders_to_scan_next);

        let _ = &temp_vec.push(hashie);

        if !folder_is_empty{
            // continue loop
            let this = scan_folders(deps, address, folders_to_scan_next.to_vec(), vk.clone());
            for each_folder in this.0 {
                let _ = &temp_vec.push(each_folder);
            }
            
            for each_file in this.1{
                let _ = &files_vec.push(each_file);
            }
        }

    }
     (temp_vec, files_vec)
}

// MISC.

// fn get_folder_from_path<'a, S: Storage>(store: &'a mut S, root: &'a mut Folder, path: Vec<String>) -> String{

//     if path.len() > 1 {

//         let mut f = root.child_folder_names.clone();
//         let mut s = path[0].clone();
        
//         for i in 1..path.len() {
//             for x in 0..f.len() {
//                 if f.get(x).unwrap() == &path[i]  {
//                     f = bucket_load_folder(store, path[i].clone()).child_folder_names.clone();
//                     s = path[i].clone();
//                 }
//             }
//         }

//         return s;
        

//     }

//     if path.len() == 1 {

//         for x in 0..root.child_folder_names.len() {
//             if root.child_folder_names.get(x).unwrap() == &path[0]  {
//                 return path[0].clone();
//             }
//         }

//     }

//     return path[0].clone();


// }

// fn vec_to_string(path: Vec<String>) -> String {
//     let mut s: String = String::from(&path[0]);
//     if path.len() > 1 {
//         for i in 1..path.len()-1 {
//             s.push_str(&path[i]);
//         }
//     }

//     s

// }

// fn path_to_vec(path: String) -> Vec<String> {
//     let split = path.split("/");
//     let vec: Vec<&str> = split.collect();

//     let mut vec2: Vec<String> = Vec::new();

//     for i in 0..vec.len() {
//         vec2.push(String::from(vec[i]));
//     }

//     vec2

// }


// pub fn add_folder(parent : &mut Folder, mut child: Folder){
//     child.owner = parent.owner.clone();

//     for (_, entry) in child.child_folders.iter_mut() {
//         entry.owner = parent.owner.clone();
//     }

//     parent.child_folders.insert(child.name, child);
// }

// pub fn print_folder(folder : &Folder, level : u16){
//     if level > 0 {
//         for _i in 0..(level - 1){
//             print!("     ");
//         }
//         print!("\\");
//         print!("----");
//     }
    
//     println!("> {}", folder.name);
    

//     for f in folder.child_folders.iter(){
//         print_folder(f, level + 1);
//     }

//     for file in folder.files.iter(){
//         print_file(file, level + 1);
//     }
// }

// pub fn print_file(file : &File, level : u16){
//     if level > 0 {
//         for _i in 0..(level - 1){
//             print!("     ");
//         }
//         print!("\\");
//         print!("----");
//     }
    
//     println!("> {}", file.name);
// }

// pub fn remove_folder(parent: &mut Folder, name : &str) -> Folder {
//     let mut x = 0;
//     for (_, entry) in parent.child_folders.iter_mut() {
//         if entry.name.eq(name) {
//             break;
//         }
//         x += 1;
//     }

//     parent.child_folders.remove(x)

// }

// pub fn remove_file(parent: &mut Folder, name : &str) -> File {
//     let mut x = 0;
//     for entry in parent.files.iter_mut() {
//         if entry.name.eq(name) {
//             break;
//         }
//         x += 1;
//     }

//     parent.files.remove(x)

// }

// pub fn move_folder(parent: &mut Folder, name: &str, new_parent: &mut Folder){
//     let child = remove_folder(parent, name);
//     add_folder(new_parent, child);
// }

// pub fn move_file(parent: &mut Folder, name: &str, new_parent: &mut Folder){
//     let child = remove_file(parent, name);
//     add_file(new_parent, child);
// }

// pub fn build_file<'a, S: Storage>(store: &'a mut S, parent : &mut Folder, name : &str, contents: &str){
//     let mut f = make_file(name, &parent.owner.clone(), contents);
//     add_file(store, parent, f);
// }

// pub fn build_child(parent : &mut Folder, name : &str){
//     let mut f = make_folder(name, &parent.owner.clone());
//     add_folder(parent, f);
// }

// pub fn get_folder<'a>(parent: &'a mut Folder, name : &str) -> &'a mut Folder {
//     let mut x = 0;
//     for entry in parent.child_folders.iter_mut() {
//         if entry.name.eq(name) {
//             break;
//         }
//         x += 1;
//     }

//     &mut parent.child_folders[x]
// }

// pub fn mut_get_file<'a>(parent: &'a mut Folder, name : &str) -> &'a mut File {
//     let mut x = 0;
//     for entry in parent.files.iter_mut() {
//         if entry.name.eq(name) {
//             break;
//         }
//         x += 1;
//     }

//     &mut parent.files[x]
// }


// pub fn mut_traverse_to_file<'a>(parent: &'a mut Folder, path: Vec<&str>) -> &'a mut File {

//     if path.len() > 1 {
//         return mut_get_file(traverse_folders(parent, path[0..path.len() - 1].to_vec()), path[path.len() - 1]);
//     }

//     mut_get_file(parent, path[0])
// }

// pub fn traverse_to_file<'a>(parent: &'a mut Folder, path: Vec<&str>) -> File {

//     if path.len() > 1 {
//         return get_file(traverse_folders(parent, path[0..path.len() - 1].to_vec()), path[path.len() - 1]);
//     }

//     get_file(parent, path[0])
// }

// pub fn traverse_folders<'a>(parent: &'a mut Folder, path: Vec<&str>) -> &'a mut Folder {

//     let mut folder = parent;

//     for i in 0..path.len() {
//         let f = path[i];
//         folder = get_folder(folder, f);
//     }

//     folder

// }

// fn main(){
//     println!("Starting test...\n\n");

//     let mut f = make_folder("root", "me");
//     let mut c = make_folder("child1", "me2");
//     let mut c2 = make_folder("child2", "me3");
//     let mut c3 = make_folder("child3", "me3");

//     build_child(&mut c, "testing");

//     build_file(traverse_folders(&mut c, vec!["testing"]), "f2.txt", "wow this is awesome.");

//     build_file(&mut c, "f.txt", "wow this is awesome.");


//     let mut f2 = make_folder("root2", "me4");

//     add_folder(&mut c, c2);
//     add_folder(&mut c, c3);
//     add_folder(&mut f, c);

//     print_folder(&mut f, 0);
//     print_folder(&mut f2, 0);

//     move_folder(&mut f, "child1", &mut f2);

//     println!("\nAfter move... \n");
//     print_folder(&mut f, 0);
//     print_folder(&mut f2, 0);

//     println!("\nGrabbing a nested folder... \n");

//     print_folder(traverse_folders(&mut f2, vec!["child1", "testing"]), 0);


// }