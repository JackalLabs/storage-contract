use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::CanonicalAddr;
use cosmwasm_std::HumanAddr;

use std::collections::HashMap;

use crate::backend::{Folder, File};

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct InitMsg {
    // pub home_folders: HashMap<HumanAddr, Folder>,
    // pub api_keys: HashMap<HumanAddr, String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    InitAddress { seed_phrase: String },
    // CreateFolder { name : String, path: Vec<String> },
    CreateFile { name: String, contents: String, path: String },
    CreateFolder {name : String, path: String},
    TestCreateFolder {name : String, path: String, address: String},
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    // GetCount {},
    GetFile { address: String, path: String },
    GetFolderContents {address: String, path: String},
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
