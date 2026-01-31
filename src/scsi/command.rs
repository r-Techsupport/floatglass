//! Representations for SCSI commands and responses.
//!
//! This module uses the term "command descriptor" to describe a struct and implementation specific
//! details behind a CDB, and uses the term "command block" to describe a "black box" containing
//! a valid CDB.
//!
//! Commands are exposed as a function that returns a [`CommandBlock`]. These functions wrap
//! the more granular [`ShortCommandDescriptor`] and [`LongCommandDescriptor`] structs.

use super::command_descriptor::*;
use crate::{
    scsi::response::{ResponseParser, inquiry_response, no_response},
    usb::cbw::CBWDirection,
};

/// A serialized command block ready to be submitted
pub struct CommandBlock<'a> {
    command: &'a [u8],
    pub direction: CBWDirection,
    pub data_transfer_len: u32,
    pub response_parser: ResponseParser,
}

impl CommandBlock<'_> {
    /// Returns the length of the underlying command block.
    ///
    /// Will always be less than 16 bytes.
    pub fn len(&self) -> usize {
        self.command.len()
    }

    /// Returns a valid command block, prepared as described by USB Mass
    /// Storage Class - Bulk Only Transport section 5.1 (CBWCB).
    pub fn get(&self) -> [u8; 16] {
        let mut output_buf: [u8; 16] = [0; 16];
        let (subslice, _) = output_buf.split_at_mut(self.command.len());
        subslice.copy_from_slice(self.command);
        output_buf
    }
}

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
    CommandBlock {
        command: X6CommandDescriptor {
            operation_code: OpCode::TestUnitReady,
            logical_block_address: [0, 0, 0],
            misc_len: 0,
            control: 0,
        }
        .as_slice(),
        direction: CBWDirection::NonDirectional,
        data_transfer_len: 0,
        response_parser: no_response,
    }
}

/// "The INQUIRY command requests that information regarding parameters
/// of the target and a component logical unit be sent to the application client.
/// Options allow the client to request additional information."
///
/// Defined in SPC2 7.3.1 table 45
pub fn inquiry() -> CommandBlock<'static> {
    CommandBlock {
        command: X6CommandDescriptor {
            operation_code: OpCode::Inquiry,
            logical_block_address: [0, 0, 0],
            // For inquiry, is ALLOCATION LENGTH,
            // "The standard INQUIRY data shall contain at least 36 bytes"
            // (table 46)
            misc_len: 36,
            control: 0,
        }
        .as_slice(),
        direction: CBWDirection::DataIn,
        data_transfer_len: 36,
        response_parser: inquiry_response,
    }
}

/// "The PREVENT ALLOW MEDIUM REMOVAL" command (see table 77) requests that
/// the target enable or disable the removal of the medium in the logical unit.
/// The logical unit shall not allow medium removal if any initiator current
/// has medium removal prevented."
///
/// SPC-2 7.12
pub fn prevent_allow_medium_removal() -> CommandBlock<'static> {
    CommandBlock {
        command: X6CommandDescriptor {
            operation_code: OpCode::PreventAllowMediumRemoval,
            logical_block_address: [0, 0, 0],
            // See table 78, prohibits all form of medium removal
            misc_len: 0b0000_0011,
            control: 0,
        }
        .as_slice(),
        direction: CBWDirection::NonDirectional,
        data_transfer_len: 0,
        response_parser: no_response,
    }
}

/// "The `READ CAPACITY` command provides a means for the application client
/// to request information regarding the capacity of the block device."
///
/// SBC-2 5.1.10
pub fn read_capacity() -> CommandBlock<'static> {
    CommandBlock {
        command: X10CommandDescriptor {
            operation_code: OpCode::ReadCapacity,
            // Request a "long response" (SBC-2 table 29),
            // with the relative response field set to zero (required
            // for long responses)
            service_action: 0b0000_0010,
            logical_block_address: 0_u32.to_le_bytes(),
            misc_len: 0_u16.to_le_bytes(),
            control: 0,
        }
        .as_slice(),
        direction: CBWDirection::DataIn,
        data_transfer_len: 12,
        response_parser: todo!(),
    }
}

#[cfg(test)]
mod tests {
    use super::CommandBlock;
    use crate::usb::cbw::CBWDirection;
    #[test]
    fn validate_command_block() {
        // Ensures that a single byte is packed successfully
        let cmd = [1];
        let cb = CommandBlock {
            command: &cmd,
            direction: CBWDirection::NonDirectional,
            data_transfer_len: 0,
            response_parser: no_response,
        };
        let mut serialized_cb = cb.get().into_iter();
        assert!(serialized_cb.next() == Some(1));
        assert!(serialized_cb.all(|b| b == 0));
    }
}
