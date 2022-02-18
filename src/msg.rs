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
    InitAddress { contents: String },
    Create {contents: String, path: String , pkey: String, skey: String},
    CreateMulti { contents_list: Vec<String>, path_list: Vec<String>, pkeys: Vec<String>, skeys: Vec<String> },
    Remove {path: String},
    Move {old_path: String, new_path: String},
    CreateViewingKey {entropy: String, padding: Option<String>},
    AllowRead {path: String, address: String},
    DisallowRead {path: String, address: String},
    InitNode {ip: String, address: String},
    ClaimReward {path: String, key: String, address: String},
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetContents { behalf: HumanAddr, path: String, key: String },
    GetNodeIP {index: u64},
    GetNodeListSize {},
    GetNodeList{size: u64},
    GetNodeCoins{address: String},
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
    pub folders: Vec<String>,
    pub files: Vec<String>,
}

impl QueryMsg {
    pub fn get_validation_params(&self) -> (Vec<&HumanAddr>, ViewingKey) {
        match self {
            Self::GetContents { behalf, key, .. } => (vec![behalf], ViewingKey(key.clone())),
            _ => panic!("This query type does not require authentication"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    Success,
    Failure,
}