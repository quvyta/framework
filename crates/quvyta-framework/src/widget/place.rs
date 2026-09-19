//! Children placed at a rectangle of their own inside a stack, see [`View::place`](super::View::place).

use std::any::Any;

use super::context::{MeasureCx, PaintCx};
use super::{Node, Widget};
use crate::geometry::{Rect, Size};

/// Cells a placed child may draw past its right edge and below its bottom edge, where a
/// window's shadow falls. They are painted over what lies there and never take the pointer.
pub(crate) const SPILL: u16 = 1;

/// The children of [`View::place`](super::View::place): they fill `rect`, drawn on top of each
/// other like the children of a stack. Inside a stack the rectangle is where they go; anywhere
/// else only its size counts.
pub(crate) struct Placed<Msg> {
    /// Where the children go, relative to the top left corner of the stack.
    rect: Rect,
    children: Vec<Node<Msg>>,
}

impl<Msg: 'static> Placed<Msg> {
    pub(crate) fn new(rect: Rect, children: Vec<Node<Msg>>) -> Self {
        Self { rect, children }
    }

    /// The placement of `node`, when it was added with [`View::place`](super::View::place).
    pub(crate) fn of(node: &Node<Msg>) -> Option<Rect> {
        (&*node.widget as &dyn Any).downcast_ref::<Self>().map(|placed| placed.rect)
    }
}

/// `rect` grown by [`SPILL`] to the right and downwards.
pub(crate) fn with_spill(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y, rect.width.saturating_add(SPILL), rect.height.saturating_add(SPILL))
}

impl<Msg: 'static> Widget<Msg> for Placed<Msg> {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        self.rect.size().min(available)
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        for child in &self.children {
            cx.paint_child_spilling(child, area, with_spill(area));
        }
    }

    fn children(&self) -> &[Node<Msg>] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Node<Msg>] {
        &mut self.children
    }
}

#[cfg(test)]
mod tests {
    use crate::event::{Event, MouseButton, MouseKind};
    use crate::geometry::{Rect, Size};
    use crate::runtime::{App, Command, Harness};
    use crate::style::CellStyle;
    use crate::widget::{EventCx, Length, MeasureCx, PaintCx, View, Widget};
    use crate::widgets::{Button, Text};

    /// Two buttons placed so the second covers the right half of the first.
    #[derive(Default)]
    struct Overlap(Vec<&'static str>);

    impl App for Overlap {
        type Msg = &'static str;
        fn update(&mut self, pressed: &'static str) -> Command<&'static str> {
            self.0.push(pressed);
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, &'static str>) {
            ui.stack(|ui| {
                ui.place(Rect::new(1, 1, 12, 1), |ui| {
                    ui.add(Button::new("under").on_press("under"));
                });
                ui.place(Rect::new(6, 1, 12, 1), |ui| {
                    ui.add(Button::new("over").on_press("over"));
                });
            })
            .fill();
        }
    }

    #[test]
    fn a_click_where_placed_children_overlap_reaches_the_later_one() {
        let mut h = Harness::new(Overlap::default(), 24, 3);
        h.click(7, 1);
        assert_eq!(h.app().0, ["over"]);
        h.click(2, 1);
        assert_eq!(h.app().0, ["over", "under"], "the uncovered part of the first stays clickable");
    }

    /// Text placed partly past every edge of a stack in the middle of the screen.
    #[derive(Default)]
    struct Overflow(Vec<&'static str>);

    impl App for Overflow {
        type Msg = &'static str;
        fn update(&mut self, pressed: &'static str) -> Command<&'static str> {
            self.0.push(pressed);
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, &'static str>) {
            ui.add(Text::new("top"));
            ui.stack(|ui| {
                ui.place(Rect::new(-2, -1, 8, 2), |ui| {
                    ui.add(Text::new("hidden\nleftup"));
                });
                ui.place(Rect::new(6, 1, 10, 1), |ui| {
                    ui.add(Button::new("right edge").on_press("pressed"));
                });
            })
            .height(Length::Cells(2))
            .width(Length::Cells(10));
            ui.add(Text::new("bottom"));
        }
    }

    #[test]
    fn a_placed_child_past_the_edges_is_cut_off() {
        let h = Harness::new(Overflow::default(), 14, 4);
        let screen = h.screen();
        let lines: Vec<&str> = screen.lines().collect();
        assert_eq!(lines[0], "top", "nothing reaches above the stack: {screen}");
        assert_eq!(lines[1], "ftup", "two columns and a row are cut: {screen}");
        assert!(lines[2].len() <= 10, "nothing is drawn right of the stack: {screen}");
        assert_eq!(lines[3], "bottom", "{screen}");
    }

    #[test]
    fn the_cut_off_part_takes_no_pointer() {
        let mut h = Harness::new(Overflow::default(), 14, 4);
        h.click(11, 2);
        assert!(h.app().0.is_empty(), "right of the stack the button is not there");
        h.click(8, 2);
        assert_eq!(h.app().0, ["pressed"], "its visible part is");
    }

    /// Grips that raise themselves to the top when pressed and report drags.
    struct Grip(&'static str);

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Heard {
        Raise(&'static str),
        Drag(&'static str, i32),
    }

    impl Widget<Heard> for Grip {
        fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
            available
        }
        fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
            cx.register_hit(area);
            cx.text(area.x, area.y, self.0, CellStyle::default(), area.width);
        }
        fn event(&self, cx: &mut EventCx<'_, Heard>, event: &Event) -> bool {
            let Event::Mouse(mouse) = event else { return false };
            match mouse.kind {
                MouseKind::Down(MouseButton::Left) => {
                    cx.capture_pointer();
                    cx.emit(Heard::Raise(self.0));
                }
                MouseKind::Drag(MouseButton::Left) => cx.emit(Heard::Drag(self.0, mouse.x)),
                _ => {}
            }
            true
        }
    }

    /// Grips in stacking order, bottom first.
    struct Raising {
        order: Vec<&'static str>,
        drags: Vec<(&'static str, i32)>,
    }

    impl App for Raising {
        type Msg = Heard;
        fn update(&mut self, heard: Heard) -> Command<Heard> {
            match heard {
                Heard::Raise(name) => {
                    self.order.retain(|other| *other != name);
                    self.order.push(name);
                }
                Heard::Drag(name, x) => self.drags.push((name, x)),
            }
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, Heard>) {
            ui.stack(|ui| {
                for (index, name) in self.order.iter().enumerate() {
                    let x = if *name == "a" { 0 } else { 3 };
                    ui.place(Rect::new(x, i32::try_from(index).unwrap_or(0), 6, 1), |ui| {
                        ui.add(Grip(name));
                    })
                    .id(*name);
                }
            })
            .fill();
        }
    }

    #[test]
    fn a_named_placed_child_keeps_its_drag_when_it_comes_to_the_front() {
        let mut h = Harness::new(Raising { order: vec!["a", "b"], drags: Vec::new() }, 12, 3);
        h.mouse(MouseKind::Down(MouseButton::Left), 1, 0);
        assert_eq!(h.app().order, ["b", "a"], "pressed, the first grip came to the front");
        h.mouse(MouseKind::Drag(MouseButton::Left), 5, 0);
        h.mouse(MouseKind::Drag(MouseButton::Left), 7, 2);
        h.mouse(MouseKind::Up(MouseButton::Left), 7, 2);
        assert_eq!(h.app().drags, [("a", 5), ("a", 7)], "the drag stayed with the grip that moved");
    }

    /// A stack sized by what is placed in it, above a line of text.
    struct Sized;

    impl App for Sized {
        type Msg = ();
        fn update(&mut self, (): ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.stack(|ui| {
                ui.place(Rect::new(2, 1, 4, 2), |ui| {
                    ui.add(Text::new("box"));
                });
            });
            ui.add(Text::new("after"));
        }
    }

    #[test]
    fn a_stack_reaches_as_far_as_its_placed_children() {
        let h = Harness::new(Sized, 10, 5);
        assert_eq!(h.screen(), "\n  box\n\nafter\n\n");
    }

    /// Paints everything it may reach, as a window paints its shadow.
    struct Spill;

    impl Widget<()> for Spill {
        fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
            available
        }
        fn paint(&self, cx: &mut PaintCx<'_>, _area: Rect) {
            let reach = cx.clip();
            for y in reach.y..reach.bottom() {
                cx.text(reach.x, y, &"#".repeat(usize::from(reach.width)), CellStyle::default(), reach.width);
            }
        }
    }

    struct Spilling;

    impl App for Spilling {
        type Msg = ();
        fn update(&mut self, (): ()) -> Command<()> {
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.stack(|ui| {
                ui.place(Rect::new(1, 1, 3, 2), |ui| {
                    ui.add(Spill);
                });
            })
            .fill();
        }
    }

    #[test]
    fn a_placed_child_may_draw_one_cell_right_and_below() {
        let h = Harness::new(Spilling, 8, 5);
        assert_eq!(h.screen(), "\n ####\n ####\n ####\n\n");
    }
}
