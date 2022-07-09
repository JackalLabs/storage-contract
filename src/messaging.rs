use std::convert::TryInto;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{ Storage, HumanAddr, StdResult, StdError, HandleResponse, Api, Querier, Extern, Env, debug_print};
use cosmwasm_storage::{ PrefixedStorage, ReadonlyPrefixedStorage, bucket, bucket_read};
use secret_toolkit::storage::{AppendStore, AppendStoreMut};

use crate::msg::MessageResponse;
use crate::backend::WalletInfo;

//Attach to message_list_counter (in wallet info) to help implement delete_all_messages()
const PREFIX_MSGS_RECEIVED: &[u8] = b"MESSAGES_RECEIVED";

//namespace for every user's wallet info bucket
static WALLET_INFO_LOCATION: &[u8] = b"WALLET_INFO";

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

    pub fn store_message<S: Storage, A: Api, Q: Querier>(&self, deps: &mut Extern<S, A, Q>, to: &HumanAddr) -> StdResult<()>{
        append_message(deps, &self, to)
    }

    //returns length of the collection that this message belongs in. Used for testing
    pub fn len<S: Storage, A: Api, Q: Querier>(deps: &mut Extern<S, A, Q>, for_address: &HumanAddr) -> u32 {

        let wrapped_msg_namespace = get_msg_list_namespace(deps, &for_address.to_string());
        //If the string below is "invalid namespace", then you will get a length of 0
        let msg_namespace = return_namespace_string(wrapped_msg_namespace);

        let store = ReadonlyPrefixedStorage::multilevel(
            &[msg_namespace.as_bytes(), for_address.0.as_bytes()],
            &deps.storage
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
pub fn append_message<S: Storage, A: Api, Q: Querier> (
    deps: &mut Extern<S, A, Q>,
    message: &Message,
    for_address: &HumanAddr,
) -> StdResult<()>{

    let wrapped_msg_namespace = get_msg_list_namespace(deps, &for_address.to_string());
    let msg_namespace = return_namespace(wrapped_msg_namespace)?;

    let option_error_message = format!("Provided storage doesn't seem like an AppendStore");
    let mut store = PrefixedStorage::multilevel(&[msg_namespace.as_bytes(), for_address.0.as_bytes()], &mut deps.storage);
    let mut store = AppendStoreMut::attach(&mut store).unwrap_or(Err(StdError::generic_err(option_error_message)))?;

    store.push(message)
}

pub fn create_empty_collection<S: Storage, A: Api, Q: Querier> (
    deps: &mut Extern<S, A, Q>,
    for_address: &HumanAddr,
) -> StdResult<HandleResponse>{

    let wrapped_msg_namespace = get_msg_list_namespace(deps, &for_address.to_string());
    let msg_namespace = return_namespace(wrapped_msg_namespace)?;

    let mut store = PrefixedStorage::multilevel(
        &[msg_namespace.as_bytes(), for_address.0.as_bytes()],
        &mut deps.storage
    );
    let _store = AppendStoreMut::<Message, _, _>::attach_or_create(&mut store)?;
    Ok(HandleResponse::default())
}

pub fn collection_exist<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    for_address: &HumanAddr,

) -> bool{

    let wrapped_msg_namespace = get_msg_list_namespace(&deps, &for_address.to_string());
    //If the string below is "invalid namespace", this function will return false by design
    let msg_namespace = return_namespace_string(wrapped_msg_namespace);

    let store = ReadonlyPrefixedStorage::multilevel(
        &[msg_namespace.as_bytes(), for_address.0.as_bytes()],
        &deps.storage
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

pub fn get_collection_owner<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    behalf: &HumanAddr,
) -> StdResult<String> {

    let wrapped_msg_namespace = get_msg_list_namespace(&deps, &behalf.to_string());
    let msg_namespace = return_namespace(wrapped_msg_namespace)?;

    let option_error_message = format!("Provided storage doesn't seem like an AppendStore");
    let mut store = ReadonlyPrefixedStorage::multilevel(&[msg_namespace.as_bytes(), behalf.0.as_bytes()], &deps.storage);
    let store = AppendStore::<Message, _, _>::attach(&mut store).unwrap_or(Err(StdError::generic_err(option_error_message)))?;

    //retrieve message at index 0 which holds the owner of the collection
    let message = store.get_at(0)?;
    let owner = message.get_owner();

    Ok(String::from(owner))

}

pub fn get_messages<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    behalf: &HumanAddr,

) -> StdResult<Vec<Message>> {

    let wrapped_msg_namespace = get_msg_list_namespace(&deps, &behalf.to_string());
    let msg_namespace = return_namespace(wrapped_msg_namespace)?;

    let store = ReadonlyPrefixedStorage::multilevel(
        &[msg_namespace.as_bytes(), behalf.0.as_bytes()],
        &deps.storage
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

pub fn delete_all_messages<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    let ha = deps
        .api
        .human_address(&deps.api.canonical_address(&env.message.sender)?)?;
    let adr = String::from(ha.as_str());

    let load_bucket: Result<WalletInfo, StdError> =
        bucket_read(WALLET_INFO_LOCATION, &deps.storage).load(adr.as_bytes());
    let mut wallet_info = load_bucket?;

    let new_counter = wallet_info.message_list_counter + 1;
    wallet_info.message_list_counter = new_counter;

    bucket(WALLET_INFO_LOCATION, &mut deps.storage)
        .save(ha.as_str().as_bytes(), &wallet_info)
        .map_err(|err| println!("{:?}", err))
        .ok();

    let _empty_list = create_empty_collection(deps, &env.message.sender);
    let dummy_message = Message::new(String::from("Placeholder contents created by you"), String::from(env.message.sender.as_str()));
    let _appended_message = append_message(deps, &dummy_message, &env.message.sender);
    Ok(HandleResponse::default())
}

// handle
pub fn send_message<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: &Env,
    to: HumanAddr,
    contents: &String,
) -> StdResult<HandleResponse> {

    let message = Message::new(String::from(contents), env.message.sender.to_string());

    let already_init = collection_exist(deps, &to);
    //if "to" does not have a collection yet, the owner of this dummy message will be to because it will be placed
    //in the collection that this function makes for them
    let dummy_message = Message::new(String::from("Dummy contents created by someone else"), String::from(to.as_str()));

    match already_init{
        false => {
            //if recipient does not have a list, make one for them. We let them make their own viewing key. - how to notify that they need to make one?
            let _storage_space = create_empty_collection(deps, &to);
            let _dummy_messages = append_message(deps, &dummy_message, &to);
            let _saved_message = append_message(deps, &message, &to);
            debug_print(format!("message stored successfully to {}", to));
            Ok(HandleResponse::default())
        }
        true => {

            message.store_message(deps, &to)?;
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

    let owner = get_collection_owner(&deps, &behalf)?;

    if owner == behalf.to_string() {
        let msgs = get_messages(
            deps,
            &behalf,
        )?;
        _messages = msgs
    } else {
        return Err(StdError::generic_err("Can only query your own messages!"));
    }

    Ok(MessageResponse {messages: _messages})
}


pub fn get_msg_list_namespace<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, address: &String) -> StdResult<String> {

    //It's possible that a user could send a message to an address that has not been initiated, so we use may_load.
    let loaded_wallet: Result<Option<WalletInfo>, StdError> = bucket_read(WALLET_INFO_LOCATION, &deps.storage).may_load(address.as_bytes());
    let unwrapped_wallet = loaded_wallet.expect("wallet not found");
    let wallet_info = match unwrapped_wallet {
        Some(wallet_info) => wallet_info,
        None => WalletInfo { init: false, namespace: "empty".to_string(), counter: 0, message_list_counter: 0 },
    };

    Ok(format!("{:#?}{}",PREFIX_MSGS_RECEIVED, wallet_info.message_list_counter))

}

pub fn return_namespace(wrapped_namespace: StdResult<String>) -> StdResult<String> {

    let namespace = match wrapped_namespace {
        Ok(namespace) => Ok(namespace),
        Err(_) => Err(StdError::NotFound { kind: String::from("Unable to load namespace for your list of messages"), backtrace: None }),
    };
    namespace

}

pub fn return_namespace_string(wrapped_namespace: StdResult<String>) -> String {

    let namespace = match wrapped_namespace {
        Ok(namespace) => namespace,
        Err(_) => String::from("invalid namespace"),
    };
    namespace

}

// pub fn return_wallet(x: Option<WalletInfo>) -> WalletInfo {

//     match x {
//         Some(i) => i,//if exists, their wallet init could be false or true, and their namespace is present,
//         //If none, it means the user has never called init before, so we return a wallet info that can be altered and saved right away
//         None => WalletInfo { init: false, namespace: "empty".to_string(), counter: 0, message_list_counter: 0 },

//     }
// }

// pub fn get_counter<'a, S: Storage>(store: &'a S, sender: &String) -> StdResult<i32> {
//     let loaded_wallet: Result<WalletInfo, StdError> = bucket_read(WALLET_INFO_LOCATION, store).load(sender.as_bytes());
//     let unwrapped_wallet = loaded_wallet?;
//     Ok(unwrapped_wallet.counter)
// }

// pub fn delete_all_messages<S: Storage, A: Api, Q: Querier>(
//     deps: &mut Extern<S, A, Q>,
//     env: Env

// ) -> StdResult<HandleResponse> {
//     // Try to access the collection for the account.
//     // If it doesn't exist yet, return an empty collection.
//     let option_error_message = format!("Provided storage doesn't seem like an AppendStore");
//     let mut store = PrefixedStorage::multilevel(&[PREFIX_MSGS_RECEIVED, env.message.sender.0.as_bytes()], &mut deps.storage);
//     let mut store = AppendStoreMut::<Message, _, _>::attach(&mut store).unwrap_or(Err(StdError::generic_err(option_error_message)))?;
//     // let mut store = AppendStoreMut::<Message, _, _>::attach(&mut store).set_length(0);

//     //store.clear();

//     let dummy_message = Message::new(String::from("Placeholder contents created by you"), String::from(env.message.sender.as_str()));
//     let _appending_message = append_message(deps, &dummy_message, &env.message.sender);

//     Ok(HandleResponse::default())
// }

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