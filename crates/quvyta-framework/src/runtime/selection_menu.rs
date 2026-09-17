//! The menu a right press on the mouse selection opens: Copy and Raw copy.

use std::marker::PhantomData;

use crate::event::Event;
use crate::geometry::{Rect, Size};
use crate::widget::{EventCx, MeasureCx, PaintCx, Widget};
use crate::widgets::edit_menu::TextMenu;

/// The name of the runtime node that holds the selection menu.
pub(crate) const NAME: &str = "quvyta-selection-menu";

/// A runtime node added after the application's view while text is selected. It takes no
/// space; the engine opens its menu on a right press on the selection and paints it above the
/// selection highlight.
pub(crate) struct SelectionMenu<Msg> {
    marker: PhantomData<fn() -> Msg>,
}

impl<Msg> SelectionMenu<Msg> {
    pub(crate) fn new() -> Self {
        Self { marker: PhantomData }
    }
}

impl<Msg: 'static> Widget<Msg> for SelectionMenu<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, _available: Size) -> Size {
        Size::default()
    }

    fn paint(&self, _cx: &mut PaintCx<'_>, _area: Rect) {}

    fn paint_overlay(&self, cx: &mut PaintCx<'_>, anchor: Rect) {
        TextMenu::copy(cx.env()).paint_overlay(cx, anchor);
    }

    fn event(&self, cx: &mut EventCx<'_, Msg>, event: &Event) -> bool {
        let (used, chosen) = TextMenu::copy(cx.env()).event(cx, event);
        if let Some(kind) = chosen {
            cx.copy_selection(kind);
        }
        used
    }
}
