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

fn execute(cmd: &String) {
    let output = Command::new("sh").arg("-c").arg(cmd).output();

    match output {
        Ok(out) => {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                print!("{}", stdout);
            } else {
                let stderr = String::from_utf8_lossy(&out.stderr);
                println!("Command failed: {}", stderr);
            }
        }
        Err(e) => {
            println!("Failed to execute: {} with error {}", cmd, e);
        }
    }
}

fn main() {
    let mut files_to_watch: Vec<PathBuf> = vec![];
    let args = Args::parse();
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        files_to_watch.push(PathBuf::from(line.unwrap()));
    }

    // Create a channel to receive the events
    let (tx, rx) = std::sync::mpsc::channel();

    // create watcher
    let mut watcher: Box<dyn Watcher> = if RecommendedWatcher::kind() == WatcherKind::Inotify {
        let config = Config::default().with_poll_interval(Duration::from_secs(1));
        Box::new(INotifyWatcher::new(tx, config).unwrap())
    } else {
        // use default config for everything else
        Box::new(RecommendedWatcher::new(tx, Config::default()).unwrap())
    };

    // Add the paths to be watched. All of them will use the same event mask.
    for file in &files_to_watch {
        if file.is_dir() && args.recursive {
            watcher
                .watch(file, RecursiveMode::Recursive)
                .expect(format!("Failed to watch dir {:?}", file).as_str());
        } else {
            watcher
                .watch(file, RecursiveMode::NonRecursive)
                .expect(format!("Failed to watch {:?}", file).as_str());
        }
    }

    // Loop over the received events
    loop {
        match rx.recv() {
            Ok(event) => {
                let event = event.unwrap();
                /* If the event is a modifies the file metadata for one of the watched files,
                Execute some arbitrary command */
                if let EventKind::Modify(ModifyKind::Metadata(MetadataKind::Any)) = event.kind {
                    if let Some(path) = event.paths.get(0) {
                        if event.paths.contains(path) {
                            if args.log {
                                print!(
                                    "FILE {:?}: Executing {} ",
                                    path.file_name().expect("Should be a file name"),
                                    &args.cmd
                                );
                                println!();
                            }
                            execute(&args.cmd);
                        }
                    }
                }
            }
            Err(e) => println!("watch error: {:?}", e),
        }
    }
}
