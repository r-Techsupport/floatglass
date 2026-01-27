//! Representations for responses to SCSI commands.

use color_eyre::eyre::ensure;

#[repr(C, packed)]
pub struct InquiryResponse {
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

impl InquiryResponse {
    pub fn from_slice(buf: &[u8]) -> color_eyre::Result<&InquiryResponse> {
        ensure!(
            buf.len() == std::mem::size_of::<InquiryResponse>(),
            "provided slice length does not match struct size"
        );
        // SAFETY: it's been validated that the slice size matches the struct size
        let s: &'_ InquiryResponse = unsafe { &*(buf.as_ptr() as *const InquiryResponse) };
        Ok(s)
    }
}
