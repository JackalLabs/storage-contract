use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

use cosmwasm_std::{Storage, HumanAddr};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};

use crate::backend::{Folder, File};

pub static CONFIG_KEY: &[u8] = b"config";

static API_NAME: &str = "API";

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct State {
    pub owner: HumanAddr,
}

pub fn config<S: Storage>(storage: &mut S) -> Singleton<S, State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, State> {
    singleton_read(storage, CONFIG_KEY)
}
