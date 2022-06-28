use rdesktopd::CaptureManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let mut manager = CaptureManager::new().await?;

    manager.begin_capture().await?;

    loop {
        std::thread::yield_now();
    }
}
