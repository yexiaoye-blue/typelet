use tauri::State;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowPlatform {
    pub backend: String,
    pub pin_supported: bool,
}

impl WindowPlatform {
    pub fn detect(window: &tauri::WebviewWindow) -> tauri::Result<Self> {
        #[cfg(target_os = "linux")]
        {
            use gtk::prelude::*;

            let display = window.gtk_window()?.display();
            let backend = display.type_().name();
            // GTK 3's Wayland implementation of set_keep_above is a no-op.
            // Report the real backend instead of switching to XWayland for pinning.
            Ok(Self {
                backend: backend.to_owned(),
                pin_supported: backend == "GdkX11Display",
            })
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = window;
            Ok(Self {
                backend: std::env::consts::OS.to_owned(),
                pin_supported: true,
            })
        }
    }
}

#[tauri::command]
pub fn get_window_platform(platform: State<'_, WindowPlatform>) -> WindowPlatform {
    platform.inner().clone()
}
