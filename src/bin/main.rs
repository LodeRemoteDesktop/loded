use std::time::Duration;

use log::{debug, error, info, warn};

use loded::{ApiManager, CaptureManager, InputManager};

use tokio::sync::broadcast::channel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let (ds_tx, mut ds_rx) = channel(1);

    let mut cap_manager = CaptureManager::new().await?;

    let (input_manager, ime_tx) = InputManager::new(ds_tx.subscribe())?;

    let mut api_manager = ApiManager::new(ds_tx.subscribe(), ime_tx).await?;

    let desktops = cap_manager.begin_capture(&ds_tx).await?;

    debug!("Desktops: {:#?}", desktops);

    tokio::spawn(async move {
        match api_manager.run(desktops.clone()).await {
            Ok(_) => info!("ApiManager exited successfully"),
            Err(e) => warn!("ApiManager failed to exit successfully: {e}"),
        };
    });

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        ds_tx.send(()).unwrap();
        debug!("5 second exit timeout started");
        tokio::time::sleep(Duration::from_secs(5)).await;
        warn!("Exiting forcefully (Timeout met)");
        std::process::exit(-1);
    });

    tokio::spawn(async move {
        match input_manager.listen().await {
            Ok(_) => info!("InputManager terminated successfully"),
            Err(e) => error!("InputManager did not exit successfully: {e}"),
        }
    });

    loop {
        if (ds_rx.recv().await).is_ok() {
            break;
        } else {
            panic!("Failed to receive death signal");
        }
    }
    info!("Exiting");

    Ok(())
}
