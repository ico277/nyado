use std::collections::HashMap;
use users::uid_t;

enum PermissionType {
    ALL,
    REGEX(()),
}

pub struct NyadoConfig {
    user_permission_map: HashMap<uid_t, PermissionType>,
    group_permission_map: HashMap<uid_t, PermissionType>,
}
