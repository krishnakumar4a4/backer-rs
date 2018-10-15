use git2::{Commit, ObjectType, Repository, Index, Error, Signature, Oid};
use git2::Status;
use git2::IndexAddOption;
use git2::IndexMatchedPath;
use std::path::Path;

pub struct Repo<'a> {
    pub repo_path: &'a str,
    pub repo: Repository,
}

impl<'a> Repo<'a> {
    pub fn new(repo_path: &str) -> Repo {
        let repo = Repository::init(Path::new(repo_path)).unwrap();
        Repo {
            repo_path: repo_path,
            repo: repo,
        }
    }
    pub fn open(repo_path: &str) -> Repo {
        let repo = Repository::open(Path::new(repo_path)).unwrap();
        Repo {
            repo_path: repo_path,
            repo: repo,
        }
    }
    pub fn find_last_commit(&self) -> Result<Commit, Error> {
        let obj = self.repo.head()?.resolve()?.peel(ObjectType::Commit)?;
        obj.into_commit().map_err(|_| Error::from_str("Couldn't find last commit"))
    }
}


pub fn add_and_commit(repo: &Repo, file_paths: Vec<String>, message: &str) -> Result<Oid, Error> {
    let repository = &repo.repo;
    let mut index = repository.index()?;
    for path in file_paths {
        match index.add_path(Path::new(&path)) {
            Ok(()) => {
                println!("Succefully added path {}", path);
            }
            Err(e) => {
                println!("Error while adding path {}",e.message());
            }
        }
    }
    index.write();
    let oid = index.write_tree()?;
    println!("oid {:?}",oid);
    let signature = Signature::now("Krishna Kumar Thokala", "krishna.thokala2010@gmail.com")?;
    match repo.find_last_commit() {
        Ok(parent_commit) => {
            println!("last commit {:?}",&parent_commit.id());
            match repository.find_tree(oid) {
                Ok(tree) => {
                    repository.commit(Some("HEAD"), //  point HEAD to our new commit
                                      &signature, // author
                                      &signature, // committer
                                      message, // commit message
                                      &tree, // tree
                                      &[&parent_commit]) // parents
                },
                Err(e) => {
                    println!("Error while finding tree with oid, {}",e.message());
                    Err(e)
                }
            }
        }
        Err(e) => {
            println!("Error while finding the last commit, {}",e);
            match repository.find_tree(oid) {
                Ok(tree) => {
                    repository.commit(Some("HEAD"), //  point HEAD to our new commit
                                      &signature, // author
                                      &signature, // committer
                                      message, // commit message
                                      &tree, // tree
                                      &[]) // parents
                },
                Err(e) => {
                    println!("Error while finding tree with oid, {}",e.message());
                    Err(e)
                }
            }
        }
    }
}


pub fn add_all_and_commit(repo: &Repo, message: &str) -> Result<Oid, Error> {
    let repository = &repo.repo;
    let mut index:Index = repository.index()?;
    let callback = &mut |path: &Path, _matched_spec: &[u8]| -> i32 {
        let status = repository.status_file(path).unwrap();

        let ret = if status.contains(Status::WT_MODIFIED) {
            println!("modified '{}'", path.display());
            0
        }
        else if status.contains(Status::WT_NEW) {
            println!("add '{}'", path.display());
            0
        } else {
            1
        };
        ret
    };

    index.add_all(["*"].into_iter(),IndexAddOption::DEFAULT , Some(&mut (|a, _b| { println!("path {}",a.to_str().unwrap()); 0 })));
    index.write();
    let oid = index.write_tree()?;
    println!("oid {:?}",oid);
    let signature = Signature::now("Krishna Kumar Thokala", "krishna.thokala2010@gmail.com")?;
    match repo.find_last_commit() {
        Ok(parent_commit) => {
            println!("last commit {:?}",&parent_commit.id());
            match repository.find_tree(oid) {
                Ok(tree) => {
                    repository.commit(Some("HEAD"), //  point HEAD to our new commit
                                      &signature, // author
                                      &signature, // committer
                                      message, // commit message
                                      &tree, // tree
                                      &[&parent_commit]) // parents
                },
                Err(e) => {
                    println!("Error while finding tree with oid, {}",e.message());
                    Err(e)
                }
            }
        }
        Err(e) => {
            println!("Error while finding the last commit, {}",e);
            match repository.find_tree(oid) {
                Ok(tree) => {
                    repository.commit(Some("HEAD"), //  point HEAD to our new commit
                                      &signature, // author
                                      &signature, // committer
                                      message, // commit message
                                      &tree, // tree
                                      &[]) // parents
                },
                Err(e) => {
                    println!("Error while finding tree with oid, {}",e.message());
                    Err(e)
                }
            }
        }
    }
}

/// Unlike regular "git init", this example shows how to create an initial empty
/// commit in the repository. This is the helper function that does that.
pub fn create_initial_commit(repo: &Repository) -> Result<(), Error> {
    // First use the config to initialize a commit signature for the user.
    let sig = try!(repo.signature());

    // Now let's create an empty tree for this commit
    let tree_id = {
        let mut index = try!(repo.index());

        // Outside of this example, you could call index.add_path()
        // here to put actual files into the index. For our purposes, we'll
        // leave it empty for now.

        try!(index.write_tree())
    };

    let tree = try!(repo.find_tree(tree_id));

    // Ready to create the initial commit.
    //
    // Normally creating a commit would involve looking up the current HEAD
    // commit and making that be the parent of the initial commit, but here this
    // is the first commit so there will be no parent.
    try!(repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]));

    Ok(())
}
