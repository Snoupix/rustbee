use std::ffi::c_uchar as uint8_t;
use std::ptr;
use std::sync::OnceLock;

use interprocess::local_socket::Stream;
use tokio::runtime::{Builder, Runtime};

use crate::constants::{masks::*, ADDR_LEN, DATA_LEN, SET};
use crate::device::{CmdOutput, HueDevice, EMPTY_BUFFER, FFI};
use crate::utils;

static THREAD: OnceLock<Runtime> = OnceLock::new();

macro_rules! block_on {
    ($async_fn:expr) => {{
        THREAD
            .get_or_init(|| Builder::new_current_thread().enable_all().build().unwrap())
            .block_on($async_fn)
    }};
}

#[repr(C)]
struct Device {
    addr: [uint8_t; ADDR_LEN],
    inner: HueDevice<FFI>,
}

impl std::ops::Deref for Device {
    type Target = HueDevice<FFI>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Device {
    fn new(addr: [uint8_t; ADDR_LEN]) -> Self {
        Self {
            addr,
            inner: HueDevice::new(addr),
        }
    }

    fn boxed(self) -> Box<Self> {
        Box::new(self)
    }

    fn send_to_socket(&mut self, masks: u16, buffer: [u8; DATA_LEN + 1]) -> CmdOutput {
        Self::_send_to_socket(
            &mut HueDevice::<FFI>::get_file_socket(),
            Some(self.addr),
            masks,
            buffer,
        )
    }

    fn _send_to_socket(
        stream: &mut Stream,
        addr: Option<[u8; ADDR_LEN]>,
        masks: u16,
        buffer: [u8; DATA_LEN + 1],
    ) -> CmdOutput {
        HueDevice::<FFI>::send_packet_to_daemon(stream, addr, masks, buffer)
    }
}

#[no_mangle]
extern "C" fn new_device(addr_ptr: *const [uint8_t; ADDR_LEN]) -> *mut Device {
    unsafe { Box::into_raw(Device::new(*addr_ptr).boxed()) }
}

#[no_mangle]
extern "C" fn free_device(device_ptr: *mut Device) {
    if device_ptr.is_null() {
        return;
    }

    unsafe {
        drop(Box::from_raw(device_ptr));
    }
}

// For some reason, if this fn is called "connect" it seg faults
// the program without even calling it, must be conflicting somewhere
// REASON: https://github.com/rust-lang/rust/issues/28179 fixed in Rust 1.82
#[no_mangle]
extern "C" fn try_connect(device_ptr: *mut Device) -> bool {
    if device_ptr.is_null() {
        eprintln!("[ERROR] Device pointer is null");
        return false;
    }

    let device = unsafe { &mut *device_ptr };

    let mut buf = EMPTY_BUFFER;
    buf[0] = SET;

    device.send_to_socket(CONNECT, buf).0.is_success()
}

#[no_mangle]
extern "C" fn try_disconnect(device_ptr: *mut Device) -> bool {
    if device_ptr.is_null() {
        eprintln!("[ERROR] Device pointer is null");
        return false;
    }

    let device = unsafe { &mut *device_ptr };

    let mut buf = EMPTY_BUFFER;
    buf[0] = SET;

    device.send_to_socket(DISCONNECT, buf).0.is_success()
}

#[no_mangle]
extern "C" fn set_power(device_ptr: *mut Device, state: *const uint8_t) -> bool {
    if device_ptr.is_null() {
        eprintln!("[ERROR] Device pointer is null");
        return false;
    }

    let device = unsafe { &mut *device_ptr };

    let mut buf = EMPTY_BUFFER;
    buf[0] = SET;
    buf[1] = unsafe { *state };

    device.send_to_socket(CONNECT | POWER, buf).0.is_success()
}

#[no_mangle]
extern "C" fn set_brightness(device_ptr: *mut Device, value: *const uint8_t) -> bool {
    if device_ptr.is_null() {
        eprintln!("[ERROR] Device pointer is null");
        return false;
    }

    let device = unsafe { &mut *device_ptr };

    let mut buf = EMPTY_BUFFER;
    buf[0] = SET;
    buf[1] = unsafe { *value };

    device
        .send_to_socket(CONNECT | BRIGHTNESS, buf)
        .0
        .is_success()
}

#[no_mangle]
extern "C" fn get_brightness(device_ptr: *mut Device) -> *const uint8_t {
    if device_ptr.is_null() {
        eprintln!("[ERROR] Device pointer is null");
        return ptr::null();
    }

    let device = unsafe { &mut *device_ptr };

    let bit = device.send_to_socket(BRIGHTNESS, EMPTY_BUFFER).1[0];
    let brightness = ((bit as f32 / 255.) * 100.) as uint8_t;

    ptr::from_ref(&brightness)
}

#[no_mangle]
extern "C" fn launch_daemon() -> bool {
    block_on!(utils::launch_daemon()).is_ok()
}

#[no_mangle]
extern "C" fn shutdown_daemon(force: *const uint8_t) -> bool {
    utils::shutdown_daemon(unsafe { *force == 1 }).is_ok()
}
