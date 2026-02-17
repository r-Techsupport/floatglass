//! SCSI protocol and format implementation as described in:
//! - SCSI Primary Commands – 2 (SPC-2):
//!   <https://www.rockbox.org/wiki/pub/Main/DataSheets/spc2r20.pdf>
//!   This is an older version of the SCSI specification.
//!   It has enough information to describe almost every command we need to know,
//!   except for some information specific to block devices, which is described in the next SCSI
//!   specification linked below.
//! - SCSI Block Commands – 2 (SBC-2)
//!   <https://raw.githubusercontent.com/carmark/papers/master/storage/scsi/sbc2r16.pdf>
//!   This is an older version of the SCSI block commands specification. It contains information
//!   about commands specific to block devices.

pub mod command;
mod command_descriptor;
pub mod response;

use std::time::Duration;

use color_eyre::{Result, eyre::Context};
use tracing::{debug, info};

use crate::{
    scsi::response::{Response, ResponseParser},
    usb::USBDrive,
};

/// An abstraction over an underlying USB
/// mass storage device.
///
/// Commands are defined in the `command` module, and
/// issued to the device with the `.issue_command` method.
pub struct SCSIDevice {
    drive: USBDrive,
    /// The size of the drive in *blocks*
    drive_size: u32,
    /// The block size of the storage medium in *bytes*
    block_size: u32,
}

impl SCSIDevice {
    /// Performs SCSI initialization on the drive,
    /// and returns a new [`SCSIDevice`].
    ///
    /// This initialization sequence follows the order
    /// described here: <https://www.downtowndougbrown.com/2018/12/usb-mass-storage-with-embedded-devices-tips-and-quirks/>.
    /// They are not formally documented anywhere, so the author reverse engineered from various OS implementatations.
    pub async fn new(mut drive: USBDrive) -> Result<Self> {
        info!("starting device configuration");
        // 3. Keep trying the sequence of "TEST UNIT READY" followed by "INQUIRY"
        // until they both return success back-to-back
        debug!("submitting TEST UNIT READY");
        drive.submit_cbw(command::test_unit_ready()).await?;
        // At this point it's more convenient to move up a layer of abstraction and finish
        // initialization recursively
        let mut drive = Self {
            drive,
            // Will be updated later
            drive_size: 0,
            block_size: 0,
        };
        debug!("submitting INQUIRY");
        // TODO: actually make something of the response, i.e deserialize into response::InquiryResponse
        let _response = drive.issue_command(command::inquiry()).await?;
        debug!("submitting PREVENT ALLOW MEDIUM REMOVAL");
        // According to the reference blog post, the result can be ignored, and many
        // drives do not support this command, but it's submitted anyway to mimic other
        // operating systems.
        let _ = drive
            .issue_command(command::prevent_allow_medium_removal())
            .await;
        debug!("submitting READ CAPACITY");
        let Response::ReadCapacity(drive_size, block_size) = drive
            .issue_command(command::read_capacity())
            .await?
            .into_response()?
        else {
            unreachable!();
        };
        info!(
            "drive size: {:.2}GiB, block size: {block_size}B",
            (u64::from(drive_size) * u64::from(block_size)) / 1024_u64.pow(3)
        );
        drive.drive_size = drive_size;
        drive.block_size = block_size;
        Ok(drive)
    }

    /// Issues a command to the device.
    ///
    /// This function will submit the command to the device, and wait for the
    /// response.
    pub async fn issue_command(
        &mut self,
        command: command::CommandBlock<'_>,
    ) -> Result<ResponseBytes<'_>> {
        let parser = command.response_parser;
        let response_bytes =
            tokio::time::timeout(Duration::from_millis(1000), self.drive.submit_cbw(command))
                .await
                .context("Drive failed to respond by timeout")??;
        Ok(ResponseBytes {
            bytes: response_bytes,
            parser,
        })
    }
}

pub struct ResponseBytes<'a> {
    bytes: &'a [u8],
    parser: ResponseParser,
}

impl<'a> ResponseBytes<'a> {
    /// Returns a reference to the slice containing the response.
    pub fn raw(&self) -> &'a [u8] {
        self.bytes
    }

    /// Deserializes the slice into a [`Response`]
    pub fn into_response(self) -> Result<Response<'a>> {
        (self.parser)(self.bytes)
    }
}
