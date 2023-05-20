extern crate git2;
extern crate notify;

mod git;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc::{channel};
use std::time::Duration;

extern crate timer;
extern crate chrono;
use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use timer::Guard;

extern crate clap;
use clap::{Arg, App};

use git2::{PushOptions, ProxyOptions, RemoteCallbacks, Cred, CredentialType};
use std::path::Path;

extern crate notify_rust;
use notify_rust::{Notification};

extern crate log;
use log::{error, info, trace, warn};

extern crate simple_logger;
use simple_logger::SimpleLogger;

pub struct BackerConfig {
    pub repo_path: String,
    pub file_monitor_freq: String,
    pub commit_delay: String,
    pub sign_name: String,
    pub sign_email: String,
    pub default_commit_msg: String,
    pub should_push_to_remote: bool,
    pub ssh_private_key: String
}

fn main() {
    // Name and path of repo
    // Common commit message
    // File watcher frequency
    // Commit frequency
    // Signature name
    // Signature email
    // remote ssh url to push the code to

    if let Err(e) = SimpleLogger::new().init() {
        show_desktop_notification(&format!("Error initializing terminal logger, {:?}", e), notify_rust::Timeout::Milliseconds(5000));
        panic!("Error initializing terminal logger: {:?}", e);
    }

    let matches = App::new("Backer").version("1.0").author("Krishna Kumar Thokala")
        .about("A git based backup tool")
        .arg(Arg::with_name("path").short("p").long("path").value_name("path").help("Set path to monitor").required(true))
        .arg(Arg::with_name("ffreq").short("f").long("ffreq").value_name("ffreq").help("Set frequency for monitoring file changes (seconds)"))
        .arg(Arg::with_name("cfreq").short("c").long("cfreq").value_name("cfreq").help("Set the delay to make commit after first file change event (seconds)"))
        .arg(Arg::with_name("sname").short("n").long("sname").value_name("sname").help("Add name of the author").required(true))
        .arg(Arg::with_name("semail").short("e").long("semail").value_name("semail").help("Add email of the author").required(true))
        .arg(Arg::with_name("defcommitmsg").short("d").long("defcommitmsg").value_name("defaultcommitmsg").help("Set default commit message"))
        .arg(Arg::with_name("remoteurl").short("u").long("remoteurl").value_name("remoteurl").help("Set Optional remote url for push and fetch"))
        .arg(Arg::with_name("sshprivatekey").short("k").long("pkey").value_name("pkey").help("Set ssh key to be used for pushing"))
        .get_matches();

    // TODO: Can also add destination path to optionally sync the data for backup

    let repo_path = matches.value_of("path").unwrap().to_owned();
    let file_monitor_freq = matches.value_of("ffreq").unwrap_or("2").to_owned();
    let commit_delay = matches.value_of("cfreq").unwrap_or("5").to_owned();
    let sign_name = matches.value_of("sname").unwrap().to_owned();
    let sign_email = matches.value_of("semail").unwrap().to_owned();
    let default_commit_msg = matches.value_of("defaultcommitmsg").unwrap_or("Committed all changes").to_owned();
    let remote_url = matches.value_of("remoteurl");
    let ssh_private_key = matches.value_of("sshprivatekey").unwrap_or("").to_owned();

    let should_push = match remote_url {
        Some(url) => {
            if ssh_private_key == "" {
                show_desktop_notification("Missing private key parameter for the repository to enable auto push, Please check logs", notify_rust::Timeout::Milliseconds(1000));
                error!("Missing private key parameter for the repository to enable auto push");
                panic!("Missing private key parameter for the repository to enable auto push");
            }
            init_repo(repo_path.clone(), Some(url));
            true
        }
        None => {
            init_repo(repo_path.clone(), None);
            false
        }
    };

    let config = BackerConfig {
        repo_path,
        file_monitor_freq,
        commit_delay,
        sign_name,
        sign_email,
        default_commit_msg,
        should_push_to_remote: should_push,
        ssh_private_key
    };

    if let Err(e) = watch(config) {
        show_desktop_notification("Error initializing inotify for file watches, Please check logs", notify_rust::Timeout::Milliseconds(1000));
        error!("Error initializing inotify for file watches: {:?}", e);
        panic!("Error initializing inotify for file watches: {:?}", e)
    }
}

fn init_repo(repo_path: String, remote_url: Option<&str>) {
    let repo = git::Repo::new(&repo_path);
    let repository = &repo.repo;
    match remote_url {
        Some(url) => {
            // TODO: If remote already exists, respect it
            match repo.repo.remote("origin",url) {
                Ok(_) => {
                    info!("Added remote {:?}", repo.repo.remotes().unwrap().get(0));
                }
                Err(e) => {
                    info!("Tried to add remote {:?}", e.message());
                }
            };
        }
        None => ()
    };
    // Ignore the result if it can't create the first commit
    let _ = git::create_initial_commit(repository);
}

fn add_all_changed(repo_path: &str, default_commit_msg: &str, sign_name: &str, sign_email: &str, should_push: bool, ssh_pkey: &str) {
    let mut repo = git::Repo::open(&repo_path);
    match git::add_all_and_commit(&mut repo, &default_commit_msg, &sign_name, &sign_email) {
        Ok(oid) => {
            trace!("Commit id {}",oid);
            if should_push {
                let callback = move |_url: &str, _uname: Option<&str>, _ctype: CredentialType| {
                    Cred::ssh_key("git", None, Path::new(ssh_pkey), None)
                };
                match repo.repo.find_remote("origin") {
                    Ok(mut remote) => {
                        let mut rcb = RemoteCallbacks::default();
                        rcb.credentials(&callback);
                        let mut po = PushOptions::default();
                        po.remote_callbacks(rcb).proxy_options(ProxyOptions::new());
                        trace!("About to push commits to remote");

                        // Push to remote refspec
                        match remote.push(&["refs/heads/master"], Some(&mut po)) {
                            Ok(_) => {
                                info!("Pushed the commits successfully to remote");
                            }
                            Err(e) => {
                                show_desktop_notification("Failed to push commits to remote, Please check logs", notify_rust::Timeout::Milliseconds(1000));
                                error!("Failed to push commits to remote, {}", e);
                            }
                        };
                        trace!("Push done");
                    }
                    Err(e) => {
                        show_desktop_notification("Could not find a remote to push, Please check logs", notify_rust::Timeout::Milliseconds(1000));
                        error!("Could not find a remote to push, error {:?}", e);
                    }
                }
            }
        },
        Err(e) => {
            show_desktop_notification("Unable to perform commit of local changes, Please check logs", notify_rust::Timeout::Milliseconds(1000));
            error!("Unable to commit, reason {}",e.message());
        }
    }
}

fn watch(config: BackerConfig) -> notify::Result<()> {
    // Create a channel to receive the events.
    let (tx, rx) = channel();
    let repo_path = config.repo_path;
    let default_commit_msg = config.default_commit_msg;
    let sign_name = config.sign_name;
    let sign_email = config.sign_email;
    let should_push = config.should_push_to_remote;
    let ssh_pkey = config.ssh_private_key;

    let file_monitor_freq: u64 = config.file_monitor_freq.parse().unwrap();
    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(file_monitor_freq))?;

    // TODO: It seems recursive mode is not working as expected, fix it
    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(&repo_path, RecursiveMode::NonRecursive)?;

    let time_done = Arc::new(AtomicBool::new(false));
    let timer = timer::Timer::new();
    let commit_delay: i64 = config.commit_delay.parse().unwrap();
    let time_done1 = time_done.clone();
    let callback = move || {
        trace!("Commiting changes now");
        add_all_changed(&repo_path, &default_commit_msg, &sign_name, &sign_email, should_push, &ssh_pkey);
        time_done1.store(false, Ordering::Relaxed);
    };
    let mut _guard: Option<Guard>  = None;
    loop {
        match rx.recv() {
            // TODO: If more events are received before commit, reset timer?? This can be optional feature as well
            Ok(event) => {
                trace!("{:?}", event);
                if ! time_done.load(Ordering::Relaxed) {
                    //let time_done1 = time_done.clone();
                    _guard = Some(timer.schedule_with_delay(chrono::Duration::seconds(commit_delay), callback.clone()));
                    trace!("Commit timer started, will be committed in {} seconds", commit_delay);
                    time_done.store(true, Ordering::Relaxed);
                }
            },
            Err(e) => error!("watch error: {:?}", e),
        }
    }
}

fn show_desktop_notification(body: &str, timeout: notify_rust::Timeout) {
    if let Err(e) = Notification::new()
    .summary("Category: backer-rs")
    .body(body)
    .timeout(timeout) // this however is
    .show() {
        warn!("Error showing desktop notification - Category: backer-rs, body: {}, timeout: {:?} to user, error: {:?}", body, timeout, e);
    }
}