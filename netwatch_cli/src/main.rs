mod transfer;

use pnet::datalink::{self, NetworkInterface};

use std::env;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use netwatch::packet_monitor::{self, PacketMonitor};
use netwatch::process::ProcessPIDTable;
use transfer::Transfer;

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
    // ---

    // TODO: perhaps have a look at https://github.com/lsof-org/lsof/blob/master/main.c
    // `sudo lsof -n -l -i ':50170'`
    // `sudo netstat -np | rg 50982`
    let _ = ProcessPIDTable::new();

    // ---

    let transfer = Arc::new(Mutex::new(Transfer::new()));

    let transfer_for_monitor = transfer.clone();
    let mut monitor = PacketMonitor::new(interface);
    monitor.set_handler_ethernet_frame(move |iface, eth| {
        use pnet::packet::Packet;

        let mut transfer = transfer_for_monitor.lock().unwrap();
        if packet_monitor::is_incoming(iface, eth) {
            transfer.incr_incoming(eth.packet().len() as u64);
        } else {
            transfer.incr_outgoing(eth.packet().len() as u64);
        }
    });
    monitor.start();

    // TODO: potentially move this into Transfer itself? and configure its time interval there?
    let transfer_for_output = transfer.clone();
    thread::spawn(move || loop {
        let mut transfer = transfer_for_output.lock().unwrap();

        println!("\r{}    ", transfer.stats());
        io::stdout().flush().unwrap();
        transfer.reset();

        thread::sleep(Duration::from_millis(1000));
    });

    // let monitor = PacketMonitor::logger(interface);
    // monitor.start();
}
