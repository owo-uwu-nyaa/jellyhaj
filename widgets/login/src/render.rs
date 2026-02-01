use ratatui::{
    layout::Rect,
    style::{Color, Modifier},
    text::Text,
    widgets::{
        Block,
        BorderType::{self, Double},
        Padding, Paragraph, Widget, Wrap,
    },
};

use crate::{LoginInfo, LoginSelection};

pub fn render_login(
    info: &LoginInfo,
    selection: LoginSelection,
    error: &str,
    area: ratatui::prelude::Rect,
    buf: &mut ratatui::prelude::Buffer,
) {
    let outer = Block::bordered()
        .padding(Padding::uniform(1))
        .title("Enter Jellyfin Server / Login Information");
    let main = outer.inner(area);
    let normal_block = Block::bordered();
    let current_block = Block::bordered().border_type(Double);

    let server = Paragraph::new(info.server_url.as_str()).block(
        if LoginSelection::Server == selection {
            current_block.clone()
        } else {
            normal_block.clone()
        }
        .title("Jellyfin URL"),
    );
    let username = Paragraph::new(info.username.as_str()).block(
        if LoginSelection::Username == selection {
            current_block.clone()
        } else {
            normal_block.clone()
        }
        .title("Username"),
    );
    let password = Paragraph::new(
        Text::from(if info.password_cmd.is_some() {
            "from command"
        } else if info.password.is_empty() {
            ""
        } else {
            "<hidden>"
        })
        .style(Modifier::HIDDEN),
    )
    .block(
        if LoginSelection::Password == selection {
            current_block.clone()
        } else {
            normal_block.clone()
        }
        .title("Password"),
    );
    let button = Paragraph::new("Connect").block(if LoginSelection::Retry == selection {
        current_block.clone()
    } else {
        Block::bordered().border_type(BorderType::Thick)
    });

    let error = Paragraph::new(error)
        .block(Block::bordered().border_style(Color::Red))
        .wrap(Wrap::default());

    server.render(
        Rect {
            x: main.x,
            y: main.y,
            width: main.width,
            height: 3,
        },
        buf,
    );
    username.render(
        Rect {
            x: main.x,
            y: main.y + 4,
            width: main.width,
            height: 3,
        },
        buf,
    );
    password.render(
        Rect {
            x: main.x,
            y: main.y + 8,
            width: main.width,
            height: 3,
        },
        buf,
    );
    button.render(
        Rect {
            x: main.x,
            y: main.y + 12,
            width: main.width,
            height: 3,
        },
        buf,
    );
    error.render(
        Rect {
            x: main.x,
            y: main.y + 16,
            width: main.width,
            height: main.height - 16,
        },
        buf,
    );
}
