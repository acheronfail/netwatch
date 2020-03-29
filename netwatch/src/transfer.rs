use bytesize::ByteSize;

use std::fmt::{Display, Formatter, Error};
use std::result::Result;

pub const DEFAULT_INTERVAL_MILLIS: u64 = 1_000;

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

  pub fn stats(&self, millis: u64) -> (ByteSize, ByteSize) {
    let normaliser = 1_000 / millis;
    let incoming = ByteSize(self.incoming * normaliser);
    let outgoing = ByteSize(self.outgoing * normaliser);

    (incoming, outgoing)
  }

  pub fn merge(&mut self, other: &Transfer) {
    self.incoming += other.incoming;
    self.outgoing += other.outgoing;
  }
}

impl Display for Transfer {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
    let (incoming, outgoing) = self.stats(DEFAULT_INTERVAL_MILLIS);
    let incoming = format!("{}", incoming);
    let outgoing = format!("{}", outgoing);
    f.pad(&format!("{:>8} {:>8}", incoming, outgoing))
  }
}
