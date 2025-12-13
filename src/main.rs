use anyhow::Context;
use std::io::stdin;
use tray_icon::{Icon, TrayIconBuilder};

fn main() -> anyhow::Result<()> {
    std::thread::spawn(|| {
        gtk::init().unwrap();
        let icon = load_icon(include_bytes!("../icons/inactive.png"));
        let _tray_icon = TrayIconBuilder::new()
            .with_tooltip("system-tray - tray icon library!")
            .with_icon(icon)
            .build()
            .expect("Failed to build tray icon.");
        gtk::main();
    });
    let mut inp = String::new();
    stdin()
        .read_line(&mut inp)
        .context("Failed to read stdin.")?;
    println!("Got {inp:?}");
    Ok(())
}

fn load_icon(bytes: &[u8]) -> Icon {
    let image = image::load_from_memory(bytes)
        .expect("Failed to load icon")
        .into_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height).unwrap()
}

// fn load_icon(path: &std::path::Path) -> tray_icon::Icon {
//     let (icon_rgba, icon_width, icon_height) = {
//         let image = image::open(path)
//             .expect("Failed to open icon path")
//             .into_rgba8();
//         let (width, height) = image.dimensions();
//         let rgba = image.into_raw();
//         (rgba, width, height)
//     };
//     tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
// }
