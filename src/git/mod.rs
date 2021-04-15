use git2::{Commit, ObjectType, Repository, Index, Error, Signature, Oid};
use git2::IndexAddOption;
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

pub fn add_all_and_commit(repo: &Repo, message: &str, sign_name: &str, sign_email: &str) -> Result<Oid, Error> {
    let repository = &repo.repo;
    let mut index:Index = repository.index()?;

    index.add_all(["*"].iter(),IndexAddOption::DEFAULT , Some(&mut (|a, _b| { println!("path {}",a.to_str().unwrap()); 0 })))?;
    index.write().unwrap();
    let oid = index.write_tree()?;
    println!("oid {:?}",oid);
    let signature = Signature::now(&sign_name, &sign_email)?;
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

/// Unlike regular "git init", this shows how to create an initial empty
/// commit in the repository. This is the helper function that does that.
pub fn create_initial_commit(repo: &Repository) -> Result<(), Error> {
    // First use the config to initialize a commit signature for the user.
    let sig = repo.signature()?;

    // Now let's create an empty tree for this commit
    let tree_id = {
        let mut index = repo.index()?;
        index.write_tree()?
    };

    let tree = repo.find_tree(tree_id)?;

    // Ready to create the initial commit.
    //
    // Normally creating a commit would involve looking up the current HEAD
    // commit and making that be the parent of the initial commit, but here this
    // is the first commit so there will be no parent.
    if let Err(e) = repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]) {
        Err(e)
    } else {
        Ok(())
    }
}
