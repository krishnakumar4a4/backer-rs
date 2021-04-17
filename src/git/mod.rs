use git2::{Commit, Cred, CredentialType, ObjectType, Repository, Index, Error, Signature, Oid};
use git2::IndexAddOption;
use std::path::Path;
use std::io::{self, Write};

use log::{error, trace, warn};

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


fn do_fetch<'a>(
    repo: &'a git2::Repository,
    refs: &[&str],
    remote: &'a mut git2::Remote,
    ssh_pkey: &str,
) -> Result<git2::AnnotatedCommit<'a>, git2::Error> {
    let mut cb = git2::RemoteCallbacks::new();
    // Print out our transfer progress.
    cb.credentials(
        move |_url: &str, _uname: Option<&str>, _ctype: CredentialType| {
            Cred::ssh_key("git", None, Path::new(ssh_pkey), None)
        },
    );
    cb.transfer_progress(|stats| {
        if stats.received_objects() == stats.total_objects() {
            print!(
                "Resolving deltas {}/{}\r",
                stats.indexed_deltas(),
                stats.total_deltas()
            );
        } else if stats.total_objects() > 0 {
            print!(
                "Received {}/{} objects ({}) in {} bytes\r",
                stats.received_objects(),
                stats.total_objects(),
                stats.indexed_objects(),
                stats.received_bytes()
            );
        }
        io::stdout().flush().unwrap();
        true
    });

    let mut fo = git2::FetchOptions::new();
    fo.remote_callbacks(cb);
    // Always fetch all tags.
    // Perform a download and also update tips
    fo.download_tags(git2::AutotagOption::All);
    trace!("Fetching {} for repo", remote.name().unwrap());
    remote.fetch(refs, Some(&mut fo), None)?;

    // If there are local objects (we got a thin pack), then tell the user
    // how many objects we saved from having to cross the network.
    let stats = remote.stats();
    if stats.local_objects() > 0 {
        trace!(
            "\rReceived {}/{} objects in {} bytes (used {} local \
             objects)",
            stats.indexed_objects(),
            stats.total_objects(),
            stats.received_bytes(),
            stats.local_objects()
        );
    } else {
        trace!(
            "\rReceived {}/{} objects in {} bytes",
            stats.indexed_objects(),
            stats.total_objects(),
            stats.received_bytes()
        );
    }

    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    Ok(repo.reference_to_annotated_commit(&fetch_head)?)
}

fn fast_forward(
    repo: &Repository,
    lb: &mut git2::Reference,
    rc: &git2::AnnotatedCommit,
) -> Result<(), git2::Error> {
    let name = match lb.name() {
        Some(s) => s.to_string(),
        None => String::from_utf8_lossy(lb.name_bytes()).to_string(),
    };
    let msg = format!("Fast-Forward: Setting {} to id: {}", name, rc.id());
    trace!("{}", msg);
    lb.set_target(rc.id(), &msg)?;
    repo.set_head(&name)?;
    repo.checkout_head(Some(
        git2::build::CheckoutBuilder::default()
            // For some reason the force is required to make the working directory actually get updated
            // I suspect we should be adding some logic to handle dirty working directory states
            // but this is just an example so maybe not.
            .force(),
    ))?;
    Ok(())
}

fn normal_merge(
    repo: &Repository,
    local: &git2::AnnotatedCommit,
    remote: &git2::AnnotatedCommit,
) -> Result<(), git2::Error> {
    let local_tree = repo.find_commit(local.id())?.tree()?;
    let remote_tree = repo.find_commit(remote.id())?.tree()?;
    let ancestor = repo
        .find_commit(repo.merge_base(local.id(), remote.id())?)?
        .tree()?;
    let mut idx = repo.merge_trees(&ancestor, &local_tree, &remote_tree, None)?;

    if idx.has_conflicts() {
        warn!("Merge conficts detected...");
        repo.checkout_index(Some(&mut idx), None)?;
        return Ok(());
    }
    let result_tree = repo.find_tree(idx.write_tree_to(repo)?)?;
    // now create the merge commit
    let msg = format!("Merge: {} into {}", remote.id(), local.id());
    let sig = repo.signature()?;
    let local_commit = repo.find_commit(local.id())?;
    let remote_commit = repo.find_commit(remote.id())?;
    // Do our merge commit and set current branch head to that commit.
    let _merge_commit = repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        &msg,
        &result_tree,
        &[&local_commit, &remote_commit],
    )?;
    // Set working tree to match head.
    repo.checkout_head(None)?;
    Ok(())
}

fn do_merge<'a>(
    repo: &'a Repository,
    remote_branch: &str,
    fetch_commit: git2::AnnotatedCommit<'a>,
) -> Result<(), git2::Error> {
    // 1. do a merge analysis
    let analysis = repo.merge_analysis(&[&fetch_commit])?;

    // 2. Do the appopriate merge
    if analysis.0.is_fast_forward() {
        trace!("Doing a fast forward");
        // do a fast forward
        let refname = format!("refs/heads/{}", remote_branch);
        match repo.find_reference(&refname) {
            Ok(mut r) => {
                fast_forward(repo, &mut r, &fetch_commit)?;
            }
            Err(_) => {
                // The branch doesn't exist so just set the reference to the
                // commit directly. Usually this is because you are pulling
                // into an empty repository.
                repo.reference(
                    &refname,
                    fetch_commit.id(),
                    true,
                    &format!("Setting {} to {}", remote_branch, fetch_commit.id()),
                )?;
                repo.set_head(&refname)?;
                repo.checkout_head(Some(
                    git2::build::CheckoutBuilder::default()
                        .allow_conflicts(true)
                        .conflict_style_merge(true)
                        .force(),
                ))?;
            }
        };
    } else if analysis.0.is_normal() {
        // do a normal merge
        let head_commit = repo.reference_to_annotated_commit(&repo.head()?)?;
        normal_merge(&repo, &head_commit, &fetch_commit)?;
    } else {
        trace!("Nothing to do...");
    }
    Ok(())
}

pub fn pull(repo: &Repo, ssh_pkey: &str) -> Result<(), Error> {
    let repository = &repo.repo;
    let mut remote = repository.find_remote("origin")?;
    let fetch_commit = do_fetch(&repository, &["master"], &mut remote, ssh_pkey)?;
    do_merge(&repository, &"master", fetch_commit)
}

pub fn add_all_and_commit(repo: &Repo, message: &str, sign_name: &str, sign_email: &str) -> Result<Oid, Error> {
    let repository = &repo.repo;
    let mut index:Index = repository.index()?;

    index.add_all(["*"].iter(),IndexAddOption::DEFAULT , Some(&mut (|a, _b| { trace!("path {}",a.to_str().unwrap()); 0 })))?;
    index.write().unwrap();
    let oid = index.write_tree()?;
    trace!("oid {:?}",oid);
    let signature = Signature::now(&sign_name, &sign_email)?;
    match repo.find_last_commit() {
        Ok(parent_commit) => {
            trace!("last commit {:?}",&parent_commit.id());
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
                    error!("Error while finding tree with oid, {}",e.message());
                    Err(e)
                }
            }
        }
        Err(e) => {
            error!("Error while finding the last commit, {}",e);
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
                    error!("Error while finding tree with oid, {}",e.message());
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
