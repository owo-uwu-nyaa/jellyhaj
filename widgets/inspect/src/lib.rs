use std::convert::Infallible;

use jellyhaj_core::render::{StateStack, StateValue};
use jellyhaj_widgets_core::{
    ContextRef, JellyhajWidget, MouseEventKind, TreeVisitor,
    ratatui::{crossterm::event::MouseButton, style::Modifier, widgets::StatefulWidget},
    spawn::tracing::info_span,
};
use tokio::sync::oneshot::{Receiver, channel};
use tui_tree_widget::{Block, Tree, TreeItem, TreeState};
use valuable::{Fields, NamedValues, StructDef, Structable, Valuable, Value};

type Id = (usize, usize);

type IdTreeItem = TreeItem<'static, Id>;

fn inspect_valuable(
    mut name: String,
    view_id: usize,
    id_gen: &mut usize,
    val: Value<'_>,
) -> IdTreeItem {
    let id = *id_gen;
    *id_gen += 1;
    match val {
        Value::Bool(v) => {
            name.push_str(if v { "true" } else { "false" });
            TreeItem::new_leaf((view_id, id), name)
        }
        Value::Char(c) => {
            name.push('\'');
            name.push(c);
            name.push('\'');
            TreeItem::new_leaf((view_id, id), name)
        }
        Value::F32(f) => {
            name += &f.to_string();
            TreeItem::new_leaf((view_id, id), name)
        }
        Value::F64(f) => {
            name += &f.to_string();
            TreeItem::new_leaf((view_id, id), name)
        }
        Value::I8(i) => {
            name += &i.to_string();
            TreeItem::new_leaf((view_id, id), name)
        }
        Value::I16(i) => {
            name += &i.to_string();
            TreeItem::new_leaf((view_id, id), name)
        }
        Value::I32(i) => {
            name += &i.to_string();
            TreeItem::new_leaf((view_id, id), name)
        }
        Value::I64(i) => {
            name += &i.to_string();
            TreeItem::new_leaf((view_id, id), name)
        }
        Value::I128(i) => {
            name += &i.to_string();
            TreeItem::new_leaf((view_id, id), name)
        }
        Value::Isize(i) => {
            name += &i.to_string();
            TreeItem::new_leaf((view_id, id), name)
        }
        Value::String(s) => {
            name.push('"');
            name.push_str(s);
            name.push('"');
            TreeItem::new_leaf((view_id, id), name)
        }
        Value::U8(i) => {
            name += &i.to_string();
            TreeItem::new_leaf((view_id, id), name)
        }
        Value::U16(i) => {
            name += &i.to_string();
            TreeItem::new_leaf((view_id, id), name)
        }
        Value::U32(i) => {
            name += &i.to_string();
            TreeItem::new_leaf((view_id, id), name)
        }
        Value::U64(i) => {
            name += &i.to_string();
            TreeItem::new_leaf((view_id, id), name)
        }
        Value::U128(i) => {
            name += &i.to_string();
            TreeItem::new_leaf((view_id, id), name)
        }
        Value::Usize(i) => {
            name += &i.to_string();
            TreeItem::new_leaf((view_id, id), name)
        }
        Value::Path(path) => {
            name += &path.display().to_string();
            TreeItem::new_leaf((view_id, id), name)
        }
        Value::Error(_) => {
            name += "Error";
            TreeItem::new_leaf((view_id, id), name)
        }
        Value::Listable(listable) => {
            name += "[]";
            let mut visitor = ListVisitor {
                index: 0,
                view_id,
                id_gen,
                values: Vec::new(),
            };
            listable.visit(&mut visitor);
            TreeItem::new((view_id, id), name, visitor.values).expect("should always be unique")
        }
        Value::Structable(structable) => {
            name += "struct ";
            name += structable.definition().name();
            let mut visitor = StructVisitor {
                index: 0,
                view_id,
                id_gen,
                values: Vec::new(),
            };
            structable.visit(&mut visitor);
            TreeItem::new((view_id, id), name, visitor.values).expect("should always be unique")
        }
        Value::Tuplable(tuplable) => {
            name += "(...)";
            let mut visitor = StructVisitor {
                index: 0,
                view_id,
                id_gen,
                values: Vec::new(),
            };
            tuplable.visit(&mut visitor);
            TreeItem::new((view_id, id), name, visitor.values).expect("should always be unique")
        }
        Value::Enumerable(enumerable) => {
            name += "enum ";
            name += enumerable.definition().name();
            name += "::";
            name += enumerable.variant().name();
            let mut visitor = StructVisitor {
                index: 0,
                view_id,
                id_gen,
                values: Vec::new(),
            };
            enumerable.visit(&mut visitor);
            TreeItem::new((view_id, id), name, visitor.values).expect("should always be unique")
        }
        Value::Mappable(mappable) => {
            name += "{...}";
            let mut visitor = MapVisitor {
                index: 0,
                view_id,
                id_gen,
                values: Vec::new(),
            };
            mappable.visit(&mut visitor);
            TreeItem::new((view_id, id), name, visitor.values).expect("should always be unique")
        }
        Value::Unit => {
            name += "()";
            TreeItem::new_leaf((view_id, id), name)
        }
        _ => {
            name += "Unknown";
            TreeItem::new_leaf((view_id, id), name)
        }
    }
}

struct ListVisitor<'g> {
    index: usize,
    view_id: usize,
    id_gen: &'g mut usize,
    values: Vec<IdTreeItem>,
}

impl valuable::Visit for ListVisitor<'_> {
    fn visit_value(&mut self, value: Value<'_>) {
        let index = self.index;
        self.index += 1;
        let prefix = format!("[{index}]: ");
        self.values
            .push(inspect_valuable(prefix, self.view_id, self.id_gen, value));
    }
}

struct StructVisitor<'g> {
    index: usize,
    view_id: usize,
    id_gen: &'g mut usize,
    values: Vec<IdTreeItem>,
}

impl valuable::Visit for StructVisitor<'_> {
    fn visit_value(&mut self, value: Value<'_>) {
        match value {
            Value::Structable(s) => s.visit(self),
            Value::Tuplable(t) => t.visit(self),
            Value::Enumerable(e) => e.visit(self),
            _ => {}
        }
    }
    fn visit_named_fields(&mut self, named_values: &valuable::NamedValues<'_>) {
        for (field, value) in named_values.iter() {
            let mut prefix = field.name().to_string();
            prefix += ": ";
            self.values
                .push(inspect_valuable(prefix, self.view_id, self.id_gen, *value));
        }
    }
    fn visit_unnamed_fields(&mut self, values: &[Value<'_>]) {
        for value in values {
            let index = self.index;
            self.index += 1;
            let prefix = format!("({index}): ");
            self.values
                .push(inspect_valuable(prefix, self.view_id, self.id_gen, *value));
        }
    }
}

struct MapVisitor<'g> {
    index: usize,
    view_id: usize,
    id_gen: &'g mut usize,
    values: Vec<IdTreeItem>,
}

impl valuable::Visit for MapVisitor<'_> {
    fn visit_value(&mut self, value: Value<'_>) {
        if let Value::Mappable(m) = value {
            m.visit(self)
        }
    }

    fn visit_entry(&mut self, key: Value<'_>, value: Value<'_>) {
        let index = self.index;
        self.index += 1;
        let id = *self.id_gen;
        *self.id_gen += 1;
        let childs = vec![
            inspect_valuable("key: ".to_string(), self.view_id, self.id_gen, key),
            inspect_valuable("value: ".to_string(), self.view_id, self.id_gen, value),
        ];
        self.values.push(
            TreeItem::new((self.view_id, id), format!("{{{index}}}"), childs)
                .expect("should always be unique"),
        );
    }
}

struct WidgetVisitor<'g> {
    view_id: usize,
    id_gen: &'g mut usize,
    values: Vec<IdTreeItem>,
}

impl TreeVisitor for WidgetVisitor<'_> {
    fn enter(
        &mut self,
        name: &'static str,
        state: &dyn valuable::Valuable,
        visit_children: &dyn Fn(&mut dyn TreeVisitor),
    ) {
        let id = *self.id_gen;
        *self.id_gen += 1;
        let values = vec![inspect_valuable(
            "Widget state: ".to_string(),
            self.view_id,
            self.id_gen,
            state.as_value(),
        )];
        let mut visitor = WidgetVisitor {
            values,
            view_id: self.view_id,
            id_gen: self.id_gen,
        };
        visit_children(&mut visitor);
        let name = "Widget ".to_owned() + name;
        self.values.push(
            TreeItem::new((self.view_id, id), name, visitor.values)
                .expect("should always be unique"),
        );
    }
}

type ViewInfo = (String, Id, Option<Receiver<Vec<IdTreeItem>>>);

fn inspect_state_value(val: &StateValue, view_id: usize) -> Option<ViewInfo> {
    match val {
        StateValue::Suspended(suspended_inner) => {
            let (send, recv) = channel();
            let _ = suspended_inner.send_visitor.send(Box::new(move |f| {
                let mut id_gen = 1;
                let mut visitor = WidgetVisitor {
                    values: Vec::new(),
                    view_id,
                    id_gen: &mut id_gen,
                };
                f(&mut visitor);
                let _ = send.send(visitor.values);
            }));
            Some((
                format!("Widget {}", suspended_inner.name),
                (view_id, 0),
                Some(recv),
            ))
        }
        StateValue::Empty => None,
        StateValue::WithoutTui(_pin) => Some(("Without Tui".to_owned(), (view_id, 0), None)),
    }
}

fn inspect_state_inner(state: &StateStack) -> Vec<ViewInfo> {
    let mut res = Vec::new();
    let mut view_id = 0;
    state.visit(|state| {
        let id = view_id;
        view_id += 1;
        if let Some(v) = inspect_state_value(state, id) {
            res.push(v);
        }
    });
    res
}

async fn collect_tree_items(items: Vec<ViewInfo>) -> Vec<IdTreeItem> {
    let mut res = Vec::with_capacity(items.len());
    for (name, id, children) in items {
        res.push(if let Some(children) = children {
            if let Ok(children) = children.await {
                TreeItem::new(id, name, children).expect("there should be no duplicates")
            } else {
                let child = TreeItem::new_leaf((id.0, id.1 + 1), "Inspecting view failed");
                TreeItem::new(id, name, vec![child]).expect("there should be no duplicates")
            }
        } else {
            TreeItem::new_leaf(id, name)
        })
    }
    res
}

pub fn inspect_state(state: &StateStack) -> impl Future<Output = Vec<IdTreeItem>> + Send + 'static {
    collect_tree_items(inspect_state_inner(state))
}

#[derive(Default)]
pub struct InspectWidget {
    items: Vec<IdTreeItem>,
    state: TreeState<Id>,
}

impl Valuable for InspectWidget {
    fn as_value(&self) -> Value<'_> {
        Value::Structable(self)
    }

    fn visit(&self, visit: &mut dyn valuable::Visit) {
        visit.visit_named_fields(&NamedValues::new(&[], &[]))
    }
}
impl Structable for InspectWidget {
    fn definition(&self) -> StructDef<'_> {
        StructDef::new_static("InspectWidget", Fields::Named(&[]))
    }
}

#[derive(Debug)]
pub enum InspectAction {
    Content(Vec<IdTreeItem>),
    Toggle,
    Open,
    CloseMoveParent,
    Close,
    Up,
    Down,
}

impl<R: ContextRef<StateStack> + 'static> JellyhajWidget<R> for InspectWidget {
    type Action = InspectAction;

    type ActionResult = Infallible;

    const NAME: &str = "inspect";

    fn visit_children(&self, _visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {}

    fn init(
        &mut self,
        cx: jellyhaj_widgets_core::WidgetContext<
            '_,
            Self::Action,
            impl jellyhaj_widgets_core::Wrapper<Self::Action>,
            R,
        >,
    ) {
        let f = inspect_state(cx.refs.as_ref());
        cx.submitter
            .wrap_with(InspectAction::Content)
            .spawn_task_infallible(f, info_span!("collect-inspect"), "collect-inspect");
    }

    fn min_width(&self) -> Option<u16> {
        Some(5)
    }

    fn min_height(&self) -> Option<u16> {
        Some(3)
    }

    fn accepts_text_input(&self) -> bool {
        false
    }

    fn accept_char(&mut self, _: char) {}

    fn accept_text(&mut self, _: String) {}

    fn apply_action(
        &mut self,
        _cx: jellyhaj_widgets_core::WidgetContext<
            '_,
            Self::Action,
            impl jellyhaj_widgets_core::Wrapper<Self::Action>,
            R,
        >,
        action: Self::Action,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        match action {
            InspectAction::Content(tree_items) => {
                self.items = tree_items;
                self.state = TreeState::default();
                self.state.select_first();
            }
            InspectAction::Toggle => {
                self.state.toggle_selected();
            }
            InspectAction::Open => {
                self.state.key_right();
            }
            InspectAction::CloseMoveParent => {
                self.state.key_left();
            }
            InspectAction::Close => {
                let selection = self.state.selected().to_vec();
                self.state.close(&selection);
            }
            InspectAction::Up => {
                self.state.key_up();
            }
            InspectAction::Down => {
                self.state.key_down();
            }
        }
        Ok(None)
    }

    fn click(
        &mut self,
        _cx: jellyhaj_widgets_core::WidgetContext<
            '_,
            Self::Action,
            impl jellyhaj_widgets_core::Wrapper<Self::Action>,
            R,
        >,
        _position: jellyhaj_widgets_core::Position,
        _size: jellyhaj_widgets_core::Size,
        kind: jellyhaj_widgets_core::MouseEventKind,
        _modifier: jellyhaj_widgets_core::KeyModifiers,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        if let MouseEventKind::Down(MouseButton::Left) = kind {
            //TODO implement click
        }
        Ok(None)
    }

    fn render_fallible_inner(
        &mut self,
        area: jellyhaj_widgets_core::Rect,
        buf: &mut jellyhaj_widgets_core::Buffer,
        _cx: jellyhaj_widgets_core::WidgetContext<
            '_,
            Self::Action,
            impl jellyhaj_widgets_core::Wrapper<Self::Action>,
            R,
        >,
    ) -> jellyhaj_widgets_core::Result<()> {
        Tree::new(&self.items)
            .expect("distinct")
            .block(Block::bordered().title("Inspect Views"))
            .highlight_style(Modifier::REVERSED.into())
            .render(area, buf, &mut self.state);
        Ok(())
    }
}
