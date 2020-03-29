use pnet::datalink::Channel::Ethernet;
use pnet::datalink::{self, NetworkInterface};
use pnet::packet::arp::ArpPacket;
use pnet::packet::ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket};
use pnet::packet::icmp::{echo_reply, echo_request, IcmpPacket, IcmpTypes};
use pnet::packet::icmpv6::Icmpv6Packet;
use pnet::packet::ip::{IpNextHeaderProtocol, IpNextHeaderProtocols};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::ipv6::Ipv6Packet;
use pnet::packet::tcp::TcpPacket;
use pnet::packet::udp::UdpPacket;
use pnet::packet::Packet;
use pnet::util::MacAddr;

use std::net::IpAddr;
use std::thread;

#[derive(Debug)]
pub struct SrcDest(pub IpAddr, pub IpAddr);

// TODO: document this
// handle_ethernet_frame
//  handle_arp_packet
//  handle_ipv4_packet, handle_ipv6_packet
//      handle_transport_protocol
//          handle_tcp_packet
//          handle_udp_packet
//          handle_icmp_packet
//          handle_icmpv6_packet
// TODO: use lifetimes rather than `'static + FnMut`
pub struct PacketMonitor {
  pub interface: NetworkInterface,

  handler_ethernet_frame: Option<Box<dyn FnMut(&NetworkInterface, &EthernetPacket)>>,

  handler_arp_packet: Option<Box<dyn FnMut(&NetworkInterface, &EthernetPacket, &ArpPacket)>>,
  handler_ipv4_packet: Option<Box<dyn FnMut(&NetworkInterface, &EthernetPacket, &Ipv4Packet)>>,
  handler_ipv6_packet: Option<Box<dyn FnMut(&NetworkInterface, &EthernetPacket, &Ipv6Packet)>>,
  handler_transport_protocol:
    Option<Box<dyn FnMut(&NetworkInterface, &SrcDest, IpNextHeaderProtocol, &[u8])>>,

  handler_tcp_packet: Option<Box<dyn FnMut(&NetworkInterface, &SrcDest, &TcpPacket)>>,
  handler_udp_packet: Option<Box<dyn FnMut(&NetworkInterface, &SrcDest, &UdpPacket)>>,
  handler_icmp_packet: Option<Box<dyn FnMut(&NetworkInterface, &SrcDest, &IcmpPacket)>>,
  handler_icmpv6_packet: Option<Box<dyn FnMut(&NetworkInterface, &SrcDest, &Icmpv6Packet)>>,
}

impl PacketMonitor {
  pub fn new(interface: NetworkInterface) -> PacketMonitor {
    PacketMonitor {
      interface,

      handler_ethernet_frame: None,

      handler_arp_packet: None,
      handler_ipv4_packet: None,
      handler_ipv6_packet: None,
      handler_transport_protocol: None,

      handler_icmp_packet: None,
      handler_icmpv6_packet: None,
      handler_tcp_packet: None,
      handler_udp_packet: None,
    }
  }

  pub fn logger(interface: NetworkInterface) -> PacketMonitor {
    let mut packet_monitor = PacketMonitor::new(interface);

    packet_monitor.set_handler_arp_packet(|iface, eth, arp| {
      println!(
        "[{}]: ARP packet: {}({}) > {}({}); operation: {:?}",
        iface.name,
        eth.get_source(),
        arp.get_sender_proto_addr(),
        eth.get_destination(),
        arp.get_target_proto_addr(),
        arp.get_operation()
      )
    });

    packet_monitor.set_handler_icmp_packet(|iface, src_dest, icmp| match icmp.get_icmp_type() {
      IcmpTypes::EchoReply => {
        let echo_reply_packet = echo_reply::EchoReplyPacket::new(icmp.packet()).unwrap();
        println!(
          "[{}]: ICMP echo reply {} -> {} (seq={:?}, id={:?})",
          iface.name,
          src_dest.0,
          src_dest.1,
          echo_reply_packet.get_sequence_number(),
          echo_reply_packet.get_identifier()
        );
      }
      IcmpTypes::EchoRequest => {
        let echo_request_packet = echo_request::EchoRequestPacket::new(icmp.packet()).unwrap();
        println!(
          "[{}]: ICMP echo request {} -> {} (seq={:?}, id={:?})",
          iface.name,
          src_dest.0,
          src_dest.1,
          echo_request_packet.get_sequence_number(),
          echo_request_packet.get_identifier()
        );
      }
      _ => println!(
        "[{}]: ICMP packet {} -> {} (type={:?})",
        iface.name,
        src_dest.0,
        src_dest.1,
        icmp.get_icmp_type()
      ),
    });

    packet_monitor.set_handler_icmpv6_packet(|iface, src_dest, icmpv6| {
      println!(
        "[{}]: ICMPv6 packet {} -> {} (type={:?})",
        iface.name,
        src_dest.0,
        src_dest.1,
        icmpv6.get_icmpv6_type()
      )
    });

    packet_monitor.set_handler_tcp_packet(|iface, src_dest, tcp| {
      println!(
        "[{}]: TCP Packet: {}:{} > {}:{}; length: {}",
        iface.name,
        src_dest.0,
        tcp.get_source(),
        src_dest.1,
        tcp.get_destination(),
        tcp.packet().len()
      )
    });

    packet_monitor.set_handler_udp_packet(|iface, src_dest, udp| {
      println!(
        "[{}]: UDP Packet: {}:{} > {}:{}; length: {}",
        iface.name,
        src_dest.0,
        udp.get_source(),
        src_dest.1,
        udp.get_destination(),
        udp.get_length() // TODO: should this be `udp.packet().len()` instead?
      )
    });

    packet_monitor
  }

  // ----------------------

  // TODO: implement a `stop` function that turns this off
  pub fn start(&mut self) {
    // Create a channel to receive on
    let (_, mut rx) = match datalink::channel(&self.interface, Default::default()) {
      Ok(Ethernet(tx, rx)) => (tx, rx),
      Ok(_) => panic!("packetdump: unhandled channel type: {}"),
      Err(e) => panic!("packetdump: unable to create channel: {}", e),
    };

    thread::spawn(move || {
      loop {
        let mut buf: [u8; 1600] = [0u8; 1600];
        let mut fake_ethernet_frame = MutableEthernetPacket::new(&mut buf[..]).unwrap();
        match rx.next() {
          Ok(packet) => {
            if cfg!(target_os = "macos")
              && self.interface.is_up()
              && !self.interface.is_broadcast()
              && !self.interface.is_loopback()
              && self.interface.is_point_to_point()
            {
              // Maybe is TUN interface
              let version = Ipv4Packet::new(&packet).unwrap().get_version();
              if version == 4 {
                fake_ethernet_frame.set_destination(MacAddr(0, 0, 0, 0, 0, 0));
                fake_ethernet_frame.set_source(MacAddr(0, 0, 0, 0, 0, 0));
                fake_ethernet_frame.set_ethertype(EtherTypes::Ipv4);
                fake_ethernet_frame.set_payload(&packet);
                self.handle_ethernet_frame(&fake_ethernet_frame.to_immutable());
                continue;
              } else if version == 6 {
                fake_ethernet_frame.set_destination(MacAddr(0, 0, 0, 0, 0, 0));
                fake_ethernet_frame.set_source(MacAddr(0, 0, 0, 0, 0, 0));
                fake_ethernet_frame.set_ethertype(EtherTypes::Ipv6);
                fake_ethernet_frame.set_payload(&packet);
                self.handle_ethernet_frame(&fake_ethernet_frame.to_immutable());
                continue;
              }
            }
            self.handle_ethernet_frame(&EthernetPacket::new(packet).unwrap());
          }
          Err(e) => panic!("packetdump: unable to receive packet: {}", e),
        }
      }
    });
  }

  // ----------------------

  pub fn set_handler_ethernet_frame<H: 'static + FnMut(&NetworkInterface, &EthernetPacket)>(
    &mut self,
    handler: H,
  ) {
    self.handler_ethernet_frame = Some(Box::new(handler));
  }

  pub fn set_handler_arp_packet<
    H: 'static + FnMut(&NetworkInterface, &EthernetPacket, &ArpPacket),
  >(
    &mut self,
    handler: H,
  ) {
    self.handler_arp_packet = Some(Box::new(handler));
  }

  pub fn set_handler_ipv4_packet<
    H: 'static + FnMut(&NetworkInterface, &EthernetPacket, &Ipv4Packet),
  >(
    &mut self,
    handler: H,
  ) {
    self.handler_ipv4_packet = Some(Box::new(handler));
  }
  pub fn set_handler_ipv6_packet<
    H: 'static + FnMut(&NetworkInterface, &EthernetPacket, &Ipv6Packet),
  >(
    &mut self,
    handler: H,
  ) {
    self.handler_ipv6_packet = Some(Box::new(handler));
  }
  pub fn set_handler_transport_protocol<
    H: 'static + FnMut(&NetworkInterface, &SrcDest, IpNextHeaderProtocol, &[u8]),
  >(
    &mut self,
    handler: H,
  ) {
    self.handler_transport_protocol = Some(Box::new(handler));
  }

  pub fn set_handler_tcp_packet<H: 'static + FnMut(&NetworkInterface, &SrcDest, &TcpPacket)>(
    &mut self,
    handler: H,
  ) {
    self.handler_tcp_packet = Some(Box::new(handler));
  }
  pub fn set_handler_udp_packet<H: 'static + FnMut(&NetworkInterface, &SrcDest, &UdpPacket)>(
    &mut self,
    handler: H,
  ) {
    self.handler_udp_packet = Some(Box::new(handler));
  }
  pub fn set_handler_icmp_packet<H: 'static + FnMut(&NetworkInterface, &SrcDest, &IcmpPacket)>(
    &mut self,
    handler: H,
  ) {
    self.handler_icmp_packet = Some(Box::new(handler));
  }
  pub fn set_handler_icmpv6_packet<
    H: 'static + FnMut(&NetworkInterface, &SrcDest, &Icmpv6Packet),
  >(
    &mut self,
    handler: H,
  ) {
    self.handler_icmpv6_packet = Some(Box::new(handler));
  }

  // ----------------------

  fn handle_ethernet_frame(&mut self, ethernet: &EthernetPacket) {
    if let Some(handler) = self.handler_ethernet_frame.as_mut() {
      handler(&self.interface, ethernet);
    }

    let interface_name = &self.interface.name.clone()[..];
    match ethernet.get_ethertype() {
      EtherTypes::Ipv4 => self.handle_ipv4_packet(ethernet),
      EtherTypes::Ipv6 => self.handle_ipv6_packet(ethernet),
      EtherTypes::Arp => self.handle_arp_packet(ethernet),
      _ => eprintln!(
        "[{}]: Unknown packet: {} > {}; ethertype: {:?} length: {}",
        interface_name,
        ethernet.get_source(),
        ethernet.get_destination(),
        ethernet.get_ethertype(),
        ethernet.packet().len()
      ),
    }
  }

  // ---------------------------

  fn handle_arp_packet(&mut self, ethernet: &EthernetPacket) {
    let header = ArpPacket::new(ethernet.payload());
    if let Some(header) = header {
      if let Some(handler) = self.handler_arp_packet.as_mut() {
        handler(&self.interface, ethernet, &header);
      }
    } else {
      eprintln!("[{}]: Malformed ARP Packet", self.interface.name);
    }
  }

  fn handle_ipv4_packet(&mut self, ethernet: &EthernetPacket) {
    let header = Ipv4Packet::new(ethernet.payload());
    if let Some(header) = header {
      if let Some(handler) = self.handler_ipv4_packet.as_mut() {
        handler(&self.interface, ethernet, &header);
      }

      self.handle_transport_protocol(
        SrcDest(
          IpAddr::V4(header.get_source()),
          IpAddr::V4(header.get_destination()),
        ),
        header.get_next_level_protocol(),
        header.payload(),
      );
    } else {
      eprintln!("[{}]: Malformed IPv4 Packet", self.interface.name);
    }
  }

  fn handle_ipv6_packet(&mut self, ethernet: &EthernetPacket) {
    let header = Ipv6Packet::new(ethernet.payload());
    if let Some(header) = header {
      if let Some(handler) = self.handler_ipv6_packet.as_mut() {
        handler(&self.interface, ethernet, &header);
      }

      self.handle_transport_protocol(
        SrcDest(
          IpAddr::V6(header.get_source()),
          IpAddr::V6(header.get_destination()),
        ),
        header.get_next_header(),
        header.payload(),
      );
    } else {
      eprintln!("[{}]: Malformed IPv6 Packet", self.interface.name);
    }
  }

  fn handle_transport_protocol(
    &mut self,
    src_dest: SrcDest,
    protocol: IpNextHeaderProtocol,
    packet: &[u8],
  ) {
    if let Some(handler) = self.handler_transport_protocol.as_mut() {
      handler(&self.interface, &src_dest, protocol, packet);
    }

    match protocol {
      IpNextHeaderProtocols::Udp => self.handle_udp_packet(src_dest, packet),
      IpNextHeaderProtocols::Tcp => self.handle_tcp_packet(src_dest, packet),
      IpNextHeaderProtocols::Icmp => self.handle_icmp_packet(src_dest, packet),
      IpNextHeaderProtocols::Icmpv6 => self.handle_icmpv6_packet(src_dest, packet),
      _ => eprintln!(
        "[{}]: Unknown {} packet: {} > {}; protocol: {:?} length: {}",
        self.interface.name,
        match src_dest.0 {
          IpAddr::V4(..) => "IPv4",
          _ => "IPv6",
        },
        src_dest.0,
        src_dest.1,
        protocol,
        packet.len()
      ),
    }
  }

  // ---------------------------

  fn handle_icmp_packet(&mut self, src_dest: SrcDest, packet: &[u8]) {
    let icmp_packet = IcmpPacket::new(packet);
    if let Some(icmp_packet) = icmp_packet {
      if let Some(handler) = self.handler_icmp_packet.as_mut() {
        handler(&self.interface, &src_dest, &icmp_packet);
      }
    } else {
      eprintln!("[{}]: Malformed ICMP Packet", self.interface.name);
    }
  }

  fn handle_icmpv6_packet(&mut self, src_dest: SrcDest, packet: &[u8]) {
    let icmpv6_packet = Icmpv6Packet::new(packet);
    if let Some(icmpv6_packet) = icmpv6_packet {
      if let Some(handler) = self.handler_icmpv6_packet.as_mut() {
        handler(&self.interface, &src_dest, &icmpv6_packet);
      }
    } else {
      eprintln!("[{}]: Malformed ICMPv6 Packet", self.interface.name);
    }
  }

  fn handle_tcp_packet(&mut self, src_dest: SrcDest, packet: &[u8]) {
    let tcp = TcpPacket::new(packet);
    if let Some(tcp) = tcp {
      if let Some(handler) = self.handler_tcp_packet.as_mut() {
        handler(&self.interface, &src_dest, &tcp);
      }
    } else {
      eprintln!("[{}]: Malformed TCP Packet", self.interface.name);
    }
  }

  fn handle_udp_packet(&mut self, src_dest: SrcDest, packet: &[u8]) {
    let udp = UdpPacket::new(packet);

    if let Some(udp) = udp {
      if let Some(handler) = self.handler_udp_packet.as_mut() {
        handler(&self.interface, &src_dest, &udp);
      }
    } else {
      eprintln!("[{}]: Malformed UDP Packet", self.interface.name);
    }
  }
}
