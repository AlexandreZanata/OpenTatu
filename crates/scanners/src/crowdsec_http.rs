//! Bounded authenticated Local API transport. No redirects/proxies/retries or raw errors.
use crate::crowdsec_config::{Credentials, CrowdSecProfile};
use opentatu_core::{MonitorFailure as F, Secret};
use serde_json::Value;

pub(crate) struct Api {
    client: reqwest::Client,
    base: reqwest::Url,
    credentials: Credentials,
    token: Secret,
}
impl Api {
    pub(crate) async fn connect(p: &CrowdSecProfile) -> Result<Self, F> {
        p.validate().map_err(|_| F::Configuration)?;
        let credentials = Credentials::load(&p.credentials_file)?;
        let mut builder = reqwest::Client::builder()
            .user_agent(concat!("opentatu/", env!("CARGO_PKG_VERSION")))
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .timeout(std::time::Duration::from_secs(crate::crowdsec::API_SECONDS))
            .connect_timeout(std::time::Duration::from_secs(3));
        if let Some(path) = &p.ca_file {
            use std::io::Read as _;
            let f = std::fs::File::open(path).map_err(|_| F::Configuration)?;
            let mut bytes = Vec::new();
            f.take(256 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| F::Configuration)?;
            if bytes.len() > 256 * 1024 {
                return Err(F::Configuration);
            }
            let certificates =
                reqwest::Certificate::from_pem_bundle(&bytes).map_err(|_| F::Configuration)?;
            if certificates.is_empty() {
                return Err(F::Configuration);
            }
            for c in certificates {
                builder = builder.add_root_certificate(c);
            }
        }
        let client = builder.build().map_err(|_| F::Configuration)?;
        let base = reqwest::Url::parse(&p.base_url).map_err(|_| F::Configuration)?;
        let url = base
            .join("v1/watchers/login")
            .map_err(|_| F::Configuration)?;
        let login=read(client.post(url).json(&serde_json::json!({"machine_id":credentials.machine_id,"password":credentials.password.expose()}))).await?;
        let token = login
            .get("token")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty() && s.len() <= 8192)
            .ok_or(F::Protocol)?;
        Ok(Self {
            client,
            base,
            credentials,
            token: Secret::new(token),
        })
    }
    pub(crate) fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        bouncer: bool,
    ) -> Result<reqwest::RequestBuilder, F> {
        let url = self.base.join(path).map_err(|_| F::Configuration)?;
        let r = self.client.request(method, url);
        Ok(if bouncer {
            let mut key =
                reqwest::header::HeaderValue::from_str(self.credentials.bouncer_key.expose())
                    .map_err(|_| F::Configuration)?;
            key.set_sensitive(true);
            r.header("X-Api-Key", key)
        } else {
            r.bearer_auth(self.token.expose())
        })
    }
    pub(crate) async fn get(
        &self,
        path: &str,
        params: &[(&str, String)],
        bouncer: bool,
    ) -> Result<Value, F> {
        read(
            self.request(reqwest::Method::GET, path, bouncer)?
                .query(params),
        )
        .await
    }
    pub(crate) async fn create(&self, payload: &Value) -> Result<(), F> {
        read(
            self.request(reqwest::Method::POST, "v1/alerts", false)?
                .json(payload),
        )
        .await?;
        Ok(())
    }
    pub(crate) async fn delete(&self, id: u64) -> Result<(), F> {
        read(self.request(
            reqwest::Method::DELETE,
            &format!("v1/decisions/{id}"),
            false,
        )?)
        .await?;
        Ok(())
    }
}
async fn read(request: reqwest::RequestBuilder) -> Result<Value, F> {
    let mut response = request.send().await.map_err(|_| F::Transport)?;
    if response.status() == reqwest::StatusCode::UNAUTHORIZED
        || response.status() == reqwest::StatusCode::FORBIDDEN
    {
        return Err(F::Authentication);
    }
    if !response.status().is_success() {
        return Err(F::Transport);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| F::Transport)? {
        if bytes.len().saturating_add(chunk.len()) > crate::crowdsec::API_BYTES {
            return Err(F::Saturated);
        }
        bytes.extend_from_slice(&chunk);
    }
    if bytes.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_slice(&bytes).map_err(|_| F::Protocol)
}

#[cfg(all(test, target_os = "linux"))]
#[path = "crowdsec_http_tests.rs"]
mod tests;
