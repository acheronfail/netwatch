use procfs::process::Process;

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

type Port = u16;
type PID = i32;
type Inode = u32;
type InodePIDMap = HashMap<Inode, Vec<PID>>;

#[derive(Debug)]
pub struct PortToProcessTable {
  inner: HashMap<Port, Vec<Process>>,
}

impl PortToProcessTable {
  pub fn get(&self, port: &Port) -> Option<&Vec<Process>> {
    self.inner.get(port)
  }
}
