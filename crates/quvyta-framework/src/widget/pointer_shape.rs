//! The shape of the mouse pointer a widget asks for over part of itself.

/// The shape the terminal gives the mouse pointer, asked for while painting with
/// [`PaintCx::pointer_shape`](super::PaintCx::pointer_shape).
///
/// The runtime tells the terminal with OSC 22 (`ESC ] 22 ; <name> ESC \`), which foot, kitty and
/// WezTerm understand, and only when the shape under the pointer changed. Other terminals are not
/// sent anything and keep their own pointer. The names are the CSS cursor names those terminals
/// use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PointerShape {
    /// The terminal's usual pointer.
    #[default]
    Default,
    /// A horizontal double arrow, for a side that moves left and right (`ew-resize`).
    EwResize,
    /// A vertical double arrow, for a side that moves up and down (`ns-resize`).
    NsResize,
    /// A diagonal double arrow from the top left to the bottom right corner (`nwse-resize`).
    NwseResize,
    /// A diagonal double arrow from the top right to the bottom left corner (`nesw-resize`).
    NeswResize,
}

impl PointerShape {
    /// The shape's name as OSC 22 carries it, such as `"ew-resize"`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::EwResize => "ew-resize",
            Self::NsResize => "ns-resize",
            Self::NwseResize => "nwse-resize",
            Self::NeswResize => "nesw-resize",
        }
    }
}
