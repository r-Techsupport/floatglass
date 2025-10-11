//! A USB packet containing a command block wrapper and associated
//! information.

/// Signature that identifies a packet as a CBW.
///
/// This packet contains the below magic number (little endian).
///
/// See USB Mass Storage Class - Bulk Only Transport, section 5
const CBW_SIGNATURE: u32 = 0x434235355;

/// The CBW is always exactly 31 bytes in size, and in little endian
#[repr(packed)]
struct CommandBlockWrapper {
    /// "Signature that helps identify this packet as a CBW. The signature field
    /// shall contain the value 43425355h (little endian), indicating a CBW."
    signature: u32,
    /// "A cCommand Block Tag sent by the host. The device shall echo the contents
    /// of this field back to the host in the [tag] field of the associated CSW.
    /// The [tag] positvely associates a CSW with the corrosponding CBW"
    tag: u32,
}
