package main

/*
#cgo LDFLAGS: -L. -lrustbee
#include "librustbee.h"
*/
import "C"

import (
	"flag"
	"fmt"
	"os"
	"strings"
	"sync"
	"unsafe"
)

var ADDRS [2][6]C.uint8_t = [2][6]C.uint8_t{
	{0xE8, 0xD4, 0xEA, 0xC4, 0x62, 0x00},
	{0xEC, 0x27, 0xA7, 0xD6, 0x5A, 0x9C},
}

func main() {
	power := flag.Uint("p", 0, "Power state, 1 for ON, 0 for OFF")
	brigtness := flag.Uint("b", 0, "Brightness, value between 0 and 100")
	flag.Parse()

	var wg sync.WaitGroup

	if !C.launch_daemon() {
		fmt.Fprintf(os.Stderr, "[ERROR] Failed to launch daemon")
		os.Exit(1)
	}

	// FIXME: This segfaults
	// defer func() {
	// 	if !C.shutdown_daemon((C.uint8_t)(0)) {
	// 		fmt.Fprintf(os.Stderr, "[ERROR] Failed to shutdown daemon")
	// 		os.Exit(1)
	// 	}
	// }()

	for _, addr := range ADDRS {
		wg.Add(1)

		go func() {
			addr_ptr := (*[6]C.uint8_t)(unsafe.Pointer(&addr))
			device_ptr := C.new_device(addr_ptr)
			defer C.free_device(device_ptr)

			if !C.try_connect(device_ptr) {
				fmt.Fprintf(os.Stderr, "[ERROR] Failed to connect\n")
				return
			}

			if power != nil && !C.set_power(device_ptr, (C.uint8_t)(*power)) {
				fmt.Fprintf(os.Stderr, "[ERROR] Failed to set power\n")
				return
			}

			if brigtness != nil && *brigtness != 0 && !C.set_brightness(device_ptr, (C.uint8_t)(*brigtness)) {
				fmt.Fprintf(os.Stderr, "[ERROR] Failed to set brightness\n")
				return
			}

			_power_state := C.get_power(device_ptr)
			power_state := "ON"
			if !_power_state {
				power_state = "OFF"
			}

			name_ptr := C.get_name(device_ptr)
			name := getName(name_ptr)
			defer C.free_name(name_ptr)

			rgb_ptr := C.get_color_rgb(device_ptr)
			defer C.free_color_rgb(rgb_ptr)

			fmt.Printf(
				"%s %v\nPower %s\nBrightness %d%%\nRGB Color %v\n",
				name,
				*addr_ptr,
				power_state,
				C.get_brightness(device_ptr),
				*rgb_ptr,
			)

			wg.Done()
		}()
	}

	wg.Wait()
}

func getName(buffer *[19]C.uint8_t) string {
	name := strings.Builder{}

	for _, b := range *buffer {
		name.WriteByte(byte(b))
	}

	res := name.String()

	if len(res) == 0 {
		return "Unknown"
	}

	return res
}
