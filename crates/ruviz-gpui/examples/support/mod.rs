use std::time::Duration;

#[cfg(target_os = "macos")]
use std::rc::Rc;

#[cfg(target_os = "macos")]
pub fn application() -> gpui::Application {
    gpui::Application::with_platform(Rc::new(gpui_macos::MacPlatform::new(false)))
}

#[cfg(not(target_os = "macos"))]
pub fn application() -> gpui::Application {
    gpui::Application::new()
}

#[allow(dead_code)]
pub async fn sleep(duration: Duration) {
    smol::Timer::after(duration).await;
}
