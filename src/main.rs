mod hueblue;

use hueblue::*;

use uuid::{uuid, Uuid};

const HUE_BAR_1_ADDR: [u8; 6] = [0xE8, 0xD4, 0xEA, 0xC4, 0x62, 0x00];
const HUE_BAR_2_ADDR: [u8; 6] = [0xEC, 0x27, 0xA7, 0xD6, 0x5A, 0x9C];

// Thanks to https://gist.github.com/shinyquagsire23/f7907fdf6b470200702e75a30135caf3 for the UUIDs
const LIGHT_SERVICE: Uuid = uuid!("932c32bd-0000-47a2-835a-a8d455b859dd");

const MODEL: Uuid = uuid!("00002a24-0000-1000-8000-00805f9b34fb");
const POWER: Uuid = uuid!("932c32bd-0002-47a2-835a-a8d455b859dd");
const COLOR: Uuid = uuid!("932c32bd-0005-47a2-835a-a8d455b859dd");
const BRIGHTNESS: Uuid = uuid!("932c32bd-0003-47a2-835a-a8d455b859dd");

#[tokio::main]
async fn main() -> bluer::Result<()> {
    let mut tasks = Vec::new();
    let hue_bars = get_devices(&[HUE_BAR_1_ADDR, HUE_BAR_2_ADDR]).await?;

    for hue_bar in hue_bars {
        tasks.push(tokio::spawn(job(hue_bar)));
    }

    for task in tasks {
        task.await??;
    }

    Ok(())
}

async fn job(mut hue_bar: HueBar) -> bluer::Result<()> {
    hue_bar.init_connection().await?;

    // println!("power is: {:?}", bar_one.get_power_state(POWER).await?);
    if !hue_bar.set_power_state(POWER, false).await? {
        println!(
            "[ERROR] Failed to write power state to hue bar address: {}",
            hue_bar.addr
        );
    }

    hue_bar.disconnect().await?;

    Ok(())
}

// for service in bar_one.services().await? {
//     println!("{:?}", service.all_properties().await?);
//     println!("{:?}", service.characteristics().await?);
// }

// [Primary(true), Uuid(0000180a-0000-1000-8000-00805f9b34fb), Includes([])]
// [
//     Characteristic { adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 14, id: 15 },
//     Characteristic { adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 14, id: 19 },
//     Characteristic { adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 14, id: 17 }
// ]
//
// [Primary(true), Uuid(9da2ddf1-0000-44d0-909c-3f3d3cb34a7b), Includes([])]
// [
//     Characteristic { adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 102, id: 103 }
// ]
//
// [Primary(true), Uuid(b8843add-0000-4aa1-8794-c3f462030bda), Includes([])]
// [
//     Characteristic { adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 92, id: 98 },
//     Characteristic { adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 92, id: 93 },
//     Characteristic { adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 92, id: 100 },
//     Characteristic { adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 92, id: 95 }
// ]
//
// [Primary(true), Uuid(0000fe0f-0000-1000-8000-00805f9b34fb), Includes([])]
// [
//     Characteristic { adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 21, id: 47 },
//     Characteristic { adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 21, id: 49 },
//     Characteristic { adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 21, id: 45 },
//     Characteristic { adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 21, id: 24 },
//     Characteristic { adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 21, id: 39 },
//     Characteristic { adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 21, id: 43 },
//     Characteristic { adapter_name:hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 21, id: 36 },
//     Characteristic { adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 21, id: 31 },
//     Characteristic { adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 21, id: 28 },
//     Characteristic { adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 21, id: 41 },
//     Characteristic { adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 21, id: 33 },
//     Characteristic { adapter_name: hci0,device_address: EC:27:A7:D6:5A:9C, service_id: 21, id: 22 },
//     Characteristic { adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 21, id: 26 }
// ]
//
// [Primary(true), Uuid(932c32bd-0000-47a2-835a-a8d455b859dd), Includes([])]
// [
//     Characteristic { adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 51, id: 57 },
//     Characteristic { adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 51, id: 66 },
//     Characteristic { adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 51, id: 60 },
//     Characteristic{ adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 51, id: 63 },
//     Characteristic { adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 51, id: 54 },
//     Characteristic { adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 51, id: 68 },
//     Characteristic { adapter_name:hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 51, id: 71 },
//     Characteristic { adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 51, id: 52 }
// ]
//
// [Primary(true), Uuid(00001801-0000-1000-8000-00805f9b34fb), Includes([])]
// [
//     Characteristic { adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 1, id: 2 },
//     Characteristic { adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 1, id: 5 },
//     Characteristic { adapter_name: hci0, device_address: EC:27:A7:D6:5A:9C, service_id: 1, id: 7 }
// ]
