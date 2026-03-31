use std::fmt::Display;
use std::time::Duration;

#[cfg(target_os = "macos")]
use std::rc::Rc;

#[cfg(target_os = "macos")]
pub fn application() -> gpui::Application {
    gpui::Application::with_platform(Rc::new(gpui_macos::MacPlatform::new(false)))
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
pub fn application() -> gpui::Application {
    gpui_platform::application()
}

pub fn exit_on_window_open_failure<T, E>(result: Result<T, E>, example_name: &str) -> T
where
    E: Display,
{
    match result {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{example_name} window could not open: {err}");

            #[cfg(target_os = "linux")]
            if std::env::var_os("DISPLAY").is_none()
                && std::env::var_os("WAYLAND_DISPLAY").is_none()
            {
                eprintln!(
                    "No GUI session was detected. Run this example from a local X11/Wayland desktop session."
                );
            }

            std::process::exit(1);
        }
    }
}

#[allow(dead_code)]
pub async fn sleep(duration: Duration) {
    smol::Timer::after(duration).await;
}
