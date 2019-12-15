package socket

import (
	"fmt"
	"net"
)

// Test ...
func Test() {
	test()
}

// SockAddr represents an ip:port pair
type SockAddr struct {
	IP   net.IP
	Port uint16
}

func (s *SockAddr) String() string {
	return fmt.Sprintf("%v:%d", s.IP, s.Port)
}

// Process holds the PID and process name to which each socket belongs
type Process struct {
	pid  int
	name string
}

func (p *Process) String() string {
	return fmt.Sprintf("%d/%s", p.pid, p.name)
}

// ConnectionState type represents socket connection state
type ConnectionState uint8
