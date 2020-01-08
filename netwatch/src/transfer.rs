use bytesize::ByteSize;

use std::fmt::{Display, Formatter, Error};
use std::result::Result;

#[derive(Debug, Copy, Clone)]
pub struct Transfer {
  incoming: u64,
  outgoing: u64,
}

impl Transfer {
  pub fn new() -> Transfer {
    Transfer {
      incoming: 0,
      outgoing: 0,
    }
  }

  pub fn incr_incoming(&mut self, incr: u64) {
    self.incoming += incr;
  }

  pub fn incr_outgoing(&mut self, incr: u64) {
    self.outgoing += incr;
  }

  pub fn reset(&mut self) {
    self.incoming = 0;
    self.outgoing = 0;
  }

  pub fn stats(&self) -> (ByteSize, ByteSize) {
    // TODO: needs to be calc'd over a time interval
    (ByteSize(self.incoming), ByteSize(self.outgoing))
  }

  pub fn merge(&mut self, other: &Transfer) {
    self.incoming += other.incoming;
    self.outgoing += other.outgoing;
  }
}

impl Display for Transfer {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
    let (incoming, outgoing) = self.stats();
    let incoming = format!("{}", incoming);
    let outgoing = format!("{}", outgoing);
    f.pad(&format!("{:>8} {:>8}", incoming, outgoing))
  }
}
