use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use users::{get_group_by_name, get_user_by_name, get_user_by_uid, uid_t};

#[derive(Debug, PartialEq)]
pub enum PermissionType {
    // commands
    COMMANDS(Vec<String>),
    COMMANDS_ALL,
    COMMANDS_REGEX(String),
    COMMANDS_NOPASSWD(Vec<String>),
    // password
    NOPASSWD,
}
impl PermissionType {
    fn get_by_args(args: Vec<&str>) -> Vec<PermissionType> {
        let mut permissions = vec![];

        // iterate through each argument
        for arg in args {
            // split argument in 2 with a colon
            match arg.split_once(":") {
                Some(("permit", "all")) => permissions.push(PermissionType::COMMANDS_ALL),
                Some(("permit", r)) => permissions.push(PermissionType::COMMANDS(
                    r.split(",").map(|s| s.to_string()).collect(),
                )),
                Some(("permit_nopasswd", r)) => {
                    permissions.push(PermissionType::COMMANDS_NOPASSWD(
                        r.split(",").map(|s| s.to_string()).collect(),
                    ))
                }
                Some((a, b)) => panic!("No such argument {}:{}", a, b),
                // argument without a paremeter
                _ => match arg {
                    "nopasswd" => permissions.push(PermissionType::NOPASSWD),
                    _ => panic!("No such argument: {}", arg),
                },
            }
        }

        permissions
    }
}

#[derive(Debug)]
pub struct NyadoConfig {
    user_permission_map: HashMap<uid_t, Vec<PermissionType>>,
    group_permission_map: HashMap<uid_t, Vec<PermissionType>>,
}
impl NyadoConfig {
    pub fn new<P: AsRef<std::path::Path> + std::fmt::Display>(path: P) -> Self {
        let mut conf = Self {
            user_permission_map: HashMap::new(),
            group_permission_map: HashMap::new(),
        };

        let file = File::open(path.as_ref()).expect(format!("No such file: {path}").as_str());
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line.expect("Error reading line");
            let line = line.trim();
            // allow for comments
            if line.starts_with('#') {
                continue;
            }

            // split line by whitespace
            let mut split = line.split_whitespace();
            match (split.next(), split.next()) {
                // user permission
                (Some("user"), Some(u)) => {
                    let split = split.collect::<Vec<&str>>();
                    // get ID by name and use PermissionType::get_by_args() to parse permissions
                    conf.user_permission_map.insert(
                        get_user_by_name(u)
                            .expect(format!("No such user: {u}").as_str())
                            .uid(),
                        PermissionType::get_by_args(split),
                    );
                }
                // group permission
                (Some("group"), Some(u)) => {
                    let split = split.collect::<Vec<&str>>();
                    // get ID by name and use PermissionType::get_by_args() to parse permissions
                    conf.group_permission_map.insert(
                        get_group_by_name(u)
                            .expect(format!("No such group: {u}").as_str())
                            .gid(),
                        PermissionType::get_by_args(split),
                    );
                }
                // unknown first field
                (Some(s), _) => panic!("No such field: {s}"),
                // empty line
                (None, None) => continue,
                // anything else
                _ => panic!("Unexpected line: {line}"),
            }
        }

        conf
    }

    // 0: command allowed
    // 1: password needed
    pub fn user_match_perms(&self, uid: uid_t, cmd: String) -> (bool, bool) {
        // if user is not in the permission map -> check for groups
        if !self.user_permission_map.contains_key(&uid) {
            return self.group_match_perms(uid, cmd);
        }

        let user =
            get_user_by_uid(uid).expect(format!("Failed to get user by uid: {}", uid).as_str());

        self.match_perms(self.user_permission_map.get(&uid).unwrap(), &cmd)
    }

    pub fn group_match_perms(&self, uid: uid_t, cmd: String) -> (bool, bool) {
        let user =
            get_user_by_uid(uid).expect(format!("Failed to get user by uid: {}", uid).as_str());

        // get GIDs of user
        let groups = user
            .groups()
            .expect("Failed to get user groups")
            .into_iter()
            .map(|g2| g2.gid())
            .collect::<Vec<_>>();

        // filter out every group that is not shared with the group permission map
        let matching_groups = self
            .group_permission_map
            .keys()
            .filter(|&g| groups.contains(g))
            .collect::<Vec<_>>();

        // loop through each matching group and check permission
        // until all groups are done with
        let mut result = (false, true);
        for gid in matching_groups {
            result = self.match_perms(self.group_permission_map.get(gid).unwrap(), &cmd);
        }

        // return result
        result
    }

    fn match_perms(&self, permissions: &[PermissionType], cmd: &String) -> (bool, bool) {
        let mut command = false;
        let mut password = true;

        for perm in permissions {
            match perm {
                PermissionType::COMMANDS_ALL => {
                    command = true;
                    break;
                }
                PermissionType::COMMANDS(v) => {
                    if v.contains(cmd) {
                        command = true;
                    }
                }
                PermissionType::COMMANDS_REGEX(_r) => panic!("Regex not implemented"),
                PermissionType::COMMANDS_NOPASSWD(v) => {
                    if v.contains(cmd) {
                        command = true;
                        password = false;
                        break;
                    } else {
                        password = true;
                    }
                }
                PermissionType::NOPASSWD => {
                    password = false;
                }
            }
        }

        (command, password)
    }
}
