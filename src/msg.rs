use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::CanonicalAddr;
use std::collections::HashMap;
use crate::backend::{Folder, File, WrappedAddress, traverse_folders, get_folder, build_child, build_file, move_file, move_folder, remove_file, remove_folder, print_file, print_folder, make_file, make_folder, add_folder, add_file};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub home_folders: HashMap<WrappedAddress, Folder>,
    pub api_keys: HashMap<WrappedAddress, String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    InitAddress { seed_phrase: String },
    // CreateFolder { name : String, path: Vec<String> },
    // CreateFile { name: String, path: Vec<String>  },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    // GetCount {},
}

// // We define a custom struct for each query response
// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
// pub struct CountResponse {
//     pub count: i32,
// }
