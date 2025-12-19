//! SCSI protocol and format implementation as described in:
//! - SCSI Primary Commands – 2 (SPC-2):
//! <https://www.rockbox.org/wiki/pub/Main/DataSheets/spc2r20.pdf>
//!         - This is an older version of the SCSI specification.
//!       It has enough information to describe almost every command we need to know,
//!       except for some information specific to block devices, which is described in the next SCSI
//!       specification linked below.
//! - SCSI Block Commands – 2 (SBC-2)
//! <https://raw.githubusercontent.com/carmark/papers/master/storage/scsi/sbc2r16.pdf>
//! This is an older version of the SCSI block commands specification. It contains information
//! about commands specific to block devices.

/// We chose to implement a 16 byte CDB as described in SPC-2 4.3.2, table 4
pub const CDB_SIZE: usize = 16;

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
}

/// "A command is communicated by sending a command descriptor block
/// to the device ...."
///
/// This struct implements the format described in
/// "SCSI Primary Commands - 2 (SPC-2)" 4.3.2 The fixed length CDB formats
/// Table 4 -- Typical CDB for 16-byte commands
#[repr(packed)]
pub struct CommandDescriptorBlock {
    ///"The `OPERATION CODE` field contains the code value identifying the operation
    /// being requested by the CDB. SAM-2 defines the general structure of the operation
    /// code value. The `OPERATION CODE` field has a consistently defined meaning across
    /// all commands. This standard specifies the operation code values used by the commands
    /// defined herein."
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
    /// "The `PARAMETER LISLT LENGTH` field is used to specify the number of bytes sent
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

impl CommandDescriptorBlock {
    pub fn as_slice(&'_ self) -> &[u8] {
        const {
            assert!(
                std::mem::size_of::<CommandDescriptorBlock>() == CDB_SIZE,
                "CommandDescriptorBlock not 16 bytes in size"
            );
        };
        // SAFETY: the const assertion above
        // guarantees that the size is as we expected,
        // and the reference is tied to the correct lifetime
        let slice: &'_ [u8] = unsafe {
            let ptr = self as *const CommandDescriptorBlock as *const u8;
            std::slice::from_raw_parts(ptr, CDB_SIZE)
        };
        slice
    }

    /// "The TEST UNIT READY command provides a means to check if the logical unit is ready.
    ///
    /// If the logical unit is able to accept an appropriate medium access command without
    /// returning CHECK CONDITION status, this command shall return a GOOD sstatus. If the logical
    /// unit is unable to become operational or is in a state such that an applicaton client action
    /// (e.g START UNIT command) is required to make the unit ready, the device server shall return
    /// CHECK CONDITION status with a sense key of NOT READY."
    ///
    /// Defined in SPC2 7.25
    pub fn test_unit_ready() -> CommandDescriptorBlock {
        Self {
            operation_code: OpCode::TestUnitReady,
            misc_info: 0,
            logical_block_address: 0,
            param: 0,
            _reserved: 0,
            control: 0,
        }
    }
}
