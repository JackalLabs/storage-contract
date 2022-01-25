use cosmwasm_std::HumanAddr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{backend::File, viewing_key::ViewingKey};

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct InitMsg {
    // pub home_folders: HashMap<HumanAddr, Folder>,
    // pub api_keys: HashMap<HumanAddr, String>,
    pub prng_seed: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    InitAddress {  },
    CreateFile { name: String, contents: String, path: String },
    RemoveFile {name: String, path: String},
    MoveFile {name: String, old_path: String, new_path: String},
    CreateFolder {name : String, path: String},
    RemoveFolder {name : String, path: String},
    MoveFolder {name : String, old_path: String, new_path: String},
    CreateViewingKey {entropy: String, padding: Option<String>},
    AllowRead {path: String, address: String},
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetFile { address: HumanAddr, behalf: HumanAddr, path: String, key: String },
    GetFolderContents {address: HumanAddr, behalf: HumanAddr, path: String, key: String},
    GetBigTree {address: HumanAddr, behalf: HumanAddr, path: String, key: String},
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum HandleAnswer {
    DefaultAnswer { status:ResponseStatus},
    CreateViewingKey { key: ViewingKey },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FileResponse {
    pub file: File,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FolderContentsResponse {
    pub parent: String,
    pub folders: Vec<String>,
    pub files: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BigTreeResponse {
    pub data: Vec<String>,
}

impl QueryMsg {
    pub fn get_validation_params(&self) -> (Vec<&HumanAddr>, ViewingKey) {
        match self {
            Self::GetFile { behalf, key, .. } => (vec![behalf], ViewingKey(key.clone())),
            Self::GetFolderContents { behalf, key, .. } => (vec![behalf], ViewingKey(key.clone())),
            Self::GetBigTree { behalf, key, .. } => (vec![behalf], ViewingKey(key.to_string())),
            // _ => panic!("This query type does not require authentication"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    Success,
    Failure,
}