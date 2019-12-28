use pnet::datalink::{self, NetworkInterface};
use pnet::packet::Packet;

use std::env;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use netwatch::connections::ConnectionTable;
use netwatch::incoming::IsIncoming;
use netwatch::packet_monitor::PacketMonitor;
use netwatch::port::PortMapper;
use netwatch::transfer::Transfer;

/// TODO: the design
///
/// - [x] Use pnet to catch all TCP & UDP packets
/// - [x] Check whether they're incoming or outgoing depending on their destination
/// - [ ] Figure out a way to map packets into a "Connection"
/// - [ ] Figure out a way to map "Connection"s into a "Process"
/// - [ ] Build a nice terminal UI for this
/// - [ ] Automatically monitor all interfaces

fn main() {
    let iface_name = match env::args().nth(1) {
        Some(n) => n,
        None => {
            writeln!(io::stderr(), "USAGE: packetdump <NETWORK INTERFACE>").unwrap();
            for interface in datalink::interfaces() {
                writeln!(io::stderr(), "- {}", interface.name).unwrap();
            }
            std::process::exit(1);
        }
    };

    // Find the network interface with the provided name
    let interfaces = datalink::interfaces();
    let interface = interfaces
        .into_iter()
        .filter(|iface: &NetworkInterface| iface.name == iface_name)
        .next()
        .unwrap();

    // TODO: show all interfaces in UI
    // TODO: handle terminations/allow quitting/etc
    // ---

    let connections = ConnectionTable::new();
    let connections = Arc::new(Mutex::new(connections));

    // {
    //     let mut monitor = PacketMonitor::logger(interface.clone());
    //     monitor.start();
    // }

    let mut monitor = PacketMonitor::new(interface);

    // ---
    // NOTE: handle total incoming and outgoing

    let total_transfer = Arc::new(Mutex::new(Transfer::new()));

    let total_transfer_ethernet = total_transfer.clone();
    monitor.set_handler_ethernet_frame(move |iface, eth| {
        let mut total_transfer = total_transfer_ethernet.lock().unwrap();
        let size = eth.packet().len() as u64;
        if eth.is_incoming(iface) {
            println!("<-- {}", size);
            total_transfer.incr_incoming(size);
        } else {
            println!("--> {}", size);
            total_transfer.incr_outgoing(size);
        }
    });

    // ---
    // NOTE: handle per-process incoming and outgoing

    let connections_tcp = connections.clone();
    monitor.set_handler_tcp_packet(move |iface, src_dest, tcp| {
        let mut connections = connections_tcp.lock().unwrap();

        let port = tcp.get_destination();
        let size = tcp.packet().len() as u64;
        if src_dest.1.is_incoming(iface) {
            connections.incr_incoming(port, size);
        } else {
            connections.incr_outgoing(port, size);
        }
    });

    let connections_udp = connections.clone();
    monitor.set_handler_udp_packet(move |iface, src_dest, udp| {
        let mut connections = connections_udp.lock().unwrap();

        let port = udp.get_destination();
        let size = udp.packet().len() as u64;
        if src_dest.1.is_incoming(iface) {
            connections.incr_incoming(port, size);
        } else {
            connections.incr_outgoing(port, size);
        }
    });

    // ---
    // TODO: thread to periodically iterate connection table with port mapper
    // and print connection information

    println!("Hello");
    let interval = Duration::from_millis(1_000);
    let connections_thread = connections.clone();
    let total_transfer_thread = total_transfer.clone();
    thread::spawn(move || loop {
        let (incoming, outgoing) = total_transfer_thread.lock().unwrap().stats();
        println!("Total transfer: DOWN: {} UP: {}", incoming, outgoing);

        let port_mapper = PortMapper::new();
        // NOTE: locking the connections here means we can't get packets in our handlers
        // TODO: is there a way to get read access only to the struct so others can write to it?
        // TODO: is there a better way?
        {
            let connections = connections_thread.lock().unwrap();
            for (port, transfer) in connections.inner.iter() {
                let (incoming, outgoing) = transfer.stats();
                if let Some(processes) = port_mapper.get(&port) {
                    let process = processes.first().unwrap();
                    let name = process.cmdline().unwrap().join(" ");

                    println!("DOWN: {} UP: {} PROC: {}", incoming, outgoing, name);
                } else {
                    println!("DOWN: {} UP: {} PROC: ???", incoming, outgoing);
                }
            }
        }

        println!();
        thread::sleep(interval);
    });

    // ---
    // NOTE: this currently blocks.
    monitor.start();
}
