
# JACKAL Storage Contract
- [Introduction](#Introduction)
-  [Sections](#Sections)
    - [Init](#Init)
    - [Handle](#Handle)
        - [InitAddress](#--InitAddress)
        -  [CreateViewingKey](#--CreateViewingKey)
        -  [CreateMulti](#--CreateMulti)
        -  [RemoveMulti](#--RemoveMulti)
        -  [MoveMulti](#--MoveMulti)
        -  [AllowRead](#--AllowRead)
        -  [DisallowRead](#--DisallowRead)
        -  [ResetRead](#--ResetRead)
        -  [AllowWrite](#--AllowWrite)
        -  [DisallowWrite](#--DisallowWrite)
        -  [ResetWrite](#--ResetWrite)
        -  [CloneParentPermission](#--CloneParentPermission)
        -  [InitNode](#--InitNode)
        -  [ClaimReward](#--ClaimReward)
        -  [ForgetMe](#--ForgetMe)
     - [Query](#Query))  
        - [GetContents](#--GetContents)
        - [GetWalletInfo](#--GetWalletInfo)
        - [YouUpBro](#--YouUpBro)
        - [GetNodeIP](#--GetNodeIP)
        - [GetNodeListSize](#--GetNodeListSize)
        - [GetNodeList](#--GetNodeList)
        - [GetNodeCoins](#--GetNodeCoins)


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
|content  | String  | 
|entropy  | String  |  "entropy" is a term in physics, originally. In cryptography, it's usually used to talk about "source of randomness". 

##### Response
```json
{
  "create_viewing_key": {
    "key": "anubis_key_Th1s1sAn3xAMpl3+WfrGzBWrVdsh8="
  }
}
```

### - CreateViewingKey
**InitAddress** already create a viewing key for you when you first start using Jackal, but in case you want a new one, this will replace your current viewing key with a new one.
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

### - CreateMulti
Create file(s)
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|content_list | string[]  | 
|path_list    | string[]  |   
|pkey_list    | string[]  |  
|skey_list    | string[]  |  

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
|old_path_list  | string[]  |   list of paths to move from
|new_path_list  | string[]  |  list of new paths 

### - AllowRead
Input address(es) to give READ access to a certain path
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|path  | String  | path to modify permission
|address_list  | String[]  | list of address to get access

### - DisallowRead
Input address(es) to remove READ access to a certain path
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|path  | String  | path to modify permission
|address_list  | String[]  |  list of address to remove from access list

### - ResetRead
Remove ALL READ access to a certain path
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|path  | String  | path to reset READ permission

### - AllowWrite
Input address(es) to give WRITE access to a certain path
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|path  | String  | path to modify permission
|address_list  | String[]  |  list of address to get access 

### - DisallowWrite
Input address(es) to remove WRITE access to a certain path
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|path  | String  | path to modify permission
|address_list  | String[]  |  list of address to remove from access list

### - ResetWrite
Remove ALL WRITE access to a certain path
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|path  | String  | path to reset WRITE permission

### - CloneParentPermission
Input a path and this will give all the children/grandchildren the same permissions that the given path has
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|path  | String  | parent path you want to clone permission 
```
Example: /meme/ folder contains /pepe/ and /pepo
- "bob" has write access to /meme/
- "alice" has read access to /meme/
After running CloneParentPermission,
- "bob" will have write access to /meme/, /pepe/, and /pepo/
- "alice" will have read access to /meme/, /pepe/, and /pepo/
```

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



## Queries

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
    "allow_read_list": {
      "data": ["alice", "bob"]
     },
    "allow_write_list": {
      "data": ["charlie"]
     },
    "contents": "",
    "owner": "scrt10wn3radre555",
    "public": false, 
  }
}
```

#### - GetWalletInfo
Returns init (bool) and all paths that have been created
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|behalf | String  | user address
|key    | String  | viewing key

##### Response
```
{
  "init": true,
  "all_path": "["scrt10wn3radre555/", "scrt10wn3radre555/alpha/"]"
}
```

#### - YouUpBro
Returns a bool indicate if a wallet has already ran InitAddress
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|address | String  | address you want to check


##### Response
```json
{
  "init": true,
  "all_path": "[]"
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
