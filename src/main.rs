mod NyadoConfig;

use clap::Parser;
use std::env::args;
use std::ffi::OsStr;
use std::fmt::Display;
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::process::Stdio;
use users::{get_current_uid, get_effective_uid, get_user_by_uid, uid_t};

#[derive(Parser, Debug)]
#[command(version = env!("CARGO_PKG_VERSION"), about = None, long_about = None)]
struct Args {
    /// The user to use by name (e.g. toor)
    #[arg(short = 'u', long = "user")]
    username: Option<String>,
    /// The user to use by id (e.g. 1000)
    #[arg(short = 'U', long = "userid")]
    userid: Option<uid_t>,

    #[arg(short, long)]
    login: bool,
}

fn run_command<S: AsRef<OsStr> + Display>(uid: uid_t, app_name: S, cmd: S, args: Vec<S>) -> ! {
    // run command piping all the out/inputs directly to the application
    let mut cmd = Command::new(cmd);
    let cmd = cmd
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    unsafe {
        cmd.pre_exec(move || {
            // get GID from UID
            let gid = match get_user_by_uid(uid) {
                Some(user) => user.primary_group_id(),
                None => panic!("No such user: {}", uid),
            };
            // set permission
            libc::setuid(uid);
            libc::setgid(gid);
            Ok(())
        });
    }

    // actually run the command and get the status
    let status = cmd.status();

    // check status
    match status {
        // it is possible that the cmd thread panics therefore return -1 just in case
        Ok(s) => std::process::exit(s.code().unwrap_or(-1)),
        // TODO: add more errors checks to handle errors a bit more fancy
        Err(e) => panic!("{}: Failed to execute command: {}", app_name, e),
    }
}

fn main() {
    // TODO argument parsing
    // cut out app name
    let mut args = args();
    let app_name = args.next().unwrap();
    let mut args: Vec<String> = args.collect();

    // cut out flags
    let mut flags = vec![];
    let mut flags_end_index = 0;
    for i in 0..args.len() {
        let arg = &args[i];
        // if arg is a flag -> store it into flags
        if arg.starts_with('-') {
            flags.push(arg.to_string());
        } else {
            // end of flags found -> break loop and store the index
            flags_end_index = i;
            break;
        }
    }
    // remove the unneeded flags
    args.drain(0..flags_end_index);

    // argument parsing
    let flags = Args::parse_from(flags);

    // get users for permission check
    let cur_uid = get_current_uid();
    let eff_uid = get_effective_uid();
    #[cfg(debug_assertions)] {
        let username = match get_user_by_uid(cur_uid) {
            Some(user) => user.name().to_string_lossy().into_owned(),
            None => "unknown".to_string(),
        };
        println!("Current user: {}", username);
    }
    // check permission of executable
    if eff_uid == 0 && cur_uid != eff_uid {
        #[cfg(debug_assertions)]
        println!("This executable is setuid-ed to run as root.");
    } else if eff_uid == 0 {
        #[cfg(debug_assertions)]
        println!("This executable is running as root.");
    } else {
        println!("Warning: executable does not have root permission!");
    }


    // run the command
    let cmd = args.remove(0);
    run_command(10200, app_name, cmd, args);
}
