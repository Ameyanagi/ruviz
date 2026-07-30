//! Adapter-neutral actions exposed by plot context menus.
//!
//! This module deliberately describes user intent only. GUI adapters remain
//! responsible for presenting menus, choosing save destinations, and writing
//! images to the system clipboard.

#[cfg(feature = "3d")]
use super::plot3d::CameraView3D;

/// An action selected from a plot's context menu.
///
/// Adapters can use this common vocabulary without coupling core plotting
/// state to a particular GUI toolkit or native menu implementation.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlotContextMenuAction {
    /// Restore the plot's original view.
    ResetView,
    /// Recenter the plot and restore its default zoom.
    ///
    /// In the current 2D viewport model, the natural data bounds are also the
    /// reset view, so adapters intentionally resolve this to the same bounds as
    /// [`Self::ResetView`]. In 3D, fitting preserves the current orientation.
    FitToContent,
    /// Save the currently displayed plot as an image.
    SaveImage,
    /// Copy the currently displayed plot image to the system clipboard.
    CopyImage,
    /// Enable or disable pointer-driven plot interaction.
    ToggleInteraction,
    /// Apply a named orientation to a 3D camera.
    #[cfg(feature = "3d")]
    CameraView(CameraView3D),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_actions_are_copyable_adapter_messages() {
        fn assert_copy<T: Copy>() {}
        assert_copy::<PlotContextMenuAction>();

        assert_eq!(
            PlotContextMenuAction::FitToContent,
            PlotContextMenuAction::FitToContent
        );
    }

    #[cfg(feature = "3d")]
    #[test]
    fn camera_view_action_carries_the_shared_view() {
        assert_eq!(
            PlotContextMenuAction::CameraView(CameraView3D::Top),
            PlotContextMenuAction::CameraView(CameraView3D::Top)
        );
    }
}
