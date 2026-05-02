use std::cmp::{max, min};

use crate::{
    CommandMapper, KeybindAction, KeybindWidget, KeybindWrapper, MAX_KEYBIND_WIDGET_HEIGHT,
};
use color_eyre::Result;
use itertools::Itertools;
use jellyhaj_core::Config;
use jellyhaj_widgets_core::{
    ContextRef, GetFromContext, JellyhajWidget, JellyhajWidgetExt, WidgetContext, Wrapper,
};
use keybinds::{Command, KeyBinding};
use ratatui::{
    layout::{Position, Rect, Size},
    style::Color,
    symbols::merge::MergeStrategy,
    text::{Line, Span},
    widgets::{Block, Padding, Paragraph, Widget},
};

pub fn render_keybinds<
    R: ContextRef<Config> + 'static,
    T: Command,
    W: JellyhajWidget<R>,
    M: CommandMapper<T, A = W::Action>,
>(
    this: &mut KeybindWidget<T, W, M>,
    area: Rect,
    buf: &mut ratatui::buffer::Buffer,
    cx: WidgetContext<'_, KeybindAction<W::Action>, impl Wrapper<KeybindAction<W::Action>>, R>,
) -> Result<()> {
    let task = cx.wrap_with(KeybindWrapper);
    let len: usize = this.current_map.iter().map(|v| v.len()).sum();
    if len > 0 {
        let width = (area.width - 4) / 20;
        let full_usable_height = len.div_ceil(width as usize);
        let full_height = full_usable_height + 4;
        let height = u16::try_from(min(
            full_height,
            max(MAX_KEYBIND_WIDGET_HEIGHT as usize, area.height as usize / 4),
        ))
        .expect("should be bounded by max height");
        let usable_height = height - 4;
        let num_views = full_usable_height.div_ceil(usable_height as usize);
        this.current_view = min(this.current_view, num_views - 1);
        this.inner.render_fallible(
            Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: area.height - height as u16 + 1,
            },
            buf,
            task,
        )?;
        let area = Rect {
            x: area.x,
            y: area.y + area.height - height as u16,
            width: area.width,
            height: height as u16,
        };
        let mut block = Block::bordered()
            .padding(Padding::uniform(1))
            .merge_borders(MergeStrategy::Fuzzy);
        if num_views > 1 {
            block = block
                .title_bottom(format!("{} of {}", this.current_view, num_views))
                .title_bottom("switch with Ctrl+left/right");
        }
        let main = block.inner(area);
        let items_per_screen = width as usize * usable_height as usize;
        for ((key, binding), pos) in this
            .current_map
            .iter()
            .map(|v| v.iter())
            .kmerge_by(|(a, _), (b, _)| a < b)
            .skip(items_per_screen * this.current_view)
            .take(items_per_screen)
            .zip((0u16..usable_height).flat_map(|y| {
                (0u16..width).map(move |x| Position {
                    x: main.x + x * 20,
                    y: main.y + y,
                })
            }))
        {
            let binding = match binding {
                KeyBinding::Command(c) => Span::styled(c.to_name(), Color::Green),
                KeyBinding::Group { map: _, name } => Span::styled(name.as_str(), Color::Blue),
                KeyBinding::Invalid(name) => Span::styled(name.as_str(), Color::Red),
            };
            Paragraph::new(Line::from(vec![
                Span::raw(key.to_string()),
                Span::raw(" "),
                binding,
            ]))
            .render(
                (
                    pos,
                    Size {
                        width: 16,
                        height: 1,
                    },
                )
                    .into(),
                buf,
            );
        }
        block.render(area, buf);
    } else {
        this.inner.render_fallible(area, buf, task)?;
        let help_prefixes = &Config::get_ref(cx.refs).help_prefixes;
        let len = help_prefixes.len();
        if len != 0 {
            let area = Rect {
                x: area.x + 1,
                y: area.y + area.height - 1,
                width: area.width - 2,
                height: 1,
            };
            let mut message = "For help press ".to_string();
            for (i, bind) in help_prefixes.iter().enumerate() {
                if i == 0 {
                } else if i == len - 1 {
                    message.push_str(" or ");
                } else {
                    message.push_str(", ");
                }
                message.push_str(bind);
            }
            message.push('.');
            message.render(area, buf);
        }
    }
    Ok(())
}
