//! Trusted guest controller. Refuses host execution before reading input or spawning.
use crate::{
    Error, GuestReceipt, LabActor, engine,
    lab::{GuestInput, hash_file},
};
use opentatu_core::{ContentHasher, LabExecution, LabObservation, LabOutcome};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

/// Capability obtainable only after checking the VM marker, DMI and absence of NICs.
pub(crate) struct GuestPermit(());

pub(crate) fn guest_permit() -> Result<GuestPermit, Error> {
    let marker = std::fs::read_to_string("/etc/opentatu-lab-image").unwrap_or_default();
    let vendor = std::fs::read_to_string("/sys/class/dmi/id/sys_vendor").unwrap_or_default();
    let offline = std::fs::read_dir("/sys/class/net").is_ok_and(|entries| {
        entries
            .into_iter()
            .all(|e| e.is_ok_and(|e| e.file_name() == "lo"))
    });
    use std::os::unix::fs::MetadataExt as _;
    let root = std::fs::metadata("/proc/self").is_ok_and(|m| m.uid() == 0);
    if !root || marker.trim() != "opentatu-offline-lab-v1" || vendor.trim() != "QEMU" || !offline {
        return Err(Error::Engine(
            "guest controller requires the prepared offline QEMU VM; host execution refused".into(),
        ));
    }
    Ok(GuestPermit(()))
}

/// Executes the approved case in the guest, returning only minimized evidence. No dates
/// are generated here; services timestamps the attempt using the injected host Clock.
pub async fn run_guest(manifest: &Path) -> Result<GuestReceipt, Error> {
    let permit = guest_permit()?;
    if manifest != Path::new("/run/opentatu/input/manifest.json") {
        return Err(Error::Config("unexpected guest input path".into()));
    }
    let bytes = std::fs::read(manifest).map_err(|e| Error::Engine(e.to_string()))?;
    let input: GuestInput = serde_json::from_slice(&bytes)
        .map_err(|_| Error::Config("invalid private guest manifest".into()))?;
    input.profile.validate()?;
    let runner = Path::new("/run/opentatu/input/runtime/opentatu");
    if hash_file(runner)? != input.runner_digest {
        return Err(Error::Engine("guest controller digest mismatch".into()));
    }
    let case = input
        .profile
        .case(input.run.context.case_id.as_str())
        .ok_or_else(|| Error::Config("case not in approved profile".into()))?;
    root_step(
        &permit,
        "/usr/bin/mount",
        &[
            "-t",
            "tmpfs",
            "-o",
            "size=1536m,nodev,nosuid",
            "tmpfs",
            "/work",
        ],
    )
    .await?;
    let app = Path::new("/work/application");
    let zap = Path::new("/work/scanner");
    directory(app, 2001)?;
    directory(zap, 2002)?;
    let source = Path::new("/run/opentatu/input/source");
    copy_tree(source, app, 2001, 128 * 1024 * 1024)?;
    if source_digest(app)? != input.source_digest {
        return Err(Error::Engine("guest source copy digest mismatch".into()));
    }
    let cache = Path::new("/opt/opentatu-gocache");
    if cache.is_dir() {
        copy_tree(
            cache,
            &app.join(".lab-build-cache"),
            2001,
            512 * 1024 * 1024,
        )?;
    }
    let cwd = app.join(&input.profile.workdir);
    let mut env: Vec<_> = input
        .profile
        .fixture_env
        .iter()
        .map(|(k, v)| (k.clone(), v.expose().to_owned()))
        .collect();
    env.push((
        "GOCACHE".into(),
        app.join(".lab-build-cache").to_string_lossy().into(),
    ));
    env.push(("GOTOOLCHAIN".into(), "local".into()));
    env.push(("GOPROXY".into(), "off".into()));
    env.push(("GOSUMDB".into(), "off".into()));
    let mut build = engine::guest_child(&permit, 2001, &input.profile.build, &cwd, &env)?;
    let status = tokio::time::timeout(
        Duration::from_secs(input.profile.timeout_seconds.saturating_sub(120).max(1)),
        build.wait(),
    )
    .await
    .map_err(|_| Error::Engine("guest build timed out".into()))?
    .map_err(|e| Error::Engine(e.to_string()))?;
    if !status.success() {
        return Err(Error::Engine(
            "offline guest application build failed".into(),
        ));
    }
    let mut child = engine::guest_child(&permit, 2001, &input.profile.run, &cwd, &env)?;
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(3))
        .build()
        .map_err(|e| Error::Engine(e.to_string()))?;
    let origin = format!("http://127.0.0.1:{}", input.profile.port);
    let mut ready = false;
    for _ in 0..30 {
        if child
            .try_wait()
            .map_err(|e| Error::Engine(e.to_string()))?
            .is_some()
        {
            break;
        }
        if client
            .get(format!("{origin}{}", input.profile.readiness_path))
            .send()
            .await
            .is_ok_and(|r| r.status().is_success())
        {
            ready = true;
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    if !ready {
        return Err(Error::Engine("guest application failed readiness".into()));
    }
    let url = format!("{origin}{}", case.path);
    let control = observe(
        &client,
        &url,
        &input.profile.actors[&case.control_actor],
        &case.marker,
    )
    .await?;
    let subject = observe(
        &client,
        &url,
        &input.profile.actors[&case.subject_actor],
        &case.marker,
    )
    .await?;
    let outcome = LabOutcome::from_observations(&control, &subject);
    let started = Instant::now();
    let (scan, note) =
        match crate::lab_zap::run(&permit, &input.profile, case, &url, zap, input.run.number).await
        {
            Ok(mut scan) => {
                scan.duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
                (Some(scan), String::new())
            }
            Err(_) => (
                None,
                "Passive ZAP inspection did not complete; coverage is untested.".into(),
            ),
        };
    let _ = child.kill().await;
    Ok(GuestReceipt {
        context: input.run.context,
        number: input.run.number,
        nonce: input.nonce,
        execution: LabExecution {
            outcome,
            control: Some(control),
            subject: Some(subject),
            image_digest: input.profile.image_digest,
            source_digest: input.source_digest,
            runner_digest: input.runner_digest,
            zap: scan,
            note,
        },
    })
}

async fn observe(
    client: &reqwest::Client,
    url: &str,
    actor: &LabActor,
    marker: &str,
) -> Result<LabObservation, Error> {
    let mut request = client.get(url);
    for (name, value) in &actor.headers {
        request = request.header(name, value.expose());
    }
    let mut response = request
        .send()
        .await
        .map_err(|_| Error::Engine("guest HTTP request failed".into()))?;
    let status = response.status().as_u16();
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| Error::Engine("guest HTTP body failed".into()))?
    {
        if body.len().saturating_add(chunk.len()) > 64 * 1024 {
            return Err(Error::Engine("guest HTTP response exceeds 64 KiB".into()));
        }
        body.extend_from_slice(&chunk);
    }
    let mut digest = ContentHasher::default();
    digest.update(&body);
    Ok(LabObservation {
        status,
        bytes: u32::try_from(body.len()).unwrap_or(u32::MAX),
        body_digest: digest.finish(),
        marker_found: String::from_utf8_lossy(&body).contains(marker),
    })
}

async fn root_step(_permit: &GuestPermit, program: &str, args: &[&str]) -> Result<(), Error> {
    if program.ends_with("mount") && args.last() == Some(&"/work") {
        std::fs::create_dir_all("/work").map_err(|e| Error::Engine(e.to_string()))?;
    }
    let result = engine::lab_execute(
        program,
        &args.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
        Duration::from_secs(10),
    )
    .await?;
    if result.code != 0 {
        return Err(Error::Engine("guest isolation setup failed".into()));
    }
    Ok(())
}

pub(crate) fn directory(path: &Path, uid: u32) -> Result<(), Error> {
    use std::os::unix::fs::{PermissionsExt as _, chown};
    std::fs::create_dir_all(path)
        .and_then(|()| chown(path, Some(uid), Some(uid)))
        .and_then(|()| std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)))
        .map_err(|e| Error::Engine(e.to_string()))
}
fn copy_tree(source: &Path, destination: &Path, uid: u32, max: u64) -> Result<(), Error> {
    let files = tree_files(source)?;
    let mut bytes = 0u64;
    directory(destination, uid)?;
    for path in files {
        bytes = bytes.saturating_add(
            std::fs::metadata(&path)
                .map_err(|e| Error::Engine(e.to_string()))?
                .len(),
        );
        if bytes > max {
            return Err(Error::Engine("guest copy exceeds its byte budget".into()));
        }
        let relative = path
            .strip_prefix(source)
            .map_err(|e| Error::Engine(e.to_string()))?;
        let target = destination.join(relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::Engine(e.to_string()))?;
        }
        std::fs::copy(&path, &target)
            .and_then(|_| std::os::unix::fs::chown(&target, Some(uid), Some(uid)))
            .and_then(|()| {
                use std::os::unix::fs::PermissionsExt as _;
                std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600))
            })
            .map_err(|e| Error::Engine(e.to_string()))?;
    }
    // The app must be able to create build outputs in nested source directories.
    let mut dirs = vec![destination.to_owned()];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).map_err(|e| Error::Engine(e.to_string()))? {
            let e = entry.map_err(|e| Error::Engine(e.to_string()))?;
            if e.file_type()
                .map_err(|e| Error::Engine(e.to_string()))?
                .is_dir()
            {
                dirs.push(e.path());
            }
        }
        std::os::unix::fs::chown(&dir, Some(uid), Some(uid))
            .map_err(|e| Error::Engine(e.to_string()))?;
    }
    Ok(())
}
fn tree_files(source: &Path) -> Result<Vec<PathBuf>, Error> {
    let mut dirs = vec![source.to_owned()];
    let mut files = Vec::new();
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(dir).map_err(|e| Error::Engine(e.to_string()))? {
            let e = entry.map_err(|e| Error::Engine(e.to_string()))?;
            let kind = e.file_type().map_err(|e| Error::Engine(e.to_string()))?;
            if kind.is_dir() {
                dirs.push(e.path());
            } else if kind.is_file() {
                files.push(e.path());
            } else {
                return Err(Error::Engine("non-regular guest source input".into()));
            }
        }
    }
    files.sort();
    Ok(files)
}
fn source_digest(source: &Path) -> Result<String, Error> {
    let mut hash = ContentHasher::default();
    for path in tree_files(source)? {
        let relative = path
            .strip_prefix(source)
            .map_err(|e| Error::Engine(e.to_string()))?;
        hash.update_entry(
            &relative.to_string_lossy(),
            &std::fs::read(&path).map_err(|e| Error::Engine(e.to_string()))?,
        );
    }
    Ok(hash.finish())
}
