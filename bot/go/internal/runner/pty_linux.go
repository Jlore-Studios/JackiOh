package runner

import (
	"os"
	"strconv"
	"syscall"
	"unsafe"
)

// openPTY is pty.openpty() on Linux: a pseudo-terminal's master and slave, opened as glibc's
// openpty(3) opens them (/dev/ptmx, unlocked, its /dev/pts/N; neither a controlling terminal).
// The master stays non-blocking, so its reads take deadlines.
func openPTY() (master, slave *os.File, err error) {
	master, err = os.OpenFile("/dev/ptmx", os.O_RDWR|syscall.O_NOCTTY, 0)
	if err != nil {
		return nil, nil, err
	}
	var unlock int32
	var n uint32
	err = ioctl(master, syscall.TIOCSPTLCK, unsafe.Pointer(&unlock))
	if err == nil {
		err = ioctl(master, syscall.TIOCGPTN, unsafe.Pointer(&n))
	}
	if err == nil {
		slave, err = os.OpenFile("/dev/pts/"+strconv.Itoa(int(n)), os.O_RDWR|syscall.O_NOCTTY, 0)
	}
	if err != nil {
		master.Close()
		return nil, nil, err
	}
	return master, slave, nil
}

// winsize is struct winsize (`struct.pack("HHHH", rows, cols, 0, 0)`).
type winsize struct{ rows, cols, xpixel, ypixel uint16 }

// setWinsize is fcntl.ioctl(f, termios.TIOCSWINSZ, ...).
func setWinsize(f *os.File, rows, cols uint16) error {
	ws := winsize{rows: rows, cols: cols}
	return ioctl(f, syscall.TIOCSWINSZ, unsafe.Pointer(&ws))
}

// ioctl runs one ioctl on f's descriptor without putting it into blocking mode (as f.Fd would).
func ioctl(f *os.File, request uintptr, arg unsafe.Pointer) error {
	conn, err := f.SyscallConn()
	if err != nil {
		return err
	}
	var callErr error
	err = conn.Control(func(fd uintptr) {
		if _, _, errno := syscall.Syscall(syscall.SYS_IOCTL, fd, request, uintptr(arg)); errno != 0 {
			callErr = errno
		}
	})
	if err != nil {
		return err
	}
	return callErr
}
