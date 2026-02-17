//! Representations for responses to SCSI commands.

use color_eyre::eyre::ensure;

pub type ResponseParser = fn(&[u8]) -> color_eyre::Result<Response>;

pub enum Response<'a> {
    Inquiry(&'a Inquiry),
    /// A tuple of (DRIVE SIZE, BLOCK_SIZE)
    /// where drive size is in blocks, and block size
    /// is in bytes
    ReadCapacity(u32, u32),
    None,
}

pub fn no_response(buf: &[u8]) -> color_eyre::Result<Response<'_>> {
    ensure!(buf.is_empty());
    Ok(Response::None)
}

pub fn inquiry_response(buf: &[u8]) -> color_eyre::Result<Response<'_>> {
    ensure!(
        buf.len() == std::mem::size_of::<Inquiry>(),
        "provided slice length does not match struct size"
    );
    // SAFETY: it's been validated that the slice size matches the struct size
    let s: &'_ Inquiry = unsafe { &*(buf.as_ptr() as *const Inquiry) };
    Ok(Response::Inquiry(s))
}

/// Described in SBC-2 Table 29
pub fn read_capacity_response(buf: &[u8]) -> color_eyre::Result<Response<'_>> {
    ensure!(buf.len() == 8);
    let mut capacity_bytes = [0u8; 4];
    capacity_bytes.copy_from_slice(&buf[0..4]);
    let mut block_size_bytes = [0u8; 4];
    block_size_bytes.copy_from_slice(&buf[4..]);
    // Yes, these are big endian while everything else is little endian, no, I don't know why
    Ok(Response::ReadCapacity(
        // The response technically contains the address of the last block, so we need to
        // correct for the zero-based index
        u32::from_be_bytes(capacity_bytes) + 1,
        u32::from_be_bytes(block_size_bytes),
    ))
}

#[repr(C, packed)]
pub struct Inquiry {
    /// Contains both the PERIPHERAL QUALIFIER (bits 7:5) and PERIPHERAL DEVICE TYPE (bits 4:0)
    /// and PERIPHERAL DEVICE TYPE (bits 4:0) fields.
    ///
    /// The PERIPHERAL QUALIFIER field describes the current state
    /// of the device.
    ///
    /// - 0b0000 – The specified device type is currently connected. This
    ///   does not mean the device is ready for access.
    /// (see SPC-2 table 47 for exact definitions).
    /// In this implementation it is assumed that any other case is a failure.
    ///
    /// I believe the PERIPHERAL QUALIFIER field should
    /// always be set to zero in the context of a USB flash drive.
    ///
    /// The PERIPHERAL DEVICE TYPE field should also be 0h0 because a USB flash drive
    /// is a direct access device. (see table 48)
    pub peripheral_info: u8,
    /// Fields that are not needed
    unparsed: [u8; 35],
}
