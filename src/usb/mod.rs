//! Interactions with USB mass storage devices

mod cbw;

// Scratchpad:
// https://www.downtowndougbrown.com/2018/12/usb-mass-storage-with-embedded-devices-tips-and-quirks/

// When a flash drive is plugged in, the computer looks at its device,
// configuration, interface, and endpoint descriptors to determine what type of device it is.
// Flash drives use the mass storage class (0x08), SCSI transparent command set subclass (0x06),
// and the bulk-only transport protocol (0x50). The specification indicates that this should be
// specified in the interface descriptor, so the device descriptor should indicate the class is
// defined at the interface level.

// What does this all mean? It just means that there will be two bulk endpoints: one for sending data from the host computer to the flash drive (OUT) and one for receiving data from the flash drive to the computer (IN). Data sent and received on these endpoints will adhere to the bulk-only transport protocol specification linked above. In addition, there are a few commands (read max LUN and bulk-only reset) that are sent over the control endpoint.

// The host starts out by sending a 31-byte command block wrapper (CBW) to the drive, optionally sending or receiving data depending on what command it is, and then reading a 13-byte command status wrapper (CSW) containing the result of the command. The CBW and CSW are simply wrappers around Small Computer System Interface (SCSI) commands. Descriptions of the SCSI commands are available in the last two specifications I linked above.

// That’s all there is to it…except I haven’t said anything about which SCSI commands you’re supposed to use, or when. SCSI is a huge standard. Reading the entire standard document would take a ridiculous amount of time, and it wouldn’t really help you much anyway. Unfortunately, the standards don’t provide a section entitled “recommended sequence of commands for talking to flash drives over USB”.

use std::io::BufRead;
use std::time::Duration;

use color_eyre::eyre::ensure;
use color_eyre::Result;
use nusb::io::{EndpointRead, EndpointWrite};
use nusb::transfer::{Bulk, ControlIn, ControlType, In, Out};
use nusb::{Device, DeviceInfo, list_devices};
use tokio::io::{AsyncBufRead, AsyncWrite};
use tracing::debug;

/// https://www.usb.org/defined-class-codes
const MASS_STORAGE_USB_CLASS: u8 = 0x08;

/// Returns a list of every USB storage device currently connected to the host machine
pub async fn enumerate_usb_storage_devices() -> Result<impl Iterator<Item = DeviceInfo>> {
    let all_usb_devices = list_devices().await?;

    // Each USB device typically exposes one or more *interfaces* as a
    // way to interact with specific functionality of the device.
    let usb_storage_devices = all_usb_devices.filter(|dev| {
        debug!("scanning usb device: {:#?}", dev);
        dev.class() == MASS_STORAGE_USB_CLASS
            || dev
                .interfaces()
                .find(|interface| interface.class() == MASS_STORAGE_USB_CLASS)
                .is_some()
    });
    Ok(usb_storage_devices)
}

pub struct USBDrive {
    bulk_write: EndpointWrite<Bulk>,
    bulk_read: EndpointRead<Bulk>,
}

/// As described by  the USB Mass Storage Class - Bulk Only Transport spec,
/// section 3.2.
///
/// LUN stands for Logical Unit Number, and it's a number
/// used as a unique identifier for a storage device or logical volume.
///
/// <https://en.wikipedia.org/wiki/Logical_unit_number>
const MAX_LUN_REQUEST: ControlIn = ControlIn {
            control_type: ControlType::Class,
            recipient: nusb::transfer::Recipient::Interface,
            request: 0xfe,
            value: 0,
            index: 0,
            length: 1,
};


/// Opens the provided USB mass storage device.
/// 
/// This initialization sequence follows the order
/// described here: <https://www.downtowndougbrown.com/2018/12/usb-mass-storage-with-embedded-devices-tips-and-quirks/>,
/// 
/// where the author obtained it with a USB hardware signal analyzer and reverse engineering the implementations on macos, windows, and linux
#[tracing::instrument]
pub async fn open_usb_device(device_info: DeviceInfo) -> Result<USBDrive> {
    // 1. Claim the USB device to read and write to it
    debug!("opening device");
    let device: Device = device_info.open().await?;
    let interface: nusb::Interface = device.claim_interface(0).await?;
    // 2. Request the maximum LUN
    let max_lun = interface.control_in(MAX_LUN_REQUEST, Duration::from_millis(500)).await?.len();
    ensure!(max_lu
    ).await?.len() == 1, "devices with more than one LUN are not supported");
    // 3. Keep trying the sequence of "TEST UNIT READY" followed by "INQUIRY"
    // until they both return success back-to-back

    // let writer = interface
    //     .endpoint::<Bulk, Out>(0x03)?
    //     .writer(128)
    //     .with_num_transfers(8);

    // let reader = interface
    //     .endpoint::<Bulk, In>(0x03)?
    //     .reader(128)
    //     .with_num_transfers(8);

    // Ok(USBDrive {
    //     bulk_write: writer,
    //     bulk_read: reader,
    // })
    todo!();
}
