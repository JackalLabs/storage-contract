use std::convert::TryInto;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{ Storage, HumanAddr, StdResult, StdError, ReadonlyStorage, HandleResponse, Api, Querier, Extern, Env, debug_print};
use cosmwasm_storage::{ PrefixedStorage, ReadonlyPrefixedStorage };
use secret_toolkit::storage::{AppendStore, AppendStoreMut};

use crate::msg::MessageResponse;

const PREFIX_MSGS_RECEIVED: &[u8] = b"messages_received"; //A prefix to make namespace longer

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Debug, Clone)]
pub struct Message{
    contents: String,
    owner: String
}

impl Message {

    pub fn new(contents: String, owner: String) -> Self {
        Self {
            contents,
            owner,
        }
    }

    pub fn get_contents(&self) -> &str {
        &self.contents
    }

    pub fn get_owner(&self) -> &str {
        &self.owner
    }

    pub fn store_message<S:Storage>(&self, store: &mut S, to: &HumanAddr) -> StdResult<()>{
        append_message(store, &self, to)
    }

    //returns length of the collection that this message belongs in. Used for testing
    pub fn len<S: ReadonlyStorage>(storage: &S,
                                   for_address: &HumanAddr) -> u32 {
        let store = ReadonlyPrefixedStorage::multilevel(
            &[PREFIX_MSGS_RECEIVED, for_address.0.as_bytes()],
            storage
        );
        let store = AppendStore::<Message, _, _>::attach(&store);
        let store = if let Some(result) = store {
            if result.is_err() {
                return 0;
            } else {
                result.unwrap()
            }
        } else {
            return 0;
        };

        return store.len();
    }
}

//see notes below regarding AppendStore
pub fn append_message<S: Storage> (
    store: &mut S,
    message: &Message,
    for_address: &HumanAddr,
) -> StdResult<()>{

    let option_error_message = format!("Provided storage doesn't seem like an AppendStore");
    let mut store = PrefixedStorage::multilevel(&[PREFIX_MSGS_RECEIVED, for_address.0.as_bytes()], store);
    let mut store = AppendStoreMut::attach(&mut store).unwrap_or(Err(StdError::generic_err(option_error_message)))?;

    store.push(message)
}

pub fn create_empty_collection<S: Storage> (
    store: &mut S,
    for_address: &HumanAddr,
) -> StdResult<HandleResponse>{

    let mut store = PrefixedStorage::multilevel(
        &[PREFIX_MSGS_RECEIVED, for_address.0.as_bytes()],
        store
    );
    let _store = AppendStoreMut::<Message, _, _>::attach_or_create(&mut store)?;
    Ok(HandleResponse::default())
}

pub fn collection_exist<'a, S: Storage>(
    store: &'a mut S,
    for_address: &HumanAddr,

) -> bool{

    let store = ReadonlyPrefixedStorage::multilevel(
        &[PREFIX_MSGS_RECEIVED, for_address.0.as_bytes()],
        store
    );

    // Try to access the storage of files for the account.
    let store = AppendStore::<Message, _, _>::attach(&store);
    let store = if let Some(result) = store {
        result
    } else {
        return false
    };

    match store {
        Ok(_v) => {return true},
        Err(_e) => return false,
    };
}

pub fn get_collection_owner<S: ReadonlyStorage>(
    storage: &S,
    behalf: &HumanAddr,
) -> StdResult<String> {

    let option_error_message = format!("Provided storage doesn't seem like an AppendStore");
    let mut store = ReadonlyPrefixedStorage::multilevel(&[PREFIX_MSGS_RECEIVED, behalf.0.as_bytes()], storage);
    let store = AppendStore::<Message, _, _>::attach(&mut store).unwrap_or(Err(StdError::generic_err(option_error_message)))?;

    //retrieve message at index 0 which holds the owner of the collection
    let message = store.get_at(0)?;
    let owner = message.get_owner();

    Ok(String::from(owner))

}

pub fn get_messages<S: ReadonlyStorage>(
    storage: &S,
    behalf: &HumanAddr,

) -> StdResult<Vec<Message>> {
    let store = ReadonlyPrefixedStorage::multilevel(
        &[PREFIX_MSGS_RECEIVED, behalf.0.as_bytes()],
        storage
    );

    // Try to access the collection for the account.
    // If it doesn't exist yet, return an empty collection.
    let store = AppendStore::<Message, _, _>::attach(&store);
    let store = if let Some(result) = store {
        result?
    } else {
        return Ok(vec![]);
    };

    let tx_iter = store
        .iter()
        .take(store.len().try_into().unwrap());

    let txs: StdResult<Vec<Message>> = tx_iter
        .map(|tx| tx)
        .collect();
        txs.map(|txs| (txs))
}

// fn get_message<S: ReadonlyStorage>(
//     storage: &S,
//     for_address: &HumanAddr,
//     position: u32
// ) -> StdResult<Message> {

//     let store = ReadonlyPrefixedStorage::multilevel(
//         &[PREFIX_MSGS_RECEIVED, for_address.0.as_bytes()],
//         storage
//     );

//     // Try to access the storage of files for the account.
//     // If it doesn't exist yet, return a Message with path called "Does Not Exist"
//     let store = AppendStore::<Message, _, _>::attach(&store);

//     let store = if let Some(result) = store {
//         result?
//     } else {
//         return Ok(Message::new(String::from("Does Not Exist/"), String::from("None")))
//     };

//     store.get_at(position)
// }

pub fn delete_all_messages<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env

) -> StdResult<HandleResponse> {
    // Try to access the collection for the account.
    // If it doesn't exist yet, return an empty collection.
    let option_error_message = format!("Provided storage doesn't seem like an AppendStore");
    let mut store = PrefixedStorage::multilevel(&[PREFIX_MSGS_RECEIVED, env.message.sender.0.as_bytes()], &mut deps.storage);
    let mut store = AppendStoreMut::<Message, _, _>::attach(&mut store).unwrap_or(Err(StdError::generic_err(option_error_message)))?;
    // let mut store = AppendStoreMut::<Message, _, _>::attach(&mut store).set_length(0);

    // store.clear();

    let dummy_message = Message::new(String::from("Placeholder contents"), String::from(env.message.sender.as_str()));
    let _appending_message = append_message(&mut deps.storage, &dummy_message, &env.message.sender);

    Ok(HandleResponse::default())
}

// handle
pub fn send_message<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    to: HumanAddr,
    contents: String,
) -> StdResult<HandleResponse> {

    let message = Message::new(String::from(contents), env.message.sender.to_string());

    let already_init = collection_exist(&mut deps.storage, &to);
    //if "to" does not have a collection yet, the owner of this dummy message will be to because it will be placed
    //in the collection that this function makes for them
    let dummy_message = Message::new(String::from("Dummy_contents.jpg"), String::from(to.as_str()));

    match already_init{
        false => {
            //if recipient does not have a list, make one for them. We let them make their own viewing key. - how to notify that they need to make one?
            let _storage_space = create_empty_collection(&mut deps.storage, &to);
            let _dummy_messages = append_message(&mut deps.storage, &dummy_message, &to);
            let _saved_message = append_message(&mut deps.storage, &message, &to);
            debug_print(format!("message stored successfully to {}", to));
            Ok(HandleResponse::default())
        }
        true => {

            message.store_message(&mut deps.storage, &to)?;
            debug_print(format!("message stored successfully to {}", to));
            Ok(HandleResponse::default())
        }
        }
}

//query
pub fn query_messages<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    behalf: &HumanAddr,
) -> StdResult<MessageResponse> {

    let mut _messages: Vec<Message> = Vec::new();

    let owner = get_collection_owner(&deps.storage, &behalf)?;

    if owner == behalf.to_string() {
        let msgs = get_messages(
            &deps.storage,
            &behalf,
        )?;
        _messages = msgs
    } else {
        return Err(StdError::generic_err("Can only query your own messages!"));
    }


    Ok(MessageResponse {messages: _messages})
}