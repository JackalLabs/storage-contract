# JACKAL Storage Contract
- [Introduction](#Introduction)
-  [Sections](#Sections)
    - [Init](#Init)
    - [Handle](#Handle)
        - [InitAddress](#InitAddress)
        -  [CreateViewingKey](#CreateViewingKey)
     - [Query](#Query)
       - [GetContents](#GetContents)
        - [GetWalletInfo](#GetWalletInfo)


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
  "data": {
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
  "data": {
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
|path_list  | string[]  |   

### - MoveMulti
Move file(s) to a new path
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|old_path_list  | string[]  |   
|new_path_list  | string[]  |   

### - NameHere
Description goes here
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|content  | String  | 
|entropy  | String  |   

##### Response
```json
{
  "data": {
    "name": "data"
  }
}
```


## Queries

#### - GetContents
Get content of a file 
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|behalf | String  | user address
|path   | String  | path of the file you want to query
|key    | String  | viewing key

##### Response
```json
{
  "file": {
    "contents": ""
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
Return init(bool) and all paths that have been created
##### Request
|Name|Type|Description|                                                                                       
|--|--|--|
|behalf | String  | user address
|key    | String  | viewing key

##### Response
```json
{
  "init": true,
  "all_path": "["scrt10wn3radre555/", "scrt10wn3radre555/alpha/"]"
  
}
```