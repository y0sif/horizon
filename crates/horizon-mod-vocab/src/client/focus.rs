//! What a control a mod's screen paints itself is to a controller's focus
//! ([`crate::DrawContext::focus_target`]).

/// What a controller does with a control a mod's screen paints itself, registered with
/// [`DrawContext::focus_target`](crate::DrawContext::focus_target).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
#[derive(horizon_mod_api_macros::Wire)]
pub enum FocusKind {
    /// A control A presses, with a click at its middle.
    Button,
    /// A slider. A takes hold of it and lets it go; while it is held, left and right click its
    /// track a step along instead of moving the focus.
    Slider {
        /// The value as drawn, 0..1.
        value: f32,
        /// `[x, width]` in GUI pixels: the x where a click sets 0, and the width a click spans
        /// to 1.
        track: [f32; 2],
        /// The part of the track one press moves, 0..1: 0.25 for a slider of 5 values. A held
        /// direction moves faster. 0 moves it by GUI pixels, as the game's own sliders do.
        step: f32,
    },
    /// A row of a list, clicked with A. The D-pad walks the rows, scrolling their list when it
    /// is registered with [`DrawContext::scroll_area`](crate::DrawContext::scroll_area).
    Row,
    /// A tab, clicked with A. The bumpers step through the screen's tabs from the selected one.
    Tab {
        /// This is the tab shown.
        selected: bool,
    },
}
