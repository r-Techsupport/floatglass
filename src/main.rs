pub mod scsi;
pub mod usb;

use color_eyre::{Result, eyre::ContextCompat};
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
    Ok(())
}
