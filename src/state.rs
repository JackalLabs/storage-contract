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
pub const PREFIX_CONFIG: &[u8] = b"config";
pub const KEY_CONSTANTS: &[u8] = b"constants";

// static API_NAME: &str = "API";

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct State {
    pub owner: HumanAddr,
    pub prng_seed: Vec<u8>,
}

// Config

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct Constants {
    pub admin: HumanAddr,
    pub prng_seed: Vec<u8>,
}
pub struct ReadonlyConfig<'a, S: ReadonlyStorage> {
    storage: ReadonlyPrefixedStorage<'a, S>,
}

impl<'a, S: ReadonlyStorage> ReadonlyConfig<'a, S> {
    pub fn from_storage(storage: &'a S) -> Self {
        Self {
            storage: ReadonlyPrefixedStorage::new(PREFIX_CONFIG, storage),
        }
    }

    fn as_readonly(&self) -> ReadonlyConfigImpl<ReadonlyPrefixedStorage<S>> {
        ReadonlyConfigImpl(&self.storage)
    }

    pub fn constants(&self) -> StdResult<Constants> {
        self.as_readonly().constants()
    }
}

pub struct Config<'a, S: Storage> {
    storage: PrefixedStorage<'a, S>,
}

impl<'a, S: Storage> Config<'a, S> {
    pub fn from_storage(storage: &'a mut S) -> Self {
        Self {
            storage: PrefixedStorage::new(PREFIX_CONFIG, storage),
        }
    }

    fn as_readonly(&self) -> ReadonlyConfigImpl<PrefixedStorage<S>> {
        ReadonlyConfigImpl(&self.storage)
    }

    pub fn constants(&self) -> StdResult<Constants> {
        self.as_readonly().constants()
    }

    pub fn set_constants(&mut self, constants: &Constants) -> StdResult<()> {
        set_bin_data(&mut self.storage, KEY_CONSTANTS, constants)
    }
}

fn ser_bin_data<T: Serialize>(obj: &T) -> StdResult<Vec<u8>> {
    bincode2::serialize(&obj).map_err(|e| StdError::serialize_err(type_name::<T>(), e))
}
fn set_bin_data<T: Serialize, S: Storage>(storage: &mut S, key: &[u8], data: &T) -> StdResult<()> {
    let bin_data = ser_bin_data(data)?;

    storage.set(key, &bin_data);
    Ok(())
}

struct ReadonlyConfigImpl<'a, S: ReadonlyStorage>(&'a S);

impl<'a, S: ReadonlyStorage> ReadonlyConfigImpl<'a, S> {
    fn constants(&self) -> StdResult<Constants> {
        let consts_bytes = self
            .0
            .get(KEY_CONSTANTS)
            .ok_or_else(|| StdError::generic_err("no constants stored in configuration"))?;
        bincode2::deserialize::<Constants>(&consts_bytes)
            .map_err(|e| StdError::serialize_err(type_name::<Constants>(), e))
    }
}

//OLD

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

// Viewing Keys

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
