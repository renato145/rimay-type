use crate::{application::AppEvent, groq_client::TranscribeOpts};
use anyhow::Context;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState, hotkey::HotKey};
use std::collections::HashMap;
use tokio::sync::mpsc;

pub fn run_hotkey_listener(
    keys_config: HashMap<HotKey, TranscribeOpts>,
    event_tx: mpsc::Sender<AppEvent>,
) -> anyhow::Result<()> {
    let hotkey_manager = GlobalHotKeyManager::new().context("Failed to create hotkey manager")?;
    let keys = keys_config.keys().copied().collect::<Vec<_>>();
    hotkey_manager
        .register_all(&keys)
        .context("Failed to register hotkeys")?;
    let keys_config = keys_config
        .into_iter()
        .map(|(k, v)| (k.id(), v))
        .collect::<HashMap<_, _>>();
    let receiver = GlobalHotKeyEvent::receiver();
    loop {
        if let Ok(event) = receiver.recv()
            && let Some(opts) = keys_config.get(&event.id())
        {
            let ev = match event.state {
                HotKeyState::Pressed => AppEvent::KeyPressed(event.id(), opts.clone()),
                HotKeyState::Released => AppEvent::KeyReleased(event.id()),
            };
            let _ = event_tx.blocking_send(ev);
        }
    }
}
