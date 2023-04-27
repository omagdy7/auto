use clap::Parser;
use notify::event::{MetadataKind, ModifyKind};
use notify::{
    inotify::INotifyWatcher, Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
    WatcherKind,
};
use std::io::{self, BufRead};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(required = false, short, long)]
    recursive: bool,
    #[arg(required = false, short, long)]
    log: bool,
    #[arg(required = true)]
    cmd: String,
}

fn log(file: &PathBuf, command: &String) {
    print!(
        "FILE {:?}: Executing {} ",
        file.file_name().expect("Should be a file name"),
        command
    );
    println!();
}

// Takes a command and executes via sh -c
fn execute(cmd: &String) {
    let output = Command::new("sh").arg("-c").arg(cmd).output();

    match output {
        Ok(out) => {
            if out.status.success() {
                // Direct the output to the user
                let stdout = String::from_utf8_lossy(&out.stdout);
                print!("{}", stdout);
            } else {
                // Direct the errors to stderr
                let stderr = String::from_utf8_lossy(&out.stderr);
                println!("Command failed: {}", stderr);
            }
        }
        Err(e) => {
            println!("Failed to execute: {} with error {}", cmd, e);
            std::process::exit(1);
        }
    }
}

fn main() {
    let mut files_to_watch: Vec<PathBuf> = vec![];
    let args = Args::parse();
    let stdin = io::stdin();

    // Read files from stdin to watch
    for line in stdin.lock().lines() {
        match line {
            Ok(line) => files_to_watch.push(PathBuf::from(line)),
            Err(e) => println!("Error {}, while reading from stdin", e),
        }
    }

    // Create a channel to receive the events
    let (tx, rx) = std::sync::mpsc::channel();

    // create watcher
    let mut watcher: Box<dyn Watcher> = if RecommendedWatcher::kind() == WatcherKind::Inotify {
        let config = Config::default().with_poll_interval(Duration::from_secs(1));

        let watcher = match INotifyWatcher::new(tx, config) {
            Ok(watcher) => watcher,
            Err(e) => {
                println!("Error intializing INotifyWatcher {}", e);
                std::process::exit(1);
            }
        };

        Box::new(watcher)
    } else {
        // use default config for everything else
        let watcher = match RecommendedWatcher::new(tx, Config::default()) {
            Ok(watcher) => watcher,
            Err(e) => {
                println!("Error intializing INotifyWatcher {}", e);
                std::process::exit(1);
            }
        };

        Box::new(watcher)
    };

    // Add the paths to be watched. All of them will use the same event mask.
    for file in &files_to_watch {
        if file.is_dir() && args.recursive {
            match watcher.watch(file, RecursiveMode::Recursive) {
                Ok(_) => {}
                Err(e) => {
                    println!("Failed to watch dir {:?}, with error {}", file, e);
                    std::process::exit(1);
                }
            }
        } else {
            match watcher.watch(file, RecursiveMode::NonRecursive) {
                Ok(_) => {}
                Err(e) => {
                    println!("Failed to watch file {:?}, with error {}", file, e);
                    std::process::exit(1);
                }
            }
        }
    }

    // Loop over the received events
    loop {
        match rx.recv() {
            Ok(_event) => {
                match _event {
                    Ok(event) => {
                        /* If the event is a modifies the file metadata for one of the watched files,
                        Execute some arbitrary command */
                        if let EventKind::Modify(ModifyKind::Metadata(MetadataKind::Any)) =
                            event.kind
                        {
                            if let Some(path) = event.paths.get(0) {
                                if event.paths.contains(path) {
                                    if args.log {
                                        log(path, &args.cmd)
                                    }
                                    execute(&args.cmd);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("Event error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            Err(e) => {
                println!("Watch error: {}", e);
                std::process::exit(1);
            }
        }
    }
}
