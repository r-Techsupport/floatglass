//! A USB packet containing a command block wrapper and associated
//! information.

use super::scsi;
use color_eyre::eyre::ensure;

/// Signature that identifies a packet as a CBW.
///
/// This packet contains the below magic number (little endian).
///
/// See USB Mass Storage Class - Bulk Only Transport, section 5
const CBW_SIGNATURE: u32 = 0x43425355;
/// Signature that identifies a packet as a CSW.
///
/// The packet will start with the below magic number (little endian).
const CSW_SIGNATURE: u32 = 0x53425355;

/// A command block wrapper is *always* 31 bytes in size*
const CBW_SIZE: usize = 31;

pub enum CBWDirection {
    /// Data-Out: from host to the device
    DataOut = 0b100_0000,
    /// Data-In: from the device to the host
    DataIn = 0,
    /// For when the CBW has a data transfer length of zero.
    ///
    /// According to the spec, this field is ignored entirely if the data transfer
    /// length field is zero, so it exists in the enum purely as an abstraction
    NonDirectional = 255,
}

/// The CBW wraps an SCSi command.
///
/// The CBW is always exactly 31 bytes in size, and in little endian format.
///
/// Spec info can be found in the USB Mass Storage Class - Bulk Only Transport document,
/// section 5.
#[repr(C, packed)]
pub struct CommandBlockWrapper {
    /// `dCBWSignature` -"Signature that helps identify this packet as a CBW.
    /// The signature field shall contain the value 43425355h (little endian),
    /// indicating a CBW."
    ///
    /// This value should always be set to [`CBW_SIGNATURE`]
    signature: [u8; 4],
    /// `dCBWTag` - "A Command Block Tag sent by the host. The device shall echo
    /// the contents of this field back to the host in the [tag] field of the associated CSW.
    /// The [tag] positvely associates a CSW with the corrosponding CBW"
    ///
    /// See [`TagGenerator`] for tooling.
    pub tag: [u8; 4],
    /// `dCBWDataTransferLength` - "The number of bytes that the host expects
    /// to transfer on the Bulk-In or Bulk-Out endpoint (as indicated by the
    /// *Direction* bit) during the execution of this command. If this field
    /// is zero, the device and the host shall transfer no data between the CBW
    /// and associated CSW, and the device shall ignore the value of the *Direction*
    /// bit in *bmCBWFlags*."
    pub data_transfer_length: [u8; 4],
    /// `bmCBWFlags` - A one byte field specifying the direction
    /// of data transfer.
    pub direction: CBWDirection,
    /// `bCBWLUN` - "The device Logical Unit Number (LUN) to which the command block
    /// is being sent. For devices that support multiple LUNs, the host shall
    /// place into this field, the LUN to which this command block is addressed.
    /// Otherwise, the host shall set this field to zero."
    ///
    /// Multiple LUNs are not currently supported, so this field can just be zero.
    pub lun: u8,
    /// `bCBWCBLength` - "The valid length of the *CBWCB* in bytes. This defines the
    /// valid length of the command block. The only legal values are 1 through 16
    /// (01h through 10h). All other values are reserved."
    pub command_block_length: u8,
    /// `CBWCB` - "The command block to be executed by the device. The device shall
    /// first interpret the *bCBWCBLength* bytes in this field as a command block as
    /// defined by the command set *bInterfaceSubClass*. If the command set supported by the
    /// device uses command blocks of fewer than 16 (10h) bytes in length,
    /// the significant bytes shall be transferred first, beginning with the byte
    /// at offset 15 (Fh). The device shall ignore the content of *CBWCB* field
    /// past the offset (15 + *bCBWCBLength* - 1)."
    pub command: [u8; scsi::MAX_CDB_SIZE],
}

impl CommandBlockWrapper {
    /// Creates a new [`CommandBlockWrapper`].
    pub fn new(
        command: scsi::CommandBlock,
        data_transfer_length: u32,
        direction: CBWDirection,
        tag: u32,
    ) -> Self {
        Self {
            signature: CBW_SIGNATURE.to_le_bytes(),
            tag: tag.to_le_bytes(),
            data_transfer_length: data_transfer_length.to_le_bytes(),
            direction,
            lun: 0,
            command_block_length: command.len() as u8,
            command: command.get(),
        }
    }

    /// Returns a slice containing the entirety of `self` that is exactly [`CBW_SIZE`] bytes in length
    pub fn as_slice(&'_ self) -> &[u8] {
        const {
            assert!(
                std::mem::size_of::<CommandBlockWrapper>() == CBW_SIZE,
                "CommandBlockWrapper not 31 bytes in size"
            );
        };
        // SAFETY: the const assertion above
        // guarantees that the size is as we expected,
        // and we know the lifetime of `self` is valid.
        let slice: &'_ [u8] = unsafe {
            let ptr = self as *const CommandBlockWrapper as *const u8;
            std::slice::from_raw_parts(ptr, CBW_SIZE)
        };
        slice
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum CommandStatus {
    Passed = 0,
    Failed = 1,
    PhaseError = 2,
}

/// A packet containing the status/return value of a command block executed by the USB device.
#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct CommandStatusWrapper {
    /// `dCSWSignature` - "Signature that helps identify this data packet as a CSW.
    /// The signature field shall contain the value 53425355h (little endian), indicating CSW."
    signature: u32,
    /// `dCSWTag` - "The device shall set this field to the value received in the *dCBWTag* of
    /// the associated CBW."
    pub tag: u32,
    /// `dCSWDataResidue` - "For Data-Out the device shall report in the *dCSWDataResidue* the
    /// difference between the amount of data expected as stated in the *dCBWDataTransferLength*,
    /// and the actual amount of data processed by the device. For Data-In the device
    /// shall report in the *dCSWDataResidue* the difference between the amount of data expected
    /// as stated in the *dCBWDataTransferLength* and the actual amount of relevant
    /// data sent by the device. The *dCSWDataResidue* shall not exceed the value sent in the
    /// *dCBWDataTransferLength*."
    pub data_residue: u32,
    /// `bCSWStatus` "*bCSWStatus indicates the success or failure of the command. The device
    /// shall set this byte to zero if the command completed successfully. A non-zero value
    /// shall indicate a failure during command execution according to the following table:
    ///
    /// | Value | Description                    |
    /// | ----- | ------------------------------ |
    /// | 0x00  | Command Passed ("good status") |
    /// | 0x01  | Command Failed                 |
    /// | 0x02  | Phase Error                    |
    /// | _     | All other values are reserved  |
    pub status: CommandStatus,
}

impl CommandStatusWrapper {
    /// Cast the provided slice into a command status wrapper.
    ///
    /// This function validates that the `signature` is correct.
    pub fn from_slice(buf: &[u8]) -> color_eyre::Result<&CommandStatusWrapper> {
        ensure!(
            buf.len() == std::mem::size_of::<CommandStatusWrapper>(),
            "provided buffer *must* be same size as struct (CSW_SIZE), was instead {}",
            buf.len()
        );
        // Casting to an enum if it might be an invalid option is undefined behavior.
        // Valid options (as defined by the spec) are values between 0 and 2
        let last_byte = buf.last().unwrap();
        ensure!(
            (0..=2).contains(last_byte),
            "the command status field is invalid, should be in 0..=2, was {last_byte}"
        );

        // SAFETY: The buffer *must* be the same size as the struct
        let csw: &'_ CommandStatusWrapper =
            unsafe { &*(buf.as_ptr() as *const CommandStatusWrapper) };
        let signature = csw.signature;
        ensure!(
            signature == CSW_SIGNATURE,
            "invalid magic number for command status wrapper, should be 0x53425355, is 0x{:X}",
            signature
        );

        Ok(csw)
    }
}

/// Used for generating unique-ish command block tags.
pub struct TagGenerator(u32);

impl TagGenerator {
    /// Initialize the tag generator.
    pub fn new() -> TagGenerator {
        // 123 was chosen as a distinct, human-readable pattern to differentiate it from the rest
        // of the packet
        Self(123)
    }

    /// Returns a unique-ish u32 that's different from the previously returned value.
    pub fn tag(&mut self) -> u32 {
        let output = self.0;
        self.0 = self.0.wrapping_add(1);
        output
    }
}

#[cfg(test)]
mod tests {
    use crate::usb::cbw::CommandStatusWrapper;

    #[test]
    fn catch_invalid_enum_repr() {
        // Captured from an actual USB device, with the last byte (command_status) modified to
        // an invalid value (0xaa)
        let input_packet = [0x55, 0x53, 0x42, 0x53, 0, 0, 0, 0, 0, 0, 0, 0, 0xaa];
        let r = CommandStatusWrapper::from_slice(&input_packet);
        let e = r.expect_err("should catch invalid command status");
        assert!(e.root_cause().to_string().contains("command status"));
    }
}
