package util

import (
	"fmt"
	"net"

	"github.com/google/gopacket/pcap"
)

// FindDeviceIpv4 Gets the IPv4 address of the network card
func FindDeviceIpv4(device pcap.Interface) string {
	for _, addr := range device.Addresses {
		if ipv4 := addr.IP.To4(); ipv4 != nil {
			return ipv4.String()
		}
	}

	panic("device has no IPv4")
}

// FindMacAddrByIP Obtains the MAC address according to the IPv4 address of the NIC
// This method is used because gopacket does not encapsulate the method of obtaining the MAC address
// internally, so look for the MAC address by finding the NIC with the same IPv4 address.
func FindMacAddrByIP(ip string) (string, error) {
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

	return "", fmt.Errorf("no device has given ip: %s", ip)
}