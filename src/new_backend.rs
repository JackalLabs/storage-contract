use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct Folder{
    name: &str,
    owner: &str,
}

impl PartialEq<Folder> for Folder {
    fn eq(&self, other: &Folder) -> bool {
        self.name == other.name
    }
}


#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Debug, Clone)]
pub struct File{
    contents: &str,
    name: &str,
    owner: &str,
}

pub fn make_folder(name: &str, owner: &str) -> Folder{
    Folder {
        // child_folders: Vec::<Folder>::new(),
        // files: Vec::<File>::new(),
        name: &String::from(name),
        owner: &String::from(owner),
    }
}

pub fn make_file(name: &str, owner: &str, contents: &str) -> File{
    File {
        contents: &String::from(contents),
        name: &String::from(name),
        owner: &String::from(owner),
    }
}