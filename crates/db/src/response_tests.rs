//! Response spec immutability, compare-and-swap and pending cleanup queries.
use super::*;
use crate::{Database, monitor::tests::fixture};
use opentatu_core::{Actor, EventKind, ResponseState};
#[tokio::test]
async fn r04_response_repositories_reject_rewrites_and_stale_receipts() {
    let db = Database::in_memory().await.unwrap();
    let mut c = db.connection().await.unwrap();
    let (_, _, _, mut p) = fixture(&mut c).await;
    let event = NewEvent::new(EventKind::ResponsePlanned, Actor::Owner, p.updated_at)
        .with_app(p.spec.app.clone());
    assert_eq!(next_number(&mut c, &p.spec.app).await.unwrap(), 1);
    insert(&mut c, &p, &event).await.unwrap();
    assert_eq!(next_number(&mut c, &p.spec.app).await.unwrap(), 2);
    assert_eq!(get(&mut c, &p.spec.app, 1).await.unwrap(), Some(p.clone()));
    assert_eq!(list(&mut c, &p.spec.app, 10).await.unwrap(), [p.clone()]);
    assert!(pending(&mut c, &p.spec.app).await.unwrap().is_empty());
    assert!(sqlx::query("UPDATE response_plans SET content=json_set(content,'$.spec.ttl_seconds',999),revision=revision+1").execute(&mut *c).await.is_err());
    p.revision = 1;
    p.state = ResponseState::Applying;
    p.started_at = Some(p.updated_at);
    p.expires_at = Some(p.updated_at.add_seconds(p.spec.ttl_seconds).unwrap());
    update(&mut c, &p, 0, &event).await.unwrap();
    assert!(update(&mut c, &p, 0, &event).await.is_err());
    assert_eq!(pending(&mut c, &p.spec.app).await.unwrap(), [p.clone()]);
    assert!(sqlx::query("UPDATE response_plans SET content=json_set(content,'$.expires_at','2099-01-01T00:00:00Z'),revision=revision+1").execute(&mut *c).await.is_err());
    assert!(
        sqlx::query("DELETE FROM response_plans")
            .execute(&mut *c)
            .await
            .is_err()
    );
    drop(c);
    let mut tx = db.begin().await.unwrap();
    p.revision = 2;
    p.state = ResponseState::Unknown;
    update(&mut tx, &p, 1, &event).await.unwrap();
    drop(tx);
    let mut c = db.connection().await.unwrap();
    assert_eq!(
        get(&mut c, &p.spec.app, 1).await.unwrap().unwrap().revision,
        1
    );
}
