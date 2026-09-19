//! Toasts: short notifications that stack in a screen corner and go away on their own.
//!
//! Applications show a toast by returning [`Command::toast`](crate::runtime::Command::toast)
//! from `update`. The runtime owns the stack, because a toast outlives the view that asked for
//! it, counts down while the application is idle and must be drawn above every layer. The
//! application only hears back through the toast's action message.
//!
//! A toast never covers an open modal layer, such as a dialog or the command palette: the stack
//! keeps to the rows between its corner and the dialog's surface, and a toast that finds no
//! room there waits until it does, when the dialog closes or the screen grows. A toast's time
//! runs only while it is on screen, so a waiting toast is not missed.

use std::time::Duration;

use super::Spinner;
use super::cells;
use super::close_mark;
use crate::animation::AnimationName;
use crate::color::Rgb;
use crate::geometry::{Rect, clamp_u16};
use crate::motion::{Easing, steps};
use crate::style::CellStyle;
use crate::text;
use crate::widget::{Key, PaintCx, Widget, WidgetId};

/// What a toast reports; picks its status colour and icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToastKind {
    /// Something finished well.
    Success,
    /// Something needs attention.
    Warning,
    /// Something failed.
    Danger,
    /// Neutral news.
    #[default]
    Info,
}

impl ToastKind {
    /// Every kind.
    pub const ALL: [Self; 4] = [Self::Success, Self::Warning, Self::Danger, Self::Info];

    /// A short name, e.g. for settings screens.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Danger => "danger",
            Self::Info => "info",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Danger => "error",
            other => other.name(),
        }
    }
}

/// The screen corner toasts stack in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Corner {
    /// Top right; the newest toast is on top.
    TopRight,
    /// Bottom right; the newest toast is at the bottom.
    #[default]
    BottomRight,
    /// Bottom left; the newest toast is at the bottom.
    BottomLeft,
    /// Top left; the newest toast is on top.
    TopLeft,
}

impl Corner {
    /// Every corner.
    pub const ALL: [Self; 4] = [Self::TopRight, Self::BottomRight, Self::BottomLeft, Self::TopLeft];

    /// A short name, e.g. for settings screens.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::TopRight => "top-right",
            Self::BottomRight => "bottom-right",
            Self::BottomLeft => "bottom-left",
            Self::TopLeft => "top-left",
        }
    }

    fn right(self) -> bool {
        matches!(self, Self::TopRight | Self::BottomRight)
    }

    fn bottom(self) -> bool {
        matches!(self, Self::BottomRight | Self::BottomLeft)
    }
}

/// How long a toast stays when nothing else is said.
const DEFAULT_DURATION: Duration = Duration::from_secs(5);

/// How long a toast with an action stays, so there is time to reach the button.
const ACTION_DURATION: Duration = Duration::from_secs(8);

/// Widest a toast grows, in cells.
const MAX_WIDTH: u16 = 56;

/// Cells from the inner left edge of a toast to its text: a cell before the icon, the icon
/// (`icon_width` cells) and two cells after it.
fn text_indent(icon_width: u16) -> u16 {
    icon_width.saturating_add(3)
}

/// Cells an action button labelled `label` takes: the label with a cell on each side.
fn action_width(label: &str) -> u16 {
    text::width(label).saturating_add(2)
}

/// The title of `toast` on a toast whose content is `inner_width` cells wide: the first line
/// beside the action and the close mark, the rest wrapped under them to the right edge.
fn title_lines<Msg>(toast: &Toast<Msg>, inner_width: u16, icon_width: u16) -> Vec<String> {
    let rest = inner_width.saturating_sub(text_indent(icon_width)).max(1);
    // One cell of surface before the close mark, and two before an action.
    let mut first = rest.saturating_sub(close_mark::WIDTH);
    if let Some((label, _)) = &toast.action {
        first = first.saturating_sub(action_width(label).saturating_add(2));
    }
    let title = toast.title.as_str();
    let Some(line) = text::wrap_ranges(title, first.max(1)).into_iter().next() else {
        return vec![String::new()];
    };
    let remainder = title[line.end..].trim_start();
    if remainder.is_empty() {
        return vec![title.to_owned()];
    }
    let mut lines = vec![title[line].trim_end().to_owned()];
    lines.extend(text::wrap(remainder, rest));
    lines
}

/// A notification: a status marker and icon, a title, an optional body and action, and the
/// shared three-cell close mark at the end of the title row.
///
/// A title too long for its row wraps, the lines after the first running under the action and
/// the close mark to the right edge, so a narrow screen or a longer language never cuts it. The
/// action and the mark stay on the first row.
///
/// Only a press on the close mark dismisses a toast; a press on the rest of it does nothing unless
/// [`on_press`](Self::on_press) gives it a message, e.g. to open where the news came from.
///
/// Style keys: `toast` (`bg`, `padding`) with `hover` while the pointer is on a pressable toast,
/// `toast-title`, `toast-body`, `toast-action` with `active` on a raised toast and `hover`, and
/// `close-mark` (see tabs). The marker and icon use the `success`, `warning`,
/// `danger` and `info` colours; an animated icon uses `spinner` with the kind as its variant.
pub struct Toast<Msg> {
    kind: ToastKind,
    title: String,
    body: Option<String>,
    action: Option<(String, Msg)>,
    duration: Option<Duration>,
    key: Option<String>,
    icon_motion: Option<AnimationName>,
    /// Makes the message a press on the toast sends; called on every press, because the toast stays.
    on_press: Option<Box<dyn Fn() -> Msg>>,
}

impl<Msg> Toast<Msg> {
    /// A toast of `kind` saying `title`.
    #[must_use]
    pub fn new(kind: ToastKind, title: impl Into<String>) -> Self {
        Self {
            kind,
            title: title.into(),
            body: None,
            action: None,
            duration: None,
            key: None,
            icon_motion: None,
            on_press: None,
        }
    }

    /// A success toast.
    #[must_use]
    pub fn success(title: impl Into<String>) -> Self {
        Self::new(ToastKind::Success, title)
    }

    /// A warning toast.
    #[must_use]
    pub fn warning(title: impl Into<String>) -> Self {
        Self::new(ToastKind::Warning, title)
    }

    /// A danger toast.
    #[must_use]
    pub fn danger(title: impl Into<String>) -> Self {
        Self::new(ToastKind::Danger, title)
    }

    /// An info toast.
    #[must_use]
    pub fn info(title: impl Into<String>) -> Self {
        Self::new(ToastKind::Info, title)
    }

    /// A faint line of detail under the title, wrapped to the toast width.
    #[must_use]
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// A button on the title row; clicking it sends `message` and dismisses the toast.
    #[must_use]
    pub fn action(mut self, label: impl Into<String>, message: Msg) -> Self {
        self.action = Some((label.into(), message));
        self
    }

    /// How long the toast stays while the pointer is not on it. Default: 5 s, or 8 s with an
    /// action.
    #[must_use]
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Names the toast: showing another toast with the same key replaces it, and
    /// [`Command::dismiss_toast`](crate::runtime::Command::dismiss_toast) removes it.
    #[must_use]
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Plays a one-cell [`Spinner`] animation in the icon cell instead of the kind's icon, in
    /// the kind's colour, e.g. [`SpinnerStyle::Pulse`](super::SpinnerStyle::Pulse) while something
    /// is still running. It takes a spinner style or the name of any
    /// [cell animation](crate::animation), such as one a theme defines. With reduced motion the
    /// kind's icon stands still instead. Replace the toast through its [`key`](Self::key) with one
    /// without animation when the work is done.
    ///
    /// ```
    /// use qframe::prelude::*;
    /// use qframe::widgets::{SpinnerStyle, Toast};
    ///
    /// let running: Toast<()> = Toast::info("Deploying api-gateway").icon_motion(SpinnerStyle::Pulse).key("deploy");
    /// let done: Toast<()> = Toast::success("Deployed api-gateway").key("deploy");
    /// let _ = Command::batch([Command::toast(running), Command::toast(done)]);
    /// ```
    #[must_use]
    pub fn icon_motion(mut self, animation: impl Into<AnimationName>) -> Self {
        self.icon_motion = Some(animation.into());
        self
    }
}

impl<Msg: 'static> Toast<Msg> {
    /// The same toast sending `map(message)` for its action and presses.
    pub(crate) fn map<B: 'static>(self, map: std::sync::Arc<dyn Fn(Msg) -> B + Send + Sync>) -> Toast<B> {
        let action = self.action.map(|(label, message)| (label, map(message)));
        let on_press = self.on_press.map(|press| Box::new(move || map(press())) as Box<dyn Fn() -> B>);
        Toast {
            kind: self.kind,
            title: self.title,
            body: self.body,
            action,
            duration: self.duration,
            key: self.key,
            icon_motion: self.icon_motion,
            on_press,
        }
    }
}

impl<Msg: Clone + 'static> Toast<Msg> {
    /// Makes the toast pressable: a press anywhere on it but its action and close mark sends
    /// `message`, e.g. to open the log or the page the news came from. The toast stays; only the
    /// close mark dismisses it. While the pointer is on it the toast rises one tone.
    ///
    /// ```
    /// use qframe::prelude::*;
    /// use qframe::widgets::Toast;
    ///
    /// #[derive(Clone)]
    /// enum Msg {
    ///     OpenDeployLog,
    /// }
    ///
    /// let toast: Toast<Msg> = Toast::success("Deployed api-gateway").on_press(Msg::OpenDeployLog);
    /// let _ = Command::toast(toast);
    /// ```
    #[must_use]
    pub fn on_press(mut self, message: Msg) -> Self {
        self.on_press = Some(Box::new(move || message.clone()));
        self
    }
}

struct Entry<Msg> {
    toast: Toast<Msg>,
    id: WidgetId,
    remaining: Duration,
    /// When the countdown was last advanced; `None` until first painted.
    ticked: Option<Duration>,
    shown_at: Option<Duration>,
    leaving: bool,
    left_at: Option<Duration>,
    /// Found no room in the last frame: not drawn, its time stopped and its entrance still to
    /// come.
    waiting: bool,
    rect: Rect,
    action_rect: Rect,
    close_rect: Rect,
}

/// What a press on the toast stack did.
pub(crate) enum ToastPress<Msg> {
    /// The press landed on a toast that has nothing to do with it; it goes no further.
    Held,
    /// The press on a close mark dismissed its toast.
    Dismissed,
    /// The press chose a toast's action, which also dismissed it.
    Action(Msg),
    /// The press landed on a pressable toast, which stays.
    Pressed(Msg),
}

/// The toasts the runtime shows.
pub(crate) struct ToastStack<Msg> {
    entries: Vec<Entry<Msg>>,
    corner: Corner,
    next_id: u64,
}

impl<Msg> Default for ToastStack<Msg> {
    fn default() -> Self {
        Self { entries: Vec::new(), corner: Corner::default(), next_id: 0 }
    }
}

impl<Msg> ToastStack<Msg> {
    /// Adds `toast`, replacing a visible toast with the same key in place.
    pub(crate) fn push(&mut self, toast: Toast<Msg>) {
        let remaining =
            toast.duration.unwrap_or(if toast.action.is_some() { ACTION_DURATION } else { DEFAULT_DURATION });
        if let Some(key) = &toast.key
            && let Some(entry) = self.entries.iter_mut().find(|e| !e.leaving && e.toast.key.as_ref() == Some(key))
        {
            entry.toast = toast;
            entry.remaining = remaining;
            return;
        }
        self.next_id += 1;
        let id = WidgetId::ROOT.child(&Key::Named(format!("quvyta.toast.{}", self.next_id)), "Toast");
        self.entries.push(Entry {
            toast,
            id,
            remaining,
            ticked: None,
            shown_at: None,
            leaving: false,
            left_at: None,
            waiting: false,
            rect: Rect::default(),
            action_rect: Rect::default(),
            close_rect: Rect::default(),
        });
    }

    /// Starts removing the toast named `key`.
    pub(crate) fn dismiss(&mut self, key: &str) {
        for entry in self.entries.iter_mut().filter(|e| e.toast.key.as_deref() == Some(key)) {
            entry.leaving = true;
        }
    }

    /// Chooses where toasts stack.
    pub(crate) fn set_corner(&mut self, corner: Corner) {
        self.corner = corner;
    }

    /// Handles a pointer press at a cell; `None` when no toast is there. Only the close mark and
    /// the action dismiss; the rest of a toast sends its `on_press` message, if any.
    pub(crate) fn press(&mut self, x: i32, y: i32) -> Option<ToastPress<Msg>> {
        let entry = self.entries.iter_mut().rev().find(|e| !e.leaving && e.rect.contains(x, y))?;
        if entry.close_rect.contains(x, y) {
            entry.leaving = true;
            return Some(ToastPress::Dismissed);
        }
        if entry.action_rect.contains(x, y)
            && let Some((_, message)) = entry.toast.action.take()
        {
            entry.leaving = true;
            return Some(ToastPress::Action(message));
        }
        Some(entry.toast.on_press.as_ref().map_or(ToastPress::Held, |message| ToastPress::Pressed(message())))
    }

    /// Counts down, lays out and draws the stack over everything else but open modal layers,
    /// which it keeps clear of.
    pub(crate) fn paint(&mut self, cx: &mut PaintCx<'_>) {
        let now = cx.now();
        let enter = cx.env().theme().motion().enter;
        let reduced = cx.reduced_motion();
        let pointer = cx.pointer_anywhere();
        for entry in &mut self.entries {
            if entry.waiting {
                // Nobody saw it yet, so its time has not started.
                continue;
            }
            let hovered = pointer.is_some_and(|(x, y)| entry.rect.contains(x, y));
            let since = entry.ticked.unwrap_or(now);
            if !hovered && !entry.leaving {
                entry.remaining = entry.remaining.saturating_sub(now.saturating_sub(since));
            }
            entry.ticked = Some(now);
            entry.shown_at.get_or_insert(now);
            if entry.remaining.is_zero() {
                entry.leaving = true;
            }
            if entry.leaving && entry.left_at.is_none() {
                entry.left_at = Some(now);
            }
        }
        // A toast dismissed while it waited was never seen, so it goes without sliding out.
        self.entries.retain(|entry| {
            !(entry.waiting && entry.leaving) && entry.left_at.is_none_or(|left| !reduced && now < left + enter)
        });

        let screen = cx.clip();
        let style = cx.style("toast", None, &[]);
        let padding = style.padding();
        let width = MAX_WIDTH.min(screen.width.saturating_sub(4));
        let corner = self.corner;
        let x = if corner.right() { screen.right() - 2 - i32::from(width) } else { screen.x + 2 };
        let modals = cx.modal_surfaces(screen);
        let (top_limit, bottom_limit) = room(screen, Rect::new(x, screen.y, width, screen.height), corner, &modals);
        let mut y = if corner.bottom() { bottom_limit - 1 } else { top_limit + 1 };
        let mut placing = width >= 12;
        // Newest nearest the corner.
        for index in (0..self.entries.len()).rev() {
            let icon_width = text::width(&cx.env().icons().glyph(self.entries[index].toast.kind.icon()));
            let inner_width = width.saturating_sub(padding.horizontal());
            let body_width = inner_width.saturating_sub(text_indent(icon_width));
            let toast = &self.entries[index].toast;
            let title_lines = title_lines(toast, inner_width, icon_width).len();
            let body_lines = toast.body.as_deref().map_or(0, |body| text::wrap(body, body_width).len());
            let lines = clamp_u16(i32::try_from(title_lines + body_lines).unwrap_or(i32::MAX));
            let height = cells::sum([padding.vertical(), lines]);
            let top = if corner.bottom() { y - i32::from(height) } else { y };
            // The first toast without room waits, and so does every older one, so the stack
            // keeps its order.
            placing = placing && top >= top_limit && top + i32::from(height) <= bottom_limit;
            let entry = &mut self.entries[index];
            if !placing {
                if !entry.waiting {
                    entry.waiting = true;
                    entry.ticked = None;
                    entry.shown_at = None;
                    entry.rect = Rect::default();
                }
                continue;
            }
            if entry.waiting {
                // Its time starts now, and it slides in as if it had just been shown.
                entry.waiting = false;
                entry.ticked = Some(now);
                entry.shown_at = Some(now);
            }
            // Slides in from the screen edge over `motion.enter`, and back out when leaving.
            let arrived = cx.progress_since(entry.shown_at.unwrap_or(now), enter, Easing::EaseOut);
            let gone = entry.left_at.map_or(0.0, |left| cx.progress_since(left, enter, Easing::EaseIn));
            let presence = (arrived - gone).clamp(0.0, 1.0);
            let offset = i32::from(steps(1.0 - presence, width + 2));
            let shift = if corner.right() { offset } else { -offset };
            entry.rect = Rect::new(x, top, width, height);
            paint_entry(cx, entry, Rect::new(x + shift, top, width, height), presence);
            if !entry.leaving {
                cx.register_hit_as(entry.rect, entry.id);
                if !pointer.is_some_and(|(px, py)| entry.rect.contains(px, py)) {
                    cx.request_frame_in(entry.remaining);
                }
            }
            y = if corner.bottom() { top - 1 } else { top + i32::from(height) + 1 };
        }
    }
}

/// The rows toasts in `column` may use, as a top and an exclusive bottom: the whole `screen`,
/// cut back to the side of every modal surface that shares columns with the stack where the
/// corner is. A row stays free between a surface and the toasts, as between two toasts.
fn room(screen: Rect, column: Rect, corner: Corner, modals: &[Rect]) -> (i32, i32) {
    let (mut top, mut bottom) = (screen.y, screen.bottom());
    for modal in modals.iter().filter(|modal| modal.x < column.right() && column.x < modal.right()) {
        if corner.bottom() {
            top = top.max(modal.bottom() + 1);
        } else {
            bottom = bottom.min(modal.y - 1);
        }
    }
    (top, bottom)
}

fn paint_entry<Msg>(cx: &mut PaintCx<'_>, entry: &mut Entry<Msg>, rect: Rect, presence: f32) {
    let pointer = cx.pointer_anywhere();
    // Only a pressable toast answers the pointer; the close mark and action light on their own.
    let raised = entry.toast.on_press.is_some() && pointer.is_some_and(|(x, y)| entry.rect.contains(x, y));
    let states = if raised { vec![crate::theme::State::Hover] } else { Vec::new() };
    let style = cx.style("toast", None, &states);
    let padding = style.padding();
    let background = style.text().bg.unwrap_or_else(|| cx.color("overlay"));
    // A toast floats over whatever the screen shows there and keeps apart from it; see
    // `PaintCx::floating`. The lift is known before painting, so text arrives from the surface
    // as it will show.
    let grounds = cx.grounds_around(rect);
    let lift = cx.lift_for(rect, &grounds, Some(background));
    let surface = lift.map_or(background, |lift| lift.apply(background));
    // Colours arrive with the cells: text blends from the surface as the toast slides in.
    let blend = |color: Option<Rgb>| color.map(|c| surface.mix(c, presence));
    let status = cx.color(entry.toast.kind.name());
    cx.clear(rect, background);
    cx.fill(Rect::new(rect.x, rect.y, 1, rect.height), background.mix(status, presence));

    let inner = rect.inset(padding);
    let content_x = inner.x + 1;
    let icon = cx.env().icons().glyph(entry.toast.kind.icon()).into_owned();
    let icon_width = text::width(&icon);
    match entry.toast.icon_motion.clone().filter(|_| !cx.reduced_motion()) {
        Some(animation) => {
            // The spinner draws itself in the kind's colour; the cell then arrives with the toast
            // like every other colour on it.
            let cell = Rect::new(content_x, inner.y, 1, 1);
            let spinner = Spinner::new().animation(animation).variant(entry.toast.kind.name());
            Widget::<()>::paint(&spinner, cx, cell);
            cx.tint(cell, background, 1.0 - presence);
        }
        None => {
            let style = CellStyle { fg: blend(Some(status)), ..CellStyle::default() };
            cx.text(content_x, inner.y, &icon, style, icon_width);
        }
    }
    // The text keeps its column whether the icon moves or not, so a keyed toast that settles
    // from a spinner to its icon does not jump.
    let text_x = inner.x + i32::from(text_indent(icon_width));

    // Where the stack is laid out, as opposed to where the slide draws it: presses land there.
    let slid = rect.x - entry.rect.x;
    // The close mark ends on the first cell of the right padding, as on tabs. It keeps its resting
    // whisper on the toast and lights only under the pointer, so it never looks like the target
    // of a press elsewhere on the toast.
    let mark_x = inner.right() - i32::from(close_mark::WIDTH) + 1;
    let mark = close_mark::paint(cx, mark_x, inner.y, false);
    cx.tint(mark, background, 1.0 - presence);
    entry.close_rect = Rect::new(mark.x - slid, mark.y, mark.width, 1);
    // One cell of surface between the mark and whatever comes before it.
    let mut right = mark_x - 1;
    entry.action_rect = Rect::default();
    if let Some((label, _)) = &entry.toast.action {
        let label_width = action_width(label);
        let action = Rect::new(right - i32::from(label_width), inner.y, label_width, 1);
        let target = Rect::new(action.x - slid, action.y, action.width, 1);
        // On a raised toast the button climbs with it, so it stays a step above its surface.
        let mut states = if raised { vec![crate::theme::State::Active] } else { Vec::new() };
        if pointer.is_some_and(|(x, y)| target.contains(x, y)) {
            states.push(crate::theme::State::Hover);
        }
        let action_style = cx.style("toast-action", None, &states).text();
        if let Some(bg) = action_style.bg {
            cx.fill(action, background.mix(bg, presence));
        }
        cx.text(
            action.x + 1,
            action.y,
            label,
            CellStyle { fg: blend(action_style.fg), bg: None, ..action_style },
            label_width,
        );
        entry.action_rect = target;
        right = action.x - 2;
    }

    let title_style = cx.style("toast-title", None, &[]).text();
    let title_style = CellStyle { fg: blend(title_style.fg), bg: None, ..title_style };
    let title = title_lines(&entry.toast, inner.width, icon_width);
    let body_width = clamp_u16(inner.right() - text_x);
    for (row, line) in title.iter().enumerate() {
        let y = inner.y + i32::try_from(row).unwrap_or(0);
        let budget = if row == 0 { clamp_u16(right - text_x) } else { body_width };
        cx.text(text_x, y, line, title_style, budget);
    }
    if let Some(body) = &entry.toast.body {
        let body_style = cx.style("toast-body", None, &[]).text();
        let top = inner.y + i32::try_from(title.len()).unwrap_or(1);
        for (row, line) in text::wrap(body, body_width).iter().enumerate() {
            let y = top + i32::try_from(row).unwrap_or(0);
            cx.text(text_x, y, line, CellStyle { fg: blend(body_style.fg), bg: None, ..body_style }, body_width);
        }
    }
    if let Some(lift) = lift {
        cx.lift(rect, lift);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{App, Command, Harness};
    use crate::widget::{Length, View};
    use crate::widgets::{Button, SpinnerStyle, Text};

    #[derive(Default)]
    struct Demo {
        undone: u32,
        pressed: u32,
        opened: u32,
    }

    #[derive(Clone)]
    enum Msg {
        Deployed,
        Failed,
        Progress(u32),
        Finish,
        Undo,
        Corner,
        Press,
        Loading(SpinnerStyle),
        Loaded,
        Pressable,
        OpenLog,
    }

    impl App for Demo {
        type Msg = Msg;
        fn update(&mut self, msg: Msg) -> Command<Msg> {
            match msg {
                Msg::Deployed => Command::toast(Toast::success("Deployed api-gateway").body("v2.14.0 is live")),
                Msg::Failed => Command::toast(Toast::danger("Build failed").action("Undo", Msg::Undo)),
                Msg::Progress(n) => Command::toast(Toast::info(format!("Uploading {n}%")).key("upload")),
                Msg::Finish => Command::dismiss_toast("upload"),
                Msg::Undo => {
                    self.undone += 1;
                    Command::none()
                }
                Msg::Corner => Command::toast_corner(Corner::TopLeft),
                Msg::Loading(style) => Command::toast(Toast::info("Uploading backup").icon_motion(style).key("upload")),
                Msg::Loaded => Command::toast(Toast::success("Backup uploaded").key("upload")),
                Msg::Press => {
                    self.pressed += 1;
                    Command::none()
                }
                Msg::Pressable => Command::toast(
                    Toast::success("Deployed api-gateway").action("Undo", Msg::Undo).on_press(Msg::OpenLog),
                ),
                Msg::OpenLog => {
                    self.opened += 1;
                    Command::none()
                }
            }
        }
        fn view(&self, ui: &mut View<'_, Msg>) {
            ui.column(|ui| {
                ui.add(Text::new("dashboard"));
                ui.add(Button::new("Refresh").on_press(Msg::Press)).width(Length::Cells(60));
            });
        }
    }

    #[test]
    fn slides_in_at_the_bottom_right_and_leaves_after_its_duration() {
        let mut h = Harness::new(Demo::default(), 60, 10);
        h.send(Msg::Deployed);
        assert!(!h.screen().contains("Deployed"), "starts beyond the edge: {}", h.screen());
        h.advance(Duration::from_millis(200));
        let screen = h.screen();
        let lines: Vec<&str> = screen.lines().collect();
        assert_eq!(lines[6], "     ✓  Deployed api-gateway                           ×", "{screen}");
        assert_eq!(lines[7], "        v2.14.0 is live");
        let theme = h.env().theme();
        assert_eq!(h.bg(2, 6), theme.color("success"));
        assert_eq!(h.fg(5, 6), theme.color("success"));
        assert_eq!(h.bg(20, 5), theme.color("overlay"));
        h.advance(Duration::from_secs(5));
        h.advance(Duration::from_millis(200));
        assert!(!h.screen().contains("Deployed"), "{}", h.screen());
    }

    #[test]
    fn hovering_pauses_the_countdown() {
        let mut h = Harness::new(Demo::default(), 60, 10);
        h.set_reduced_motion(true).send(Msg::Deployed);
        h.hover(30, 6).advance(Duration::from_secs(10));
        assert!(h.screen().contains("Deployed"));
        h.hover(0, 0).advance(Duration::from_secs(4));
        assert!(h.screen().contains("Deployed"));
        h.advance(Duration::from_secs(2));
        assert!(!h.screen().contains("Deployed"));
    }

    #[test]
    fn the_action_sends_its_message_and_dismisses_without_reaching_below() {
        let mut h = Harness::new(Demo::default(), 60, 10);
        h.set_reduced_motion(true).send(Msg::Failed);
        h.click_text("Undo");
        assert_eq!((h.app().undone, h.app().pressed), (1, 0));
        assert!(!h.screen().contains("Build failed"));
        h.send(Msg::Failed).send(Msg::Deployed);
        let screen = h.screen();
        let failed = h.find("Build failed").expect("stacked");
        let deployed = h.find("Deployed").expect("newest");
        assert!(failed.1 < deployed.1, "newest is nearest the corner: {screen}");
        h.click_text("Undo");
        assert!(!h.screen().contains("Build failed"));
        assert!(h.screen().contains("Deployed"), "only the toast whose action was pressed left");
        assert_eq!(h.app().undone, 2);
    }

    #[test]
    fn a_press_on_the_body_neither_dismisses_nor_reaches_below() {
        let mut h = Harness::new(Demo::default(), 60, 10);
        h.set_reduced_motion(true).send(Msg::Deployed);
        h.click_text("Deployed api-gateway").click_text("v2.14.0").click(3, 6).click(50, 7);
        assert!(h.screen().contains("Deployed api-gateway"), "{}", h.screen());
        assert_eq!((h.app().pressed, h.app().opened), (0, 0), "the presses stayed on the toast");
        h.hover(0, 0).advance(Duration::from_secs(6));
        assert!(!h.screen().contains("Deployed"), "the toast still leaves on its own");
    }

    #[test]
    fn a_pressable_toast_sends_its_message_stays_and_rises_under_the_pointer() {
        let mut h = Harness::new(Demo::default(), 60, 10);
        h.set_reduced_motion(true).send(Msg::Pressable);
        let theme = h.env().theme().clone();
        let (x, y) = h.find("Deployed").expect("toast");
        let (cx, cy) = (u16::try_from(x).unwrap_or(0), u16::try_from(y).unwrap_or(0));
        assert_eq!(h.bg(cx, cy), theme.color("overlay"), "at rest a pressable toast looks like any other");
        h.hover(x, y);
        assert_eq!(h.bg(cx, cy), theme.color("active"), "under the pointer it rises one step");
        let (ux, uy) = h.find("Undo").expect("action");
        let action = theme.style("toast-action", None, &[crate::theme::State::Active]).paint("bg");
        let action = action.map(|paint| paint.at(0.0));
        assert_eq!(h.bg(u16::try_from(ux).unwrap_or(0), u16::try_from(uy).unwrap_or(0)), action);
        assert_ne!(action, theme.color("active"), "the action stays a step above the raised toast");
        let (mx, my) = h.find("×").expect("close mark");
        let (mx, my) = (u16::try_from(mx).unwrap_or(0), u16::try_from(my).unwrap_or(0));
        assert_eq!(h.fg(mx, my), mark_rest(&theme), "the close mark keeps its whisper");

        h.click(x, y).click(x, y);
        assert_eq!((h.app().opened, h.app().pressed, h.app().undone), (2, 0, 0));
        assert!(h.screen().contains("Deployed"), "a press on the body leaves the toast");
        h.click_text("Undo");
        assert_eq!((h.app().opened, h.app().undone), (2, 1), "the action is its own target");
        assert!(!h.screen().contains("Deployed"));

        h.send(Msg::Pressable).click(i32::from(mx), i32::from(my));
        assert!(!h.screen().contains("Deployed"), "the close mark only closes");
        assert_eq!(h.app().opened, 2);
    }

    fn mark_rest(theme: &crate::theme::Theme) -> Option<Rgb> {
        theme.style("close-mark", None, &[]).paint("fg").map(|paint| paint.at(0.0))
    }

    #[test]
    fn the_close_mark_is_the_shared_three_cells_and_dismisses() {
        let mut h = Harness::new(Demo::default(), 60, 10);
        h.set_reduced_motion(true).send(Msg::Deployed);
        let theme = h.env().theme().clone();
        let mark = |states: &[crate::theme::State], key: &str| {
            theme.style("close-mark", None, states).paint(key).map(|paint| paint.at(0.0))
        };
        let rest = mark_rest(&theme);
        let lit = mark(&[crate::theme::State::Hover], "bg");
        assert_eq!(h.screen().lines().nth(6).map(|line| line.chars().nth(55)), Some(Some('×')));
        assert_eq!(h.fg(55, 6), rest, "a whisper while the toast is left alone");
        for (x, y) in [(30, 6), (8, 7), (53, 6), (57, 6), (55, 7)] {
            h.hover(x, y);
            assert_eq!(h.fg(55, 6), rest, "pointing at the toast at {x},{y} leaves the glyph alone");
            assert_eq!([h.bg(54, 6), h.bg(55, 6), h.bg(56, 6)], [theme.color("overlay"); 3], "nothing lit");
        }
        h.hover(56, 6);
        let cells = [h.bg(54, 6), h.bg(55, 6), h.bg(56, 6)];
        assert_eq!(cells, [lit, lit, lit], "the three cells light together, as on tabs");
        assert_ne!(h.fg(55, 6), rest, "and the glyph with them");
        assert_eq!(h.bg(53, 6), theme.color("overlay"));
        assert_eq!(h.bg(57, 6), theme.color("overlay"), "the lit mark keeps one cell of toast after it");
        h.click(53, 6).click(57, 6);
        assert!(h.screen().contains("Deployed"), "the cells beside the mark do not close");
        for x in [54, 55, 56] {
            if x > 54 {
                h.send(Msg::Deployed);
            }
            h.click(x, 6);
            assert!(!h.screen().contains("Deployed"), "a press on mark cell {x} dismisses");
        }
        assert_eq!(h.app().pressed, 0, "the press stays on the toast");
    }

    #[test]
    fn an_action_keeps_a_cell_of_surface_before_the_close_mark() {
        let mut h = Harness::new(Demo::default(), 60, 10);
        h.set_reduced_motion(true).send(Msg::Failed);
        let (x, y) = h.find("Undo").expect("action");
        let line = h.screen().lines().nth(usize::try_from(y).unwrap_or(0)).unwrap_or_default().to_owned();
        assert!(line.ends_with("Undo   ×"), "{line:?}");
        let theme = h.env().theme().clone();
        assert_eq!(h.bg(u16::try_from(x + 5).unwrap_or(0), 7), theme.color("overlay"), "the gap");
    }

    #[test]
    fn an_animated_icon_plays_in_the_kind_colour_and_settles_into_the_kind_icon() {
        let mut h = Harness::new(Demo::default(), 60, 10);
        h.send(Msg::Loading(SpinnerStyle::Dots)).advance(Duration::from_millis(200));
        let theme = h.env().theme().clone();
        let row = |h: &Harness<Demo>| h.screen().lines().nth(7).unwrap_or_default().to_owned();
        let first = row(&h);
        let frame = first.chars().nth(5);
        let dots = h.env().icons().animation(SpinnerStyle::Dots.animation()).expect("built in");
        let frames: Vec<&str> = dots.frames().iter().map(|dots| dots.glyph(h.env().icons().mode())).collect();
        assert!(frame.is_some_and(|frame| frames.contains(&frame.to_string().as_str())), "{first}");
        let text: String = first.chars().skip(8).collect();
        assert_eq!(text, format!("Uploading backup{}×", " ".repeat(31)), "text in its usual column");
        assert_eq!(h.fg(5, 7), theme.color("info"), "the spinner takes the kind's colour");
        h.advance(Duration::from_millis(80));
        assert_ne!(row(&h).chars().nth(5), frame, "it moves");

        h.send(Msg::Loaded).advance(Duration::from_millis(10));
        let done = row(&h);
        assert_eq!(done.chars().nth(5), Some('✓'), "the keyed toast settles into its icon: {done}");
        assert_eq!(done.chars().nth(8), Some('B'), "the text did not move");
        assert_eq!(h.fg(5, 7), theme.color("success"));
    }

    #[test]
    fn an_animated_icon_blends_in_with_the_toast_and_reduced_motion_stands_still() {
        let mut h = Harness::new(Demo::default(), 60, 10);
        h.send(Msg::Loading(SpinnerStyle::Dots)).advance(Duration::from_millis(1));
        h.advance(Duration::from_millis(60));
        let (x, y) = h.find("Uploading").expect("mid-slide");
        let (x, y) = (u16::try_from(x - 3).unwrap_or(0), u16::try_from(y).unwrap_or(0));
        let theme = h.env().theme().clone();
        let (fg, info, overlay) = (h.fg(x, y), theme.color("info"), theme.color("overlay"));
        assert_ne!(fg, info, "mid-slide the spinner has not reached its colour yet");
        assert_ne!(fg, overlay, "but it is on its way");
        let title_full = theme.style("toast-title", None, &[]).paint("fg").map(|paint| paint.at(0.0));
        assert_ne!(h.fg(x + 3, y), title_full, "arriving with the title, which blends the same way");

        let mut still = Harness::new(Demo::default(), 60, 10);
        still.set_reduced_motion(true).send(Msg::Loading(SpinnerStyle::Pulse));
        let line = still.screen().lines().nth(7).unwrap_or_default().to_owned();
        assert_eq!(line.chars().nth(5), Some('ℹ'), "reduced motion shows the kind's icon: {line}");
        let before = still.screen();
        still.advance(Duration::from_millis(900));
        assert_eq!(still.screen(), before);
        assert_eq!(still.fg(5, 7), theme.color("info"));
    }

    #[test]
    fn keyed_toasts_update_in_place_and_are_dismissed_by_key() {
        let mut h = Harness::new(Demo::default(), 60, 10);
        h.set_reduced_motion(true).send(Msg::Progress(10)).send(Msg::Progress(60));
        let screen = h.screen();
        assert!(screen.contains("Uploading 60%") && !screen.contains("Uploading 10%"), "{screen}");
        h.send(Msg::Finish);
        assert!(!h.screen().contains("Uploading"));
    }

    #[test]
    fn corner_can_move_to_the_top_left() {
        let mut h = Harness::new(Demo::default(), 60, 10);
        h.set_reduced_motion(true).send(Msg::Corner).send(Msg::Deployed);
        assert_eq!(h.find("Deployed"), Some((8, 2)));
    }

    /// Toasts with an action and a long message, in English or German.
    struct Undoable {
        german: bool,
    }

    impl App for Undoable {
        type Msg = ();
        fn update(&mut self, (): ()) -> Command<()> {
            let (message, action) = if self.german {
                ("Rust: Sitzung in den Papierkorb verschoben", "Rückgängig")
            } else {
                ("Rust: session moved to the trash", "Undo")
            };
            Command::toast(Toast::success(message).action(action, ()))
        }
        fn view(&self, ui: &mut View<'_, ()>) {
            ui.add(Text::new("records"));
        }
    }

    fn undoable(german: bool) -> Harness<Undoable> {
        let mut h = Harness::new(Undoable { german }, 40, 12);
        h.set_locale(if german { "de" } else { "en" }).set_reduced_motion(true).send(());
        h
    }

    #[test]
    fn at_forty_columns_a_long_message_wraps_and_the_action_stays_on_the_first_row() {
        for german in [false, true] {
            let h = undoable(german);
            let screen = h.screen();
            assert!(!screen.contains('…'), "{screen}");
            let action = if german { "Rückgängig" } else { "Undo" };
            let (_, action_row) = h.find(action).unwrap_or_else(|| panic!("{screen}"));
            let (_, title_row) = h.find("Rust:").unwrap_or_else(|| panic!("{screen}"));
            assert_eq!(action_row, title_row, "the action is on the first row: {screen}");
            let second = screen.lines().nth(usize::try_from(title_row + 1).unwrap_or(0)).unwrap_or_default();
            assert!(second.contains("trash") || second.contains("Papierkorb"), "{screen}");
            assert!(screen.contains('×'), "{screen}");
        }
    }

    #[test]
    fn a_wrapped_toast_still_keeps_clear_of_a_modal() {
        struct Covered;
        impl App for Covered {
            type Msg = ();
            fn update(&mut self, (): ()) -> Command<()> {
                Command::toast(Toast::success("Rust: session moved to the trash").action("Undo", ()))
            }
            fn view(&self, ui: &mut View<'_, ()>) {
                ui.add_with(crate::widgets::Modal::new().title("Open"), |ui| {
                    ui.add(Text::new("Body"));
                });
            }
        }
        let mut h = Harness::new(Covered, 40, 12);
        h.set_reduced_motion(true).send(());
        let screen = h.screen();
        let (_, title) = h.find("Open").unwrap_or_else(|| panic!("{screen}"));
        let (_, body) = h.find("Body").unwrap_or_else(|| panic!("{screen}"));
        assert!(title < body, "{screen}");
        if let Some((_, undo)) = h.find("Undo") {
            let modal_bottom = body + 2;
            assert!(undo > modal_bottom, "a toast on screen sits below the dialog: {screen}");
        }
    }
}
