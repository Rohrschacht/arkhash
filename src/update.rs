//! This module implements the update functionality

extern crate chrono;
extern crate threadpool;

use std::fs::{self, OpenOptions};
use std::path::{PathBuf};
use std::io::{BufReader, Write};
use std::thread;

use self::chrono::DateTime;

use self::threadpool::ThreadPool;


/// Updates the _algorithm_sum.txt files of some directories
///
/// # Arguments
///
/// * `opts` An Options object containing information about the program behavior
pub fn update_directories(opts: super::util::Options) {
    match opts.subdir_mode {
        false => update_hashsums(PathBuf::from(&opts.folder), opts),
        true => {
            let dirs_to_process = gather_directories_to_process(&opts);

            match opts.num_threads {
                0 => execute_threads_unlimited(&opts, dirs_to_process),
                _ => execute_threads_limited(opts, dirs_to_process)
            }
        }
    }
}

/// Reads all directories in the working directory.
///
/// # Arguments
/// * `opts` Options object containing the working directory
fn gather_directories_to_process(opts: &super::util::Options) -> Vec<PathBuf> {
    let dir_entries = fs::read_dir(&opts.folder).unwrap();

    let mut dirs_to_process = Vec::new();
    for entry in dir_entries {
        let entry = entry.unwrap();
        let metadata = entry.metadata().unwrap();

        if metadata.is_dir() {
            dirs_to_process.push(entry.path());
        }
    }

    dirs_to_process
}

/// Starts a thread for every directory in dirs_to_process and launches them all at once.
/// Waits for them to finish.
///
/// # Arguments
/// * `opts` Options object
/// * `dirs_to_process` Vector of directory paths that have to be updated
fn execute_threads_unlimited(opts: &super::util::Options, dirs_to_process: Vec<PathBuf>) -> () {
    let mut thread_handles = Vec::new();
    for entry in dirs_to_process {
        if opts.loglevel_info() {
            let now: DateTime<chrono::Local> = chrono::Local::now();
            println!("[{}] Updating Directory {}", now, entry.to_str().unwrap());
        }

        let thread_path = entry.clone();
        let thread_opts = opts.clone();
        let handle = thread::spawn(|| {
            update_hashsums(thread_path, thread_opts);
        });
        thread_handles.push(handle);
    }
    for handle in thread_handles {
        handle.join().unwrap();
    }
}

/// Starts a thread for every directory in dirs_to_process and launches opts.num_threads of them in parallel.
/// When a thread finished its work, the next one will be launched.
/// Waits for them to finish.
///
/// # Arguments
/// * `opts` Options object
/// * `dirs_to_process` Vector of directory paths that have to be updated
fn execute_threads_limited(opts: super::util::Options, dirs_to_process: Vec<PathBuf>) {
    let pool = ThreadPool::new(opts.num_threads);

    for entry in dirs_to_process {
        if opts.loglevel_info() {
            let now: DateTime<chrono::Local> = chrono::Local::now();
            println!("[{}] Updating Directory {}", now, entry.to_str().unwrap());
        }

        let thread_path = entry.clone();
        let thread_opts = opts.clone();
        pool.execute(|| {
            update_hashsums(thread_path, thread_opts);
        });
    }

    pool.join();
}

/// Updates the _algorithm_sum.txt in a directory
///
/// # Arguments
///
/// * `path` The path to the directory that is going to be updated
/// * `opts` An Options object containing information about the program behavior
fn update_hashsums(path: PathBuf, opts: super::util::Options) {
    let dirwalker = super::util::DirWalker::new(&path, opts.subdir_mode);
    let reader = BufReader::new(dirwalker);

    let filter = super::filter::Filter::new(reader, path.to_str().unwrap(), &opts);

    if let Ok(filter) = filter {
        let mut filepath = path.clone();
        filepath.push(format!("{}sum.txt", opts.algorithm));
        let mut file = OpenOptions::new().create(true).append(true).open(filepath);

        if let Ok(mut file) = file {
            for line in filter {
                let hashline = super::util::calculate_hash(line, &path, &opts);

                if let Err(e) = write!(file, "{}", hashline) {
                    eprintln!("Error writing to file: {}", e);
                }

                if opts.loglevel_info() {
                    let now: DateTime<chrono::Local> = chrono::Local::now();
                    print!("[{}] {}: {}", now, path.to_str().unwrap(), hashline);
                }
            }
        }
    }

    if opts.loglevel_info() {
        let now: DateTime<chrono::Local> = chrono::Local::now();
        println!("[{}] Directory {} Updated", now, path.to_str().unwrap());
    }
}