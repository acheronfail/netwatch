package socket

import (
	"encoding/binary"
	"errors"
	"fmt"
	"log"
	"net"
	"syscall"
	"unsafe"

	"github.com/davecgh/go-spew/spew"
)

// TODO: try this with cgo instead?
// https://github.com/golang/go/wiki/WindowsDLLs

var (
	errNoMoreFiles = errors.New("no more files have been found")

	libIPHelperAPI, _          = syscall.LoadLibrary("Iphlpapi.dll")
	procGetTCPTable2, _        = syscall.GetProcAddress(libIPHelperAPI, "GetTcpTable2")
	procGetTCP6Table2, _       = syscall.GetProcAddress(libIPHelperAPI, "GetTcp6Table2")
	procGetExtendedUDPTable, _ = syscall.GetProcAddress(libIPHelperAPI, "GetExtendedUdpTable")

	libKernel32, _        = syscall.LoadLibrary("Kernel32.dll")
	procCreateSnapshot, _ = syscall.GetProcAddress(libKernel32, "CreateToolhelp32Snapshot")
	procProcess32First, _ = syscall.GetProcAddress(libKernel32, "Process32First")
	procProcess32Next, _  = syscall.GetProcAddress(libKernel32, "Process32Next")
)

// TODO: remove
func test() {
	fmt.Println("Hello, Windows!")

	table, err := getTCPTable(procGetTCPTable2)
	if err != nil {
		log.Fatal(err)
	}

	spew.Dump(table)
}

func readIPv4FromPointer(p unsafe.Pointer) net.IP {
	a := (*[net.IPv4len]byte)(p)
	ip := make(net.IP, net.IPv4len)
	copy(ip, a[:])
	return ip
}

func readPortFromPointer(n unsafe.Pointer) uint16 {
	return binary.BigEndian.Uint16((*[2]byte)(n)[:])
}

type winSocket struct {
	Addr uint32
	Port uint32
}

// Socket returns a SockAddr from a winSocket
func (w *winSocket) Socket() *SockAddr {
	ip := readIPv4FromPointer(unsafe.Pointer(&w.Addr))
	port := readPortFromPointer(unsafe.Pointer(&w.Port))
	return &SockAddr{IP: ip, Port: port}
}

// TODO: link docs
type mibTCPRow2 struct {
	State        uint32
	LocalAddr    winSocket
	RemoteAddr   winSocket
	WinPID       uint32
	OffloadState uint32
}

func (m *mibTCPRow2) LocalSocket() *SockAddr       { return m.LocalAddr.Socket() }
func (m *mibTCPRow2) RemoteSocket() *SockAddr      { return m.RemoteAddr.Socket() }
func (m *mibTCPRow2) SocketState() ConnectionState { return ConnectionState(m.State) }

// TODO: link docs
type mibTCPTable2 struct {
	NumEntries uint32
	Table      [1]mibTCPRow2
}

// if `order` is true, then the table is sorted in the order:
// 	- local ip address
// 	- local port
// 	- remote ip address
// 	- remote port
func syscallGetTCPTable(proc uintptr, tableBuf unsafe.Pointer, size *uint32, order bool) error {
	var nArgs uintptr = 3
	var orderArg uintptr = 0
	if order {
		orderArg = 1
	}
	r1, _, callErr := syscall.Syscall(proc, nArgs, uintptr(tableBuf), uintptr(unsafe.Pointer(size)), orderArg)
	if callErr != 0 {
		return callErr
	}
	if r1 != 0 {
		return syscall.Errno(r1)
	}
	return nil
}

func getTCPTable(proc uintptr) (*mibTCPTable2, error) {
	var size uint32
	var buf []byte

	// Call first without a buffer to determine the size required
	err := syscallGetTCPTable(proc, unsafe.Pointer(nil), &size, false)
	if err != nil && err != syscall.Errno(syscall.ERROR_INSUFFICIENT_BUFFER) {
		return nil, err
	}

	// Make the call with the required buffer size
	buf = make([]byte, size)
	table := unsafe.Pointer(&buf[0])
	err = syscallGetTCPTable(proc, table, &size, true)
	if err != nil {
		return nil, err
	}

	// TODO: support TCP6 as well?
	return (*mibTCPTable2)(unsafe.Pointer(&buf[0])), nil
}
