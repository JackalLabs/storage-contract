use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
// use cosmwasm_std::CanonicalAddr;
// use cosmwasm_std::HumanAddr;

// use std::collections::HashMap;

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
    InitAddress { seed_phrase: String },
    CreateFile { name: String, contents: String, path: String },
    RemoveFile {name: String, path: String},
    MoveFile {name: String, old_path: String, new_path: String},
    CreateFolder {name : String, path: String},
    RemoveFolder {name : String, path: String},
    MoveFolder {name : String, old_path: String, new_path: String},
    CreateViewingKey {entropy: String, padding: Option<String>},
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    // GetCount {},
    GetFile { address: String, path: String },
    GetFolderContents {address: String, path: String},
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum HandleAnswer {
    CreateViewingKey {key: ViewingKey},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FileResponse {
    pub file: File,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FolderContentsResponse {
    pub folders: Vec<String>,
    pub files: Vec<String>,
}
