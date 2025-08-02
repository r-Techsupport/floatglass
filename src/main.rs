use color_eyre::Result;
use nusb::list_devices;

#[tokio::main]
async fn main() -> Result<()> {
    let devices = list_devices().await?;

    for device in devices {
        dbg!(device);
    }

    Ok(())
}
