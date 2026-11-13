//! Append-only map rule activations, windowed polls and deduplicated operational matches.
use crate::{
    Error,
    rows::{content, to_i64, to_json, unsigned},
};
use opentatu_core::{
    EventId, Fingerprint, MonitorDetection, MonitorPoll, MonitorRules, NewEvent, Slug,
};
use sqlx::{Row as _, SqliteConnection};

/// Latest explicit activation of a profile, never an implicit map-generated activation.
pub async fn current_rules(
    c: &mut SqliteConnection,
    app: &Slug,
    profile: &Slug,
) -> Result<Option<MonitorRules>, Error> {
    let row = sqlx::query(
        "SELECT content FROM monitor_rules WHERE app=? AND profile=? ORDER BY number DESC LIMIT 1",
    )
    .bind(app.as_str())
    .bind(profile.as_str())
    .fetch_optional(c)
    .await?;
    row.as_ref().map(content).transpose()
}
/// Appends an activation and its owner event in the caller transaction.
pub async fn activate(
    c: &mut SqliteConnection,
    r: &MonitorRules,
    event: &NewEvent,
) -> Result<EventId, Error> {
    sqlx::query("INSERT INTO monitor_rules(app,profile,number,map_id,content) SELECT ?,?,coalesce(max(number),0)+1,?,? FROM monitor_rules WHERE app=? AND profile=?")
        .bind(r.app.as_str()).bind(r.policy.profile.as_str()).bind(r.map_id.as_str()).bind(to_json("content",r)?).bind(r.app.as_str()).bind(r.policy.profile.as_str()).execute(&mut *c).await?;
    crate::events::append(c, event).await
}
/// Next app-local poll number, including interrupted attempts.
pub async fn next_poll(c: &mut SqliteConnection, app: &Slug) -> Result<u32, Error> {
    let row = sqlx::query("SELECT coalesce(max(number),0)+1 AS n FROM monitor_polls WHERE app=?")
        .bind(app.as_str())
        .fetch_one(c)
        .await?;
    number(&row)
}
/// Last successful poll of these rules; failures cannot conceal a growing coverage gap.
pub async fn last_poll(
    c: &mut SqliteConnection,
    app: &Slug,
    profile: &Slug,
    digest: &Fingerprint,
) -> Result<Option<MonitorPoll>, Error> {
    let row=sqlx::query("SELECT content FROM monitor_polls WHERE app=? AND profile=? AND rules_digest=? AND completed_at IS NOT NULL AND json_extract(content,'$.failure') IS NULL ORDER BY number DESC LIMIT 1")
        .bind(app.as_str()).bind(profile.as_str()).bind(digest.as_str()).fetch_optional(c).await?;
    row.as_ref().map(content).transpose()
}
/// Authoritative poll identity for completing an attempt.
pub async fn poll(
    c: &mut SqliteConnection,
    app: &Slug,
    n: u32,
) -> Result<Option<MonitorPoll>, Error> {
    let row = sqlx::query("SELECT content FROM monitor_polls WHERE app=? AND number=?")
        .bind(app.as_str())
        .bind(i64::from(n))
        .fetch_optional(c)
        .await?;
    row.as_ref().map(content).transpose()
}
/// Starts before sensor network access.
pub async fn insert_poll(
    c: &mut SqliteConnection,
    p: &MonitorPoll,
    event: &NewEvent,
) -> Result<EventId, Error> {
    sqlx::query(
        "INSERT INTO monitor_polls(app,number,profile,rules_digest,content) VALUES(?,?,?,?,?)",
    )
    .bind(p.app.as_str())
    .bind(i64::from(p.number))
    .bind(p.rules.policy.profile.as_str())
    .bind(p.rules.digest()?.as_str())
    .bind(to_json("content", p)?)
    .execute(&mut *c)
    .await?;
    crate::events::append(c, event).await
}
/// Completes once; caller also persists detections in this transaction.
pub async fn complete_poll(
    c: &mut SqliteConnection,
    p: &MonitorPoll,
    event: &NewEvent,
) -> Result<EventId, Error> {
    let changed=sqlx::query("UPDATE monitor_polls SET completed_at=?,content=? WHERE app=? AND number=? AND completed_at IS NULL")
        .bind(p.completed_at.map(|t|t.iso())).bind(to_json("content",p)?).bind(p.app.as_str()).bind(i64::from(p.number)).execute(&mut *c).await?;
    if changed.rows_affected() != 1 {
        return Err(Error::NotFound("undecided monitor poll".into()));
    }
    crate::events::append(c, event).await
}
/// Recent polls, including interrupted/failed coverage.
pub async fn polls(
    c: &mut SqliteConnection,
    app: &Slug,
    limit: u32,
) -> Result<Vec<MonitorPoll>, Error> {
    let rows =
        sqlx::query("SELECT content FROM monitor_polls WHERE app=? ORDER BY number DESC LIMIT ?")
            .bind(app.as_str())
            .bind(i64::from(limit))
            .fetch_all(c)
            .await?;
    rows.iter().map(content).collect()
}
/// Next never-reused operational detection number.
pub async fn next_detection(c: &mut SqliteConnection, app: &Slug) -> Result<u32, Error> {
    let row =
        sqlx::query("SELECT coalesce(max(number),0)+1 AS n FROM monitor_detections WHERE app=?")
            .bind(app.as_str())
            .fetch_one(c)
            .await?;
    number(&row)
}
/// Inserts only a new exact source/rule match; duplicates append no event.
pub async fn insert_detection(
    c: &mut SqliteConnection,
    d: &MonitorDetection,
    event: &NewEvent,
) -> Result<bool, Error> {
    let changed=sqlx::query("INSERT INTO monitor_detections(app,number,profile,rules_digest,source_id,entry_point,content) VALUES(?,?,?,?,?,?,?) ON CONFLICT(app,profile,rules_digest,source_id,entry_point) DO NOTHING")
        .bind(d.app.as_str()).bind(i64::from(d.number)).bind(d.profile.as_str()).bind(d.rules_digest.as_str()).bind(to_i64("source_id",d.source_id)?).bind(&d.rule.entry_point).bind(to_json("content",d)?).execute(&mut *c).await?;
    if changed.rows_affected() == 0 {
        return Ok(false);
    }
    crate::events::append(c, event).await?;
    Ok(true)
}
/// One immutable operational detection.
pub async fn detection(
    c: &mut SqliteConnection,
    app: &Slug,
    n: u32,
) -> Result<Option<MonitorDetection>, Error> {
    let row = sqlx::query("SELECT content FROM monitor_detections WHERE app=? AND number=?")
        .bind(app.as_str())
        .bind(i64::from(n))
        .fetch_optional(c)
        .await?;
    row.as_ref().map(content).transpose()
}
/// Recent minimized operational detections.
pub async fn detections(
    c: &mut SqliteConnection,
    app: &Slug,
    limit: u32,
) -> Result<Vec<MonitorDetection>, Error> {
    let rows = sqlx::query(
        "SELECT content FROM monitor_detections WHERE app=? ORDER BY number DESC LIMIT ?",
    )
    .bind(app.as_str())
    .bind(i64::from(limit))
    .fetch_all(c)
    .await?;
    rows.iter().map(content).collect()
}
fn number(row: &sqlx::sqlite::SqliteRow) -> Result<u32, Error> {
    u32::try_from(unsigned(row, "n")?).map_err(|_| Error::OutOfRange {
        column: "n",
        value: row.get("n"),
    })
}

#[cfg(test)]
#[path = "monitor_tests.rs"]
pub(crate) mod tests;
