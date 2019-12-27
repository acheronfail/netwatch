use pnet::datalink::NetworkInterface;
use pnet::packet::arp::ArpPacket;
use pnet::packet::ethernet::EthernetPacket;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::ipv6::Ipv6Packet;

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub trait IsIncoming {
    fn is_incoming(&self, interface: &NetworkInterface) -> bool;
}

impl<'a> IsIncoming for EthernetPacket<'a> {
    fn is_incoming(&self, interface: &NetworkInterface) -> bool {
        Some(self.get_destination()) == interface.mac
    }
}

impl<'a> IsIncoming for ArpPacket<'a> {
    fn is_incoming(&self, interface: &NetworkInterface) -> bool {
        IpAddr::V4(self.get_sender_proto_addr()).is_incoming(interface)
    }
}

impl<'a> IsIncoming for Ipv4Packet<'a> {
    fn is_incoming(&self, interface: &NetworkInterface) -> bool {
        IpAddr::V4(self.get_destination()).is_incoming(interface)
    }
}

impl<'a> IsIncoming for Ipv6Packet<'a> {
    fn is_incoming(&self, interface: &NetworkInterface) -> bool {
        IpAddr::V6(self.get_destination()).is_incoming(interface)
    }
}

impl IsIncoming for Ipv4Addr {
    fn is_incoming(&self, interface: &NetworkInterface) -> bool {
        let me = IpAddr::V4(*self);
        interface.ips.iter().any(|ipn| ipn.contains(me))
    }
}

impl IsIncoming for Ipv6Addr {
    fn is_incoming(&self, interface: &NetworkInterface) -> bool {
        let me = IpAddr::V6(*self);
        interface.ips.iter().any(|ipn| ipn.contains(me))
    }
}

impl IsIncoming for IpAddr {
    fn is_incoming(&self, interface: &NetworkInterface) -> bool {
        interface.ips.iter().any(|ipn| ipn.contains(*self))
    }
}
