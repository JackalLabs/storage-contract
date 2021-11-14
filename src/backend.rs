use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::StdResult;
use cosmwasm_std::{ReadonlyStorage, Storage, HumanAddr, HandleResponse};
use cosmwasm_storage::{singleton, singleton_read, bucket, bucket_read, ReadonlySingleton, Singleton, Bucket, ReadonlyBucket};

use crate::ordered_set::{OrderedSet};

static FOLDER_LOCATION: &[u8] = b"FOLDERS";
static FILE_LOCATION: &[u8] = b"FILES";

// #[derive(PartialEq, Eq, Hash)]
// pub struct WrappedHumanAddress(CanonicalHumanAddr);

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct Folder{
    child_folder_names: OrderedSet<String>,
    files: OrderedSet<String>,
    name: String,
    owner: String,
}

impl Folder {
    pub fn list_files(&self) -> Vec<String>{
        let mut files: Vec<String> = Vec::new();

        for i in 0..self.files.len() {
            files.push(String::from(self.files.get(i).unwrap()));
        }

        return files;
    }

    pub fn list_folders(&self) -> Vec<String>{
        let mut folders: Vec<String> = Vec::new();

        for i in 0..self.child_folder_names.len() {
            folders.push(String::from(self.child_folder_names.get(i).unwrap()));
        }

        return folders;
    }
}

impl PartialEq<Folder> for Folder {
    fn eq(&self, other: &Folder) -> bool {
        self.name == other.name
    }
}

pub fn save_folder<'a, S: Storage>( store: &'a mut S, path: String, folder: Folder ) {
    bucket(FOLDER_LOCATION, store).save(&path.as_bytes(), &folder);
}

pub fn load_folder<'a, S: Storage>( store: &'a mut S, path: String) -> Folder{
    bucket(FOLDER_LOCATION, store).load(&path.as_bytes()).unwrap()
}

pub fn load_readonly_folder<'a, S: Storage>( store: &'a S, path: String) -> Folder{
    bucket_read(FOLDER_LOCATION, store).load(&path.as_bytes()).unwrap()

}

pub fn save_file<'a, S: Storage>( store: &'a mut S, path: String, folder: File ) {
    bucket(FILE_LOCATION, store).save(&path.as_bytes(), &folder);
}

pub fn load_file<'a, S: Storage>( store: &'a mut S, path: String) -> File{
    bucket(FILE_LOCATION, store).load(&path.as_bytes()).unwrap()
}

pub fn load_readonly_file<'a, S: Storage>( store: &'a S, path: String ) -> File{
    bucket_read(FILE_LOCATION, store).load(&path.as_bytes()).unwrap()
}


pub fn write_folder<'a, S: Storage>(
    store: &'a mut S,
) -> Bucket<'a, S, Folder> {
    bucket(FOLDER_LOCATION, store)
}
pub fn read_folder<'a, S: ReadonlyStorage>(
    store: &'a S,
) -> ReadonlyBucket<'a, S, Folder> {
    bucket_read(FOLDER_LOCATION, store)
}


#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Debug, Clone)]
pub struct File{
    contents: String,
    name: String,
    owner: String,
}

fn get_folder_from_path<'a, S: Storage>(store: &'a mut S, root: &'a mut Folder, path: Vec<String>) -> String{

    if path.len() > 1 {

        let mut f = root.child_folder_names.clone();
        let mut s = path[0].clone();
        
        for i in 1..path.len() {
            for x in 0..f.len() {
                if f.get(x).unwrap() == &path[i]  {
                    f = load_folder(store, path[i].clone()).child_folder_names.clone();
                    s = path[i].clone();
                }
            }
        }

        return s;
        

    }

    if path.len() == 1 {

        for x in 0..root.child_folder_names.len() {
            if root.child_folder_names.get(x).unwrap() == &path[0]  {
                return path[0].clone();
            }
        }

    }

    return path[0].clone();


}

fn vec_to_string(path: Vec<String>) -> String {
    let mut s: String = String::from(&path[0]);
    if(path.len() > 1){
        for i in 1..path.len()-1 {
            s.push_str(&path[i]);
        }
    }

    s

}

fn path_to_vec(path: String) -> Vec<String> {
    let split = path.split("/");
    let vec: Vec<&str> = split.collect();

    let mut vec2: Vec<String> = Vec::new();

    for i in 0..vec.len() {
        vec2.push(String::from(vec[i]));
    }

    vec2

}

pub fn create_file<'a, S: Storage>(store: &'a mut S, root: &mut Folder, path: String, name: String, contents: String) {

    let file = make_file(&name, "", &contents);

    add_file(store, root, path, file);

}

pub fn create_folder<'a, S: Storage>(store: &'a mut S, root: &mut Folder, path: String, name: String) {

    let folder = make_folder(&name, "");

    add_folder(store, root, path, folder);

}

pub fn add_folder<'a, S: Storage>(store: &'a mut S, parent : &mut Folder, path: String, mut child: Folder){
    child.owner = parent.owner.clone();
    let mut p = path.clone();
    p.push_str(&child.name);
    p.push('/');

    parent.child_folder_names.push(p.clone());

    save_folder(store, p.clone(), child);
}

pub fn add_file<'a, S: Storage>(store: &'a mut S, parent : &mut Folder, path: String, mut child: File){
    child.owner = parent.owner.clone();
    let mut p = path.clone();
    p.push_str(&child.name);

    parent.files.push(p.clone());
    save_file(store, p.clone(), child);
}

// pub fn add_folder(parent : &mut Folder, mut child: Folder){
//     child.owner = parent.owner.clone();

//     for (_, entry) in child.child_folders.iter_mut() {
//         entry.owner = parent.owner.clone();
//     }

//     parent.child_folders.insert(child.name, child);
// }

pub fn make_folder(name: &str, owner: &str) -> Folder{
    Folder {
        child_folder_names: OrderedSet::<String>::new(),
        files: OrderedSet::<String>::new(),
        name: String::from(name),
        owner: String::from(owner),
    }
}

pub fn make_file(name: &str, owner: &str, contents: &str) -> File{
    File {
        contents: String::from(contents),
        name: String::from(name),
        owner: String::from(owner),
    }
}

// pub fn print_folder(folder : &Folder, level : u16){
//     if level > 0 {
//         for _i in 0..(level - 1){
//             print!("     ");
//         }
//         print!("\\");
//         print!("----");
//     }
    
//     println!("> {}", folder.name);
    

//     for f in folder.child_folders.iter(){
//         print_folder(f, level + 1);
//     }

//     for file in folder.files.iter(){
//         print_file(file, level + 1);
//     }
// }

// pub fn print_file(file : &File, level : u16){
//     if level > 0 {
//         for _i in 0..(level - 1){
//             print!("     ");
//         }
//         print!("\\");
//         print!("----");
//     }
    
//     println!("> {}", file.name);
// }

// pub fn remove_folder(parent: &mut Folder, name : &str) -> Folder {
//     let mut x = 0;
//     for (_, entry) in parent.child_folders.iter_mut() {
//         if entry.name.eq(name) {
//             break;
//         }
//         x += 1;
//     }

//     parent.child_folders.remove(x)

// }

// pub fn remove_file(parent: &mut Folder, name : &str) -> File {
//     let mut x = 0;
//     for entry in parent.files.iter_mut() {
//         if entry.name.eq(name) {
//             break;
//         }
//         x += 1;
//     }

//     parent.files.remove(x)

// }

// pub fn move_folder(parent: &mut Folder, name: &str, new_parent: &mut Folder){
//     let child = remove_folder(parent, name);
//     add_folder(new_parent, child);
// }

// pub fn move_file(parent: &mut Folder, name: &str, new_parent: &mut Folder){
//     let child = remove_file(parent, name);
//     add_file(new_parent, child);
// }

// pub fn build_file<'a, S: Storage>(store: &'a mut S, parent : &mut Folder, name : &str, contents: &str){
//     let mut f = make_file(name, &parent.owner.clone(), contents);
//     add_file(store, parent, f);
// }

// pub fn build_child(parent : &mut Folder, name : &str){
//     let mut f = make_folder(name, &parent.owner.clone());
//     add_folder(parent, f);
// }

// pub fn get_folder<'a>(parent: &'a mut Folder, name : &str) -> &'a mut Folder {
//     let mut x = 0;
//     for entry in parent.child_folders.iter_mut() {
//         if entry.name.eq(name) {
//             break;
//         }
//         x += 1;
//     }

//     &mut parent.child_folders[x]
// }

// pub fn mut_get_file<'a>(parent: &'a mut Folder, name : &str) -> &'a mut File {
//     let mut x = 0;
//     for entry in parent.files.iter_mut() {
//         if entry.name.eq(name) {
//             break;
//         }
//         x += 1;
//     }

//     &mut parent.files[x]
// }


// pub fn mut_traverse_to_file<'a>(parent: &'a mut Folder, path: Vec<&str>) -> &'a mut File {

//     if path.len() > 1 {
//         return mut_get_file(traverse_folders(parent, path[0..path.len() - 1].to_vec()), path[path.len() - 1]);
//     }

//     mut_get_file(parent, path[0])
// }

// pub fn traverse_to_file<'a>(parent: &'a mut Folder, path: Vec<&str>) -> File {

//     if path.len() > 1 {
//         return get_file(traverse_folders(parent, path[0..path.len() - 1].to_vec()), path[path.len() - 1]);
//     }

//     get_file(parent, path[0])
// }

// pub fn traverse_folders<'a>(parent: &'a mut Folder, path: Vec<&str>) -> &'a mut Folder {

//     let mut folder = parent;

//     for i in 0..path.len() {
//         let f = path[i];
//         folder = get_folder(folder, f);
//     }

//     folder

// }

// fn main(){
//     println!("Starting test...\n\n");

//     let mut f = make_folder("root", "me");
//     let mut c = make_folder("child1", "me2");
//     let mut c2 = make_folder("child2", "me3");
//     let mut c3 = make_folder("child3", "me3");

//     build_child(&mut c, "testing");

//     build_file(traverse_folders(&mut c, vec!["testing"]), "f2.txt", "wow this is awesome.");

//     build_file(&mut c, "f.txt", "wow this is awesome.");


//     let mut f2 = make_folder("root2", "me4");

//     add_folder(&mut c, c2);
//     add_folder(&mut c, c3);
//     add_folder(&mut f, c);

//     print_folder(&mut f, 0);
//     print_folder(&mut f2, 0);

//     move_folder(&mut f, "child1", &mut f2);

//     println!("\nAfter move... \n");
//     print_folder(&mut f, 0);
//     print_folder(&mut f2, 0);

//     println!("\nGrabbing a nested folder... \n");

//     print_folder(traverse_folders(&mut f2, vec!["child1", "testing"]), 0);


// }