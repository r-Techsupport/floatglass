pub mod scsi;
pub mod usb;

use crate::scsi::command;
use color_eyre::{Result, eyre::ContextCompat};
use std::fmt::Write;
use tracing::{info, level_filters::LevelFilter};
use usb::enumerate_usb_storage_devices;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize error handling
    color_eyre::install()?;
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::DEBUG)
        .without_time()
        .init();
    info!("starting");
    let mut devices = enumerate_usb_storage_devices().await?;
    let device = devices
        .next()
        .wrap_err("at least one usb drive should be connected")?;
    let drive = usb::USBDrive::new(device).await?;
    let mut scsi_device = scsi::SCSIDevice::new(drive).await?;

    let first_block = scsi_device
        .issue_command(command::read(1, 1, scsi_device.block_size))
        .await?;
    let mut hex_repr = String::with_capacity(512);
    let mut ascii_repr = String::with_capacity(512);
    for byte in first_block.raw().iter() {
        write!(hex_repr, "{byte:X} ")?;
        if byte.is_ascii() {
            ascii_repr.push(*byte as char);
        } else {
            ascii_repr.push('.');
        }
    }

    println!(
        "HEX REPRESENTATION:\n----------\n{hex_repr}\n-----------\nACII REPR:\n-------------\n{ascii_repr}\n-------------"
    );
    Ok(())
}
