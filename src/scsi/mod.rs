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

use color_eyre::Result;
use tracing::{debug, info};

use crate::usb::USBDrive;

/// An abstraction over an underlying USB
/// mass storage device.
///
/// Commands are defined in the `command` module, and
/// issued to the device with the `.issue_command` method.
pub struct SCSIDevice {
    drive: USBDrive,
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

        debug!("submitting INQUIRY");
        // TODO: actually make something of the response, i.e deserialize into response::InquiryResponse
        let _response = drive.submit_cbw(command::inquiry()).await?;
        debug!("submitting PREVENT ALLOW MEDIUM REMOVAL");
        // According to the reference blog post, the result can be ignored, and many
        // drives do not support this command, but it's submitted anyway to mimic other
        // operating systems.
        let _ = drive
            .submit_cbw(command::prevent_allow_medium_removal())
            .await;
        Ok(Self { drive })
    }

    /// Issues a command to the device.
    ///
    /// This function will submit the command to the device, and wait for the
    /// response.
    pub async fn issue_command(&mut self, command: command::CommandBlock<'_>) -> Result<&[u8]> {
        let response_bytes = self.drive.submit_cbw(command).await?;
        Ok(response_bytes)
    }
}
