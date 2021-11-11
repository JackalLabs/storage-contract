use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use cosmwasm_std::{CanonicalAddr, Storage};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};

use crate::backend::{Folder, File, WrappedAddress, traverse_folders, get_folder, build_child, build_file, move_file, move_folder, remove_file, remove_folder, print_file, print_folder, make_file, make_folder, add_folder, add_file};

pub static CONFIG_KEY: &[u8] = b"config";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub home_folders: HashMap<WrappedAddress, Folder>,
    pub api_keys: HashMap<WrappedAddress, String>,
    pub owner: WrappedAddress,
}

pub fn config<S: Storage>(storage: &mut S) -> Singleton<S, State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, State> {
    singleton_read(storage, CONFIG_KEY)
}
