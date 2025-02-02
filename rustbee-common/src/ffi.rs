use std::ffi::c_uchar as uint8_t;
use std::sync::OnceLock;

use interprocess::local_socket::Stream;
use tokio::runtime::{Builder, Runtime};

use crate::color_space::Rgb;
use crate::colors::Xy;
use crate::constants::{masks::*, ADDR_LEN, DATA_LEN, OUTPUT_LEN, SET};
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

macro_rules! gen_free_fn {
    ($name:ident, $type:ty) => {
        #[no_mangle]
        extern "C" fn $name(ptr: *mut $type) {
            if ptr.is_null() {
                return;
            }

            unsafe {
                drop(Box::from_raw(ptr));
            }
        }
    };
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

gen_free_fn!(free_device, Device);

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
extern "C" fn set_power(device_ptr: *mut Device, state: uint8_t) -> bool {
    if device_ptr.is_null() {
        eprintln!("[ERROR] Device pointer is null");
        return false;
    }

    let device = unsafe { &mut *device_ptr };

    let mut buf = EMPTY_BUFFER;
    buf[0] = SET;
    buf[1] = state;

    device.send_to_socket(CONNECT | POWER, buf).0.is_success()
}

#[no_mangle]
extern "C" fn get_power(device_ptr: *mut Device) -> bool {
    if device_ptr.is_null() {
        eprintln!("[ERROR] Device pointer is null");
        return false;
    }

    let device = unsafe { &mut *device_ptr };

    let output = device.send_to_socket(CONNECT | POWER, EMPTY_BUFFER);
    if !output.0.is_success() {
        println!("Error while trying to get power state and connect to daemon");
        return false;
    }

    output.1[0] == 1
}

#[no_mangle]
extern "C" fn set_brightness(device_ptr: *mut Device, value: uint8_t) -> bool {
    if device_ptr.is_null() {
        eprintln!("[ERROR] Device pointer is null");
        return false;
    }

    let device = unsafe { &mut *device_ptr };

    let mut buf = EMPTY_BUFFER;
    buf[0] = SET;
    buf[1] = (((value as f32) / 100.) * 0xff as f32) as _;

    device
        .send_to_socket(CONNECT | BRIGHTNESS, buf)
        .0
        .is_success()
}

#[no_mangle]
extern "C" fn get_brightness(device_ptr: *mut Device) -> uint8_t {
    if device_ptr.is_null() {
        eprintln!("[ERROR] Device pointer is null");
        return 0;
    }

    let device = unsafe { &mut *device_ptr };

    let output = device.send_to_socket(BRIGHTNESS, EMPTY_BUFFER);
    if !output.0.is_success() {
        println!("Error while trying to get brightness and connect to daemon");
        return 0;
    }

    let bit = output.1[0];

    ((bit as f32 / 255.) * 100.) as _
}
#[no_mangle]
extern "C" fn set_color_rgb(device_ptr: *mut Device, r: uint8_t, g: uint8_t, b: uint8_t) -> bool {
    if device_ptr.is_null() {
        eprintln!("[ERROR] Device pointer is null");
        return false;
    }

    let device = unsafe { &mut *device_ptr };

    let xy = Xy::from(Rgb::new(r.into(), g.into(), b.into()));
    let scaled_x = (xy.x * 0xFFFF as f64) as u16;
    let scaled_y = (xy.y * 0xFFFF as f64) as u16;

    let mut buf = EMPTY_BUFFER;
    buf[0] = SET;
    buf[1] = (scaled_x & 0xFF) as _;
    buf[2] = (scaled_x >> 8) as _;
    buf[3] = (scaled_y & 0xFF) as _;
    buf[4] = (scaled_y >> 8) as _;

    device
        .send_to_socket(CONNECT | COLOR_RGB, buf)
        .0
        .is_success()
}

#[no_mangle]
extern "C" fn get_color_rgb(device_ptr: *mut Device) -> *mut [uint8_t; 3] {
    let mut color_buf = Box::new([0; 3]);

    if device_ptr.is_null() {
        eprintln!("[ERROR] Device pointer is null");
        return Box::into_raw(color_buf);
    }

    let device = unsafe { &mut *device_ptr };

    let output = device.send_to_socket(CONNECT | COLOR_RGB, EMPTY_BUFFER);
    if !output.0.is_success() {
        println!("Error while trying to get color and connect to daemon");
        return Box::into_raw(color_buf);
    }

    let mut rgb = [output.1[0], output.1[1], output.1[2]];

    std::mem::swap(&mut rgb, &mut color_buf);

    Box::into_raw(color_buf)
}

gen_free_fn!(free_color_rgb, [uint8_t; 3]);

#[no_mangle]
extern "C" fn get_name(device_ptr: *mut Device) -> *mut [uint8_t; OUTPUT_LEN - 1] {
    let mut name_buf = Box::new([0; OUTPUT_LEN - 1]);

    if device_ptr.is_null() {
        eprintln!("[ERROR] Device pointer is null");
        return Box::into_raw(name_buf);
    }

    let device = unsafe { &mut *device_ptr };

    let mut output = device.send_to_socket(CONNECT | NAME, EMPTY_BUFFER);
    if !output.0.is_success() {
        println!("Error while trying to get name and connect to daemon");
        return Box::into_raw(name_buf);
    }

    std::mem::swap(&mut *name_buf, &mut output.1);

    Box::into_raw(name_buf)
}

gen_free_fn!(free_name, [uint8_t; OUTPUT_LEN - 1]);

#[no_mangle]
extern "C" fn launch_daemon() -> bool {
    block_on!(utils::launch_daemon()).is_ok()
}

#[no_mangle]
extern "C" fn shutdown_daemon(force: *const uint8_t) -> bool {
    utils::shutdown_daemon(unsafe { *force == 1 }).is_ok()
}
