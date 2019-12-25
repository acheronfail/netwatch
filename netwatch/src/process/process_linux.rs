use std::collections::{hash_map::Entry, HashMap};
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::path::Path;

use crate::process::*;

// TODO: finish porting needed parts of https://github.com/mattsta/netmatt/blob/master/net-listeners-unified.py

impl ProcessPIDTable {
  pub fn new() -> ProcessPIDTable {
    let inode_pid_map = Self::read_pid_inode_map().unwrap_or_else(|| Self::read_proc_fs());
    ProcessPIDTable { inode_pid_map }
  }

  fn read_pid_inode_map() -> Option<InodePIDMap> {
    // Check if we have the kernel module installed.
    let kernel_module_path = Path::new("/proc/pid_inode_map");
    if kernel_module_path.exists() {
      let mut inode_pid_map = HashMap::new();
      let file = File::open(&kernel_module_path).unwrap();
      let reader = BufReader::new(file);

      // Parses:
      //  PID 'PROCESS NAME' INODE INODE INODE...
      for line in reader.lines() {
        let line = line.unwrap();
        let parts = line
          .split("'")
          .map(|part| part.trim())
          .collect::<Vec<&str>>();

        let pid = parts[0].parse::<u64>().unwrap();
        // let name = parts[1];
        // TODO: de-dupe?
        let inodes = parts[2]
          .split(" ")
          .filter_map(|inode_str| inode_str.parse::<u64>().ok())
          .collect::<Vec<u64>>();

        for inode in inodes {
          match inode_pid_map.entry(inode) {
            Entry::Vacant(e) => {
              e.insert(vec![pid]);
            }
            Entry::Occupied(mut e) => {
              e.get_mut().push(pid);
            }
          }
        }
      }

      Some(inode_pid_map)
    } else {
      None
    }
  }

  fn read_proc_fs() -> InodePIDMap {
    // TODO: fallback and read /proc filesystem
    HashMap::new()
  }
}
