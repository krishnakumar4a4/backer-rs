extern crate git2;
mod git;
use std::path::Path;

fn main() {
    // let mut repo = git::Repo::open("test_repo");
    // let repository = &repo.repo;
    // let files_to_commit = changed_files();
    // let mut index = repository.index().unwrap();
    // let file1 = index.get(0).unwrap();
    // let file2 = index.get(1).unwrap();
    //println!("len {},{:?},{:?},{:?},{:?}",index.len(),index.get(0).unwrap().path,index.get(1).unwrap().path,index.get(2).unwrap().path,index.get(3).unwrap().path);
    // index.add(&file1);
    // index.add(&file2);
    // index.write();
    //index.iter().for_each(e)
    // for path in files_to_commit {
    //     match index.add_path(Path::new(&path)) {
    //         Ok(()) => {
    //             println!("Succefully added path {}", path);
    //         }
    //         Err(e) => {
    //             println!("Error while adding path {}",e.message());
    //         }
    //     }
    // }
    // index.write();
    // let oid = index.write_tree().unwrap();
    // let tree = repository.find_tree(oid).unwrap();
    // println!("tree {:?}",tree);


    //init_repo();
    //sample_commit();
    add_all_changed();
}

fn init_repo() {
    let mut repo = git::Repo::new("test_repo");
    let repository = &repo.repo;
    git::create_initial_commit(repository);
}

fn changed_files() -> Vec<String>{
    let mut files_to_commit = Vec::new();
    let path1 = String::from("test3");
    let path2 = String::from("test4");
    files_to_commit.push(path1);
    files_to_commit.push(path2);
    files_to_commit
}

fn sample_commit() {
    let mut repo = git::Repo::open("test_repo");
    let files_to_commit = changed_files();
    match git::add_and_commit(&mut repo, files_to_commit, "Auto commit") {
        Ok(oid) => {
            println!("Commit id {}",oid);
        },
        Err(e) => {
            println!("Unable to commit, reason {}",e.message());
        }
    }
}

fn add_all_changed() {
    let mut repo = git::Repo::open("test_repo");
    match git::add_all_and_commit(&mut repo, "Auto commit all") {
        Ok(oid) => {
            println!("Commit id {}",oid);
        },
        Err(e) => {
            println!("Unable to commit, reason {}",e.message());
        }
    }
}


