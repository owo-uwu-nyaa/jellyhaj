use crate::{FormAction, FormItem};

pub trait WithCurrent<T> {
    type R;
    fn process<I: FormItem<T>>(self, val: &I, sel: I::SelectionInner) -> Self::R;
}

pub trait WithCurrentMut<T> {
    type R;
    fn process<I: FormItem<T>>(self, val: &mut I, sel: &mut I::SelectionInner) -> Self::R;
}

pub struct AcceptsTextInput;

impl<T> WithCurrent<T> for AcceptsTextInput {
    type R = bool;

    fn process<I: FormItem<T>>(self, val: &I, sel: <I as FormItem<T>>::SelectionInner) -> Self::R {
        val.accepts_text_input(sel)
    }
}

pub struct ApplyChar(pub char);

impl<T> WithCurrentMut<T> for ApplyChar {
    type R = ();

    fn process<I: FormItem<T>>(self, val: &mut I, sel: &mut <I as FormItem<T>>::SelectionInner) -> Self::R {
        val.apply_char(sel, self.0);
    }
}

pub struct ApplyText(pub String);

impl<T> WithCurrentMut<T> for ApplyText {
    type R = ();

    fn process<I: FormItem<T>>(self, val: &mut I, sel: &mut <I as FormItem<T>>::SelectionInner) -> Self::R {
        val.apply_text(sel, self.0);
    }
}

pub struct AcceptsMovementAction;

impl<T> WithCurrent<T> for AcceptsMovementAction {
    type R = bool;

    fn process<I: FormItem<T>>(self, val: &I, sel: <I as FormItem<T>>::SelectionInner) -> Self::R {
        val.accepts_movement_action(sel)
    }
}

pub struct ApplyAction(pub FormAction);

impl<T> WithCurrentMut<T> for ApplyAction {
    type R = color_eyre::Result<Option<T>>;

    fn process<I: FormItem<T>>(self, val: &mut I, sel: &mut <I as FormItem<T>>::SelectionInner) -> Self::R {
        val.apply_action(sel, self.0)
    }
}
