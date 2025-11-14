use color_eyre::{Result, eyre::ContextCompat};
use tracing::{info, level_filters::LevelFilter};
use usb::enumerate_usb_storage_devices;
use usbh_scsi::commands::inquiry::InquiryCommand;

use crate::usb::open_usb_device;
mod usb;
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
    open_usb_device(device).await?;
    use usbh_scsi::*;

    //let mut devices = storage::UsbMassStorage::list()?;
    //if let Some(closed) = devices.pop() {
    //    let mut dev = closed.open()?;
    //    let mut buf = [0_u8, 36];
    //    let cmd = InquiryCommand::new(0);
    //    dev.execute_command(
    //        1,
    //        buf.len() as u32,
    //        commands::cbw::Direction::In,
    //        &cmd,
    //        Some(&mut buf),
    //    )?;
    //
    //    dbg!(&buf);
    //}

    Ok(())
}
