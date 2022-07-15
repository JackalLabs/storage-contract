// use schemars::JsonSchema;
// use serde::{Deserialize, Serialize};
// use std::ptr::null;

use cosmwasm_std::{
    debug_print, to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier,
    QueryResult, StdError, StdResult, Storage,
};

use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::nodes::{claim, get_node, get_node_size, pub_query_coins, push_node, set_node_size};
use crate::state::{read_viewing_key, save, State, CONFIG_KEY};
use crate::viewing_key::VIEWING_KEY_SIZE;

#[cfg(test)]
mod tests {
    // use std::vec;
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary, HumanAddr};

    use crate::backend::make_file;
    use crate::contract::{init, handle, query};
    use crate::messaging::Message;
    use crate::msg::{FileResponse, HandleAnswer, MessageResponse, WalletInfoResponse};
    use crate::viewing_key::ViewingKey;

    fn init_for_test<S: Storage, A: Api, Q: Querier>(
        deps: &mut Extern<S, A, Q>,
        address: String,
    ) -> ViewingKey {
        // Init Contract
        let msg = InitMsg {
            prng_seed: String::from("lets init bro"),
        };
        let env = mock_env("creator", &[]);
        let _res = init(deps, env, msg).unwrap();

        // Init Address and Create ViewingKey
        let env = mock_env(String::from(&address), &[]);
        let msg = HandleMsg::InitAddress {
            contents: String::from("{}"),
            entropy: String::from("Entropygoeshereboi"),
        };
        let handle_response = handle(deps, env, msg).unwrap();

        match from_binary(&handle_response.data.unwrap()).unwrap() {
            HandleAnswer::CreateViewingKey { key } => key,
            _ => panic!("Unexpected result from handle"),
        }
    }

    #[test]
    fn write_perms_and_notify (){
        let mut deps = mock_dependencies(20, &[]);
        let _vk = init_for_test(&mut deps, String::from("anyone"));
        let vk2 = init_for_test(&mut deps, String::from("alice"));
        let vk3 = init_for_test(&mut deps, String::from("bob"));

        // Create file
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Create {
            contents: String::from("pepe"),
            path: String::from("anyone/pepe.jpg"),
            pkey: String::from("test"),
            skey: String::from("test"),
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Create 3 Files: phrog1.png, phrog2.png, phrog3.png
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateMulti {
            contents_list: vec![String::from("phrog1"), String::from("phrog2"), String::from("phrog3")],
            path_list: vec![
                String::from("anyone/phrog1.png"),
                String::from("anyone/phrog2.png"),
                String::from("anyone/phrog3.png")
            ],
            pkey_list: vec![String::from("test"), String::from("test"), String::from("test")],
            skey_list: vec![String::from("test"), String::from("test"), String::from("test")],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Add alice and bob to pepe.jpg's allow write permissions
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::AllowWrite {
            path: String::from("anyone/pepe.jpg"),
            message: String::from("anyone has given you write access to [ anyone/pepe.jpg ]"),
            address_list: vec![String::from("alice"), String::from("bob")],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Add alice and bob to phrog1.png's allow write permissions
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::AllowWrite {
            path: String::from("anyone/phrog1.png"),
            message: String::from("anyone has given you write access to [ anyone/phrog1.png ]"),
            address_list: vec![String::from("alice"), String::from("bob")],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Add alice and bob to phrog2.png's allow write permissions
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::AllowWrite {
            path: String::from("anyone/phrog2.png"),
            message: String::from("anyone has given you write access to [ anyone/phrog2.png ]"),
            address_list: vec![String::from("alice"), String::from("bob")],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Disallow WRITE for Alice and Bob
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::DisallowWrite {
            path: String::from("anyone/pepe.jpg"),
            message: String::from("anyone has revoked write access to [ anyone/pepe.jpg ]"),
            notify: true,
            address_list: vec![
                String::from("alice"),
                String::from("bob"),
            ],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Disallow WRITE for Alice and Bob
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::DisallowWrite {
            path: String::from("anyone/phrog1.png"),
            message: String::from("anyone has revoked write access to [ anyone/phrog1.png ]"),
            notify: false,
            address_list: vec![
                String::from("alice"),
                String::from("bob"),
            ],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Disallow WRITE for Alice and Bob
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::DisallowWrite {
            path: String::from("anyone/phrog2.png"),
            message: String::from("anyone has revoked write access to [ anyone/phrog2.png ]"),
            notify: true,
            address_list: vec![
                String::from("alice"),
                String::from("bob"),
            ],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Add alice and bob to phrog3.png's allow write permissions
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::AllowWrite {
            path: String::from("anyone/phrog3.png"),
            message: String::from("anyone has given you write access to [ anyone/phrog3.png ]"),
            address_list: vec![String::from("alice"), String::from("bob")],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Reset Write
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::ResetWrite {
            path: String::from("anyone/phrog3.png"),
            message: String::from("anyone has reset write access to anyone/phrog3.png"),
            notify: true //can change to false to show that the message was not sent
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Change Owner

        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::ChangeOwner {
            path: String::from("anyone/phrog3.png"),
            message: String::from("anyone has given you ownership of anyone/phrog3.png"),
            new_owner: String::from("alice"),
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Query Messages for alice
        let query_res = query(&deps, QueryMsg::GetMessages { behalf: HumanAddr("alice".to_string()), key: vk2.to_string() },).unwrap();
        let value: MessageResponse = from_binary(&query_res).unwrap();
        println!("Alice's messages --> {:#?}", value.messages);

        // Query Messages for bob
        let query_res = query(&deps, QueryMsg::GetMessages { behalf: HumanAddr("bob".to_string()), key: vk3.to_string() },).unwrap();
        let value: MessageResponse = from_binary(&query_res).unwrap();
        println!("Bob's messages --> {:#?}", value.messages);

        //delete all messages for alice
        let env = mock_env("alice", &[]);
        let msg = HandleMsg::DeleteAllMessages {};
        let res = handle(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        //delete all messages for bob
        let env = mock_env("bob", &[]);
        let msg = HandleMsg::DeleteAllMessages {};
        let res = handle(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Query Messages for alice should now just show the dummy message
        let query_res = query(&deps, QueryMsg::GetMessages { behalf: HumanAddr("alice".to_string()), key: vk2.to_string() },).unwrap();
        let value: MessageResponse = from_binary(&query_res).unwrap();
        println!("Alice's messages after deletion --> {:#?}", value.messages);

        // Query Messages for bob should now just show the dummy message
        let query_res = query(&deps, QueryMsg::GetMessages { behalf: HumanAddr("bob".to_string()), key: vk3.to_string() },).unwrap();
        let value: MessageResponse = from_binary(&query_res).unwrap();
        println!("Bob's messages after deletion --> {:#?}", value.messages);

    }

}
