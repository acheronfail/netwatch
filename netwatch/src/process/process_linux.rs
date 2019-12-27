use procfs::net::{tcp, tcp6, udp, udp6, TcpState, UdpState};
use procfs::process::{all_processes, FDTarget, Process};

use std::collections::{hash_map::Entry, HashMap};
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::path::Path;

use crate::process::*;

impl PortToProcessTable {
  pub fn new() -> PortToProcessTable {
    PortToProcessTable {
      inner: HashMap::new(),
    }
  }

  // TODO: how to we clean out old values?
  pub fn refresh(&mut self) {
    // Get map of `inode -> port`.
    let inode_port_map = Self::get_inode_to_port();
    // Get map of `inode -> [pid, pid, ...]`.
    let inode_pid_map =
      Self::get_inodes_to_pid_kernel_module().unwrap_or_else(|| Self::get_inodes_to_pid());

    // Combine above information into a map of `port -> [process, process, ...]`.
    for (inode, port) in inode_port_map {
      if let Some(pids) = inode_pid_map.get(&inode) {
        for pid in pids {
          let process = Process::new(*pid).unwrap();
          match self.inner.entry(port) {
            Entry::Vacant(e) => {
              e.insert(vec![process]);
            }
            Entry::Occupied(mut e) => {
              e.get_mut().push(process);
            }
          }
        }
      }
    }
  }

  // ---------------------

  // Read from /proc/net/{tcp,udp}{,6}
  fn get_inode_to_port() -> HashMap<Inode, Port> {
    let mut inode_port_map = HashMap::new();

    let tcp = tcp().unwrap();
    let tcp6 = tcp6().unwrap();
    for entry in tcp.into_iter().chain(tcp6) {
      if entry.state == TcpState::Listen {
        inode_port_map.insert(entry.inode, entry.local_address.port());
      }
    }

    let udp = udp().unwrap();
    let udp6 = udp6().unwrap();
    for entry in udp.into_iter().chain(udp6) {
      // https://github.com/mattsta/netmatt/issues/1 ?
      if entry.state == UdpState::Established {
        inode_port_map.insert(entry.inode, entry.local_address.port());
      }
    }

    inode_port_map
  }

  // TODO: change this to a Result
  fn get_inodes_to_pid_kernel_module() -> Option<InodePIDMap> {
    // Check if we have the kernel module installed.
    let kernel_module_path = Path::new("/proc/pid_inode_map");
    if kernel_module_path.exists() {
      let mut inode_pid_map = HashMap::new();
      let file = File::open(&kernel_module_path).unwrap();
      let reader = BufReader::new(file);

      // Parses `/proc/pid_inode_map` which should in the format:
      //  PID 'PROCESS NAME' INODE INODE INODE...
      for line in reader.lines() {
        let line = line.unwrap();
        let parts = line
          .split("'")
          .map(|part| part.trim())
          .collect::<Vec<&str>>();

        // NOTE: We don't need `parts[1]` (the name) yet, since we populate
        // that later by reading from `/proc/{PID}/cmdline`.
        let (pid, inodes) = (parts[0].parse::<PID>().unwrap(), parts[2]);

        // TODO: de-dupe?
        let inodes = inodes
          .split(" ")
          .filter_map(|s| s.parse::<Inode>().ok())
          .collect::<Vec<Inode>>();

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

  // Fallback and read inodes and pids from `/proc/{PID}/fd/{FD}`.
  // This is much slower as it needs to traverse each `PID` and read it.
  fn get_inodes_to_pid() -> InodePIDMap {
    let mut inode_pid_map = HashMap::new();

    for process in all_processes().unwrap() {
      if let Ok(fds) = process.fd() {
        for fd in fds {
          if let FDTarget::Socket(inode) = fd.target {
            match inode_pid_map.entry(inode) {
              Entry::Vacant(e) => {
                e.insert(vec![process.pid]);
              }
              Entry::Occupied(mut e) => {
                e.get_mut().push(process.pid);
              }
            }
          }
        }
      }
    }

    inode_pid_map
  }
}
