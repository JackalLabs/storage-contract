# JACKAL
### JACKAL File Storage & Evidence Capture Tool

JACKAL aims to be the most secure and transparent way to capture, store, andmanage digital files. Using industry-leading digital forensic practices paired withblockchain technology, decentralized file storage, Secret Network, and the PylonProtocol, JACKAL is in a class of its own.

This is the contract implimentation of the file system. 

### TODO
Create privacy key system to prevent others from peaking into files.

Handle uploads of files larger than 1kb while being gas friendly.

### Running & Testing
Following deployment on a local testnet from this doc: https://build.scrt.network/dev/quickstart.html#create-initial-smart-contract

```
cargo update
cargo unit-test //Testing

cargo schema //Generating the schemas to interact with deployed contract
```

#### Interacting with the contract

Creating a folder at the root directory:
```
secretcli tx compute execute $CONTRACT '{"create_folder": {"name": "folder", "path":"/"}}' --from a --keyring-backend test
```

Creating a file at the root directory:
```
secretcli tx compute execute $CONTRACT '{"create_file": {"name": "file.txt", "contents": "FILE CONTENTS", "path":"/"}}' --from a --keyring-backend test
```

Getting the directory listing for the root folder:
```
secretcli query compute query $CONTRACT '{"get_folder_contents": {"address":"USERS ADDRESS", "path" : "/" }}'
```
