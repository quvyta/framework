//! Per-widget state kept by the runtime between frames.

use super::id::{IdMap, WidgetId};
use std::any::{Any, TypeId};

struct Entry {
    type_id: TypeId,
    value: Box<dyn Any>,
}

struct Slot {
    entries: Vec<Entry>,
    seen: u64,
    persistent: bool,
}

/// State stored per widget id and type: cursors, scroll offsets, open popovers, running
/// animations. A widget can keep several types of state side by side.
///
/// State of widgets that were not painted in a frame is dropped at the end of that frame,
/// unless it belongs to a persistent scope such as a router page.
#[derive(Default)]
pub(crate) struct Memory {
    slots: IdMap<WidgetId, Slot>,
    frame: u64,
}

impl Memory {
    /// The state of type `T` for `id`, created with `T::default()` when missing.
    pub(crate) fn get<T: Default + 'static>(&mut self, id: WidgetId, persistent: bool) -> &mut T {
        let frame = self.frame;
        let slot = self.slots.entry(id).or_insert_with(|| Slot { entries: Vec::new(), seen: frame, persistent });
        slot.seen = frame;
        slot.persistent |= persistent;
        let type_id = TypeId::of::<T>();
        let index = match slot.entries.iter().position(|entry| entry.type_id == type_id) {
            Some(index) => index,
            None => {
                slot.entries.push(Entry { type_id, value: Box::new(T::default()) });
                slot.entries.len() - 1
            }
        };
        slot.entries[index].value.downcast_mut::<T>().expect("entries are stored under their own type id")
    }

    /// Reads state without creating it.
    #[cfg(test)]
    pub(crate) fn peek<T: 'static>(&self, id: WidgetId) -> Option<&T> {
        self.slots.get(&id)?.entries.iter().find_map(|entry| entry.value.downcast_ref::<T>())
    }

    /// Marks `id` as painted in the current frame.
    pub(crate) fn touch(&mut self, id: WidgetId, persistent: bool) {
        if let Some(slot) = self.slots.get_mut(&id) {
            slot.seen = self.frame;
            slot.persistent |= persistent;
        }
    }

    /// Starts a new frame.
    pub(crate) fn begin_frame(&mut self) {
        self.frame += 1;
    }

    /// Drops state of widgets not painted this frame, keeping persistent entries.
    pub(crate) fn end_frame(&mut self) {
        let frame = self.frame;
        self.slots.retain(|_, slot| slot.persistent || slot.seen == frame);
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.slots.len()
    }
}

#[cfg(test)]
mod tests {
    use super::super::id::Key;
    use super::*;

    #[test]
    fn keeps_state_of_painted_and_persistent_widgets_only() {
        let a = WidgetId::ROOT.child(&Key::Index(0), "A");
        let b = WidgetId::ROOT.child(&Key::Index(1), "B");
        let mut memory = Memory::default();
        memory.begin_frame();
        *memory.get::<u32>(a, false) = 7;
        *memory.get::<u32>(b, true) = 9;
        memory.end_frame();

        memory.begin_frame();
        memory.touch(a, false);
        memory.end_frame();
        assert_eq!(memory.peek::<u32>(a), Some(&7));

        memory.begin_frame();
        memory.end_frame();
        assert_eq!(memory.peek::<u32>(a), None);
        assert_eq!(memory.peek::<u32>(b), Some(&9));
        assert_eq!(memory.len(), 1);
    }

    #[test]
    fn different_types_live_side_by_side() {
        let a = WidgetId::ROOT.child(&Key::Index(0), "A");
        let mut memory = Memory::default();
        *memory.get::<u32>(a, false) = 7;
        memory.get::<String>(a, false).push_str("hi");
        assert_eq!(*memory.get::<u32>(a, false), 7);
        assert_eq!(memory.get::<String>(a, false), "hi");
    }
}
