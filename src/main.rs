use anyhow::Context;
use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager,
    hotkey::{Code, HotKey, Modifiers},
};
use tokio::sync::mpsc;
use tray_icon::{Icon, TrayIconBuilder};

#[derive(Debug)]
enum AppEvent {
    HotkeyPressed,
}

enum TrayCommand {
    SetActive(bool),
}

fn load_icon(bytes: &[u8]) -> anyhow::Result<Icon> {
    let image = image::load_from_memory(bytes)
        .context("Failed to load icon.")?
        .into_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height).context("Failed to create icon.")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Channels
    let (event_tx, mut event_rx) = mpsc::channel::<AppEvent>(32);
    let (tray_tx, tray_rx) = mpsc::channel::<TrayCommand>(32);
    let (error_tx, mut error_rx) = mpsc::channel::<anyhow::Error>(1);

    // Spawn GTK thread
    let crash_tx_gtk = error_tx.clone();
    std::thread::spawn(move || {
        if let Err(e) = run_gtk(tray_rx).context("GTK thread error") {
            crash_tx_gtk
                .blocking_send(e)
                .expect("Failed to send error.");
        }
    });

    // Spawn hotkey thread
    std::thread::spawn(move || {
        if let Err(e) = run_hotkey_listener(event_tx).context("Hotkey thread error") {
            error_tx.blocking_send(e).expect("Failed to send error.");
        }
    });

    // Main async coordinator
    let mut is_active = false;
    loop {
        tokio::select! {
            Some(e) = error_rx.recv() => {
                panic!("Critical thread crashed: {}", e);
            }
            Some(event) = event_rx.recv() => {
                match event {
                    AppEvent::HotkeyPressed => {
                        is_active = !is_active;
                        println!("Toggled: {}", if is_active { "active" } else { "inactive" });
                        tray_tx.send(TrayCommand::SetActive(is_active)).await?;
                        // Future: start/stop recording, call Groq API, etc.
                    }
                }
            }
        }
    }
}

fn run_gtk(tray_rx: mpsc::Receiver<TrayCommand>) -> anyhow::Result<()> {
    gtk::init().expect("Failed to init GTK");

    let inactive_icon = load_icon(include_bytes!("../icons/inactive.png"))?;
    let active_icon = load_icon(include_bytes!("../icons/active.png"))?;

    let tray_icon = TrayIconBuilder::new()
        .with_tooltip("rimay-type")
        .with_icon(inactive_icon.clone())
        .build()
        .context("Failed to create tray icon")?;

    let main_context = glib::MainContext::default();
    main_context.spawn_local(async move {
        // Convert tokio receiver to async-compatible stream
        let mut tray_rx = tray_rx;
        while let Some(cmd) = tray_rx.recv().await {
            match cmd {
                TrayCommand::SetActive(active) => {
                    let icon = if active {
                        active_icon.clone()
                    } else {
                        inactive_icon.clone()
                    };
                    tray_icon.set_icon(Some(icon)).expect("Failed to set icon");
                }
            }
        }
    });

    gtk::main();
    Ok(())
}

fn run_hotkey_listener(event_tx: mpsc::Sender<AppEvent>) -> anyhow::Result<()> {
    let hotkey_manager = GlobalHotKeyManager::new().context("Failed to create hotkey manager")?;
    let hotkey = HotKey::new(Some(Modifiers::SUPER), Code::Semicolon);
    hotkey_manager
        .register(hotkey)
        .context("Failed to register hotkey")?;

    let receiver = GlobalHotKeyEvent::receiver();
    loop {
        if let Ok(event) = receiver.recv()
            && event.id == hotkey.id()
        {
            // println!("{event:?}");
            let _ = event_tx.blocking_send(AppEvent::HotkeyPressed);
        }
    }
}
