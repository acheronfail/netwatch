use std::collections::{hash_map::Entry, HashMap};

use crate::port::Port;
use crate::transfer::Transfer;

pub struct ConnectionTable {
  // TODO: make this private
  pub inner: HashMap<Port, Transfer>,
}

impl ConnectionTable {
  pub fn new() -> ConnectionTable {
    ConnectionTable {
      inner: HashMap::new(),
    }
  }

  fn incr(&mut self, is_incoming: bool, port: Port, size: u64) {
    match self.inner.entry(port) {
      Entry::Vacant(e) => {
        e.insert(Transfer::new());
      }
      Entry::Occupied(mut e) => {
        if is_incoming {
          e.get_mut().incr_incoming(size);
        } else {
          e.get_mut().incr_outgoing(size);
        }
      }
    }
  }

  pub fn incr_outgoing(&mut self, port: Port, size: u64) {
    self.incr(false, port, size);
  }

  pub fn incr_incoming(&mut self, port: Port, size: u64) {
    self.incr(true, port, size);
  }
}
