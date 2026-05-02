use color_eyre::{Result, eyre::Context};
use futures_util::{StreamExt, TryStreamExt, stream};
use jellyfin::{
    JellyfinClient,
    items::{GetNextUpQuery, GetResumeQuery, MediaItem},
    user_library::GetLatestQuery,
    user_views::{CollectionType, GetUserViewsQuery, UserView, UserViewType},
};
use jellyhaj_core::state::NextScreen;
use tokio::try_join;

async fn user_views(client: &JellyfinClient) -> Result<Vec<UserView>> {
    let user_id = client.get_auth().user.id.as_str();
    client
        .get_user_views(&GetUserViewsQuery {
            user_id: Some(user_id),
            include_external_content: Some(false),
            include_hidden: Some(false),
            ..Default::default()
        })
        .await
        .context("fetching user views")?
        .deserialize()
        .context("deserializing user views")
        .map(|v| v.items)
}

async fn resume(client: &JellyfinClient) -> Result<Vec<MediaItem>> {
    let user_id = client.get_auth().user.id.as_str();
    client
        .get_user_items_resume(&GetResumeQuery {
            user_id: user_id.into(),
            limit: 16.into(),
            enable_user_data: true.into(),
            image_type_limit: 1.into(),
            enable_image_types: "Thumb, Backdrop, Primary".into(),
            media_types: "Video".into(),

            fields: "Overview".into(),
            enable_total_record_count: true.into(),
            enable_images: true.into(),
            exclude_active_sessions: false.into(),
            ..Default::default()
        })
        .await
        .context("fetching resumes")?
        .deserialize()
        .context("deserializing resumes")
        .map(|v| v.items)
}

async fn next_up(client: &JellyfinClient) -> Result<Vec<MediaItem>> {
    let user_id = client.get_auth().user.id.as_str();
    client
        .get_shows_next_up(&GetNextUpQuery {
            user_id: Some(user_id),
            limit: Some(16),
            enable_user_data: Some(true),
            enable_images: Some(true),
            fields: "Overview".into(),
            image_type_limit: Some(1),
            enable_image_types: Some("Thumb, Backdrop, Primary"),
            enable_total_record_count: Some(true),
            disable_first_episode: Some(true),
            enable_resumable: Some(false),
            enable_rewatching: Some(false),
            ..Default::default()
        })
        .await
        .context("fetching next up")?
        .deserialize()
        .context("deserializing next up")
        .map(|i| i.items)
}

async fn latest(
    client: &JellyfinClient,
    user_views: &[UserView],
) -> Result<Vec<(String, Vec<MediaItem>)>> {
    let user_id = client.get_auth().user.id.as_str();
    stream::iter(user_views.iter())
        .filter_map(async |view| {
            if view.view_type == UserViewType::CollectionFolder
                && view.collection_type != CollectionType::Unknown
            {
                match client
                    .get_user_library_latest_media(&GetLatestQuery {
                        user_id: Some(user_id),
                        limit: Some(16),
                        enable_user_data: Some(true),
                        enable_images: Some(true),
                        image_type_limit: Some(1),
                        fields: "Overview".into(),
                        enable_image_types: Some("Thumb, Backdrop, Primary"),
                        parent_id: Some(&view.id),
                        group_items: Some(true),
                        ..Default::default()
                    })
                    .await
                    .with_context(|| format!("fetching latest media from {}", view.name))
                {
                    Ok(items) => match items.deserialize() {
                        Ok(items) => Some(Ok((view.name.clone(), items))),
                        Err(e) => Some(Err(e)),
                    },
                    Err(e) => Some(Err(e)),
                }
            } else {
                None
            }
        })
        .try_collect()
        .await
        .context("fetching latest media")
}

async fn latest_user_views(
    client: &JellyfinClient,
) -> Result<(Vec<UserView>, Vec<(String, Vec<MediaItem>)>)> {
    let user_view = user_views(client).await?;
    let latest = latest(client, &user_view).await?;
    Ok((user_view, latest))
}

pub async fn fetch(client: JellyfinClient) -> Result<NextScreen> {
    let (resume, next_up, (user_views, latest)) = try_join!(
        resume(&client),
        next_up(&client),
        latest_user_views(&client)
    )?;
    Ok(NextScreen::HomeScreen {
        cont: resume,
        next_up,
        libraries: user_views,
        library_latest: latest,
    })
}
