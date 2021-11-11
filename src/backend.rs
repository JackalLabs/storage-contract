use cosmwasm_std::{CanonicalAddr, Storage};

#[derive(PartialEq, Eq, Hash)]
pub struct WrappedAddress(CanonicalAddr);

pub struct Folder{
    child_folders: Vec<Folder>,
    files: Vec<File>,
    name: String,
    owner: WrappedAddress,
}

pub struct File{
    contents: String,
    name: String,
    owner: WrappedAddress,
}

pub fn add_file(parent : &mut Folder, mut child: File){
    child.owner = parent.owner.clone();
    parent.files.push(child);
}

pub fn add_folder(parent : &mut Folder, mut child: Folder){
    child.owner = parent.owner.clone();

    for entry in child.child_folders.iter_mut() {
        entry.owner = parent.owner.clone();
    }

    parent.child_folders.push(child);
}

pub fn make_folder(name: &str, owner: WrappedAddress) -> Folder{
    Folder {
        child_folders: Vec::<Folder>::new(),
        files: Vec::<File>::new(),
        name: String::from(name),
        owner: owner
    }
}

pub fn make_file(name: &str, owner: WrappedAddress, contents: &str) -> File{
    File {
        contents: String::from(contents),
        name: String::from(name),
        owner: owner
    }
}

pub fn print_folder(folder : &Folder, level : u16){
    if level > 0 {
        for _i in 0..(level - 1){
            print!("     ");
        }
        print!("\\");
        print!("----");
    }
    
    println!("> {}", folder.name);
    

    for f in folder.child_folders.iter(){
        print_folder(f, level + 1);
    }

    for file in folder.files.iter(){
        print_file(file, level + 1);
    }
}

pub fn print_file(file : &File, level : u16){
    if level > 0 {
        for _i in 0..(level - 1){
            print!("     ");
        }
        print!("\\");
        print!("----");
    }
    
    println!("> {}", file.name);
}

pub fn remove_folder(parent: &mut Folder, name : &str) -> Folder {
    let mut x = 0;
    for entry in parent.child_folders.iter_mut() {
        if entry.name.eq(name) {
            break;
        }
        x += 1;
    }

    parent.child_folders.remove(x)

}

pub fn remove_file(parent: &mut Folder, name : &str) -> File {
    let mut x = 0;
    for entry in parent.files.iter_mut() {
        if entry.name.eq(name) {
            break;
        }
        x += 1;
    }

    parent.files.remove(x)

}

pub fn move_folder(parent: &mut Folder, name: &str, new_parent: &mut Folder){
    let child = remove_folder(parent, name);
    add_folder(new_parent, child);
}

pub fn move_file(parent: &mut Folder, name: &str, new_parent: &mut Folder){
    let child = remove_file(parent, name);
    add_file(new_parent, child);
}

pub fn build_file(parent : &mut Folder, name : &str, contents: &str){
    let mut f = make_file(name, parent.owner, contents);
    add_file(parent, f);
}

pub fn build_child(parent : &mut Folder, name : &str){
    let mut f = make_folder(name, parent.owner);
    add_folder(parent, f);
}

pub fn get_folder<'a>(parent: &'a mut Folder, name : &str) -> &'a mut Folder {
    let mut x = 0;
    for entry in parent.child_folders.iter_mut() {
        if entry.name.eq(name) {
            break;
        }
        x += 1;
    }

    &mut parent.child_folders[x]
}

pub fn traverse_folders<'a>(parent: &'a mut Folder, path: Vec<&str>) -> &'a mut Folder {

    let mut folder = get_folder(parent, path[0]);

    for i in 1..path.len() {
        let f = path[i];
        folder = get_folder(folder, f);
    }

    folder

}

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