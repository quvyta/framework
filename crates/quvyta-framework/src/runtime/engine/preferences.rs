//! Following a member's preferences: telling the application at the start, and applying what
//! another application changed in the ecosystem's files while this one runs.

use super::Engine;
use crate::i18n;
use crate::runtime::App;
use crate::runtime::follow::{Follow, Heard};
use crate::storage::Preferences;

impl<A: App> Engine<A> {
    /// Follows the member's files from now on, starting from what the environment was set up
    /// with.
    pub(crate) fn follow(&mut self, follow: Follow) {
        self.follow = Some(follow);
    }

    /// Tells [`App::preferences`] what the application started with, once, before the first
    /// frame is built.
    pub(super) fn tell_preferences(&mut self) {
        let Some(follow) = self.follow.as_mut().filter(|follow| !follow.told) else {
            return;
        };
        follow.told = true;
        let preferences = follow.heard.preferences.clone();
        self.hear_preferences(&preferences);
    }

    /// Reads the member's files again when the watch heard them change; the terminal loop asks on
    /// every pass.
    pub(crate) fn follow_preferences(&mut self) {
        if self.follow.as_ref().is_some_and(Follow::take_change) {
            self.check_preferences();
        }
    }

    /// Reads the member's files again and applies what changed since they were read last:
    /// language, theme, icons and reduced motion as the ecosystem resolves them, the pillar from
    /// the application's own file. A value is applied only when the files now say something
    /// else than before and the screen does not already show it, so reading a file this
    /// application just wrote itself changes nothing, and nothing here writes a file.
    /// [`App::preferences`] hears the new preferences whenever they differ from what it heard
    /// last, and only then is a frame asked for.
    pub(crate) fn check_preferences(&mut self) {
        let Some(follow) = self.follow.as_mut() else {
            return;
        };
        let Some(now) = follow.read(self.env.i18n()) else {
            return;
        };
        let before = std::mem::replace(&mut follow.heard, now.clone());
        let applied = self.apply_heard(&before, &now);
        let told = !same(&before.preferences, &now.preferences);
        if told {
            self.hear_preferences(&now.preferences);
        }
        if applied || told {
            self.dirty = true;
        }
    }

    /// Switches the environment to each value of `now` that differs from `before` and from what
    /// is shown. Answers whether anything was switched.
    fn apply_heard(&mut self, before: &Heard, now: &Heard) -> bool {
        let env = &mut self.env;
        let mut applied = false;
        let theme = &now.preferences.theme().value;
        if *theme != before.preferences.theme().value && theme != env.theme().id() {
            env.set_theme(theme);
            applied = true;
        }
        let language = &now.preferences.language().value;
        if *language != before.preferences.language().value && language != env.i18n().active() {
            env.set_locale(language);
            applied = true;
        }
        let icons = now.preferences.icons().value;
        if icons != before.preferences.icons().value && icons != env.icon_mode() {
            env.set_icon_mode(icons);
            applied = true;
        }
        let reduced = now.preferences.reduced_motion().value;
        if reduced != before.preferences.reduced_motion().value
            && reduced != env.reduced_motion()
            && !env.reduced_motion_forced()
        {
            env.set_reduced_motion(reduced);
            applied = true;
        }
        if let Some(style) = now.pillar
            && now.pillar != before.pillar
            && Some(style) != env.pillar_style()
        {
            env.set_pillar_style(style);
            applied = true;
        }
        applied
    }

    /// Hands `preferences` to [`App::preferences`] and applies its answer.
    fn hear_preferences(&mut self, preferences: &Preferences) {
        let message = {
            let app = &self.app;
            i18n::scope(self.env.i18n_arc(), || app.preferences(preferences))
        };
        if let Some(message) = message {
            self.update(message);
        }
    }
}

/// Whether two resolutions say the same: the same values from the same places. What was found
/// wrong on the way is not compared; it is read again with every resolution.
fn same(a: &Preferences, b: &Preferences) -> bool {
    a.language() == b.language()
        && a.theme() == b.theme()
        && a.icons() == b.icons()
        && a.reduced_motion() == b.reduced_motion()
        && a.update_notice() == b.update_notice()
}
