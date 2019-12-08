package main

import (
	"errors"
	"flag"
	"fmt"
	"log"
	"net"
	"time"

	"github.com/dustin/go-humanize"
	ui "github.com/gizak/termui/v3"
	"github.com/gizak/termui/v3/widgets"
	"github.com/google/gopacket"
	"github.com/google/gopacket/layers"
	"github.com/google/gopacket/pcap"
)

var (
	// total bytes counted per unit time
	downStreamDataSize = 0
	// total number of bytes in the unit time
	upStreamDataSize = 0
	// Name of the network card to be monitored
	deviceName = flag.String("i", "eth0", "network interface device name")
)

func main() {
	flag.Parse()

	// Find all devices
	// Get all NICs
	devices, err := pcap.FindAllDevs()
	if err != nil {
		log.Fatal(err)
	}

	// Find exact device
	// Get the exact NIC from all NICs based on the NIC name
	var device pcap.Interface
	for _, d := range devices {
		if d.Name == *deviceName {
			device = d
		}
	}

	// Obtain the mac address of the network card according to the ipv4 address of the network card,
	// which is used to determine the direction of the data packet later.
	macAddr, err := findMacAddrByIp(findDeviceIpv4(device))
	if err != nil {
		panic(err)
	}

	fmt.Printf("Chosen device's IPv4:\t%s\n", findDeviceIpv4(device))
	fmt.Printf("Chosen device's MAC:\t%s\n", macAddr)

	// Get the network card handler, can be used to read or write data packets
	maxValuePerPacketRead := int32(1024)
	promiscuousMode := false
	readPacketTimeout := 30 * time.Second
	handle, err := pcap.OpenLive(*deviceName, maxValuePerPacketRead, promiscuousMode, readPacketTimeout)
	if err != nil {
		panic(err)
	}
	defer handle.Close()

	// Start the CLI monitor
	quitChannel := make(chan struct{})
	go startCLIMonitor(time.Second/2, quitChannel)

	// Start capturing packets
	packetSource := gopacket.NewPacketSource(handle, handle.LinkType())
	for packet := range packetSource.Packets() {
		select {
		case <-quitChannel:
			return
		default:
			// TODO: map packets to processes:
			// https://github.com/google/gopacket/issues/651#issuecomment-491345795

			// only get Ethernet frames
			ethernetLayer := packet.Layer(layers.LayerTypeEthernet)
			if ethernetLayer != nil {
				ethernet := ethernetLayer.(*layers.Ethernet)
				// If the destination MAC of the packet is local, it means that it is a downlink packet,
				// otherwise it is uplink.
				if ethernet.DstMAC.String() == macAddr {
					downStreamDataSize += len(packet.Data())
				} else {
					upStreamDataSize += len(packet.Data())
				}
			}
		}
	}
}

// Get the IPv4 address of the network card
func findDeviceIpv4(device pcap.Interface) string {
	for _, addr := range device.Addresses {
		if ipv4 := addr.IP.To4(); ipv4 != nil {
			return ipv4.String()
		}
	}

	panic("device has no IPv4")
}

// Obtain the MAC address according to the IPv4 address of the NIC
// This method is used because gopacket does not encapsulate the method of obtaining the MAC address
// internally, so look for the MAC address by finding the NIC with the same IPv4 address.
func findMacAddrByIp(ip string) (string, error) {
	interfaces, err := net.Interfaces()
	if err != nil {
		panic(interfaces)
	}

	for _, i := range interfaces {
		addrs, err := i.Addrs()
		if err != nil {
			panic(err)
		}

		for _, addr := range addrs {
			if a, ok := addr.(*net.IPNet); ok {
				if ip == a.IP.String() {
					return i.HardwareAddr.String(), nil
				}
			}
		}
	}

	return "", errors.New(fmt.Sprintf("no device has given ip: %s", ip))
}

// Calculate the average packet size in the second every second, and set the total number of
// downloads and uploads to zero.
func startCLIMonitor(interval time.Duration, quitChannel chan struct{}) {
	if err := ui.Init(); err != nil {
		log.Fatalf("failed to initialize termui: %v", err)
	}
	defer ui.Close()

	normaliser := time.Second / interval
	uiEvents := ui.PollEvents()
	ticker := time.NewTicker(interval)
	for {
		select {
		case e := <-uiEvents:
			switch e.ID {
			case "q", "<C-c>":
				ticker.Stop()
				close(quitChannel)
				return
			}
		case <-ticker.C:
			down := humanize.Bytes(uint64(downStreamDataSize * int(normaliser)))
			up := humanize.Bytes(uint64(upStreamDataSize * int(normaliser)))
			p := widgets.NewParagraph()
			p.Text = fmt.Sprintf("\rDown: %s \t Up: %s", down, up)
			p.SetRect(0, 0, 40, 5)
			p.Border = false

			ui.Render(p)

			// Reset
			downStreamDataSize = 0
			upStreamDataSize = 0
		}

	}
}
