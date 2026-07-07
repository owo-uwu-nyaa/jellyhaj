use std::{
    cmp::max,
    fmt::Debug,
};

use color_eyre::Result;
use jellyhaj_widgets_core::{
    Buffer, JellyhajWidget, JellyhajWidgetBase, KeyModifiers, MouseEventKind, Position, Rect, Size,
    WidgetContext, WidgetTreeVisitor, Wrapper,
    ratatui::style::Modifier,
    valuable::{Fields, NamedField, NamedValues, StructDef, Structable, Valuable, Value},
};

pub use jellyhaj_tabs_widget_macro::TabContainer;

pub struct TabbedWidgets<T> {
    inner: T,
    current: usize,
}

impl<T> TabbedWidgets<T> {
    pub const fn new(inner: T) -> Self {
        Self { inner, current: 0 }
    }
}

pub trait Tabbed: Send + 'static {
    type Action: Debug + Send + 'static;
    type ActionResult: Debug;

    const TABS: &[&str];

    fn is_next(action: &Self::Action) -> bool;
    fn is_prev(action: &Self::Action) -> bool;

    fn visit_children(&self, visitor: &mut impl WidgetTreeVisitor);
}

impl<T: Tabbed> Valuable for TabbedWidgets<T> {
    fn as_value(&self) -> Value<'_> {
        Value::Structable(self)
    }

    fn visit(&self, visit: &mut dyn jellyhaj_widgets_core::valuable::Visit) {
        visit.visit_named_fields(&NamedValues::new(
            WIDGETS_FIELDS,
            &[T::TABS.as_value(), self.current.as_value()],
        ));
    }
}

static WIDGETS_FIELDS: &[NamedField] = &[NamedField::new("tabs"), NamedField::new("current")];
impl<T: Tabbed> Structable for TabbedWidgets<T> {
    fn definition(&self) -> StructDef<'_> {
        StructDef::new_static("TabbedWidgets", Fields::Named(WIDGETS_FIELDS))
    }
}

pub trait TabContainer<R: 'static>: Tabbed {
    fn init(&mut self, cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>);

    fn min_width(&self, current: usize) -> Option<u16>;
    fn min_height(&self, current: usize) -> Option<u16>;

    fn accepts_text_input(&self, current: usize) -> bool;
    fn accept_char(&mut self, text: char, current: usize);
    fn accept_text(&mut self, text: String, current: usize);

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
        current: usize,
    ) -> Result<Option<Self::ActionResult>>;

    fn click(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        position: Position,
        size: Size,
        kind: MouseEventKind,
        modifier: KeyModifiers,
        current: usize,
    ) -> Result<Option<Self::ActionResult>>;

    fn render_fallible(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        current: usize,
    ) -> Result<()>;
}

#[allow(clippy::cast_possible_truncation)]
const fn calc_min<T: Tabbed>() -> u16 {
    let mut width = T::TABS.len() * 3 + 1;
    let mut i = 0;
    while T::TABS.len() < i {
        width += T::TABS[i].len();
        i += 1;
    }
    assert!(
        width <= (u16::MAX as usize),
        "calculating width overflowed u16"
    );
    width as u16
}

impl<T: Tabbed> JellyhajWidgetBase for TabbedWidgets<T> {
    type Action = T::Action;

    type ActionResult = T::ActionResult;

    const NAME: &str = "tabbed";

    fn visit_children(&self, visitor: &mut impl WidgetTreeVisitor) {
        self.inner.visit_children(visitor);
    }
}

impl<R: 'static, T: TabContainer<R>> JellyhajWidget<R> for TabbedWidgets<T> {
    fn init(&mut self, cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {
        self.inner.init(cx);
    }

    fn min_width(&self) -> Option<u16> {
        let min = calc_min::<T>();
        if let Some(inner_min) = self.inner.min_width(self.current) {
            Some(max(min, inner_min))
        } else {
            Some(min)
        }
    }

    fn min_height(&self) -> Option<u16> {
        if let Some(min) = self.inner.min_height(self.current) {
            Some(min + 2)
        } else {
            Some(2)
        }
    }

    fn accepts_text_input(&self) -> bool {
        self.inner.accepts_text_input(self.current)
    }

    fn accept_char(&mut self, text: char) {
        self.inner.accept_char(text, self.current);
    }

    fn accept_text(&mut self, text: String) {
        self.inner.accept_text(text, self.current);
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        if T::is_next(&action) {
            self.current = (self.current + 1) % T::TABS.len();
            Ok(None)
        } else if T::is_prev(&action) {
            self.current = if self.current == 0 {
                T::TABS.len().saturating_sub(1)
            } else {
                self.current.saturating_sub(1)
            };
            Ok(None)
        } else {
            self.inner.apply_action(cx, action, self.current)
        }
    }

    fn click(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        mut position: Position,
        mut size: Size,
        kind: MouseEventKind,
        modifier: KeyModifiers,
    ) -> Result<Option<Self::ActionResult>> {
        if position.y == 0 {
            let mut x = position.x;
            for (i, l) in T::TABS.iter().enumerate() {
                if x == 0 {
                    return Ok(None);
                }
                x -= 1;
                let len = u16::try_from(l.len() + 2).expect("u16 overflow");
                if x < len {
                    self.current = i;
                    return Ok(None);
                }
                x = x.strict_sub(len);
            }
            Ok(None)
        } else if position.y == 1 {
            Ok(None)
        } else {
            position.y = position.y.strict_sub(2);
            size.height = size.height.strict_sub(2);
            self.inner
                .click(cx, position, size, kind, modifier, self.current)
        }
    }

    fn render_fallible_inner(
        &mut self,
        mut area: Rect,
        buf: &mut Buffer,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> Result<()> {
        buf[area.as_position()].set_char(if self.current == 0 { '╻' } else { '╷' });
        buf[Position {
            x: area.x,
            y: area.y + 1,
        }]
        .set_char(if self.current == 0 { '┗' } else { '└' });
        let mut x = area.x + 1;
        for (i, label) in T::TABS.iter().enumerate() {
            let is_current = self.current == i;
            buf.set_string(
                x + 1,
                area.y,
                label,
                if is_current {
                    Modifier::REVERSED
                } else {
                    Modifier::empty()
                },
            );
            for p in (Rect {
                x,
                y: area.y + 1,
                width: u16::try_from(label.len())
                    .expect("label is to large for u16")
                    .strict_add(2),
                height: 1,
            }
            .positions())
            {
                buf[p].set_char(if is_current { '━' } else { '─' });
            }
            x = x
                .strict_add(u16::try_from(label.len()).expect("label to large for u16"))
                .strict_add(2);
            let last = i + 1 == T::TABS.len();
            let is_next = self.current == i + 1;
            buf[Position { x, y: area.y }].set_char(if is_current || is_next {
                '╻'
            } else {
                '╷'
            });
            buf[Position {
                x,
                y: area.y +1,
            }]
            .set_char(if last {
                if is_current { '┛' } else { '┘' }
            } else if is_current {
                '┹'
            } else if self.current == i + 1 {
                '┺'
            } else {
                '┴'
            });
        }
        area.height = area.height.strict_sub(2);
        area.y += 2;
        self.inner.render_fallible(area, buf, cx, self.current)
    }
}

#[doc(hidden)]
pub mod macro_exports {
    pub use super::{TabContainer, Tabbed};
    pub use color_eyre::Result;
    pub use jellyhaj_widgets_core::{
        Buffer, JellyhajWidget, JellyhajWidgetBase, JellyhajWidgetExt, KeyModifiers,
        MouseEventKind, Position, Rect, Size, WidgetContext, WidgetTreeVisitor, Wrapper,
    };
    pub use std::{
        convert::{Infallible, Into},
        fmt::Debug,
        matches,
        option::Option::{self, None, Some},
        primitive::{bool, char, u16, usize},
        result::Result::{Err, Ok},
        string::String,
        unreachable,
    };
}
