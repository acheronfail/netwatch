use std::collections::{hash_map::Entry, HashMap};
use std::collections::hash_map::IntoIter;

use crate::transfer::Transfer;

pub type PID = i32;
pub type Info = (Transfer, Vec<String>);

pub struct ConnectionList {
  /// Map of PIDs to their transfers and identifiers.
  connections: HashMap<PID, Info>,
  /// A `Transfer` for unknown PIDs.
  unknown: Transfer,
}

impl ConnectionList {
  pub fn new() -> ConnectionList {
    ConnectionList {
      connections: HashMap::new(),
      unknown: Transfer::new()
    }
  }

  pub fn insert(&mut self, pid: PID, transfer: &Transfer, process_name: String) {
    match self.connections.entry(pid) {
      Entry::Vacant(e) => {
        e.insert((transfer.clone(), vec![process_name]));
      }
      Entry::Occupied(mut e) => {
        let connections = e.get_mut();
        connections.0.merge(transfer);
        connections.1.push(process_name);
      }
    }
  }

  pub fn insert_unknown(&mut self, transfer: &Transfer) {
    self.unknown.merge(transfer);
  }

  pub fn print(&self) {
    // TODO: print out information -> maybe impl display?
  }
}

impl IntoIterator for ConnectionList {
  type Item = (PID, Info);
  type IntoIter = IntoIter<PID, Info>;

  fn into_iter(self) -> Self::IntoIter {
      self.connections.into_iter()
  }
}
