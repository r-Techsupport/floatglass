//! Interactions with USB mass storage devices

mod cbw;
mod scsi;

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

// That’s all there is to it... except I haven’t said anything about which SCSI commands you’re supposed to use, or when. SCSI is a huge standard. Reading the entire standard document would take a ridiculous amount of time, and it wouldn’t really help you much anyway. Unfortunately, the standards don’t provide a section entitled “recommended sequence of commands for talking to flash drives over USB”.

use std::time::Duration;

use color_eyre::Result;
use color_eyre::eyre::{ContextCompat, ensure};
use nusb::descriptors::TransferType;
use nusb::io::{EndpointRead, EndpointWrite};
use nusb::transfer::{Bulk, ControlIn, ControlType, Direction, In, Out};
use nusb::{Device, DeviceInfo, list_devices};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, info, warn};

use crate::usb::cbw::{CommandBlockWrapper, CommandStatusWrapper, TagGenerator};

/// https://www.usb.org/defined-class-codes
const MASS_STORAGE_USB_CLASS: u8 = 0x08;
/// SCSI transparent set subclass
const MASS_STORAGE_SCSI_SUBCLASS: u8 = 0x06;
/// Transport protocol
const MASS_STORAGE_BULK_ONLY_TRANSPORT: u8 = 0x50;
/// <https://en.wikipedia.org/wiki/Logical_unit_number>
const MAX_LUN_REQUEST: ControlIn = ControlIn {
    control_type: ControlType::Class,
    recipient: nusb::transfer::Recipient::Interface,
    request: 0xfe,
    value: 0,
    index: 0,
    length: 1,
};

/// Returns a list of every USB storage device currently connected to the host machine
pub async fn enumerate_usb_storage_devices() -> Result<impl Iterator<Item = DeviceInfo>> {
    let all_usb_devices = list_devices().await?;

    // Each USB device typically exposes one or more *interfaces* as a
    // way to interact with specific functionality of the device.
    let usb_storage_devices = all_usb_devices.filter(|dev| {
        //debug!("scanning usb device: {:#?}", dev);
        dev.class() == MASS_STORAGE_USB_CLASS
            || dev
                .interfaces()
                .find(|interface| {
                    interface.class() == MASS_STORAGE_USB_CLASS
                        && interface.subclass() == MASS_STORAGE_SCSI_SUBCLASS
                        && interface.protocol() == MASS_STORAGE_BULK_ONLY_TRANSPORT
                })
                .is_some()
    });
    Ok(usb_storage_devices)
}

pub struct USBDrive {
    bulk_write: EndpointWrite<Bulk>,
    bulk_read: EndpointRead<Bulk>,
    tag_generator: TagGenerator,
}

impl USBDrive {
    /// Opens the provided USB mass storage device, and runs through some initialization steps.
    ///
    /// This initialization sequence follows the order
    /// described here: <https://www.downtowndougbrown.com/2018/12/usb-mass-storage-with-embedded-devices-tips-and-quirks/>,
    /// reverse engineered from various OS implementatations.
    pub async fn new(device_info: DeviceInfo) -> Result<Self> {
        // 1. Claim the USB device to read and write to it
        info!("opening device...");
        let device: Device = device_info.open().await?;
        info!("device opened, claiming interface...");
        let interface: nusb::Interface = device.detach_and_claim_interface(0).await?;
        info!("interface claimed, opening endpoints");
        debug!("performing endpoint lookup");

        let mut bulk_in_address: Option<u8> = None;
        let mut bulk_out_address: Option<u8> = None;

        for endpoint in interface
            .descriptor()
            .expect("device must have descriptor")
            .endpoints()
        {
            if endpoint.transfer_type() == TransferType::Bulk {
                if endpoint.direction() == Direction::In {
                    if bulk_in_address.is_some() {
                        warn!("multiple Bulk-In endpoints, picking arbitrarily");
                    }
                    bulk_in_address = Some(endpoint.address());
                } else if endpoint.direction() == Direction::Out {
                    if bulk_out_address.is_some() {
                        warn!("multiple Bulk-Out endpoints, picking arbitrarily");
                    }
                    bulk_out_address = Some(endpoint.address());
                }
            }
        }
        // 2. Request the maximum LUN
        debug!("requesting max LUN");
        let max_lun = interface
            .control_in(MAX_LUN_REQUEST, Duration::from_millis(500))
            .await?
            .len();
        ensure!(
            max_lun == 1,
            "devices with more than one LUN are not supported"
        );

        debug!("initializing endpoints");
        // Initialize bulk in/out endpoints
        let writer = interface
            .endpoint::<Bulk, Out>(
                bulk_out_address.wrap_err("USB device has no exposed Bulk-In endpoint")?,
            )?
            .writer(128)
            .with_num_transfers(8);

        let reader = interface
            .endpoint::<Bulk, In>(
                bulk_in_address.wrap_err("USB device has no exposed Bulk-Out endpoint")?,
            )?
            .reader(128)
            .with_num_transfers(8);
        // At this point we can talk to the device, but no usb mass storage specific
        // setup has been performed
        let mut device = Self {
            bulk_write: writer,
            bulk_read: reader,
            tag_generator: TagGenerator::new(),
        };
        info!("starting device configuration");
        // 3. Keep trying the sequence of "TEST UNIT READY" followed by "INQUIRY"
        // until they both return success back-to-back
        debug!("Submitting TEST UNIT READY");
        let test_unit_ready = CommandBlockWrapper::new(
            scsi::command::test_unit_ready(),
            0,
            cbw::CBWDirection::NonDirectional,
            device.tag_generator.tag(),
        );
        let response = device.submit_cbw(test_unit_ready).await?;
        dbg!(response);

        Ok(device)
    }

    /// Submit a command block wrapper, returning the associated command block wrapper.
    ///
    /// This function validates that the status wrapper is correctly associated with the response,
    /// but does not validate that the command executed correctly, i.e it will still return `Ok(..)`
    /// if the command failed.
    pub async fn submit_cbw(
        &mut self,
        command: CommandBlockWrapper,
    ) -> Result<CommandStatusWrapper> {
        // Submit the command
        self.bulk_write.write(command.as_slice()).await?;
        self.bulk_write.flush_end_async().await?;
        debug!("command submitted, pending response");
        // Read the response
        let mut response_buffer: [u8; std::mem::size_of::<CommandStatusWrapper>()] = [0; 13];
        self.bulk_read.read_exact(&mut response_buffer).await?;
        let response = CommandStatusWrapper::from_slice(&response_buffer)?;
        debug!("response recieved");
        ensure!(response.tag == u32::from_le_bytes(command.tag));

        Ok(*response)
    }
}
