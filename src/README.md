# JACKAL Storage Contract
- [Introduction](#Introduction)
- [Sections](#Sections)
    - [Init](#Init)
    - [Handle](#Handle)
        -  [InitAddress](#--InitAddress)
        -  [Create](#--Create)
        -  [CreateMulti](#--CreateMulti)
        -  [Remove](#--Remove)
        -  [RemoveMulti](#--RemoveMulti)
        -  [MoveMulti](#--MoveMulti)
        -  [Move](#--Move)
        -  [CreateViewingKey](#--CreateViewingKey)
        -  [AllowRead](#--AllowRead)
        -  [DisallowRead](#--DisallowRead)
        -  [ResetRead](#--ResetRead)
        -  [AllowWrite](#--AllowWrite)
        -  [DisallowWrite](#--DisallowWrite)
        -  [ResetWrite](#--ResetWrite)
        -  [InitNode](#--InitNode)
        -  [ClaimReward](#--ClaimReward)
        -  [ForgetMe](#--ForgetMe)
        -  [ChangeOwner](#--ChangeOwner)
        -  [SendMessage](#--SendMessage)
        -  [DeleteAllMessages](#--DeleteAllMessages)
     - [Query](#Query))  
        - [YouUpBro](#--YouUpBro)
        - [GetNodeCoins](#--GetNodeCoins)
        - [GetNodeIP](#--GetNodeIP)
        - [GetNodeList](#--GetNodeList)
        - [GetNodeListSize](#--GetNodeListSize)
        - [Authenticated_Queries](#Authenticated_Queries))
          - [GetContents](#--GetContents)
          - [GetWalletInfo](#--GetWalletInfo)
          - [GetMessages](#--GetMessages)


# Introduction
Contract implementation of JACKAL file system.

# Sections

## Init
This is for instantiating the contract.
|Name|Type|Description|                                                                                       
|--|--|--|
|prng_seed  | String  |  Pseudo Random Number Generator (PRNG) is a starting value to use for the generation of the pseudo random sequence.

## Handle 
### - InitAddress
For first time user. Create root folder and viewing_key
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|contents_list  | String  | contents for root folder (index 0) and sub folders (index 1..n) 
|path_list  | String  | path of sub folders
|entropy  | String  |  "entropy" is a term in physics, originally. In cryptography, it's usually used to talk about "source of randomness". 

##### Response
```json
{
  "create_viewing_key": {
    "key": "anubis_key_Th1s1sAn3xAMpl3+WfrGzBWrVdsh8="
  }
}
```

### - Create
Create a file
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|contents| string  | 
|path    | string  |    

### - CreateMulti
Create file(s)
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|content_list | string[]  | 
|path_list    | string[]  |   

### - Remove
Remove a file
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|path  | string  |   a path you want to remove

### - RemoveMulti
Remove file(s)
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|path_list  | string[]  |   list of paths you want to remove

### - MoveMulti
Move file(s) to a new path
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|old_path_list  | string[]  |  list of paths to move from
|new_path_list  | string[]  |  list of new paths 

### - Move
Move a file to a new path
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|old_path  | string  |  origin path
|new_path  | string  |  destination path

### - CreateViewingKey
**InitAddress** already creates a viewing key for you when you first start using Jackal, but in case you want a new one, this will replace your current viewing key with a new one.
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|entropy  | String  |  "entropy" is a term in physics, originally. In cryptography, it's usually used to talk about "source of randomness". 
|padding  | String  |

##### Response
```json
{
  "create_viewing_key": {
    "key": "anubis_key_Th1s1sAn3xAMpl3+WfrGzBWrVdsh8="
  }
}
```

### - AllowRead
Input address(es) to give READ access to a certain path
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|path  | String  | path to modify permission
|message  | String  | notification message
|address_list  | String[]  | list of addresses to get access

### - DisallowRead
Input address(es) to remove READ access to a certain path
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|path  | String  | path to modify permission
|message  | String  | notification message
|notify  | bool  | if true, we notify
|address_list  | String[]  |  list of addresses to remove from access list

### - ResetRead
Remove ALL READ access to a certain path
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|path  | String  | path to reset READ permission
|message  | String  | notification message
|notify  | bool  | if true, we notify

### - AllowWrite
Input address(es) to give WRITE access to a certain path
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|path  | String  | path to modify permission
|message  | String  | notification message
|address_list  | String[]  |  list of addresses to get access 

### - DisallowWrite
Input address(es) to remove WRITE access to a certain path
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|path  | String  | path to modify permission
|message  | String  | notification message
|notify  | bool  | if true, we notify
|address_list  | String[]  |  list of addresses to remove from access list

### - ResetWrite
Remove ALL WRITE access to a certain path
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|path  | String  | path to reset WRITE permission
|message  | String  | notification message
|notify  | bool  | if true, we notify

### - InitNode
Init a new node
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|ip  | String  | 
|address  | String  |   

### - ClaimReward
For node to claim reward
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|path  | String  | 
|key  | String  |   
|address  | String  |   

### - ForgetMe
Reset and remove everything you have in JACKAL Storage.
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
| N/A |   | 

### - ChangeOwner
Change the owner of a file 
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|path  | String  | path of file to be given to new owner
|new_owner  | String  | address of new owner

### - SendMessage
Sends a message to another user. This handle message may be removed soon. 
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|to  | String  | recipient
|contents  | String  | the message

### - DeleteAllMessages
Delete all your messages
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
| N/A |   | 


## Queries

#### - YouUpBro
Returns a bool that indicates if a wallet has already ran InitAddress.
The number at the end of "namespace" is the same as "counter".
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|address | String  | address you want to check


##### Response
```json
{
  "init": true,
  "namespace": "scrt10wn3radre5550",
  "counter": 0
  
}
```

### - GetNodeCoins

get node coins
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|address  |  string | 

##### Response
```json
{
  "data": 11
}
```

### - GetNodeIP

get node ip
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|index  | u64  | 

##### Response
```json
{
  "data": "192.168.0.1"
}
```

### - GetNodeList

get node list
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|size  | u64  | 

##### Response
```json
{
  "data": []
}
```

### - GetNodeListSize

get node list size
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|N/A  |   | 

##### Response
```json
{
  "data": 5
}
```

## Authenticated Queries

#### - GetContents
Get content of a file 
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|behalf | String  | user address
|path   | String  | path of the file you want to query (ex: secret1d56acq6rny0uR0M0mqPhaTtrjqcju8fxhes346/folder/)
|key    | String  | viewing key

##### Response
```json
{
  "file": {
    "contents": "",
    "owner": "scrt10wn3radre555",
    "public": false, 
    "allow_read_list": {
      "data": ["alice", "bob"]
     },
    "allow_write_list": {
      "data": ["charlie"]
     },

  }
}
```

#### - GetWalletInfo
Returns a bool that indicates if a wallet has already ran InitAddress.
The number at the end of "namespace" is the same as "counter".
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|behalf | String  | user address
|key    | String  | viewing key

##### Response
```json
{
  "init": true,
  "namespace": "scrt10wn3radre5550",
  "counter": 0
  
}
```

#### - GetMessages
Returns a vector of all messages for a user
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|behalf | String  | user address
|key    | String  | viewing key

##### Response
```json
{
  "messages": [
    "alice has given you read access to 'alice_home/memes/pepe.jpg'",
    "bob has given you read access to 'bob_home/memes/hasbullah.jpg'"
    ]
}
```

