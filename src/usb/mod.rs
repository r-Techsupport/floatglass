//! Interactions with USB mass storage devices

pub mod cbw;
use std::time::Duration;

use color_eyre::Result;
use color_eyre::eyre::{ContextCompat, ensure};
use nusb::descriptors::TransferType;
use nusb::io::{EndpointRead, EndpointWrite};
use nusb::transfer::{Bulk, ControlIn, ControlType, Direction, In, Out};
use nusb::{Device, DeviceInfo, list_devices};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, info, warn};

use crate::scsi;
use crate::usb::cbw::{CommandBlockWrapper, CommandStatus, CommandStatusWrapper, TagGenerator};
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
            || dev.interfaces().any(|interface| {
                interface.class() == MASS_STORAGE_USB_CLASS
                    && interface.subclass() == MASS_STORAGE_SCSI_SUBCLASS
                    && interface.protocol() == MASS_STORAGE_BULK_ONLY_TRANSPORT
            })
    });
    Ok(usb_storage_devices)
}

pub struct USBDrive {
    bulk_write: EndpointWrite<Bulk>,
    bulk_read: EndpointRead<Bulk>,
    tag_generator: TagGenerator,
    response_buf: Vec<u8>,
}

impl USBDrive {
    /// Opens the provided USB mass storage device and performs USB level initialization.
    ///
    /// This initialization sequence follows the order
    /// described here: <https://www.downtowndougbrown.com/2018/12/usb-mass-storage-with-embedded-devices-tips-and-quirks/>.
    /// They are not formally documented anywhere, so the author reverse engineered from various OS implementatations.
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
        let device = Self {
            bulk_write: writer,
            bulk_read: reader,
            tag_generator: TagGenerator::new(),
            response_buf: vec![0; 2048],
        };
        Ok(device)
    }

    /// Submit a command block wrapper, returning a slice of any bytes recieved.
    ///
    /// No validation is performed, the input is serialized, sent, and response bytes recieved.
    #[tracing::instrument(skip_all)]
    pub async fn submit_cbw(
        &mut self,
        command_block: scsi::command::CommandBlock<'_>,
    ) -> Result<&[u8]> {
        let command = CommandBlockWrapper {
            signature: cbw::CBW_SIGNATURE.to_le_bytes(),
            command: command_block.get(),
            data_transfer_length: command_block.data_transfer_len.to_le_bytes(),
            direction: command_block.direction,
            lun: 0,
            command_block_length: command_block.len() as u8,
            tag: self.tag_generator.tag().to_le_bytes(),
        };
        // Submit the command
        {
            self.bulk_write.write_all(command.as_slice()).await?;
            self.bulk_write.flush_end_async().await?;
            debug!("command submitted, pending response");
        }
        // Ensure the response buffer can fit the response size
        let required_capacity = u32::from_le_bytes(command.data_transfer_length) as usize;
        if self.response_buf.len() < required_capacity + 13 {
            self.response_buf.resize(required_capacity + 13, 0);
        }
        let (response_bytes, status_bytes) = self.response_buf.split_at_mut(required_capacity);
        // Sometimes there's leftover space in the response buffer that we don't care about
        let status_bytes = &mut status_bytes[..13];
        self.bulk_read.read_exact(response_bytes).await?;
        debug!("response buffer filled with {} bytes", response_bytes.len());
        // The status is sent after the response? maybe?
        self.bulk_read.read_exact(status_bytes).await?;
        debug!("status buffer filled with {} bytes", status_bytes.len());

        debug!("response recieved");
        // Validate the status
        {
            let status = CommandStatusWrapper::from_slice(status_bytes)?;
            ensure!(
                status.tag == u32::from_le_bytes(command.tag),
                "invalid command tag"
            );
            ensure!(
                status.status == CommandStatus::Passed,
                "command status indicates failure, full status: {status:#?}"
            );
            ensure!(
                status.data_residue == 0,
                "support for data residue not implemented"
            );
        }
        Ok(response_bytes)
    }
}
