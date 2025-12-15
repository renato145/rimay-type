use crate::{
    audio::AudioCapture,
    groq_client::{GroqClient, TranscribeOpts},
    hotkey_listener::run_hotkey_listener,
    settings::Configuration,
};
use anyhow::Context;
use enigo::{Enigo, Keyboard};
use global_hotkey::hotkey::HotKey;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tray_icon::{Icon, TrayIconBuilder};

#[derive(Debug)]
pub enum AppEvent {
    KeyPressed(u32, TranscribeOpts),
    KeyReleased(u32),
}

impl std::fmt::Display for AppEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppEvent::KeyPressed(id, _) => write!(f, "KeyPressed({id})"),
            AppEvent::KeyReleased(id) => write!(f, "KeyReleased({id})"),
        }
    }
}

enum TrayCommand {
    SetActive(bool),
}

pub struct Application {
    groq_key: String,
    keys_config: HashMap<HotKey, TranscribeOpts>,
}

impl Application {
    pub fn new(config: Configuration) -> anyhow::Result<Self> {
        let groq_key = std::env::var("GROQ_API_KEY").context("GROQ_API_KEY var not found.")?;
        let keys_config = config.parse_keys()?;
        Ok(Application {
            groq_key,
            keys_config,
        })
    }

    pub async fn run(self) -> anyhow::Result<()> {
        // Channels
        let (event_tx, mut event_rx) = mpsc::channel::<AppEvent>(32);
        let (tray_tx, tray_rx) = mpsc::channel::<TrayCommand>(32);
        let (error_tx, mut error_rx) = mpsc::channel::<anyhow::Error>(1);

        // Spawn GTK thread
        let error_tx_gtk = error_tx.clone();
        std::thread::spawn(move || {
            if let Err(e) = run_gtk(tray_rx).context("GTK thread error") {
                error_tx_gtk
                    .blocking_send(e)
                    .expect("Failed to send error.");
            }
        });

        // Spawn hotkey thread
        std::thread::spawn(move || {
            if let Err(e) =
                run_hotkey_listener(self.keys_config, event_tx).context("Hotkey thread error")
            {
                error_tx.blocking_send(e).expect("Failed to send error.");
            }
        });

        // Main async coordinator
        let groq_client = GroqClient::new(&self.groq_key);
        let mut enigo =
            Enigo::new(&enigo::Settings::default()).context("Failed to build Enigo.")?;
        let mut capture: Option<AudioCapture> = None;
        let mut active_key_id = None;
        loop {
            tokio::select! {
                Some(e) = error_rx.recv() => {
                    panic!("Critical thread crashed: {}", e);
                }
                Some(event) = event_rx.recv() => {
                    handle_event(
                        event, &mut active_key_id, &mut capture, &tray_tx, &groq_client, &mut enigo
                    ).await?;
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

fn load_icon(bytes: &[u8]) -> anyhow::Result<Icon> {
    let image = image::load_from_memory(bytes)
        .context("Failed to load icon.")?
        .into_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height).context("Failed to create icon.")
}

#[tracing::instrument(skip_all, fields(%event))]
async fn handle_event(
    event: AppEvent,
    active_key_id: &mut Option<u32>,
    capture: &mut Option<AudioCapture>,
    tray_tx: &mpsc::Sender<TrayCommand>,
    groq_client: &GroqClient,
    enigo: &mut Enigo,
) -> anyhow::Result<()> {
    let is_active = match (event, *active_key_id) {
        (AppEvent::KeyPressed(_, _), Some(_)) | (AppEvent::KeyReleased(_), None) => {
            // Ignore event
            return Ok(());
        }
        (AppEvent::KeyReleased(a), Some(b)) => {
            if a != b {
                // Ignore event
                return Ok(());
            } else {
                *active_key_id = None;
                false
            }
        }
        (AppEvent::KeyPressed(id, _), None) => {
            *active_key_id = Some(id);
            true
        }
    };
    tracing::info!("Toggled: {}", if is_active { "active" } else { "inactive" });
    tray_tx.send(TrayCommand::SetActive(is_active)).await?;
    if is_active {
        let new_capture = AudioCapture::new().context("Failed to create AudioCapture.")?;
        new_capture.start()?;
        *capture = Some(new_capture);
    } else {
        let Some(old_capture) = capture else {
            return Ok(());
        };
        let wav_bytes = old_capture
            .collect_until_stopped()
            .context("Failed to collect audio.")?;
        if let Some(wav_bytes) = wav_bytes {
            let text = groq_client
                .transcribe(wav_bytes)
                .await
                .context("Failed to transcribe.")?;
            if !text.is_empty() {
                enigo.text(&text).context("Failed to type transcription.")?;
            }
        }
        *capture = None;
    };
    Ok(())
}
