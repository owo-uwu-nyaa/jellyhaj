use std::{future::Future, rc::Rc, time::Duration};

use sqlx::{ConnectOptions, SqliteConnection, query, sqlite::SqliteConnectOptions};

use color_eyre::{
    Result,
    eyre::{Context, OptionExt},
};
use tokio::{
    sync::Mutex,
    time::{MissedTickBehavior, interval},
};
use tracing::{Instrument, error, info, info_span, instrument};

#[instrument]
async fn open_db() -> Result<SqliteConnection> {
    let mut db_path = dirs::cache_dir().ok_or_eyre("unable to detect cache dir")?;
    db_path.push("jellyhaj.sqlite");
    let create = async || {
        info!("opening sqlite db at {}", db_path.display());
        SqliteConnectOptions::new()
            .filename(&db_path)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .create_if_missing(true)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Off)
            .pragma("foreign_keys", "ON")
            .connect()
            .await
    };
    match create().await {
        Ok(db) => Ok(db),
        Err(e) => {
            tracing::error!("error opening db: {e:?}");
            info!("cache db might be corrupted. deleting...");
            std::fs::remove_file(&db_path).context("removing corrupted db")?;
            create().await.context("creating new db")
        }
    }
}

async fn cache_maintainance<Fut: Future<Output = Result<()>>>(
    mut f: impl FnMut(Rc<Mutex<SqliteConnection>>) -> Fut,
    db: Rc<Mutex<SqliteConnection>>,
) {
    let mut interval = interval(Duration::from_hours(1));
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        interval.tick().await;
        if let Err(err) = f(db.clone()).await {
            error!("Error maintaining cache: {err:?}");
        }
    }
}

#[instrument(skip_all)]
pub async fn cache() -> Result<Rc<Mutex<SqliteConnection>>> {
    let mut db = open_db().await?;
    let migrate = info_span!("migrate");
    sqlx::migrate!("../migrations")
        .run(&mut db)
        .instrument(migrate.clone())
        .await?;
    migrate.in_scope(|| info!("migrations applied"));
    let db = Rc::new(Mutex::new(db));
    spawn::spawn_future(
        cache_maintainance(clean_images, db.clone()).instrument(info_span!("cache_maintainance")),
        "cache_maintainance",
    );
    Ok(db)
}

#[instrument]
pub async fn clean_images(db: Rc<Mutex<SqliteConnection>>) -> Result<()> {
    let res = query!("delete from image_cache where (added+7*24*60*60)<unixepoch()")
        .execute(&mut *db.lock().await)
        .await
        .context("deleting old images from cache")?;
    if res.rows_affected() > 0 {
        info!("removed {} images from cache", res.rows_affected());
    }
    Ok(())
}
