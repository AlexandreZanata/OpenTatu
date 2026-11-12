//! `opentatu app ...`

use std::path::Path;

use opentatu_core::{NewApp, Slug};

use crate::cli::AddAppArgs;
use crate::output;

/// `opentatu app add`: registers an application (R01).
pub async fn add(data_dir: &Path, args: AddAppArgs) -> anyhow::Result<()> {
    let services = super::services(data_dir).await?;
    let app = services
        .register_app(NewApp {
            slug: Slug::new(&args.slug)?,
            name: args.name,
            repo_url: args.repo,
            branch: args.branch,
            stack: trimmed(args.stack),
            sensitive_data: trimmed(args.sensitive_data),
            lab_url: args.lab_url,
        })
        .await?;
    output::line(&format!(
        "registered {} ({}) at {}",
        app.slug(),
        app.profile.name,
        app.created_at.display_pt_br()
    ));
    output::line(&format!("next: opentatu audit create --app {}", app.slug()));
    Ok(())
}

/// `opentatu app list`.
pub async fn list(data_dir: &Path) -> anyhow::Result<()> {
    let apps = super::services(data_dir).await?.apps().await?;
    if apps.is_empty() {
        output::line("no apps registered (opentatu app add --help)");
    }
    for app in apps {
        output::line(&format!(
            "{}\t{}\t{} ({})",
            app.slug(),
            app.profile.name,
            app.profile.repo_url,
            app.profile.branch
        ));
    }
    Ok(())
}

fn trimmed(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .collect()
}
