use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use pnet::datalink::{self, NetworkInterface};
use pnet::packet::Packet;
use tui::backend::CrosstermBackend;
use tui::Terminal;

use std::env;
use std::io::{self, stdout, Write};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use netwatch::connection::ConnectionTable;
use netwatch::incoming::IsIncoming;
use netwatch::packet_monitor::PacketMonitor;
use netwatch::port::PortMapper;
use netwatch::transfer::Transfer;

mod app;

use app::{App, AppEvent};

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

    // FIXME: move all these structs/monitors/etc into `App` and combine in there to render here

    let connections = ConnectionTable::new();
    let connections = Arc::new(Mutex::new(connections));

    let mut monitor = PacketMonitor::new(interface);

    // ---
    // NOTE: handle total incoming and outgoing

    let total_transfer = Transfer::new();
    let total_transfer = Arc::new(Mutex::new(total_transfer));

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
            connections_tcp.lock().unwrap().incr_incoming(port, size);
        } else {
            connections_tcp.lock().unwrap().incr_outgoing(port, size);
        }
    });

    let connections_udp = connections.clone();
    monitor.set_handler_udp_packet(move |iface, src_dest, udp| {
        let port = udp.get_destination();
        let size = udp.packet().len() as u64;
        if src_dest.1.is_incoming(iface) {
            connections_udp.lock().unwrap().incr_incoming(port, size);
        } else {
            connections_udp.lock().unwrap().incr_outgoing(port, size);
        }
    });

    // ---
    // NOTE: thread to periodically iterate connection table with port mapper
    // and print connection information

    let interval = 1_000;
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

            // Open the total transfer lock.
            {
                let total_transfer = &mut *total_transfer_thread.lock().unwrap();
                let (incoming, outgoing) = total_transfer.stats(interval);
                println!("Total:            {} {}", incoming, outgoing);
                total_transfer.reset();
            }

            // Open the read lock on connections for as short a time as possible.
            {
                let connections = &mut *connections_thread.lock().unwrap();
                for (port, transfer) in connections {
                    println!("Port: [{:>6}] {}", port, transfer);
                    if let Some(processes) = port_mapper.get(&port) {
                        // Port's transfer...
                        let (incoming, outgoing) = transfer.stats(interval);
                        println!("Transfer:         {} {}", incoming, outgoing);

                        // Port's processes...
                        println!("\tAssociated processes: {}", processes.len());
                        for process in processes {
                            let name = process.cmdline().unwrap().join(" ");
                            println!("\t\t{}", name = name);
                        }
                    } else {
                        unknown.merge(&transfer);
                    }

                    transfer.reset();
                }

                // Unknown...
                let (incoming, outgoing) = unknown.stats(interval);
                println!("Unknown:          {} {}", incoming, outgoing);
            }

            println!("");
            thread::sleep(Duration::from_millis(interval));
        }
    });

    // --- UI setup

    terminal::enable_raw_mode().unwrap();

    let mut stdout = stdout();
    crossterm::execute!(stdout, EnterAlternateScreen).unwrap();

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.hide_cursor().unwrap();

    // Setup input handling
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        loop {
            // poll for tick rate duration, if no events, sent tick event.
            if event::poll(Duration::from_millis(interval)).unwrap() {
                if let Event::Key(key) = event::read().unwrap() {
                    tx.send(AppEvent::Input(key)).unwrap();
                }
            }

            tx.send(AppEvent::Tick).unwrap();
        }
    });

    let mut app = App::new("Crossterm Demo");

    terminal.clear().unwrap();

    loop {
        // Draw...
        app.draw(&mut terminal).unwrap();

        // Handle ...
        match rx.recv().unwrap() {
            AppEvent::Input(event) => match event.code {
                KeyCode::Char('q') => {
                    terminal::disable_raw_mode().unwrap();
                    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen).unwrap();
                    terminal.show_cursor().unwrap();
                    break;
                }
                KeyCode::Char(c) => app.on_key(c),
                KeyCode::Left => app.on_left(),
                KeyCode::Up => app.on_up(),
                KeyCode::Right => app.on_right(),
                KeyCode::Down => app.on_down(),
                _ => {}
            },
            AppEvent::Tick => {
                app.on_tick();
            }
        }

        if app.should_quit {
            break;
        }
    }

    // ---
    // NOTE: this currently blocks, might be nice to have it spawn a thread instead.
    monitor.start();
}
