use color_eyre::eyre::eyre;
use jellyhaj_context::TuiContext;
use jellyhaj_core::{state::NextScreen, widgets::shaded::widget::Erased};
use jellyhaj_login_view::LoginContext;

pub fn make_screen(screen: NextScreen, cx: TuiContext) -> Erased {
    match screen {
        NextScreen::LoadHomeScreen => jellyhaj_home_screen_view::make_fetch_home_screen(cx),
        NextScreen::HomeScreen {
            cont,
            next_up,
            libraries,
            library_latest,
        } => jellyhaj_home_screen_view::render_home_screen(
            cx,
            cont,
            next_up,
            libraries,
            library_latest,
        ),
        NextScreen::LoadUserView(user_view) => {
            jellyhaj_library_view::render_fetch_user_view(cx, user_view)
        }
        NextScreen::UserView { view, items, seen } => {
            jellyhaj_library_view::render_user_view(cx, view, items, seen)
        }
        NextScreen::FetchPlay(load_play) => jellyhaj_player_view::render_fetch_play(cx, load_play),
        NextScreen::Play { items, index } => jellyhaj_player_view::render_play(cx, items, index),
        NextScreen::Error(report) => jellyhaj_error_view::render_error(cx, &report),
        NextScreen::ItemDetails(media_item) => {
            jellyhaj_item_details_view::render_item_details(cx, media_item)
        }
        NextScreen::ItemListDetails(media_item, media_items) => {
            jellyhaj_item_details_view::render_item_list_details(cx, media_item, media_items)
        }
        NextScreen::FetchItemListDetails(media_item) => {
            jellyhaj_item_details_view::render_fetch_item_list(cx, media_item)
        }
        NextScreen::FetchItemListDetailsRef(id) => {
            jellyhaj_item_details_view::render_fetch_item_list_ref(cx, id)
        }
        NextScreen::FetchItemDetails(item) => {
            jellyhaj_item_details_view::render_fetch_item(cx, item)
        }
        NextScreen::RefreshItem(id) => jellyhaj_refresh_item_view::render_refresh_item_form(cx, id),
        NextScreen::DoRefreshItem { id, query } => {
            jellyhaj_refresh_item_view::render_do_refresh_item(cx, id, query)
        }
        NextScreen::Stats => jellyhaj_stats_view::render_stats(cx),
        NextScreen::Logs => jellyhaj_log_view::render_log(cx),
        NextScreen::Inspect => jellyhaj_inspect_view::render_inspect(cx),
        NextScreen::QuickConnect => jellyhaj_quick_connect_view::make_quick_connect(cx),
        NextScreen::QuickConnectAuth(code) => {
            jellyhaj_quick_connect_view::make_quick_connect_auth(cx, code)
        }
        NextScreen::InspectValue(value) => jellyhaj_inspect_view::render_inspect_value(cx, &value),
        NextScreen::HttpClient => jellyhaj_http_client_view::render_http_client(cx),
        NextScreen::HttpClientFetch { url } => {
            jellyhaj_http_client_view::render_http_client_fetch(cx, url)
        }
        NextScreen::FetchModifyMetadata(item) => {
            jellyhaj_metadata_editor_view::make_fetch_modify_metadata(cx, item)
        }
        NextScreen::ModifyMetadata(item, editor) => {
            jellyhaj_metadata_editor_view::make_modify_metadata(cx, item, editor)
        }
        NextScreen::DoModifyMetadata { id, new_metadata } => {
            jellyhaj_metadata_editor_view::make_do_modify_metadata(cx, id, new_metadata)
        }
        NextScreen::AddGenreFetch {
            result_sender,
            selected,
        } => jellyhaj_metadata_editor_view::make_add_genre_fetch(cx, result_sender, selected),
        NextScreen::AddGenre {
            result_sender,
            selected,
            all_genres,
        } => jellyhaj_metadata_editor_view::make_add_genre(cx, result_sender, selected, all_genres),
        NextScreen::NewGenre(submitter) => {
            jellyhaj_metadata_editor_view::make_new_genre(cx, submitter)
        }
        NextScreen::SelectServer { .. }
        | NextScreen::ConnectToServer { .. }
        | NextScreen::SelectAuthMethod { .. }
        | NextScreen::AuthPassword { .. }
        | NextScreen::AuthPasswordFetch { .. }
        | NextScreen::AuthQuickConnectFetch { .. }
        | NextScreen::AuthQuickConnectWait { .. }
        | NextScreen::AuthFinished { .. } => {
            jellyhaj_error_view::render_error(cx, &eyre!("already logged in"))
        }
        NextScreen::Logout => jellyhaj_login_view::render_logout(cx),
        NextScreen::Exit => jellyhaj_player_view::render_exit(cx),
    }
}

pub fn make_screen_login(screen: NextScreen, cx: LoginContext) -> Erased {
    match screen {
        NextScreen::Error(report) => jellyhaj_error_view::render_error(cx, &report),
        NextScreen::Stats => jellyhaj_stats_view::render_stats(cx),
        NextScreen::Logs => jellyhaj_log_view::render_log(cx),
        NextScreen::Inspect => jellyhaj_inspect_view::render_inspect(cx),
        NextScreen::InspectValue(v) => jellyhaj_inspect_view::render_inspect_value(cx, &v),
        NextScreen::QuickConnect => jellyhaj_error_view::render_error(
            cx,
            &eyre!("Authenticating another client through quick connect requires beeing logged in"),
        ),
        NextScreen::SelectServer { state, out } => {
            jellyhaj_login_view::server::render_select_server(cx, state, out)
        }
        NextScreen::ConnectToServer { state, out } => {
            jellyhaj_login_view::server::render_connect_server(
                cx,
                state,
                out,
                clap::crate_name!(),
                clap::crate_version!(),
            )
        }
        NextScreen::SelectAuthMethod {
            state,
            out,
            client,
            quick_connect_available,
            server_id,
        } => jellyhaj_login_view::select::render_select_auth_method(
            cx,
            state,
            out,
            client,
            quick_connect_available,
            server_id,
        ),
        NextScreen::AuthPassword {
            state,
            out,
            client,
            server_id,
        } => jellyhaj_login_view::password::render_password(cx, state, out, client, server_id),
        NextScreen::AuthPasswordFetch {
            state,
            out,
            client,
            server_id,
        } => {
            jellyhaj_login_view::password::render_password_fetch(cx, state, out, client, server_id)
        }
        NextScreen::AuthQuickConnectFetch {
            state,
            out,
            client,
            server_id,
        } => jellyhaj_login_view::quick_connect::render_auth_quick_connect_fetch(
            cx, state, out, client, server_id,
        ),
        NextScreen::AuthQuickConnectWait {
            state,
            out,
            client,
            secret,
            code,
            server_id,
        } => jellyhaj_login_view::quick_connect::render_auth_quick_connect_wait(
            cx, state, out, client, secret, code, server_id,
        ),
        NextScreen::AuthFinished {
            state,
            out,
            client,
            server_id,
        } => jellyhaj_login_view::render_auth_finished(cx, state, out, client, server_id),
        NextScreen::LoadHomeScreen
        | NextScreen::HomeScreen { .. }
        | NextScreen::LoadUserView(_)
        | NextScreen::UserView { .. }
        | NextScreen::FetchPlay(_)
        | NextScreen::Play { .. }
        | NextScreen::ItemDetails(_)
        | NextScreen::ItemListDetails(_, _)
        | NextScreen::FetchItemListDetails(_)
        | NextScreen::FetchItemListDetailsRef(_)
        | NextScreen::FetchItemDetails(_)
        | NextScreen::RefreshItem(_)
        | NextScreen::DoRefreshItem { .. }
        | NextScreen::QuickConnectAuth(_)
        | NextScreen::HttpClient
        | NextScreen::HttpClientFetch { .. }
        | NextScreen::FetchModifyMetadata(_)
        | NextScreen::DoModifyMetadata { .. }
        | NextScreen::NewGenre(_)
        | NextScreen::AddGenreFetch { .. }
        | NextScreen::AddGenre { .. }
        | NextScreen::ModifyMetadata(_, _)
        | NextScreen::Logout => {
            jellyhaj_error_view::render_error(cx, &eyre!("This requires beeing logged in"))
        }
        NextScreen::Exit => jellyhaj_player_view::render_exit(cx),
    }
}
