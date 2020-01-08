use std::collections::{hash_map::Entry, HashMap};

use crate::port::Port;
use crate::transfer::Transfer;

// ConnectionTable is a struct optimised for updating network usage for a specific port.
// The packet handlers will write to it and the UI will read from it.
pub struct ConnectionTable {
  inner: HashMap<Port, Transfer>,
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

// TODO: make the following a macro
// --- consuming iterator

// structure helper for consuming iterator.
pub struct IntoIteratorHelper {
  iter: ::std::collections::hash_map::IntoIter<Port, Transfer>,
}

// implement the IntoIterator trait for a consuming iterator. Iteration will
// consume the ConnectionTable structure
impl IntoIterator for ConnectionTable {
  type Item = (Port, Transfer);
  type IntoIter = IntoIteratorHelper;

  // note that into_iter() is consuming self
  fn into_iter(self) -> Self::IntoIter {
      IntoIteratorHelper {
          iter: self.inner.into_iter(),
      }
  }
}

// now, implements Iterator trait for the helper struct, to be used by adapters
impl Iterator for IntoIteratorHelper {
  type Item = (Port, Transfer);

  // just return the str reference
  fn next(&mut self) -> Option<Self::Item> {
          self.iter.next()
  }
}

// --- non-consuming iterator

// structure helper for non-consuming iterator.
pub struct IterHelper<'a> {
  iter: ::std::collections::hash_map::Iter<'a, Port, Transfer>,
}

// implement the IntoIterator trait for a non-consuming iterator. Iteration will
// borrow the ConnectionTable structure
impl<'a> IntoIterator for &'a ConnectionTable {
  type Item = (&'a Port, &'a Transfer);
  type IntoIter = IterHelper<'a>;

  // note that into_iter() is consuming self
  fn into_iter(self) -> Self::IntoIter {
      IterHelper {
          iter: self.inner.iter(),
      }
  }
}

// now, implements Iterator trait for the helper struct, to be used by adapters
impl<'a> Iterator for IterHelper<'a> {
  type Item = (&'a Port, &'a Transfer);

  // just return the str reference
  fn next(&mut self) -> Option<Self::Item> {
          self.iter.next()
  }
}

// --- mutable non-consuming iterator

// structure helper for mutable non-consuming iterator.
pub struct IterMutHelper<'a> {
  iter: ::std::collections::hash_map::IterMut<'a, Port, Transfer>,
}

// implement the IntoIterator trait for a mutable non-consuming iterator. Iteration will
// mutably borrow the ConnectionTable structure
impl<'a> IntoIterator for &'a mut ConnectionTable {
  type Item = (&'a Port, &'a mut Transfer);
  type IntoIter = IterMutHelper<'a>;

  // note that into_iter() is consuming self
  fn into_iter(self) -> Self::IntoIter {
      IterMutHelper {
          iter: self.inner.iter_mut(),
      }
  }
}

// now, implements Iterator trait for the helper struct, to be used by adapters
impl<'a> Iterator for IterMutHelper<'a> {
  type Item = (&'a Port, &'a mut Transfer);

  // just return the str reference
  fn next(&mut self) -> Option<Self::Item> {
          self.iter.next()
  }
}
