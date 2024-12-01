mod nyado_config;

use clap::Parser;
use nyado_config::NyadoConfig;
use std::env::args;
use std::ffi::OsStr;
use std::fmt::Display;
use std::io::stdin;
use std::os::unix::process::CommandExt;
use std::process::exit;
use std::process::Command;
use std::process::Stdio;
use users::{get_current_uid, get_effective_uid, get_user_by_name, get_user_by_uid, uid_t};

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
    let mut cmd = cmd
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    unsafe {
        cmd = cmd.pre_exec(move || {
            // get GID from UID
            let gid = match get_user_by_uid(uid) {
                Some(user) => user.primary_group_id(),
                None => panic!("No such user: {}", uid),
            };
            // set permission
            libc::setuid(uid);
            libc::setgid(gid);
            println!("Setting user ID to {}", uid);
            println!("Setting user GID to {}", gid);
            Ok(())
        });
    }

    // actually run the command and get the status
    let status = cmd.status();

    // check status
    match status {
        // it is possible that the cmd thread panics therefore return -1 just in case
        Ok(s) => exit(s.code().unwrap_or(-1)),
        // TODO: add more errors checks to handle errors a bit more fancy
        Err(e) => panic!("{}: Failed to execute command: {}", app_name, e),
    }
}

#[cfg(feature = "pam")]
fn ask_password(uid: uid_t) -> bool {
    let service = "login";
    let user = get_user_by_uid(uid).unwrap();
    let user = user.name();
    let mut line = String::new();
    stdin().read_line(&mut line).expect("TODO: panic message");
    let line = line.replace("\n", "");
    println!("{:#?} {:#?}", user, line);

    let mut auth = pam::Authenticator::with_password(service).unwrap();
    auth.get_handler()
        .set_credentials(user.to_string_lossy(), line);

    auth.authenticate().is_ok()
}

fn main() {
    // TODO argument parsing
    // cut out app name
    let mut args = args();
    let app_name = args.next().unwrap();
    let mut args: Vec<String> = args.collect();

    // cut out flags
    let mut flags = vec![app_name.clone()];
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
    #[cfg(debug_assertions)]
    {
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
        println!(
            "{}: Warning: executable does not have root permission!",
            app_name
        );
    }

    // read config
    let config = NyadoConfig::new("./reference/example.conf");
    //println!("{:#?}", config);

    // get the command
    let cmd = args.remove(0);
    // permission checks
    let perms = config.user_match_perms(cur_uid, cmd.clone());

    // if password required
    if perms.1 {
        if !ask_password(cur_uid) {
            eprintln!("{}: Permission denied", app_name);
            exit(1)
        }
    }

    // if command permission
    if perms.0 {
        // root by default
        let mut user = get_user_by_uid(0).unwrap();
        // userid
        if let Some(uid) = flags.userid {
            user = match get_user_by_uid(uid) {
                Some(user) => user,
                None => {
                    eprintln!("{}: No such user: {}", app_name, uid);
                    exit(1)
                }
            };
        }
        // user name
        else if let Some(username) = flags.username {
            user = match get_user_by_name(username.as_str()) {
                Some(user) => user,
                None => {
                    eprintln!("{}: No such user: {}", app_name, username);
                    exit(1)
                }
            };
        }
        // finallly, run the command
        run_command(user.uid(), app_name, cmd, args)
    } else {
        eprintln!("{}: Permission denied", app_name);
        exit(1);
    }
}
