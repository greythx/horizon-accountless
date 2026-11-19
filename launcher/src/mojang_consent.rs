//! The one-time disclaimer before the launcher downloads Minecraft's files from Mojang's
//! servers, for a player who agreed they own Minecraft: Java Edition (#434). The question is
//! only about the download; the answer is remembered in Settings.
use horizon_launcher::backend::Settings;

pub(super) enum MojangConsentRequest {
    /// The preparation, resumed with a fresh Prepare.
    Prepare,
    /// The Java instance to start, by id, and the server it joins, if any.
    Java(String, Option<String>),
}

/// Consume exactly one request. Consent is persisted before the action returns to run;
/// Cancel and Escape both use `allow = false`, and cannot return a runnable action.
pub(super) fn resolve(
    pending: &mut Option<MojangConsentRequest>,
    allow: bool,
    settings: &mut Settings,
) -> Result<Option<MojangConsentRequest>, String> {
    let Some(request) = pending.take() else {
        return Ok(None);
    };
    if !allow {
        return Ok(None);
    }
    settings.allow_mojang_downloads()?;
    Ok(Some(request))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_grants_nothing_and_allow_persists_the_consent() {
        // Settings is process-global: exercise this transition in its own isolated process.
        if std::env::var_os("HORIZON_CONSENT_TEST_CHILD").is_none() {
            let home = tempfile::tempdir().unwrap();
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "mojang_consent::tests::cancel_grants_nothing_and_allow_persists_the_consent",
                ])
                .env("HORIZON_CONSENT_TEST_CHILD", "1")
                .env("HORIZON_LAUNCHER_HOME", home.path())
                .env("HORIZON_HOME", home.path())
                .status()
                .unwrap();
            assert!(status.success());
            return;
        }
        let home = tempfile::tempdir().unwrap();
        let _ = home;
        let mut settings = Settings::default();
        let mut pending = Some(MojangConsentRequest::Prepare);
        assert!(
            resolve(&mut pending, false, &mut settings)
                .unwrap()
                .is_none(),
            "Cancel must not return an action to run"
        );
        assert!(pending.is_none());
        assert!(Settings::load().mojang_downloads_allowed_at.is_none());
        let mut pending = Some(MojangConsentRequest::Java(
            "instance".into(),
            Some("127.0.0.1".into()),
        ));
        let action = resolve(&mut pending, true, &mut settings).unwrap();
        assert!(matches!(action, Some(MojangConsentRequest::Java(..))));
        assert!(Settings::load().mojang_downloads_allowed_at.is_some());
        assert!(
            resolve(&mut pending, true, &mut settings)
                .unwrap()
                .is_none(),
            "the consent is taken once; a second call runs nothing"
        );
    }
}
