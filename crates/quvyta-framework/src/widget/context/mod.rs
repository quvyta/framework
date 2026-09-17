//! Contexts passed to widgets while measuring, painting and handling events, and the frame
//! state the runtime routes input with.

mod animation;
mod event;
mod frame;
mod paint;

pub(crate) use event::Effects;
pub use event::EventCx;
pub(crate) use frame::{FocusRequest, Frame, Interaction, LayerRecord, MeasureKey};
pub use paint::PaintCx;

use crate::env::Env;
use crate::geometry::Size;
use crate::widget::{IdMap, LayoutProps, Node, WidgetId};

/// Measuring context.
pub struct MeasureCx<'a> {
    env: &'a Env,
    layout: LayoutProps,
    /// The sizes measured so far in the frame being painted, when measuring for one.
    measures: Option<&'a mut IdMap<MeasureKey, Size>>,
}

impl<'a> MeasureCx<'a> {
    pub(crate) fn new(env: &'a Env) -> Self {
        Self { env, layout: LayoutProps::default(), measures: None }
    }

    /// A context that remembers every size it measures in `measures`, the store of the frame
    /// being painted, and answers from it when the same node is offered the same space again.
    pub(crate) fn for_frame(env: &'a Env, measures: &'a mut IdMap<MeasureKey, Size>) -> Self {
        Self { env, layout: LayoutProps::default(), measures: Some(measures) }
    }

    /// The environment.
    #[must_use]
    pub fn env(&self) -> &Env {
        self.env
    }

    /// Layout properties of the widget being measured.
    #[must_use]
    pub fn layout(&self) -> LayoutProps {
        self.layout
    }

    /// Measures a child node, including its padding.
    pub fn measure_child<M: 'static>(&mut self, node: &Node<M>, available: Size) -> Size {
        // Nodes a container never exposed through `children_mut` keep the root id; they are not
        // told apart reliably, so they are measured every time.
        let key = (node.id != WidgetId::ROOT).then(|| (node.id, std::ptr::from_ref(node).addr(), available));
        if let (Some(key), Some(measures)) = (key, self.measures.as_deref())
            && let Some(size) = measures.get(&key)
        {
            return *size;
        }
        let size = self.measure_uncached(node, available);
        if let (Some(key), Some(measures)) = (key, self.measures.as_deref_mut()) {
            measures.insert(key, size);
        }
        size
    }

    fn measure_uncached<M: 'static>(&mut self, node: &Node<M>, available: Size) -> Size {
        let padding = node.layout.padding;
        let inner = Size::new(
            available.width.saturating_sub(padding.horizontal()),
            available.height.saturating_sub(padding.vertical()),
        );
        let saved = std::mem::replace(&mut self.layout, node.layout);
        let size = node.widget.measure(self, inner);
        self.layout = saved;
        Size::new(
            size.width.saturating_add(padding.horizontal()).min(available.width),
            size.height.saturating_add(padding.vertical()).min(available.height),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::{MeasureCx, PaintCx};
    use crate::geometry::{Rect, Size};
    use crate::runtime::{App, Command, Harness};
    use crate::style::CellStyle;
    use crate::widget::{View, Widget};
    use crate::widgets::Text;

    /// Text that counts how often it is measured.
    struct Counted(Rc<Cell<u32>>);

    impl Widget<()> for Counted {
        fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
            self.0.set(self.0.get() + 1);
            Size::new(available.width.min(7), available.height.min(1))
        }

        fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
            cx.text(area.x, area.y, "counted", CellStyle::default(), area.width);
        }
    }

    /// A leaf four containers deep, and two siblings that share a name by mistake.
    struct Nested(Rc<Cell<u32>>);

    impl App for Nested {
        type Msg = ();
        fn update(&mut self, (): ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.column(|ui| {
                ui.row(|ui| {
                    ui.column(|ui| {
                        ui.row(|ui| {
                            ui.add(Counted(Rc::clone(&self.0)));
                        });
                    });
                });
                ui.row(|ui| {
                    ui.add(Text::new("a")).id("same");
                    ui.add(Text::new("longer sibling")).id("same");
                })
                .gap(1);
            });
        }
    }

    /// Draws its text limited to a number of cells.
    struct Limited(&'static str, u16);

    impl Widget<()> for Limited {
        fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
            Size::new(available.width, available.height.min(1))
        }

        fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
            let used = cx.text(area.x, area.y, self.0, CellStyle::default(), self.1);
            cx.text(area.x + 10, area.y, &used.to_string(), CellStyle::default(), 3);
        }
    }

    struct Texts;

    impl App for Texts {
        type Msg = ();
        fn update(&mut self, (): ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add(Limited("deploys", 4));
            ui.add(Limited("fade\u{301}d", 4));
            ui.add(Limited("ok", 4));
            ui.add(Limited("界ab", 2));
        }
    }

    #[test]
    fn text_stops_at_its_limit_and_keeps_marks_on_the_last_character() {
        let h = Harness::new(Texts, 14, 4);
        assert_eq!(h.screen(), "depl      4\nfade\u{301}      4\nok        2\n界        2\n");
    }

    #[test]
    fn a_frame_measures_each_node_once_per_offered_space() {
        let count = Rc::new(Cell::new(0));
        let mut h = Harness::new(Nested(Rc::clone(&count)), 20, 3);
        assert_eq!(h.screen(), "counted\na longer sibling\n\n");
        count.set(0);
        h.render();
        // Containers measure their children again before painting them; without remembering
        // sizes the leaf was measured again at every level above it. What remains are the
        // different spaces the levels offer.
        assert!(count.get() <= 5, "measured {} times in one frame, 10 without remembering", count.get());
    }
}
