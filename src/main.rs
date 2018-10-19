extern crate git2;
extern crate notify;

mod git;

use notify::{RecommendedWatcher, Watcher, RecursiveMode};
use std::sync::mpsc::channel;
use std::time::Duration;

extern crate timer;
extern crate chrono;
use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use timer::Guard;

extern crate clap;
use clap::{Arg, App};

pub struct BackerConfig {
    pub repo_path: String,
    pub file_monitor_freq: String,
    pub commit_delay: String,
    pub sign_name: String,
    pub sign_email: String,
    pub default_commit_msg: String
}

fn main() {
    // Name and path of repo
    // Common commit message
    // File watcher frequency
    // Commit frequency
    // Signature name
    // Signature email

    let matches = App::new("Backer").version("1.0").author("Krishna Kumar Thokala")
        .about("A git based backup tool")
        .arg(Arg::with_name("path").short("p").long("path").value_name("path").help("Set path to monitor").required(true))
        .arg(Arg::with_name("ffreq").short("f").long("ffreq").value_name("ffreq").help("Set frequency for monitoring file changes (seconds)"))
        .arg(Arg::with_name("cfreq").short("c").long("cfreq").value_name("cfreq").help("Set the delay to make commit after first file change event (seconds)"))
        .arg(Arg::with_name("sname").short("n").long("sname").value_name("sname").help("Add name of the author").required(true))
        .arg(Arg::with_name("semail").short("e").long("semail").value_name("semail").help("Add email of the author").required(true))
        .arg(Arg::with_name("defcommitmsg").short("d").long("defcommitmsg").value_name("defaultcommitmsg").help("Set default commit message"))
        .get_matches();
    let repo_path = matches.value_of("path").unwrap().to_owned();
    let file_monitor_freq = matches.value_of("ffreq").unwrap_or("2").to_owned();
    let commit_delay = matches.value_of("cfreq").unwrap_or("5").to_owned();
    let sign_name = matches.value_of("sname").unwrap().to_owned();
    let sign_email = matches.value_of("semail").unwrap().to_owned();
    let default_commit_msg = matches.value_of("defaultcommitmsg").unwrap_or("Committed all changes").to_owned();

    let config = BackerConfig {
        repo_path: repo_path.clone(),
        file_monitor_freq: file_monitor_freq,
        commit_delay: commit_delay,
        sign_name: sign_name,
        sign_email: sign_email,
        default_commit_msg: default_commit_msg
    };

    init_repo(repo_path);
    if let Err(e) = watch(config) {
        println!("Error initializing inotify: {:?}", e)
    }
}

fn init_repo(repo_path: String) {
    let repo = git::Repo::new(&repo_path);
    let repository = &repo.repo;
    git::create_initial_commit(repository);
}

fn add_all_changed(repo_path: &str, default_commit_msg: &str, sign_name: &str, sign_email: &str) {
    let mut repo = git::Repo::open(&repo_path);
    match git::add_all_and_commit(&mut repo, &default_commit_msg, &sign_name, &sign_email) {
        Ok(oid) => {
            println!("Commit id {}",oid);
        },
        Err(e) => {
            println!("Unable to commit, reason {}",e.message());
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

    let file_monitor_freq: u64 = config.file_monitor_freq.parse().unwrap();
    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher: RecommendedWatcher = try!(Watcher::new(tx, Duration::from_secs(file_monitor_freq)));

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    try!(watcher.watch(&repo_path, RecursiveMode::NonRecursive));

    let time_done = Arc::new(AtomicBool::new(false));
    let timer = timer::Timer::new();
    let commit_delay: i64 = config.commit_delay.parse().unwrap();
    let time_done1 = time_done.clone();
    let callback = move || {
        println!("Now commiting changes");
        add_all_changed(&repo_path, &default_commit_msg, &sign_name, &sign_email);
        time_done1.store(false, Ordering::Relaxed);
    };
    let mut _guard: Option<Guard>  = None;
    loop {
        match rx.recv() {
            Ok(event) => {
                println!("{:?}", event);
                if ! time_done.load(Ordering::Relaxed) {
                    //let time_done1 = time_done.clone();
                    _guard = Some(timer.schedule_with_delay(chrono::Duration::seconds(commit_delay), callback.clone()));
                    println!("Commit timer started, will be committed in {} seconds", commit_delay);
                    time_done.store(true, Ordering::Relaxed);
                }
            },
            Err(e) => println!("watch error: {:?}", e),
        }
    }
}
