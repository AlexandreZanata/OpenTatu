//! Use cases: registered apps (spec §A.4; R01).

use opentatu_core::{Actor, App, EventKind, NewApp, NewEvent, Slug};

use crate::{Error, Services};

impl Services {
    /// Registers an app owned by the owner (R01). Emits `APP_REGISTERED`.
    pub async fn register_app(&self, input: NewApp) -> Result<App, Error> {
        input.validate()?;
        let mut tx = self.db.begin().await?;
        if opentatu_db::apps::get(&mut tx, &input.slug)
            .await?
            .is_some()
        {
            return Err(Error::AppAlreadyRegistered(input.slug));
        }
        let app = App {
            profile: input,
            created_at: self.clock.now(),
        };
        let event = NewEvent::new(EventKind::AppRegistered, Actor::Owner, app.created_at)
            .with_app(app.slug().clone())
            .with_target(app.slug())
            .with_data(serde_json::json!({
                "name": app.profile.name,
                "repo_url": app.profile.repo_url,
                "branch": app.profile.branch,
            }));
        opentatu_db::apps::insert(&mut tx, &app, &event).await?;
        tx.commit().await?;
        Ok(app)
    }

    /// The registered app with `slug`.
    pub async fn app(&self, slug: &Slug) -> Result<App, Error> {
        let mut conn = self.db.connection().await?;
        opentatu_db::apps::get(&mut conn, slug)
            .await?
            .ok_or_else(|| Error::UnknownApp(slug.clone()))
    }

    /// Every registered app, by slug.
    pub async fn apps(&self) -> Result<Vec<App>, Error> {
        let mut conn = self.db.connection().await?;
        Ok(opentatu_db::apps::list(&mut conn).await?)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Arc;

    use opentatu_core::FixedClock;

    use super::*;

    pub(crate) fn new_app(slug: &str) -> NewApp {
        NewApp {
            slug: Slug::new(slug).unwrap(),
            name: "Abastevo".into(),
            repo_url: "/home/owner/code/abastevo".into(),
            branch: "main".into(),
            stack: vec!["Next.js".into()],
            sensitive_data: vec![],
            lab_url: None,
        }
    }

    pub(crate) async fn services() -> (Services, Arc<FixedClock>) {
        let clock = Arc::new(FixedClock::at("2026-11-12T09:00:00-03:00").unwrap());
        (Services::in_memory(clock.clone()).await.unwrap(), clock)
    }

    #[tokio::test]
    async fn r01_register_then_find() {
        let (services, _) = services().await;
        let app = services.register_app(new_app("abastevo")).await.unwrap();
        assert_eq!(app.created_at.iso(), "2026-11-12T09:00:00-03:00");
        assert_eq!(services.app(app.slug()).await.unwrap(), app);
        assert_eq!(services.apps().await.unwrap().len(), 1);
        let timeline = services.app_timeline(app.slug(), 10).await.unwrap();
        assert_eq!(timeline[0].content.kind, EventKind::AppRegistered);
        assert_eq!(timeline[0].content.actor, Actor::Owner);
    }

    #[tokio::test]
    async fn duplicates_and_invalid_profiles_are_rejected() {
        let (services, _) = services().await;
        services.register_app(new_app("abastevo")).await.unwrap();
        assert!(matches!(
            services.register_app(new_app("abastevo")).await,
            Err(Error::AppAlreadyRegistered(_))
        ));
        let mut bad = new_app("other");
        bad.repo_url = "--upload-pack=x".into();
        assert!(matches!(
            services.register_app(bad).await,
            Err(Error::Domain(_))
        ));
        assert!(matches!(
            services.app(&Slug::new("missing").unwrap()).await,
            Err(Error::UnknownApp(_))
        ));
    }
}
