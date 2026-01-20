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
//!
//! This module uses the term "command descriptor" to describe a struct and implementation specific
//! details behind a CDB, and uses the term "command block" to describe a "black box" containing
//! a valid CDB.

/// A serialized command block ready to be submitted
pub struct CommandBlock<'a>(&'a [u8]);

impl CommandBlock<'_> {
    /// Returns the length of the underlying command block.
    ///
    /// Will always be less than 16 bytes.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns a valid command block, prepared as described by USB Mass
    /// Storage Class - Bulk Only Transport section 5.1 (CBWCB).
    pub fn get(&self) -> [u8; 16] {
        let mut output_buf: [u8; 16] = [0; 16];
        let (subslice, _) = output_buf.split_at_mut(self.0.len());
        subslice.copy_from_slice(self.0);
        output_buf
    }
}

/// While the size of a command descriptor block as specified by the SCSi spec varies,
/// the *USB* implementation supports a max command block size of 16 bytes.
///
/// TODO: cite source
pub const MAX_CDB_SIZE: usize = 16;

/// Operation codes for a Command Descriptor Block, specifying what operation you want
/// to do as described in 7.1 of SPC-2.
///
/// This enum is not complete, and is intended to grow
/// as needed
#[repr(u8)]
#[non_exhaustive]
pub enum OpCode {
    /// SPC-2 7.25
    TestUnitReady = 0x0,
    /// SPC-2 7.3
    Inquiry = 0x12,
}

/// As described in 4.3.2 table 1, a typical CDB for 6 byte commands.
#[repr(C, packed)]
struct ShortCommandDescriptor {
    ///"The `OPERATION CODE` field contains the code value identifying the operation
    /// being requested by the CDB. SAM-2 defines the general structure of the operation
    /// code value. The `OPERATION CODE` field has a consistently defined meaning across
    /// all commands. This standard specifies the operation code values used by the commands
    /// defined herein."
    ///
    /// This field specifies what command is being issued by the host
    /// to the drive.
    pub operation_code: OpCode,
    /// "A six-byte CDB contains a 21-bit `LOGICAL BLOCK ADDRESS` field."
    /// The last 3 bits are reserved.
    ///
    /// The use of this field varies from command to command.
    pub logical_block_address: [u8; 3],
    /// Depending on the opcode, this field is one of `TRANSFER LENGTH` (amount of
    /// data to be transferred, usually in blocks),
    /// `PARAMETER LIST LENGTH` (number of bytes sent from the Data-Out buffer),
    /// or `ALLOCATION LENGTH` (The maximum number of bytes a client has allocated for returned
    /// data).
    ///
    ///More info can be found in SCSI SPC2 4.3
    pub misc_len: u8,
    /// "The contents of the `CONTROL` field are defined in SAM-2. The `CONTROL` field
    /// has a consistently defined meaning across all commands."
    ///
    /// As far as I can tell, this value is set to zero by most modern implementations.
    pub control: u8,
}

impl ShortCommandDescriptor {
    // TODO: Make this a generic trait implementable across
    // any command descriptor
    /// Returns a reference to a 6 byte slice of the descriptor.
    pub fn as_slice(&'_ self) -> &[u8] {
        // SAFETY: A struct is the size of itself
        let slice: &'_ [u8] = unsafe {
            let ptr = self as *const ShortCommandDescriptor as *const u8;
            std::slice::from_raw_parts(ptr, std::mem::size_of::<ShortCommandDescriptor>())
        };
        slice
    }
}
/// "A command is communicated by sending a command descriptor block
/// to the device ...."
///
/// Most commands used for interacting with the drive are 16 bytes in size.
///
/// This struct implements the format described in
/// "SCSI Primary Commands - 2 (SPC-2)" 4.3.2 The fixed length CDB formats
/// Table 4 -- Typical CDB for 16-byte commands
#[repr(C, packed)]
struct LongCommandDescriptor {
    ///"The `OPERATION CODE` field contains the code value identifying the operation
    /// being requested by the CDB. SAM-2 defines the general structure of the operation
    /// code value. The `OPERATION CODE` field has a consistently defined meaning across
    /// all commands. This standard specifies the operation code values used by the commands
    /// defined herein."
    ///
    /// This field specifies what command is being issued by the host
    /// to the drive.
    pub operation_code: OpCode,
    /// "Miscellaneous CDB information" (last 5 bits)
    pub misc_info: u8,
    /// "The logical block addresses on a logical unit or within a volume partition
    /// shall begin with block zero and be contiguous up to the last logical
    /// block of that logical unit or within that partition."
    pub logical_block_address: u64,
    /// `TRANSFER_LENGTH` or `PARAMETER_LIST_LENGTH`
    /// or `ALLOCATION LENGTH`
    ///
    /// # `TRANSFER LENGTH`
    /// "The `TRANSFER LENGTH` field specifies the amount of data to be transferred,
    /// usually in the number of blocks. Some commands use transfer length to specify
    /// the requested number of bytes to be sent as defined in the command description.
    /// See the following descriptions and individual commands for further information.
    ///
    /// Commands that use one byte for the `TRANSFER LENGTH` field allow up to 256 blocks
    /// of data to be transfered by one command. A `TRANSFER LENGTH` value of 1 to 255
    /// indicates the number of blocks that shall be transferred. A value of zero specifies
    /// that 256 blocks shall be transferred.
    ///
    /// In commands that use multiple bytes for the `TRANSFER LENGTH` field, a transfer length
    /// of zero indicates that no data transfer shall take place. A value
    /// of one or greater indicates the number of blocks that shall be transferred.
    ///
    /// Refer to the specific command description for further information."
    ///
    /// # `PARAMETER LIST LENGTH`
    /// "The `PARAMETER LIST LENGTH` field is used to specify the number of bytes sent
    /// from the Data-Out Buffer. This field is typically used in CDBs for parameters
    /// that are sent to a device server (e.g., mode parameters, diagnostic parameters,
    /// log parameters). A parameter of length zero indicates that no data shall be transferred.
    /// This condition shall not be considered an error."
    ///
    /// # `ALLOCATION LENGTH`
    /// "The `ALLOCATION LENGTH` field specifies the maximum number of bytes that an application
    /// client has allocated for returned data. An allocation length of zero indicates that no data
    /// shall be transferred. This condition shall not be considered an error. The device server
    /// shall terminate transfers to the Data-In Buffer when `ALLOCATION LENGTH` bytes have been
    /// transferred or when all available data have been transferred, whichever is less. The
    /// allocation length is used to limit the maximum number of data (e.g., sense data, mode data,
    /// log data, diagnostic data) returned to an application client. If the information being
    /// transferred to the Data-In Buffer includes fields containing counts of the number
    /// of bytes in some or all of the data, the contents of these fields shall not be altered
    /// to reflect the truncation, if any, that results from an insufficient `ALLOCATION LENGTH`
    /// value, unless the standard that describes the Data-In buffer format specifically states
    /// otherwise.
    ///
    /// If the amount of information to be transferred exceeds the maximum value that may be
    /// specified in the `ALLOCATION LENGTH` field the device server shall transfer no data
    /// and return a `CHECK CONDITION` status; the sense key shall be set to `ILLEGAL REQUEST`
    /// and the additional sense code shall be set to `INVALID FIELD IN CDB`
    pub param: u32,
    pub _reserved: u8,
    /// "The contents of the `CONTROL` field are defined in SAM-2. The `CONTROL` field
    /// has a consistently defined meaning across all commands."
    ///
    /// As far as I can tell, this value is set to zero by most modern implementations.
    pub control: u8,
}

impl LongCommandDescriptor {
    pub fn as_slice(&'_ self) -> &[u8] {
        // SAFETY: A struct is the size of itself
        let slice: &'_ [u8] = unsafe {
            let ptr = self as *const LongCommandDescriptor as *const u8;
            std::slice::from_raw_parts(ptr, std::mem::size_of::<LongCommandDescriptor>())
        };
        slice
    }
}

pub mod command {
    use crate::usb::scsi::ShortCommandDescriptor;

    use super::{CommandBlock, OpCode};

    /// "The TEST UNIT READY command provides a means to check if the logical unit is ready.
    ///
    /// If the logical unit is able to accept an appropriate medium access command without
    /// returning CHECK CONDITION status, this command shall return a GOOD status. If the logical
    /// unit is unable to become operational or is in a state such that an applicaton client action
    /// (e.g START UNIT command) is required to make the unit ready, the device server shall return
    /// CHECK CONDITION status with a sense key of NOT READY."
    ///
    /// Defined in SPC2 7.25
    pub fn test_unit_ready() -> CommandBlock<'static> {
        CommandBlock(
            ShortCommandDescriptor {
                operation_code: OpCode::TestUnitReady,
                logical_block_address: [0, 0, 0],
                misc_len: 0,
                control: 0,
            }
            .as_slice(),
        )
    }

    /// "The INQUIRY command requests that information regarding parameters
    /// of the target and a component logical unit be sent to the application client.
    /// Options allow the client to request additional information."
    ///
    /// Defined in SPC2 7.3.1 table 45
    pub fn inquiry() -> CommandBlock<'static> {
        CommandBlock(
            ShortCommandDescriptor {
                operation_code: OpCode::Inquiry,
                logical_block_address: [0, 0, 0],
                // For inquiry, is ALLOCATION LENGTH,
                // "The standard INQUIRY data shall contain at least 36 bytes"
                // (table 46)
                misc_len: 36,
                control: 0,
            }
            .as_slice(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::CommandBlock;

    #[test]
    fn validate_command_block() {
        // Ensures that a single byte is packed successfully
        let cmd = [1];
        let cb = CommandBlock(&cmd);
        let mut serialized_cb = cb.get().into_iter();
        assert!(serialized_cb.next() == Some(1));
        assert!(serialized_cb.all(|b| b == 0));
    }
}
