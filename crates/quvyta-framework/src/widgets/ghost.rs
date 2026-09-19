//! The ghost: a tone laid over the ground where something is going to land.

use crate::geometry::{Rect, Size};
use crate::widget::{MeasureCx, PaintCx, Widget};

/// How much of the accent a ghost mixes into the ground when it is not told.
const DEFAULT_MIX: f32 = 0.25;

/// A light surface that fills its area with the accent mixed into whatever lies under it: no
/// lines, no text, nothing to click. It shows where something will be, for a window dragged as a
/// ghost over a slow connection (the window stays put, only this moves, and it lands in one
/// frame when the button comes up) and for the area a window would snap to.
///
/// Place it with [`View::place`](crate::widget::View::place) in the same stack as the windows,
/// after them, so it lies over what it covers. It takes no pointer: a press goes through to the
/// window beneath, and a drag that is already under way keeps the window it belongs to.
///
/// [`mix`](Self::mix) says how strong the tone is, around a quarter for a dragged ghost and a
/// fifth for a snap preview. In 256 and 16 colours the cells beneath cannot be blended, so the
/// ghost paints the accent mixed into the theme's canvas and keeps the text on it readable.
///
/// Style keys: `ghost` (`bg`, the colour mixed into the ground; the accent when the theme is
/// silent).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ghost {
    mix: f32,
}

impl Ghost {
    /// A ghost with the usual quarter of accent in the ground.
    #[must_use]
    pub fn new() -> Self {
        Self { mix: DEFAULT_MIX }
    }

    /// How much accent goes into the ground, from 0 to 1.
    #[must_use]
    pub fn mix(mut self, ratio: f32) -> Self {
        self.mix = ratio.clamp(0.0, 1.0);
        self
    }
}

impl Default for Ghost {
    fn default() -> Self {
        Self::new()
    }
}

impl<Msg: 'static> Widget<Msg> for Ghost {
    fn measure(&self, _cx: &mut MeasureCx<'_>, available: Size) -> Size {
        available
    }

    fn paint(&self, cx: &mut PaintCx<'_>, area: Rect) {
        let color = cx.style("ghost", None, &[]).text().bg.unwrap_or_else(|| cx.color("accent"));
        cx.tint_ground(area, color, self.mix);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::ColorDepth;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::View;
    use crate::widgets::{Button, Text};

    /// A ghost over a line of text and a button, with the ratio the state says.
    struct Preview {
        mix: f32,
        pressed: u32,
    }

    impl App for Preview {
        type Msg = ();
        fn update(&mut self, (): ()) -> Command<()> {
            self.pressed += 1;
            Command::none()
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.stack(|ui| {
                ui.place(Rect::new(0, 0, 12, 2), |ui| {
                    ui.add(Text::new("under"));
                });
                ui.place(Rect::new(0, 1, 12, 1), |ui| {
                    ui.add(Button::new("press").on_press(()));
                });
                ui.place(Rect::new(2, 0, 8, 2), |ui| {
                    ui.add(Ghost::new().mix(self.mix));
                });
            })
            .fill();
        }
    }

    #[test]
    fn a_ghost_lays_the_accent_over_the_ground_without_a_line_or_a_hit_area() {
        let mut h = Harness::new(Preview { mix: 0.25, pressed: 0 }, 14, 3);
        assert_eq!(h.screen(), "under\n  press\n\n", "the ghost draws no glyph of its own");
        let theme = h.env().theme();
        let (canvas, accent) = (theme.color("canvas").expect("canvas"), theme.color("accent").expect("accent"));
        assert_eq!(h.bg(3, 0), Some(canvas.mix(accent, 0.25)));
        assert_eq!(h.bg(1, 0), Some(canvas), "left of the ghost the ground is plain");
        assert_eq!(h.fg(3, 0), h.fg(1, 0), "the text keeps its colour");
        h.click(4, 1);
        assert_eq!(h.app().pressed, 1, "the press went through to the button beneath");
    }

    #[test]
    fn the_mix_says_how_strong_the_tone_is() {
        let ground = |mix: f32| {
            let h = Harness::new(Preview { mix, pressed: 0 }, 14, 3);
            h.bg(3, 0)
        };
        let theme = Harness::new(Preview { mix: 0.0, pressed: 0 }, 14, 3);
        let canvas = theme.env().theme().color("canvas");
        assert_eq!(ground(0.0), canvas, "nothing mixed in is no ghost at all");
        let (light, strong) = (ground(0.2), ground(0.5));
        assert_ne!(light, canvas);
        assert_ne!(light, strong);
    }

    #[test]
    fn in_sixteen_colours_the_ghost_still_shows_on_the_ground() {
        let mut h = Harness::new(Preview { mix: 0.25, pressed: 0 }, 14, 3);
        h.set_depth(ColorDepth::Ansi16);
        let ghost = h.buffer()[(3, 0)].bg;
        assert_ne!(ghost, h.buffer()[(1, 0)].bg, "the tone is told from the ground");
        assert_ne!(ghost, h.buffer()[(3, 0)].fg, "and the text on it reads");
    }
}
