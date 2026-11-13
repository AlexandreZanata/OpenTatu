//! Immutable response specifications with event-backed optimistic state transitions.
use crate::{
    Error,
    rows::{content, to_json, unsigned},
};
use opentatu_core::{EventId, NewEvent, ResponsePlan, Slug};
use sqlx::{Row as _, SqliteConnection};

/// Next app-local action number under the caller write lock.
pub async fn next_number(c: &mut SqliteConnection, app: &Slug) -> Result<u32, Error> {
    let row = sqlx::query("SELECT coalesce(max(number),0)+1 AS n FROM response_plans WHERE app=?")
        .bind(app.as_str())
        .fetch_one(c)
        .await?;
    u32::try_from(unsigned(&row, "n")?).map_err(|_| Error::OutOfRange {
        column: "n",
        value: row.get("n"),
    })
}
/// Reviewable proposal, not an approval or remote write.
pub async fn insert(
    c: &mut SqliteConnection,
    p: &ResponsePlan,
    e: &NewEvent,
) -> Result<EventId, Error> {
    sqlx::query(
        "INSERT INTO response_plans(app,number,state,revision,target,content) VALUES(?,?,?,?,?,?)",
    )
    .bind(p.spec.app.as_str())
    .bind(i64::from(p.spec.number))
    .bind(p.state.as_str())
    .bind(i64::from(p.revision))
    .bind(p.spec.target.to_string())
    .bind(to_json("content", p)?)
    .execute(&mut *c)
    .await?;
    crate::events::append(c, e).await
}
/// One complete proposal/current state.
pub async fn get(
    c: &mut SqliteConnection,
    app: &Slug,
    n: u32,
) -> Result<Option<ResponsePlan>, Error> {
    let row = sqlx::query("SELECT content FROM response_plans WHERE app=? AND number=?")
        .bind(app.as_str())
        .bind(i64::from(n))
        .fetch_optional(c)
        .await?;
    row.as_ref().map(content).transpose()
}
/// Recent plans, without credentials/raw evidence.
pub async fn list(
    c: &mut SqliteConnection,
    app: &Slug,
    limit: u32,
) -> Result<Vec<ResponsePlan>, Error> {
    let rows =
        sqlx::query("SELECT content FROM response_plans WHERE app=? ORDER BY number DESC LIMIT ?")
            .bind(app.as_str())
            .bind(i64::from(limit))
            .fetch_all(c)
            .await?;
    rows.iter().map(content).collect()
}
/// All bounded pending operations, rather than recent terminal rows hiding older cleanup.
pub async fn pending(c: &mut SqliteConnection, app: &Slug) -> Result<Vec<ResponsePlan>, Error> {
    let rows=sqlx::query("SELECT content FROM response_plans WHERE app=? AND state IN (?,?,?,?) ORDER BY number LIMIT 65").bind(app.as_str()).bind(opentatu_core::ResponseState::Applying.as_str()).bind(opentatu_core::ResponseState::Active.as_str()).bind(opentatu_core::ResponseState::RollingBack.as_str()).bind(opentatu_core::ResponseState::Unknown.as_str()).fetch_all(c).await?;
    rows.iter().map(content).collect()
}
/// Compare-and-swap with its event. No stale worker can overwrite a newer receipt.
pub async fn update(
    c: &mut SqliteConnection,
    p: &ResponsePlan,
    previous_revision: u32,
    e: &NewEvent,
) -> Result<EventId, Error> {
    let changed=sqlx::query("UPDATE response_plans SET state=?,revision=?,content=? WHERE app=? AND number=? AND revision=?")
        .bind(p.state.as_str()).bind(i64::from(p.revision)).bind(to_json("content",p)?).bind(p.spec.app.as_str()).bind(i64::from(p.spec.number)).bind(i64::from(previous_revision)).execute(&mut *c).await?;
    if changed.rows_affected() != 1 {
        return Err(Error::NotFound("unchanged response revision".into()));
    }
    crate::events::append(c, e).await
}

#[cfg(test)]
#[path = "response_tests.rs"]
mod tests;
