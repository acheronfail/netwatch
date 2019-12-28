use std::collections::HashMap;

// TODO: make this a trait or generic across platforms
use procfs::process::Process;

#[cfg(target_os = "linux")]
#[path = "port_linux.rs"]
mod port_inner;

#[cfg(target_os = "macos")]
#[path = "port_macos.rs"]
mod port_inner;

#[cfg(target_os = "windows")]
#[path = "port_windows.rs"]
mod port_inner;

pub type Port = u16;

#[derive(Debug)]
pub struct PortMapper {
  inner: HashMap<Port, Vec<Process>>,
}

impl PortMapper {
  pub fn get(&self, port: &Port) -> Option<&Vec<Process>> {
    self.inner.get(port)
  }
}
