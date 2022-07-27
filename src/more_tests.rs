#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        Api, Extern, Querier, Storage,
    };

    use crate::msg::{HandleMsg, InitMsg, QueryMsg};

    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary, HumanAddr};

    use crate::backend::File;
    use crate::contract::{init, handle, query};
    use crate::messaging::Message;
    use crate::msg::{FileResponse, HandleAnswer, MessageResponse};
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
            contents_list: vec![String::from("root contents"), String::from("sub_folder_1 contents"), String::from("sub_folder_2 contents"), String::from("sub_folder_3 contents")],
            path_list: vec![String::from("movies/"), String::from("memes/"), String::from("work/")],
            entropy: String::from("Entropygoeshereboi"),
        };
        let handle_response = handle(deps, env, msg).unwrap();

        match from_binary(&handle_response.data.unwrap()).unwrap() {
            HandleAnswer::CreateViewingKey { key } => key,
            _ => panic!("Unexpected result from handle"),
        }
    }

    #[test]
    fn change_owner_and_move_test() {
        let mut deps = mock_dependencies(20, &[]);
        let _vk = init_for_test(&mut deps, String::from("anyone"));
        let vk2 = init_for_test(&mut deps, String::from("alice"));

        // Create 3 folders (test/ meme_folder/ pepe/)
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateMulti {
            contents_list: vec![
                String::from("<content inside test/>"),
                String::from("<content inside meme_folder/>"),
                String::from("<content inside junior/>"),
            ],
            path_list: vec![
                String::from("anyone/test/"),
                String::from("anyone/meme_folder/"),
                String::from("anyone/junior/"),
            ]
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Create 2 Files bunny1.png and bunny2.png
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateMulti {
            contents_list: vec![String::from("bunny1"), String::from("bunny2")],
            path_list: vec![
                String::from("anyone/test/bunny1.png"),
                String::from("anyone/test/bunny2.png"),
            ]
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Given ownership of bunny1.png to alice
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::ChangeOwner {
            path: String::from("anyone/test/bunny1.png"),
            message: String::from("anyone has given you ownership of anyone/test/bunny1.png"),
            new_owner: String::from("alice"),
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Get bunny1 with alice's viewing key to ensure alice is now owner
        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("anyone/test/bunny1.png"),
                behalf: HumanAddr("alice".to_string()),
                key: vk2.to_string(),
            },
        );
        let value: FileResponse = from_binary(&query_res.unwrap()).unwrap();
        println!("alice owns bunny1:\n {:#?}", value.file);

        // alice tries to move bunny1 to anyone/meme_folder, which will fail because she does not own anyone/
        let env = mock_env("alice", &[]);
        let msg = HandleMsg::Move {
            old_path: String::from("anyone/test/bunny1.png"),
            new_path: String::from("anyone/meme_folder/bunny1.png"),
        };
        let res = handle(&mut deps, env, msg);
        assert!(res.is_err());
        println!(
            "Alice fails to move bunny1 to anyone/meme_folder:\n {:#?}",
            res
        );

        // lets make a folder inside of alice's root directory to store her new bunny in
        let env = mock_env("alice", &[]);
        let msg = HandleMsg::Create {
            contents: "bunnys go here".to_string(),
            path: String::from("alice/bunny_home/")
        };
        let _res = handle(&mut deps, env, msg).unwrap();
        println!("Successfully created alice/bunny_home/:\n {:#?}", _res);

        // Get alice/bunny_home/ with Alice's viewing key to ensure it exists
        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("alice/bunny_home/"),
                behalf: HumanAddr("alice".to_string()),
                key: vk2.to_string(),
            },
        );
        let value: FileResponse = from_binary(&query_res.unwrap()).unwrap();
        println!("alice/bunny_home:\n {:#?}", value.file);

        //now alice can move bunny1 into alice/bunny_home/
        let env = mock_env("alice", &[]);
        let msg = HandleMsg::Move {
            old_path: String::from("anyone/test/bunny1.png"),
            new_path: String::from("alice/bunny_home/bunny1.png"),
        };
        let res = handle(&mut deps, env, msg).unwrap();
        println!(
            "Alice successfully moves bunny1 to alice/bunny_home/:\n {:#?}",
            res
        );

        // Get bunny1 with alice's viewing key to ensure it is in alice/bunny_home
        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("alice/bunny_home/bunny1.png"),
                behalf: HumanAddr("alice".to_string()),
                key: vk2.to_string(),
            },
        );
        let value: FileResponse = from_binary(&query_res.unwrap()).unwrap();
        println!("bunny1 is in alice/bunny_home:\n {:#?}", value);

        // Try to query "anyone/test/bunny1.png" to ensure it's no longer there
        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("anyone/test/bunny1.png"),
                behalf: HumanAddr("alice".to_string()),
                key: vk2.to_string(),
            },
        );
        assert!(query_res.is_err());
        println!(
            "Confirming that 'anyone/test/bunny1.png' no longer contains a file:\n{:#?}",
            query_res
        );
    }

    #[test]

    //The idea behind giving someone write permissions is to allow someone else to add files to a folder that you own?
    fn alice_moving_within_anyone_rootfolder() {
        let mut deps = mock_dependencies(20, &[]);
        let _vk = init_for_test(&mut deps, String::from("anyone"));
        let vk2 = init_for_test(&mut deps, String::from("alice"));

        // Create 2 folders (test/, junior/)
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::CreateMulti {
            contents_list: vec![
                String::from("<content inside test/>"),
                String::from("<content inside junior/>"),
            ],
            path_list: vec![String::from("anyone/test/"), String::from("anyone/junior/")]
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Create bunny.png
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Create {
            contents: String::from("bunny"),
            path: String::from("anyone/test/bunny.png")
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Given ownership of bunny.png to alice
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::ChangeOwner {
            path: String::from("anyone/test/bunny.png"),
            message: String::from("anyone has given you ownership of anyone/test/bunny.png"),
            new_owner: String::from("alice"),
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Get bunny with alice's viewing key to ensure alice is now owner
        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("anyone/test/bunny.png"),
                behalf: HumanAddr("alice".to_string()),
                key: vk2.to_string(),
            },
        );
        let value: FileResponse = from_binary(&query_res.unwrap()).unwrap();
        println!("alice owns bunny:\n {:#?}", value.file);

        // alice tries to move bunny to anyone/junior, which will fail because she does not own or have write access to anyone/junior/
        let env = mock_env("alice", &[]);
        let msg = HandleMsg::Move {
            old_path: String::from("anyone/test/bunny.png"),
            new_path: String::from("anyone/junior/bunny.png"),
        };
        let res = handle(&mut deps, env, msg);
        assert!(res.is_err());
        println!("Alice fails to move bunny to anyone/junior:\n {:#?}", res);

        // add alice to write permissions of anyone/junior/
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::AllowWrite {
            path: "anyone/junior/".to_string(),
            message: String::from("anyone has given you write access to [anyone/junior/]"),
            address_list: vec!["alice".to_string()],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Get anyone/junior/ with alice's viewing key to ensure she has write access
        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("anyone/junior/"),
                behalf: HumanAddr("alice".to_string()),
                key: vk2.to_string(),
            },
        );
        let value: FileResponse = from_binary(&query_res.unwrap()).unwrap();
        println!("alice has write access to anyone/junior/:\n {:#?}", value.file);

        // alice again tries to move bunny to anyone/junior, will succeed
        let env = mock_env("alice", &[]);
        let msg = HandleMsg::Move {
            old_path: String::from("anyone/test/bunny.png"),
            new_path: String::from("anyone/junior/bunny.png"),
        };
        let res = handle(&mut deps, env, msg).unwrap();
        println!(
            "Alice successfully moves bunny to anyone/junior:\n {:#?}",
            res
        );

        // Get bunny with alice's viewing key to ensure it is in anyone/junior
        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("anyone/junior/bunny.png"),
                behalf: HumanAddr("alice".to_string()),
                key: vk2.to_string(),
            },
        );
        let value: FileResponse = from_binary(&query_res.unwrap()).unwrap();
        println!("bunny is in anyone/junior:\n {:#?}", value.file);

        // Try to query "anyone/test/bunny.png" to ensure it's no longer there
        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("anyone/test/bunny.png"),
                behalf: HumanAddr("alice".to_string()),
                key: vk2.to_string(),
            },
        );
        assert!(query_res.is_err());
        println!(
            "Confirming that 'anyone/test/bunny.png' no longer contains a file:\n{:#?}",
            query_res
        );
    }

    #[test]
    fn permission_test() {
        let mut deps = mock_dependencies(20, &[]);
        let vk = init_for_test(&mut deps, String::from("anyone"));
        let vk2 = init_for_test(&mut deps, String::from("alice"));

        // Create Folder Test
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Create {
            contents: String::from("<content of test/ folder>"),
            path: String::from("anyone/test/")
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Create File
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Create {
            contents: String::from("I'm sad"),
            path: String::from("anyone/pepe.jpg")
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Allow WRITE for Alice, Bob and Charlie
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::AllowWrite {
            path: String::from("anyone/test/"),
            message: String::from("anyone has given you write access to [anyone/test/]"),
            address_list: vec![
                String::from("alice"),
                String::from("bob"),
                String::from("charlie"),
            ],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Allow READ for Alice, Bob and Charlie
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::AllowRead {
            path: String::from("anyone/test/"),
            message: String::from("anyone has given you read access to [ anyone/test ]"),
            address_list: vec![
                String::from("alice"),
                String::from("bob"),
                String::from("charlie"),
            ],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Get File with Alice's viewing key
        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("anyone/test/"),
                behalf: HumanAddr("alice".to_string()),
                key: vk2.to_string(),
            },
        );
        assert!(query_res.is_ok());
        let value: FileResponse = from_binary(&query_res.unwrap()).unwrap();
        println!("alice has full permissions to anyone/test/:\n {:#?}", value.file);

        // DISAllOW WRITE for Alice, Bob and Charlie
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::DisallowWrite {
            path: String::from("anyone/test/"),
            message: String::from("anyone has revoked write access to [anyone/test/]"),
            notify: true,
            address_list: vec![
                String::from("alice"),
                String::from("bob"),
                String::from("charlie"),
            ],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // DISAllow READ for Alice, Bob and Charlie
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::DisallowRead {
            path: String::from("anyone/test/"),
            message: String::from("anyone has revoked read access to [ anyone/test ]"),
            notify: true,
            address_list: vec![
                String::from("alice"),
                String::from("bob"),
                String::from("charlie"),
            ],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Get File with Anyone's viewing key
        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("anyone/test/"),
                behalf: HumanAddr("anyone".to_string()),
                key: vk.to_string(),
            },
        );
        let value: FileResponse = from_binary(&query_res.unwrap()).unwrap();
        let test = File::new("anyone", "<content of test/ folder>");
        assert_eq!(test, value.file);
        println!("permissions disallowed for anyone/test/:\n {:#?}", value.file);


    }

    #[test]
    fn test_owner_change() {
        let mut deps = mock_dependencies(20, &[]);
        let vk_anyone = init_for_test(&mut deps, String::from("anyone"));
        let vk_alice = init_for_test(&mut deps, String::from("alice"));

        // Create File
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Create {
            contents: String::from("Rainbows"),
            path: String::from("anyone/test/")
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Get File with viewing key to see the owner
        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("anyone/test/"),
                behalf: HumanAddr("anyone".to_string()),
                key: vk_anyone.to_string(),
            },
        )
        .unwrap();
        let value: FileResponse = from_binary(&query_res).unwrap();
        println!("See owner --> {:#?}", value.file);

        // Change owner. At the moment, only anyone (the owner) can do this
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::ChangeOwner {
            path: String::from("anyone/test/"),
            message: String::from("anyone has given you ownership of anyone/test/"),
            new_owner: String::from("alice"),
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Now alice can query "anyone/test/" but anyone cannot.
        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("anyone/test/"),
                behalf: HumanAddr("alice".to_string()),
                key: vk_alice.to_string(),
            },
        )
        .unwrap();
        let value: FileResponse = from_binary(&query_res).unwrap();
        println!(
            "Only alice can query 'anyone/test/' See owner --> {:#?}",
            value.file
        );

        // Query File as anyone will fail
        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("anyone/test/"),
                behalf: HumanAddr("anyone".to_string()),
                key: vk_anyone.to_string(),
            },
        );
        assert!(query_res.is_err());

        // alice can add Anyone to allow_read
        let env = mock_env("alice", &[]);
        let msg = HandleMsg::AllowRead {
            path: String::from("anyone/test/"),
            message: String::from("alice has given you read access to [ anyone/test ]"),
            address_list: vec![String::from("anyone")],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Now anyone can also read file
        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("anyone/test/"),
                behalf: HumanAddr("anyone".to_string()),
                key: vk_anyone.to_string(),
            },
        )
        .unwrap();
        let value: FileResponse = from_binary(&query_res).unwrap();
        println!("alice added anyone to allow_read, and now anyone can also read the file. See allow_read list --> {:#?}", value.file);

        // alice can change owner back to anyone
        let env = mock_env("alice", &[]);
        let msg = HandleMsg::ChangeOwner {
            path: String::from("anyone/test/"),
            message: String::from("alice has given you ownership of anyone/test/"),
            new_owner: String::from("anyone"),
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        let query_res = query(
            &deps,
            QueryMsg::GetContents {
                path: String::from("anyone/test/"),
                behalf: HumanAddr("anyone".to_string()),
                key: vk_anyone.to_string(),
            },
        )
        .unwrap();
        let value: FileResponse = from_binary(&query_res).unwrap();
        println!("alice gave ownership back to anyone --> {:#?}", value.file);
    }

    // Tests for Messaging
    #[test]
    fn send_messages_and_query() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));
        let vk_anyone = init_for_test(&mut deps, String::from("anyone"));

        //Changing 'nuggie' to 'anyone' will cause error: "user has already been initiated!"
        let vk_nuggie = init_for_test(&mut deps, String::from("nuggie"));

        //sending a message to anyone's address
        let env = mock_env("sender", &[]);
        let msg = HandleMsg::SendMessage {
            to: HumanAddr("anyone".to_string()),
            contents: "Hello: sender has shared Pepe.jpg with you".to_string(),
        };
        let res = handle(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        //sending another message to anyone's address
        let env = mock_env("sender", &[]);
        let msg = HandleMsg::SendMessage {
            to: HumanAddr("anyone".to_string()),
            contents: "Hello: sender has shared Hasbullah.jpg with you".to_string(),
        };
        let res = handle(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Query Anyone's Messages
        let query_res = query(
            &deps,
            QueryMsg::GetMessages {
                behalf: HumanAddr("anyone".to_string()),
                key: vk_anyone.to_string(),
            },
        )
        .unwrap(); //changing viewing key causes error
        let value: MessageResponse = from_binary(&query_res).unwrap();
        println!("All messages --> {:#?}", value.messages);

        let length = Message::len(&mut deps, &HumanAddr::from("anyone"));
        println!("Length of anyone's collection is {}\n", length);
        assert_eq!(3, length);

        //Query with a different viewing key will fail
        let query_res = query(
            &deps,
            QueryMsg::GetMessages {
                behalf: HumanAddr("anyone".to_string()),
                key: vk_nuggie.to_string(),
            },
        ); //changing viewing key causes error
        assert!(query_res.is_err());

        //sending a message to nuggie's address
        let env = mock_env("sender", &[]);
        let msg = HandleMsg::SendMessage {
            to: HumanAddr("nuggie".to_string()),
            contents: "Sender/pepe.jpg".to_string(),
        };
        let res = handle(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Query Nuggies's Messages
        let query_res = query(
            &deps,
            QueryMsg::GetMessages {
                behalf: HumanAddr("nuggie".to_string()),
                key: vk_nuggie.to_string(),
            },
        )
        .unwrap(); //changing viewing key causes error
        let value: MessageResponse = from_binary(&query_res).unwrap();
        println!("All messages --> {:#?}", value.messages);

        let length = Message::len(&mut deps, &HumanAddr::from("nuggie"));
        println!("Length of nuggie's collection is {}\n", length);
        assert_eq!(2, length);

        //Using anyone's viewing key to query nuggie's messages will fail
        let query_res = query(
            &deps,
            QueryMsg::GetMessages {
                behalf: HumanAddr("nuggie".to_string()),
                key: vk_anyone.to_string(),
            },
        ); //changing viewing key causes error
        assert!(query_res.is_err());
    }

    #[test]
    fn send_to_uninitiated_address() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        // Init Contract
        let msg = InitMsg {
            prng_seed: String::from("lets init bro"),
        };
        let env = mock_env("creator", &[]);
        let _res = init(&mut deps, env, msg).unwrap();

        //sending a message to anyone's address - anyone has NOT initituate a collection for their address
        let env = mock_env("sender", &[]);
        let msg = HandleMsg::SendMessage {
            to: HumanAddr("anyone".to_string()),
            contents: "Hello: sender has shared Pepe.jpg with you".to_string(),
        };
        let res = handle(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        //SendMessage above will have made a collection for anyone, and placed above message next to dummy message in this collection.

        //sending another message to anyone's address
        let env = mock_env("sender", &[]);
        let msg = HandleMsg::SendMessage {
                to: HumanAddr("anyone".to_string()),
                contents: "Hello: sender has shared Hasbullah.jpg with you".to_string(),
        };
        let res = handle(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        //At this point, anyone has a list, so we only need to create a viewing key for anyone

        // create viewingkey
        let env = mock_env("anyone", &[]);
        let create_vk_msg = HandleMsg::CreateViewingKey {
            entropy: "supbro".to_string(),
            padding: None,
        };
        let handle_response = handle(&mut deps, env, create_vk_msg).unwrap();

        let vk_anyone = match from_binary(&handle_response.data.unwrap()).unwrap() {
            HandleAnswer::CreateViewingKey { key } => {
                // println!("viewing key here: {}",key);
                key
            }
            _ => panic!("Unexpected result from handle"),
        };

        // Query Anyone's Messages
        let query_res = query(
            &deps,
            QueryMsg::GetMessages {
                behalf: HumanAddr("anyone".to_string()),
                key: vk_anyone.to_string(),
            },
        )
        .unwrap(); //changing viewing key causes error
        let value: MessageResponse = from_binary(&query_res).unwrap();
        println!("All messages --> {:#?}", value.messages);

        let length = Message::len(&mut deps, &HumanAddr::from("anyone"));
        assert_eq!(3, length);
        println!("Length of anyone's collection is {}\n", length);
    }

    #[test]
    fn delete_all_messages() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));
        let vk = init_for_test(&mut deps, String::from("anyone"));

        //sending a message to anyone's address
        let env = mock_env("sender", &[]);
        let msg = HandleMsg::SendMessage {
            to: HumanAddr("anyone".to_string()),
            contents: "hey bro".to_string(),
        };
        let res = handle(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        //sending another message to anyone's address
        let env = mock_env("sender", &[]);
        let msg = HandleMsg::SendMessage {
            to: HumanAddr("anyone".to_string()),
            contents: "watcha doing".to_string(),
        };
        let res = handle(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Query Messages
        let query_res = query(&deps, QueryMsg::GetMessages { behalf: HumanAddr("anyone".to_string()), key: vk.to_string() },).unwrap(); //changing viewing key causes error
        let value: MessageResponse = from_binary(&query_res).unwrap();
        println!("All messages --> {:#?}", value.messages);

        let length = Message::len(&mut deps, &HumanAddr::from("anyone"));
        assert_eq!(3, length);
        println!("Length of anyone's collection is {}\n", length);

        //delete all messages
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::DeleteAllMessages {};
        let res = handle(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Query Messages should now only display the dummy message
        let query_res = query(&deps, QueryMsg::GetMessages { behalf: HumanAddr("anyone".to_string()), key: vk.to_string() },).unwrap(); //changing viewing key causes error
        let value: MessageResponse = from_binary(&query_res).unwrap();
        println!("After calling delete. All messages --> {:#?}", value.messages);

        let length = Message::len(&mut deps, &HumanAddr::from("anyone"));
        println!("Length of anyone's collection is {}\n", length);

    }

    #[test]
    fn read_perms_and_notify (){
        let mut deps = mock_dependencies(20, &[]);
        let _vk = init_for_test(&mut deps, String::from("anyone"));
        let vk2 = init_for_test(&mut deps, String::from("alice"));
        let vk3 = init_for_test(&mut deps, String::from("bob"));

        // Create file
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::Create {
            contents: String::from("pepe"),
            path: String::from("anyone/pepe.jpg")
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
            ]
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Add alice and bob to pepe.jpg's allow read permissions
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::AllowRead {
            path: String::from("anyone/pepe.jpg"),
            message: String::from("anyone has given you read access to [ anyone/pepe.jpg ]"),
            address_list: vec![String::from("alice"), String::from("bob")],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Add alice and bob to phrog1.png's allow read permissions
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::AllowRead {
            path: String::from("anyone/phrog1.png"),
            message: String::from("anyone has given you read access to [ anyone/phrog1.png ]"),
            address_list: vec![String::from("alice"), String::from("bob")],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Add alice and bob to phrog2.png's allow read permissions
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::AllowRead {
            path: String::from("anyone/phrog2.png"),
            message: String::from("anyone has given you read access to [ anyone/phrog2.png ]"),
            address_list: vec![String::from("alice"), String::from("bob")],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Disallow READ for Alice and Bob
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::DisallowRead {
            path: String::from("anyone/pepe.jpg"),
            message: String::from("anyone has revoked read access to [ anyone/pepe.jpg ]"),
            notify: true,
            address_list: vec![
                String::from("alice"),
                String::from("bob"),
            ],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Disallow READ for Alice and Bob
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::DisallowRead {
            path: String::from("anyone/phrog1.png"),
            message: String::from("anyone has revoked read access to [ anyone/phrog1.png ]"),
            notify: false,
            address_list: vec![
                String::from("alice"),
                String::from("bob"),
            ],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Disallow READ for Alice and Bob
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::DisallowRead {
            path: String::from("anyone/phrog2.png"),
            message: String::from("anyone has revoked read access to [ anyone/phrog2.png ]"),
            notify: true,
            address_list: vec![
                String::from("alice"),
                String::from("bob"),
            ],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Add alice and bob to phrog3.png's allow read permissions
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::AllowRead {
            path: String::from("anyone/phrog3.png"),
            message: String::from("anyone has given you read access to [ anyone/phrog3.png ]"),
            address_list: vec![String::from("alice"), String::from("bob")],
        };
        let _res = handle(&mut deps, env, msg).unwrap();

        // Reset Read
        let env = mock_env("anyone", &[]);
        let msg = HandleMsg::ResetRead {
            path: String::from("anyone/phrog3.png"),
            message: String::from("anyone has reset read access to anyone/phrog3.png"),
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
            path: String::from("anyone/pepe.jpg")
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
            ]
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
