package socket

import (
	"fmt"
	"net"
	"os/exec"
	"strconv"
	"strings"

	"github.com/davecgh/go-spew/spew"
)

func test() {
	fmt.Println("Hello, Darwin!")
	spew.Dump(darwinListConnections())
}

const (
	netstatBinary = "netstat"
	lsofBinary    = "lsof"
	lsofFields    = "cn"
)

type darwinProcess struct {
	Name string
	PID  uint
}

// darwinConnection is a (TCP) connection. The Process struct may be nil.
type darwinConnection struct {
	Transport     string
	LocalAddress  net.IP
	LocalPort     uint16
	RemoteAddress net.IP
	RemotePort    uint16
	inode         uint64
	Process       darwinProcess
}

// Connections returns all established (TCP) connections.
// You need to be root to find all processes.
func darwinListConnections() ([]darwinConnection, error) {
	// netstat
	out, err := exec.Command(
		netstatBinary,
		"-n", // no number resolving
		"-W", // Wide output
		// "-l",     // full IPv6 addresses // What does this do?
		"-p", "tcp", // only TCP
	).CombinedOutput()
	if err != nil {
		return nil, err
	}

	connections := parseNetstat(string(out))

	// lsof
	out, err = exec.Command(
		lsofBinary,
		"-i",       // only Internet files
		"-n", "-P", // no number resolving
		"-w",             // no warnings
		"-F", lsofFields, // \n based output of only the fields we want.
	).CombinedOutput()
	if err != nil {
		return nil, err
	}

	processes, err := parseLSOF(string(out))
	if err != nil {
		return nil, err
	}

	for processLocalAddr, process := range processes {
		for i, connection := range connections {
			localAddr := net.JoinHostPort(
				connection.LocalAddress.String(),
				strconv.Itoa(int(connection.LocalPort)),
			)
			if localAddr == processLocalAddr {
				connections[i].Process = process
			}
		}
	}

	return connections, nil
}

// parseNetstat parses netstat output. (Linux has ip:port, darwin
// ip.port. The 'Proto' column value also differs.)
func parseNetstat(out string) []darwinConnection {
	//
	//  Active Internet connections
	//  Proto Recv-Q Send-Q  Local Address          Foreign Address        (state)
	//  tcp4       0      0  10.0.1.6.58287         1.2.3.4.443      		ESTABLISHED
	//
	res := []darwinConnection{}
	for i, line := range strings.Split(out, "\n") {
		if i == 0 || i == 1 {
			// Skip headers
			continue
		}

		// Fields are:
		fields := strings.Fields(line)
		if len(fields) != 6 {
			continue
		}

		if fields[5] != "ESTABLISHED" {
			continue
		}

		t := darwinConnection{
			Transport: "tcp",
		}

		// Format is <ip>.<port>
		locals := strings.Split(fields[3], ".")
		if len(locals) < 2 {
			continue
		}

		var (
			localAddress = strings.Join(locals[:len(locals)-1], ".")
			localPort    = locals[len(locals)-1]
		)

		t.LocalAddress = net.ParseIP(localAddress)

		p, err := strconv.Atoi(localPort)
		if err != nil {
			return nil
		}

		t.LocalPort = uint16(p)

		remotes := strings.Split(fields[4], ".")
		if len(remotes) < 2 {
			continue
		}

		var (
			remoteAddress = strings.Join(remotes[:len(remotes)-1], ".")
			remotePort    = remotes[len(remotes)-1]
		)

		t.RemoteAddress = net.ParseIP(remoteAddress)

		p, err = strconv.Atoi(remotePort)
		if err != nil {
			return nil
		}

		t.RemotePort = uint16(p)

		res = append(res, t)
	}

	return res
}

// parseLSOF parses lsof out with `-F cn` argument.
//
// Format description: the first letter is the type of record, records are
// newline seperated, the record starting with 'p' (pid) is a new processid.
// There can be multiple connections for the same 'p' record in which case the
// 'p' is not repeated.
//
// For example, this is one process with two listens and one connection:
//
//   p13100
//   cmpd
//   n[::1]:6600
//   n127.0.0.1:6600
//   n[::1]:6600->[::1]:50992
//
func parseLSOF(out string) (map[string]darwinProcess, error) {
	var (
		res = map[string]darwinProcess{} // Local addr -> darwinProcess
		cp  = darwinProcess{}
	)
	for _, line := range strings.Split(out, "\n") {
		if len(line) <= 1 {
			continue
		}

		var (
			field = line[0]
			value = line[1:]
		)
		switch field {
		case 'p':
			pid, err := strconv.Atoi(value)
			if err != nil {
				return nil, fmt.Errorf("invalid 'p' field in lsof output: %#v", value)
			}
			cp.PID = uint(pid)

		case 'n':
			// 'n' is the last field, with '-F cn'
			// format examples:
			// "192.168.2.111:44013->54.229.241.196:80"
			// "[2003:45:2b57:8900:1869:2947:f942:aba7]:55711->[2a00:1450:4008:c01::11]:443"
			// "*:111" <- a listen
			addresses := strings.SplitN(value, "->", 2)
			if len(addresses) != 2 {
				// That's a listen entry.
				continue
			}
			res[addresses[0]] = darwinProcess{
				Name: cp.Name,
				PID:  cp.PID,
			}

		case 'c':
			cp.Name = value

		case 'f':
			/*
				lsof:	ID    field description
					a    access: r = read; w = write; u = read/write
					c    command name
					C    file struct share count
					d    device character code
					D    major/minor device number as 0x<hex>
					f    file descriptor (always selected)
				mac platform ,lsof version revision: 4.89, file descriptor (always selected)
			*/
			continue

		default:
			return nil, fmt.Errorf("unexpected lsof field: %c in %#v", field, value)
		}
	}

	return res, nil
}
