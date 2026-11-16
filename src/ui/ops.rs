//! Questions to the widget tree about keyboard focus. The pad hotkeys are ordinary letters, so they must hold back
//! while the user types into a field of this window.
use iced::advanced::widget::operation::{Focusable, Outcome};
use iced::advanced::widget::{operate, Id, Operation};
use iced::{Rectangle, Task};

struct IsFocused(bool);

impl Operation<bool> for IsFocused {
    fn container(&mut self, _id: Option<&Id>, _bounds: Rectangle, operate_on_children: &mut dyn FnMut(&mut dyn Operation<bool>)) {
        operate_on_children(self);
    }

    fn focusable(&mut self, state: &mut dyn Focusable, _id: Option<&Id>) {
        self.0 |= state.is_focused();
    }

    fn finish(&self) -> Outcome<bool> {
        Outcome::Some(self.0)
    }
}

/// Whether a text field has the keyboard.
pub fn typing() -> Task<bool> {
    operate(IsFocused(false))
}

struct Unfocus;

impl Operation<()> for Unfocus {
    fn container(&mut self, _id: Option<&Id>, _bounds: Rectangle, operate_on_children: &mut dyn FnMut(&mut dyn Operation<()>)) {
        operate_on_children(self);
    }

    fn focusable(&mut self, state: &mut dyn Focusable, _id: Option<&Id>) {
        state.unfocus();
    }
}

/// Takes the keyboard away from every field.
pub fn unfocus() -> Task<()> {
    operate(Unfocus)
}
