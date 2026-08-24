use std::sync::Arc;

use jellyfin::{
    JellyfinVec,
    connect::JsonResponseHelper,
    items::{GetGenreQuery, MediaItem, MetadataEditor, MetadataUpdate},
};
use jellyhaj_context::TuiContext;
use jellyhaj_core::{
    state::{Navigation, NextScreen},
    widgets::shaded::widget::{Erased, make_new_erased},
};
use jellyhaj_fetch_view::{make_fetch, make_nav_fetch};
use jellyhaj_form_widget::form::{FormCommandMapper, FormDataExt};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_metadata_editor_widget::{
    ModifyMetadata, ModifyMetadataAction, ModifyMetadataActionMapper,
    genre::{
        AddGenre, AddGenreAction, GenreSelection, GenreSelectionAction, GenreSelectionActionMapper,
    },
};
use jellyhaj_widgets_core::{
    async_task::ErasedSubmitter,
    mapper::ActionMapperWidget,
    outer::{Named, OuterWidget, UnwrapWidget},
};

pub fn make_fetch_modify_metadata(cx: TuiContext, item: Box<MediaItem>) -> Erased {
    let jellyfin = cx.jellyfin.clone();
    let fut = async move {
        let editor = jellyfin.metadata_editor(&item.id).deserialize().await?;
        Ok(NextScreen::ModifyMetadata(item, editor))
    };
    make_fetch(cx, "fetch metadata editor information", fut)
}

struct ModifyMetadataName;
impl Named for ModifyMetadataName {
    const NAME: &str = "modify-metadata";
}

pub fn make_modify_metadata(
    cx: TuiContext,
    item: Box<MediaItem>,
    editor: MetadataEditor,
) -> Erased {
    let widget = ModifyMetadata::new(item, editor).make_with_default();
    let widget = UnwrapWidget::new(widget);
    let widget = KeybindWidget::new(
        widget,
        cx.config.keybinds.form.clone(),
        FormCommandMapper::<ModifyMetadataAction>::default(),
    );
    let widget = UnwrapWidget::new(widget);
    let widget =
        ActionMapperWidget::<ModifyMetadataName, _, _>::new(widget, ModifyMetadataActionMapper);
    make_new_erased(cx, widget)
}

pub fn make_do_modify_metadata(
    cx: TuiContext,
    id: String,
    new_metadata: Box<MetadataUpdate>,
) -> Erased {
    let jellyfin = cx.jellyfin.clone();
    let fut = async move {
        jellyfin.update_item(&id, &new_metadata).await?;
        Ok(Navigation::PopContext)
    };
    make_nav_fetch(cx, "Updating metadata", fut)
}

pub fn make_add_genre_fetch(
    cx: TuiContext,
    result_sender: Arc<dyn ErasedSubmitter<String>>,
    selected: Vec<String>,
) -> Erased {
    let jellyfin = cx.jellyfin.clone();
    let fut = async move {
        let all_genres = JellyfinVec::stream(async |start| {
            jellyfin
                .get_genre_items(&GetGenreQuery {
                    start_index: Some(start),
                    limit: Some(48),
                    enable_images: Some(true),
                    enable_image_types: Some("Primary"),
                    image_type_limit: Some(2),
                })
                .deserialize()
                .await
        })
        .map_collect(|g| g.name)
        .await?;
        Ok(NextScreen::AddGenre {
            result_sender,
            selected,
            all_genres,
        })
    };
    make_fetch(cx, "getting all exisiting genres", fut)
}

struct AddGenreName;
impl Named for AddGenreName {
    const NAME: &str = "add-genre";
}

pub fn make_add_genre(
    cx: TuiContext,
    result_sender: Arc<dyn ErasedSubmitter<String>>,
    selected: Vec<String>,
    mut all_genres: Vec<String>,
) -> Erased {
    all_genres.retain(|genre| !selected.contains(genre));
    let widget = GenreSelection::new(selected, all_genres, result_sender).make_with_default();
    let widget = UnwrapWidget::new(widget);
    let widget = KeybindWidget::new(
        widget,
        cx.config.keybinds.form.clone(),
        FormCommandMapper::<GenreSelectionAction>::default(),
    );
    let widget = UnwrapWidget::new(widget);
    let widget = ActionMapperWidget::<AddGenreName, _, _>::new(widget, GenreSelectionActionMapper);
    make_new_erased(cx, widget)
}

struct NewGenreName;
impl Named for NewGenreName {
    const NAME: &str = "new-genre";
}

pub fn make_new_genre(cx: TuiContext, submitter: Arc<dyn ErasedSubmitter<String>>) -> Erased {
    let widget = AddGenre::new(submitter).make_with_default();
    let widget = UnwrapWidget::new(widget);
    let widget = KeybindWidget::new(
        widget,
        cx.config.keybinds.form.clone(),
        FormCommandMapper::<AddGenreAction>::default(),
    );
    let widget = OuterWidget::<NewGenreName, _>::new(widget);
    make_new_erased(cx, widget)
}
