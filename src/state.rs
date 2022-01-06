use std::any::type_name;

use schemars::JsonSchema;
use secret_toolkit::serialization::{Bincode2, Serde};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;

// use std::collections::HashMap;

use cosmwasm_std::{Storage, HumanAddr, StdResult, StdError, ReadonlyStorage, CanonicalAddr};
use cosmwasm_storage::{ReadonlyPrefixedStorage, PrefixedStorage};

use crate::viewing_key::ViewingKey;

// use crate::backend::{Folder, File};

pub static CONFIG_KEY: &[u8] = b"config";
pub const PREFIX_VIEWING_KEY: &[u8] = b"viewingkey";

// static API_NAME: &str = "API";

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct State {
    pub owner: HumanAddr,
    pub prng_seed: Vec<u8>,
}

pub fn save<T: Serialize, S: Storage>(storage: &mut S, key: &[u8],value: &T) -> StdResult<()> {
    storage.set(key, &Bincode2::serialize(value)?);
    Ok(())
}

pub fn load<T: DeserializeOwned, S: ReadonlyStorage>(storage: &S, key: &[u8]) -> StdResult<T> {
    Bincode2::deserialize(
        &storage
            .get(key)
            .ok_or_else(|| StdError::not_found(type_name::<T>()))?,
    )
}

pub fn write_viewing_key<S: Storage>(store: &mut S, owner: &CanonicalAddr, key: &ViewingKey) {
    let mut user_key_store = PrefixedStorage::new(PREFIX_VIEWING_KEY, store);
    user_key_store.set(owner.as_slice(), &key.to_hashed());
}

pub fn read_viewing_key<S: Storage>(store: &S, owner: &CanonicalAddr) -> Option<Vec<u8>> {
    let user_key_store = ReadonlyPrefixedStorage::new(PREFIX_VIEWING_KEY, store);
    user_key_store.get(owner.as_slice())
}


// OLD Schtuff

// pub fn config<S: Storage>(storage: &mut S) -> Singleton<S, State> {
//     singleton(storage, CONFIG_KEY)
// }

// pub fn config_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, State> {
//     singleton_read(storage, CONFIG_KEY)
// }
