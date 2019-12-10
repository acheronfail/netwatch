package main

import (
	"flag"
	"fmt"
	"log"
	"os"
	"time"

	"github.com/dustin/go-humanize"
	ui "github.com/gizak/termui/v3"
	"github.com/gizak/termui/v3/widgets"
	"github.com/google/gopacket"
	"github.com/google/gopacket/layers"
	"github.com/google/gopacket/pcap"

	"github.com/acheronfail/netwatch-go/src/socket"
	"github.com/acheronfail/netwatch-go/src/util"
)

var (
	// total bytes counted per unit time
	downStreamDataSize = 0
	// total number of bytes in the unit time
	upStreamDataSize = 0
	// Name of the network card to be monitored
	deviceName = flag.String("i", "eth0", "network interface device name")
	// Whether to use a full text ui
	simpleUI = flag.Bool("simple-ui", false, "use a simple text ui")
)

func main() {
	flag.Parse()

	socket.Test()
	os.Exit(0)

	fmt.Println("simpleUI:", *simpleUI)
	fmt.Println("deviceName:", *deviceName)

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
		fmt.Println(d.Name)
		if d.Name == *deviceName {
			device = d
		}
	}

	// Obtain the mac address of the network card according to the ipv4 address of the network card,
	// which is used to determine the direction of the data packet later.
	deviceIPv4 := util.FindDeviceIpv4(device)
	macAddr, err := util.FindMacAddrByIP(deviceIPv4)
	if err != nil {
		panic(err)
	}

	fmt.Printf("Chosen device's IPv4:\t%s\n", deviceIPv4)
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
	updateInterval := time.Second / 2
	if *simpleUI {
		go startSimpleMonitor(updateInterval)
	} else {
		go startCLIMonitor(updateInterval, quitChannel)
	}

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

func getNextStats(interval time.Duration) string {
	normaliser := time.Second / interval
	down := humanize.Bytes(uint64(downStreamDataSize * int(normaliser)))
	up := humanize.Bytes(uint64(upStreamDataSize * int(normaliser)))

	// Reset
	downStreamDataSize = 0
	upStreamDataSize = 0

	return fmt.Sprintf("Down: %s Up: %s", down, up)
}

func startSimpleMonitor(interval time.Duration) {
	for {
		// TODO: clear whole line
		fmt.Printf("\r" + getNextStats(interval))
		time.Sleep(interval)
	}
}

// Calculate the average packet size in the second every second, and set the total number of
// downloads and uploads to zero.
func startCLIMonitor(interval time.Duration, quitChannel chan struct{}) {
	if err := ui.Init(); err != nil {
		log.Fatalf("failed to initialize termui: %v", err)
	}
	defer ui.Close()

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
			p := widgets.NewParagraph()
			p.Text = getNextStats(interval)
			p.SetRect(0, 0, 40, 5)
			p.Border = false

			ui.Render(p)
		}

	}
}
