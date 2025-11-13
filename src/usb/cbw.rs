//! A USB packet containing a command block wrapper and associated
//! information.

/// Signature that identifies a packet as a CBW.
///
/// This packet contains the below magic number (little endian).
///
/// See USB Mass Storage Class - Bulk Only Transport, section 5
const CBW_SIGNATURE: u32 = 0x43425355;
enum CBWDirection {
    /// Data-Out: from host to the device
    DataOut = 0b1000_000,
    /// Data-In: from the device to the host
    DataIn = 0,
}

/// The CBW wraps an SCSi command.
///
/// The CBW is always exactly 31 bytes in size, and in little endian format.
///
/// Spec info can be found in the USB Mass Storage Class - Bulk Only Transport document,
/// section 5.
#[repr(packed)]
pub struct CommandBlockWrapper {
    /// `dCBWSignature` -"Signature that helps identify this packet as a CBW.
    /// The signature field shall contain the value 43425355h (little endian),
    /// indicating a CBW."
    ///
    /// This value should always be set to [`CBW_SIGNATURE`]
    signature: u32,
    /// `dCBWTag` - "A Command Block Tag sent by the host. The device shall echo
    /// the contents of this field back to the host in the [tag] field of the associated CSW.
    /// The [tag] positvely associates a CSW with the corrosponding CBW"
    tag: u32,
    /// `dCBWDataTransferLength` - "The number of bytes that the host expects
    /// to transfer on the Bulk-In or Bulk-Out endpoint (as indicated by the
    /// *Direction* bit) during the execution of this command. If this field
    /// is zero, the device and the host shall transfer no data between the CBW
    /// and associated CSW, and the device shall ignore the value of the *Direction*
    /// bit in *bmCBWFlags*."
    data_transfer_length: u32,
    /// `bmCBWFlags` - A one byte field specifying the direction
    /// of data transfer.
    direction: CBWDirection,
    /// `bCBWLUN` - "The device Logical Unit Number (LUN) to which the command block
    /// is being sent. For devices that support multiple LUNs, the host shall
    /// place into this field, the LUN to which this command block is addressed.
    /// Otherwise, the host shall set this field to zero."
    ///
    /// Multiple LUNs are not currently supported, so this field can just be zero.
    lun: u8,
    /// `bCBWCBLength` - "The valid length of the *CBWCB* in bytes. This defines the
    /// valid length of the command block. The only legal values are 1 through 16
    /// (01h through 10h). All other values are reserved."
    command_block_length: u8,
    /// `CBWCB` - "The command block to be executed by the device. The device shall
    /// first interpret the *bCBWCBLength* bytes in this field as a command block as
    /// defined by the command set *bInterfaceSubClass*. If the command set supported by the
    /// device uses command blocks of fewer than 16 (10h) bytes in length,
    /// the significant bytes shall be transferred first, beginning with the byte
    /// at offset 15 (Fh). The device shall ignore the content of *CBWCB* field
    /// past the offset (15 + *bCBWCBLength* - 1)."
    command: [u8; 16],
}

/// A packet containing the status/result of the command block.
#[repr(packed)]
pub struct CommandStatusWrapper {
    /// `dCSWSignature` - "Signature that helps identify this data packet as a CSW.
    /// The signature field shall contain the value 53425355h (little endian), indicating CSW."
    signature: u32,
    /// `dCSWTag` - "The device shall set this field to the value received in the *dCBWTag* of
    /// the associated CBW."
    tag: u32,
    /// `dCSWDataResidue` - "For Data-Out the device shall report in the *dCSWDataResidue* the
    /// difference between the amount of data expected as stated in the *dCBWDataTransferLength*,
    /// and the actual amount of data processed by the device. For Data-In the device
    /// shall report in the *dCSWDataResidue* the difference between the amount of data expected
    /// as stated in the *dCBWDataTransferLength* and the actual amount of relevant
    /// data sent by the device. The *dCSWDataResidue* shall not exceed the value sent in the
    /// *dCBWDataTransferLength*."
    data_residue: u32,
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
    status: u8,
}
