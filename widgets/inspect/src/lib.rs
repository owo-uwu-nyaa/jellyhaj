use std::{
    convert::Infallible,
    fmt::{Display, Write},
    path::PathBuf,
};

use color_eyre::eyre::Context;
use crossterm::{clipboard::CopyToClipboard, execute};
use jellyhaj_core::widgets::state::{StateStack, StateValue};
use jellyhaj_widgets_core::{
    ContextRef, JellyhajWidget, JellyhajWidgetBase, MouseEventKind, RenderFlag, TreeVisitor,
    ratatui::{crossterm::event::MouseButton, style::Modifier, widgets::StatefulWidget},
    spawn::tracing::{self, info_span, instrument},
};
use tokio::sync::oneshot::{Receiver, channel};
use tui_tree_widget::{Block, Tree, TreeItem, TreeState};
use valuable::{Fields, NamedValues, StructDef, Structable, Valuable, Value};

type Id = usize;

type IdTreeItem = TreeItem<'static, Id>;

#[derive(Debug, Valuable)]
pub enum LeafValue {
    Bool(bool),
    Char(char),
    F(f64),
    I(i64),
    I128(i128),
    ISize(isize),
    U(u64),
    U128(u128),
    USize(usize),
    Str(String),
    Path(PathBuf),
    Unit,
}

impl Display for LeafValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bool(v) => f.write_str(if *v { "true" } else { "false" }),
            Self::Char(c) => f.write_char(*c),
            Self::F(v) => Display::fmt(v, f),
            Self::I(v) => Display::fmt(v, f),
            Self::I128(v) => Display::fmt(v, f),
            Self::ISize(v) => Display::fmt(v, f),
            Self::U(v) => Display::fmt(v, f),
            Self::U128(v) => Display::fmt(v, f),
            Self::USize(v) => Display::fmt(v, f),
            Self::Str(v) => f.write_str(v),
            Self::Path(v) => Display::fmt(&v.display(), f),
            Self::Unit => Ok(()),
        }
    }
}

#[derive(Debug, Valuable)]
pub enum ValueTree {
    Tree(Vec<(Id, Self)>),
    Leaf(LeafValue),
}

impl ValueTree {
    fn leaf_to_string(&self) -> Option<String> {
        match self {
            Self::Tree(_) => None,
            Self::Leaf(leaf_value) => Some(leaf_value.to_string()),
        }
    }
}

fn sort_value_tree(tree: &mut [(Id, ValueTree)]) {
    tree.sort_unstable_by_key(|(i, _)| *i);
    tree.iter_mut()
        .filter_map(|(_, v)| {
            if let ValueTree::Tree(v) = v {
                Some(v.as_mut_slice())
            } else {
                None
            }
        })
        .for_each(sort_value_tree);
}
fn find_value_tree<'t>(tree: &'t [(Id, ValueTree)], path: &[Id]) -> Option<&'t ValueTree> {
    let id = path.first()?;
    if let Ok(i) = tree.binary_search_by_key(id, |(k, _)| *k) {
        let val = &tree[i].1;
        let path = &path[1..];
        if path.is_empty() {
            Some(val)
        } else if let ValueTree::Tree(tree) = val {
            find_value_tree(tree, path)
        } else {
            None
        }
    } else {
        None
    }
}

#[allow(clippy::too_many_lines)]
fn inspect_valuable(
    mut name: String,
    id_gen: &mut usize,
    val: Value<'_>,
) -> (IdTreeItem, (Id, ValueTree)) {
    let id = *id_gen;
    *id_gen += 1;
    let (t1, t2) = match val {
        Value::Bool(v) => {
            name.push_str(if v { "true" } else { "false" });
            (
                TreeItem::new_leaf(id, name),
                ValueTree::Leaf(LeafValue::Bool(v)),
            )
        }
        Value::Char(c) => {
            name.push('\'');
            name.push(c);
            name.push('\'');
            (
                TreeItem::new_leaf(id, name),
                ValueTree::Leaf(LeafValue::Char(c)),
            )
        }
        Value::F32(f) => {
            name += &f.to_string();
            (
                TreeItem::new_leaf(id, name),
                ValueTree::Leaf(LeafValue::F(f.into())),
            )
        }
        Value::F64(f) => {
            name += &f.to_string();
            (
                TreeItem::new_leaf(id, name),
                ValueTree::Leaf(LeafValue::F(f)),
            )
        }
        Value::I8(i) => {
            name += &i.to_string();
            (
                TreeItem::new_leaf(id, name),
                ValueTree::Leaf(LeafValue::I(i.into())),
            )
        }
        Value::I16(i) => {
            name += &i.to_string();
            (
                TreeItem::new_leaf(id, name),
                ValueTree::Leaf(LeafValue::I(i.into())),
            )
        }
        Value::I32(i) => {
            name += &i.to_string();
            (
                TreeItem::new_leaf(id, name),
                ValueTree::Leaf(LeafValue::I(i.into())),
            )
        }
        Value::I64(i) => {
            name += &i.to_string();
            (
                TreeItem::new_leaf(id, name),
                ValueTree::Leaf(LeafValue::I(i)),
            )
        }
        Value::I128(i) => {
            name += &i.to_string();
            (
                TreeItem::new_leaf(id, name),
                ValueTree::Leaf(LeafValue::I128(i)),
            )
        }
        Value::Isize(i) => {
            name += &i.to_string();
            (
                TreeItem::new_leaf(id, name),
                ValueTree::Leaf(LeafValue::ISize(i)),
            )
        }
        Value::String(s) => {
            name.push('"');
            name.push_str(s);
            name.push('"');
            (
                TreeItem::new_leaf(id, name),
                ValueTree::Leaf(LeafValue::Str(s.to_owned())),
            )
        }
        Value::U8(i) => {
            name += &i.to_string();
            (
                TreeItem::new_leaf(id, name),
                ValueTree::Leaf(LeafValue::U(i.into())),
            )
        }
        Value::U16(i) => {
            name += &i.to_string();
            (
                TreeItem::new_leaf(id, name),
                ValueTree::Leaf(LeafValue::U(i.into())),
            )
        }
        Value::U32(i) => {
            name += &i.to_string();
            (
                TreeItem::new_leaf(id, name),
                ValueTree::Leaf(LeafValue::U(i.into())),
            )
        }
        Value::U64(i) => {
            name += &i.to_string();
            (
                TreeItem::new_leaf(id, name),
                ValueTree::Leaf(LeafValue::U(i)),
            )
        }
        Value::U128(i) => {
            name += &i.to_string();
            (
                TreeItem::new_leaf(id, name),
                ValueTree::Leaf(LeafValue::U128(i)),
            )
        }
        Value::Usize(i) => {
            name += &i.to_string();
            (
                TreeItem::new_leaf(id, name),
                ValueTree::Leaf(LeafValue::USize(i)),
            )
        }
        Value::Path(path) => {
            name += &path.display().to_string();
            (
                TreeItem::new_leaf(id, name),
                ValueTree::Leaf(LeafValue::Path(path.to_owned())),
            )
        }
        Value::Error(_) => {
            name += "Error";
            (
                TreeItem::new_leaf(id, name),
                ValueTree::Leaf(LeafValue::Unit),
            )
        }
        Value::Listable(listable) => {
            name += "[]";
            let mut visitor = ListVisitor {
                index: 0,
                id_gen,
                w_values: Vec::new(),
                v_values: Vec::new(),
            };
            listable.visit(&mut visitor);
            (
                TreeItem::new(id, name, visitor.w_values).expect("should always be unique"),
                ValueTree::Tree(visitor.v_values),
            )
        }
        Value::Structable(structable) => {
            name += "struct ";
            name += structable.definition().name();
            let mut visitor = StructVisitor {
                index: 0,
                id_gen,
                w_values: Vec::new(),
                v_values: Vec::new(),
            };
            structable.visit(&mut visitor);
            (
                TreeItem::new(id, name, visitor.w_values).expect("should always be unique"),
                ValueTree::Tree(visitor.v_values),
            )
        }
        Value::Tuplable(tuplable) => {
            name += "(...)";
            let mut visitor = StructVisitor {
                index: 0,
                id_gen,
                w_values: Vec::new(),
                v_values: Vec::new(),
            };
            tuplable.visit(&mut visitor);
            (
                TreeItem::new(id, name, visitor.w_values).expect("should always be unique"),
                ValueTree::Tree(visitor.v_values),
            )
        }
        Value::Enumerable(enumerable) => {
            name += "enum ";
            name += enumerable.definition().name();
            name += "::";
            name += enumerable.variant().name();
            let mut visitor = StructVisitor {
                index: 0,
                id_gen,
                w_values: Vec::new(),
                v_values: Vec::new(),
            };
            enumerable.visit(&mut visitor);
            (
                TreeItem::new(id, name, visitor.w_values).expect("should always be unique"),
                ValueTree::Tree(visitor.v_values),
            )
        }
        Value::Mappable(mappable) => {
            name += "{...}";
            let mut visitor = MapVisitor {
                index: 0,
                id_gen,
                w_values: Vec::new(),
                v_values: Vec::new(),
            };
            mappable.visit(&mut visitor);
            (
                TreeItem::new(id, name, visitor.w_values).expect("should always be unique"),
                ValueTree::Tree(visitor.v_values),
            )
        }
        Value::Unit => {
            name += "()";
            (
                TreeItem::new_leaf(id, name),
                ValueTree::Leaf(LeafValue::Unit),
            )
        }
        _ => {
            name += "Unknown";
            (
                TreeItem::new_leaf(id, name),
                ValueTree::Leaf(LeafValue::Unit),
            )
        }
    };
    (t1, (id, t2))
}

struct ListVisitor<'g> {
    index: usize,
    id_gen: &'g mut usize,
    w_values: Vec<IdTreeItem>,
    v_values: Vec<(Id, ValueTree)>,
}

impl valuable::Visit for ListVisitor<'_> {
    fn visit_value(&mut self, value: Value<'_>) {
        let index = self.index;
        self.index += 1;
        let prefix = format!("[{index}]: ");
        let (t1, t2) = inspect_valuable(prefix, self.id_gen, value);
        self.w_values.push(t1);
        self.v_values.push(t2);
    }
}

struct StructVisitor<'g> {
    index: usize,
    id_gen: &'g mut usize,
    w_values: Vec<IdTreeItem>,
    v_values: Vec<(Id, ValueTree)>,
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
        for (field, value) in named_values {
            let mut prefix = field.name().to_string();
            prefix += ": ";
            let (t1, t2) = inspect_valuable(prefix, self.id_gen, *value);
            self.w_values.push(t1);
            self.v_values.push(t2);
        }
    }
    fn visit_unnamed_fields(&mut self, values: &[Value<'_>]) {
        for value in values {
            let index = self.index;
            self.index += 1;
            let prefix = format!("({index}): ");
            let (t1, t2) = inspect_valuable(prefix, self.id_gen, *value);
            self.w_values.push(t1);
            self.v_values.push(t2);
        }
    }
}

struct MapVisitor<'g> {
    index: usize,
    id_gen: &'g mut usize,
    w_values: Vec<IdTreeItem>,
    v_values: Vec<(Id, ValueTree)>,
}

impl valuable::Visit for MapVisitor<'_> {
    fn visit_value(&mut self, value: Value<'_>) {
        if let Value::Mappable(m) = value {
            m.visit(self);
        }
    }

    fn visit_entry(&mut self, key: Value<'_>, value: Value<'_>) {
        let index = self.index;
        self.index += 1;
        let id = *self.id_gen;
        *self.id_gen += 1;
        let (key1, key2) = inspect_valuable("key: ".to_string(), self.id_gen, key);
        let (val1, val2) = inspect_valuable("value: ".to_string(), self.id_gen, value);
        self.w_values.push(
            TreeItem::new(id, format!("{{{index}}}"), vec![key1, val1])
                .expect("should always be unique"),
        );
        self.v_values.push((id, ValueTree::Tree(vec![key2, val2])));
    }
}

struct WidgetVisitor<'g> {
    id_gen: &'g mut usize,
    w_values: Vec<IdTreeItem>,
    v_values: Vec<(Id, ValueTree)>,
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
        let (v1, v2) = inspect_valuable("State: ".to_string(), self.id_gen, state.as_value());
        let mut visitor = WidgetVisitor {
            id_gen: self.id_gen,
            w_values: vec![v1],
            v_values: vec![v2],
        };
        visit_children(&mut visitor);
        let name = "Widget ".to_owned() + name;
        self.w_values
            .push(TreeItem::new(id, name, visitor.w_values).expect("should always be unique"));
        self.v_values.push((id, ValueTree::Tree(visitor.v_values)));
    }
}

type ViewInfo = (
    String,
    Id,
    Option<Receiver<(Vec<IdTreeItem>, Vec<(Id, ValueTree)>)>>,
);

fn inspect_state_value(val: &StateValue, view_id: usize) -> Option<ViewInfo> {
    match val {
        StateValue::Suspended(suspended_inner) => {
            let (send, recv) = channel();
            let _ = suspended_inner.send_visitor.send(Box::new(move |f| {
                let mut id_gen = 1;
                let mut visitor = WidgetVisitor {
                    id_gen: &mut id_gen,
                    w_values: Vec::new(),
                    v_values: Vec::new(),
                };
                f(&mut visitor);
                let _ = send.send((visitor.w_values, visitor.v_values));
            }));
            Some((
                format!("View {}", suspended_inner.name),
                (view_id),
                Some(recv),
            ))
        }
        StateValue::Empty => None,
        StateValue::WithoutTui(_pin) => Some(("Without Tui".to_owned(), (view_id), None)),
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

async fn collect_tree_items(items: Vec<ViewInfo>) -> (Vec<IdTreeItem>, Vec<(Id, ValueTree)>) {
    let mut r1 = Vec::with_capacity(items.len());
    let mut r2 = Vec::with_capacity(items.len());
    for (name, id, children) in items {
        if let Some(children) = children {
            if let Ok((c1, c2)) = children.await {
                r1.push(TreeItem::new(id, name, c1).expect("there should be no duplicates"));
                r2.push((id, ValueTree::Tree(c2)));
            } else {
                r1.push(
                    TreeItem::new(
                        id,
                        name,
                        vec![TreeItem::new_leaf(0, "Inspecting view failed")],
                    )
                    .expect("there should be no duplicates"),
                );
                r2.push((
                    id,
                    ValueTree::Tree(vec![(0, ValueTree::Leaf(LeafValue::Unit))]),
                ));
            }
        } else {
            r1.push(TreeItem::new_leaf(id, name));
            r2.push((id, ValueTree::Leaf(LeafValue::Unit)));
        }
    }
    (r1, r2)
}

fn inspect_state(
    state: &StateStack,
) -> impl Future<Output = (Vec<IdTreeItem>, Vec<(Id, ValueTree)>)> + Send + 'static {
    collect_tree_items(inspect_state_inner(state))
}

fn from_number(num: &serde_json::Number) -> LeafValue {
    if let Some(v) = num.as_u64() {
        LeafValue::U(v)
    } else if let Some(v) = num.as_i64() {
        LeafValue::I(v)
    } else if let Some(v) = num.as_f64() {
        LeafValue::F(v)
    } else {
        LeafValue::Str(num.to_string())
    }
}

fn inspect_json_value_inner(
    mut name: String,
    value: &serde_json::Value,
    id_gen: &mut usize,
) -> (IdTreeItem, (Id, ValueTree)) {
    let id = *id_gen;
    *id_gen += 1;
    match value {
        serde_json::Value::Null => {
            name.push_str("null");
            (
                IdTreeItem::new_leaf(id, name),
                (id, ValueTree::Leaf(LeafValue::Unit)),
            )
        }
        serde_json::Value::Bool(v) => {
            let val = if *v { "true" } else { "false" };
            name.push_str(val);
            (
                IdTreeItem::new_leaf(id, name),
                (id, ValueTree::Leaf(LeafValue::Bool(*v))),
            )
        }
        serde_json::Value::Number(number) => {
            name.push_str(&number.to_string());
            (
                IdTreeItem::new_leaf(id, name),
                (id, ValueTree::Leaf(from_number(number))),
            )
        }
        serde_json::Value::String(s) => {
            name.push('"');
            name.push_str(s);
            name.push('"');
            (
                IdTreeItem::new_leaf(id, name),
                (id, ValueTree::Leaf(LeafValue::Str(s.clone()))),
            )
        }
        serde_json::Value::Array(values) => {
            name.push_str("[]");
            let (v1, v2) = values
                .iter()
                .enumerate()
                .map(|(index, val)| {
                    let prefix = format!("[{index}]: ");
                    inspect_json_value_inner(prefix, val, id_gen)
                })
                .collect();

            (
                IdTreeItem::new(id, name, v1).expect("should always be unique"),
                (id, ValueTree::Tree(v2)),
            )
        }
        serde_json::Value::Object(map) => {
            name.push_str("{...}");
            let (v1, v2) = map
                .iter()
                .map(|(name, val)| {
                    let prefix = format!("{name:?} : ");
                    inspect_json_value_inner(prefix, val, id_gen)
                })
                .collect();
            (
                IdTreeItem::new(id, name, v1).expect("should always be unique"),
                (id, ValueTree::Tree(v2)),
            )
        }
    }
}

fn inspect_json_value(value: &serde_json::Value) -> (Vec<IdTreeItem>, Vec<(Id, ValueTree)>) {
    let mut id = 0;
    let (v1, v2) = inspect_json_value_inner(String::new(), value, &mut id);
    let mut v2 = vec![v2];
    sort_value_tree(&mut v2);
    (vec![v1], v2)
}

pub struct InspectWidget {
    items: Vec<IdTreeItem>,
    values: Vec<(Id, ValueTree)>,
    state: TreeState<Id>,
    from_widget_state: bool,
}

impl InspectWidget {
    #[must_use]
    pub fn widget_state() -> Self {
        Self {
            items: Vec::new(),
            values: Vec::new(),
            state: TreeState::default(),
            from_widget_state: true,
        }
    }
    #[must_use]
    pub fn json_value(val: &serde_json::Value) -> Self {
        let (items, mut values) = inspect_json_value(val);
        sort_value_tree(&mut values);
        Self {
            items,
            values,
            state: TreeState::default(),
            from_widget_state: false,
        }
    }
}

impl Valuable for InspectWidget {
    fn as_value(&self) -> Value<'_> {
        Value::Structable(self)
    }

    fn visit(&self, visit: &mut dyn valuable::Visit) {
        visit.visit_named_fields(&NamedValues::new(&[], &[]));
    }
}
impl Structable for InspectWidget {
    fn definition(&self) -> StructDef<'_> {
        StructDef::new_static("InspectWidget", Fields::Named(&[]))
    }
}

#[derive(Debug)]
pub enum InspectAction {
    Content(Vec<IdTreeItem>, Vec<(Id, ValueTree)>),
    Toggle,
    Open,
    CloseMoveParent,
    Close,
    Up,
    Down,
    Copy,
}

impl JellyhajWidgetBase for InspectWidget {
    type Action = InspectAction;

    type ActionResult = Infallible;

    const NAME: &str = "inspect";

    fn visit_children(&self, _visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {}

    fn min_width(&self) -> Option<u16> {
        Some(5)
    }
    fn min_height(&self) -> Option<u16> {
        Some(3)
    }
}

impl<R: ContextRef<StateStack> + 'static> JellyhajWidget<R> for InspectWidget {
    fn init(
        &mut self,
        cx: jellyhaj_widgets_core::WidgetContext<
            '_,
            Self::Action,
            impl jellyhaj_widgets_core::Wrapper<Self::Action>,
            R,
        >,
    ) {
        if self.from_widget_state {
            let f = inspect_state(cx.refs.as_ref());
            cx.submitter
                .wrap_with(|(v1, v2)| InspectAction::Content(v1, v2))
                .spawn_task_infallible(f, info_span!("collect-inspect"), "collect-inspect");
        }
    }

    fn apply_action(
        &mut self,
        _cx: jellyhaj_widgets_core::WidgetContext<
            '_,
            Self::Action,
            impl jellyhaj_widgets_core::Wrapper<Self::Action>,
            R,
        >,
        action: Self::Action,
        render_flag: &mut RenderFlag,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        match action {
            InspectAction::Content(items, values) => {
                self.items = items;
                self.values = values;
                self.state = TreeState::default();
                self.state.select_first();
                render_flag.set();
            }
            InspectAction::Toggle => {
                self.state.toggle_selected();
                render_flag.set();
            }
            InspectAction::Open => {
                self.state.key_right();
                render_flag.set();
            }
            InspectAction::CloseMoveParent => {
                self.state.key_left();
                render_flag.set();
            }
            InspectAction::Close => {
                let selection = self.state.selected().to_vec();
                self.state.close(&selection);
                render_flag.set();
            }
            InspectAction::Up => {
                self.state.key_up();
                render_flag.set();
            }
            InspectAction::Down => {
                self.state.key_down();
                render_flag.set();
            }
            InspectAction::Copy => {
                if let Some(val) = find_value_tree(&self.values, self.state.selected())
                    .and_then(ValueTree::leaf_to_string)
                {
                    execute!(std::io::stdout(), CopyToClipboard::to_clipboard_from(val))
                        .context("sending clipboard cmd")?;
                }
            }
        }
        Ok(None)
    }

    #[instrument(skip_all, name = "click_inspect")]
    fn click(
        &mut self,
        _cx: jellyhaj_widgets_core::WidgetContext<
            '_,
            Self::Action,
            impl jellyhaj_widgets_core::Wrapper<Self::Action>,
            R,
        >,
        position: jellyhaj_widgets_core::Position,
        _size: jellyhaj_widgets_core::Size,
        kind: jellyhaj_widgets_core::MouseEventKind,
        _modifier: jellyhaj_widgets_core::KeyModifiers,
        render_flag: &mut RenderFlag,
    ) -> jellyhaj_widgets_core::Result<Option<Self::ActionResult>> {
        if kind == MouseEventKind::Down(MouseButton::Left)
            && let Some(at) = self.state.rendered_at(position)
        {
            tracing::debug!(rendered_at=?at, selected=?self.state.selected(), "clicked");
            if at == self.state.selected() {
                self.state.toggle_selected();
            } else {
                self.state.select(at.to_vec());
            }
            render_flag.set();
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
