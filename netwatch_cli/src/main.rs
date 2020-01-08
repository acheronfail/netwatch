use pnet::datalink::{self, NetworkInterface};
use pnet::packet::Packet;

use std::env;
use std::io::{self, Write};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;

use netwatch::connection::{ConnectionTable};
use netwatch::incoming::IsIncoming;
use netwatch::packet_monitor::PacketMonitor;
use netwatch::port::PortMapper;
use netwatch::transfer::Transfer;

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

    // NOTE: potentially create multiple connection tables so TCP/UDP packets don't fight for the lock?
    let connections = ConnectionTable::new();
    let connections = Arc::new(RwLock::new(connections));

    let mut monitor = PacketMonitor::new(interface);

    // ---
    // NOTE: handle total incoming and outgoing

    let total_transfer = Arc::new(Mutex::new(Transfer::new()));

    let total_transfer_ethernet = total_transfer.clone();
    monitor.set_handler_ethernet_frame(move |iface, eth| {
        let mut total_transfer = total_transfer_ethernet.lock().unwrap();
        let size = eth.packet().len() as u64;
        if eth.is_incoming(iface) {
            total_transfer.incr_incoming(size);
        } else {
            total_transfer.incr_outgoing(size);
        }
    });

    // ---
    // NOTE: handle per-process incoming and outgoing
    // TODO: should we handle more than just TCP and UDP?

    let connections_tcp = connections.clone();
    monitor.set_handler_tcp_packet(move |iface, src_dest, tcp| {
        let port = tcp.get_destination();
        let size = tcp.packet().len() as u64;
        // TODO: a nicer way to represent `src_dest`
        if src_dest.1.is_incoming(iface) {
            connections_tcp.write().unwrap().incr_incoming(port, size);
        } else {
            connections_tcp.write().unwrap().incr_outgoing(port, size);
        }
    });

    let connections_udp = connections.clone();
    monitor.set_handler_udp_packet(move |iface, src_dest, udp| {
        let port = udp.get_destination();
        let size = udp.packet().len() as u64;
        if src_dest.1.is_incoming(iface) {
            connections_udp.write().unwrap().incr_incoming(port, size);
        } else {
            connections_udp.write().unwrap().incr_outgoing(port, size);
        }
    });

    // ---
    // NOTE: thread to periodically iterate connection table with port mapper
    // and print connection information

    let interval = Duration::from_millis(1_000);
    let connections_thread = connections.clone();
    let total_transfer_thread = total_transfer.clone();
    thread::spawn(move || {
        loop {
            // NOTE: locking the connections here means we can't get packets in our handlers...
            // TODO: combine transfers from the same process
            // TODO: reset & calc over time interval
            let mut port_mapper = PortMapper::new();
            port_mapper.refresh();

            let mut unknown = Transfer::new();

            // Open the read lock on connections for as short a time as possible.
            {
                println!("Total:         {}", total_transfer_thread.lock().unwrap());
                let connections = &*connections_thread.read().unwrap();
                for (port, transfer) in connections {
                    println!("Port: [{:>6}] {}", port, transfer);
                    if let Some(processes) = port_mapper.get(&port) {
                        println!("\tAssociated processes: {}", processes.len());
                        for process in processes {
                            let name = process.cmdline().unwrap().join(" ");
                            println!("\t\t{}", name=name);
                        }
                    } else {
                        unknown.merge(&transfer);
                    }
                }

                println!("Unknown:       {}", unknown);
            }

            println!("");

            thread::sleep(interval);
        }
    });

    // ---
    // NOTE: this currently blocks, might be nice to have it spawn a thread instead.
    monitor.start();
}
