// TODO: list processes and the sockets they currently own per platform

use std::collections::HashMap;

#[cfg(target_os = "linux")]
#[path = "process_linux.rs"]
mod process_inner;

#[cfg(target_os = "macos")]
#[path = "process_macos.rs"]
mod process_inner;

#[cfg(target_os = "windows")]
#[path = "process_windows.rs"]
mod process_inner;

// TODO: should inodes be u64?
type InodePIDMap = HashMap<u64, Vec<u64>>;

pub struct ProcessPIDTable {
  pub inode_pid_map: InodePIDMap,
}
