use std::error::Error;

use file_dialog::FileDialog;

async fn run() -> Result<(), Box<dyn Error>> {
    let dialog = FileDialog::new()?;
    let file = dialog.file_async().await?;
    log::info!("file: {file:?}");
    if let Some(file) = file {
        let bytes = file.read().await?;
        log::info!("read complete. Len: {}", bytes.len());
    }
    Ok(())
}

async fn run_catch() {
    run().await.expect("error in run");
}

fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Trace).expect("error initalizing logger");
    wasm_bindgen_futures::spawn_local(run_catch());
}
