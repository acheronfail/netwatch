use bytesize::ByteSize;

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
}
