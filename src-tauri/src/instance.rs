//! A new process and the running Hamster: handing a launch over, taking the instance lock, and
//! asking the running app on behalf of a subcommand.

use std::fs::File;
use std::path::Path;
use std::time::Duration;

use crate::ipc::{self, Forward, IpcError, IpcMessage};
use crate::{cli, fail};

const LOCK_NAME: &str = "hamster.lock";
const LOCK_POLL: Duration = Duration::from_millis(200);
const LOCK_POLLS: u32 = 25;

/// Exits when a running instance takes the message; otherwise returns the instance lock, held for the
/// app's lifetime so two launches that both find no listener can't both start. `None` when the lock
/// cannot be taken at all.
pub fn forward_or_lock(config_dir: &Path, socket: &Path, message: &IpcMessage) -> Option<File> {
    forward(socket, message);
    match lock_instance(config_dir) {
        Ok(Some(lock)) => Some(lock),
        Ok(None) => wait_for_instance(config_dir, socket, message),
        Err(e) => {
            eprintln!("Cannot lock the Hamster instance: {e}");
            None
        }
    }
}

/// `None` when another process holds the lock.
fn lock_instance(dir: &Path) -> std::io::Result<Option<File>> {
    let file = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dir.join(LOCK_NAME))?;
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(std::fs::TryLockError::WouldBlock) => Ok(None),
        Err(std::fs::TryLockError::Error(e)) => Err(e),
    }
}

/// Exits when a running instance answers or refuses the message; returns when none is listening.
fn forward(socket: &Path, message: &IpcMessage) {
    match ipc::try_forward(socket, message) {
        Ok(Forward::Delivered) => std::process::exit(0),
        Ok(Forward::NoInstance) => {}
        Ok(Forward::Rejected) => fail("The running Hamster did not accept the request"),
        Err(e) => fail(&format!("Could not reach the running Hamster: {e}")),
    }
}

/// The lock holder may still be starting up and not listen yet, or quitting after a profile switch
/// (`profiles::relaunch`); the lock is taken once it is free.
fn wait_for_instance(config_dir: &Path, socket: &Path, message: &IpcMessage) -> Option<File> {
    for _ in 0..LOCK_POLLS {
        std::thread::sleep(LOCK_POLL);
        forward(socket, message);
        if let Ok(Some(lock)) = lock_instance(config_dir) {
            return Some(lock);
        }
    }
    fail("Another Hamster is starting but does not answer")
}

/// Sends a subcommand's message to the running app and prints its answer; returns the exit code, or
/// `None` when nothing listens.
pub fn ask(socket: &Path, message: &IpcMessage, json: bool) -> Option<i32> {
    let reply = match ipc::request(socket, message.clone()) {
        Ok(Some(reply)) => reply,
        Ok(None) => return None,
        Err(e) => fail(&format!("Could not reach the running Hamster: {e}")),
    };
    match reply.into_result() {
        Ok(result) => {
            print_result(message, &result, json);
            Some(0)
        }
        Err(error) => {
            print_error(&error, json);
            Some(1)
        }
    }
}

fn print_error(error: &IpcError, json: bool) {
    if json {
        println!("{}", serde_json::to_value(error).unwrap_or_default());
    } else {
        eprintln!("{}", error.message);
    }
}

fn print_result(message: &IpcMessage, result: &serde_json::Value, json: bool) {
    if json {
        if !result.is_null() {
            println!("{result}");
        }
        return;
    }
    match cli::reply::render(message, result) {
        Ok(Some(text)) => println!("{text}"),
        Ok(None) => {}
        Err(e) => fail(&e),
    }
}

/// After a profile switch (ADR 0060): starts a new Hamster through LaunchServices and quits this one. The
/// socket goes first, so the new process
/// doesn't hand its launch to this one; it waits for this one's lock instead (`wait_for_instance`).
pub fn relaunch(app: &tauri::AppHandle) -> crate::error::AppResult<()> {
    let socket = crate::socket_path();
    if let Err(e) = std::fs::remove_file(&socket)
        && e.kind() != std::io::ErrorKind::NotFound
    {
        return Err(e.into());
    }
    let exe = std::env::current_exe()?;
    let bundle = exe
        .ancestors()
        .find(|p| p.extension().is_some_and(|e| e == "app"));
    match bundle {
        Some(bundle) => {
            std::process::Command::new("/usr/bin/open")
                .arg("-n")
                .arg(bundle)
                .status()?;
        }
        None => {
            std::process::Command::new(exe).spawn()?;
        }
    }
    app.exit(0);
    Ok(())
}
