use std::time::Duration;

pub(crate) const INSTALLER_HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
pub(crate) const INSTALLER_HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(10 * 60);
pub(crate) const INSTALLER_HTTP_MAX_REDIRECTS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallerRedirectDecision {
    Follow,
    RejectInsecureScheme,
    RejectLimit,
}

fn redirect_decision(url: &reqwest::Url, previous_count: usize) -> InstallerRedirectDecision {
    if url.scheme() != "https" {
        InstallerRedirectDecision::RejectInsecureScheme
    } else if previous_count >= INSTALLER_HTTP_MAX_REDIRECTS {
        InstallerRedirectDecision::RejectLimit
    } else {
        InstallerRedirectDecision::Follow
    }
}

pub(crate) fn build_secure_installer_http_client() -> Result<reqwest::Client, reqwest::Error> {
    let redirect_policy = reqwest::redirect::Policy::custom(|attempt| {
        match redirect_decision(attempt.url(), attempt.previous().len()) {
            InstallerRedirectDecision::Follow => attempt.follow(),
            InstallerRedirectDecision::RejectInsecureScheme => {
                attempt.error("installer redirect target must use HTTPS")
            }
            InstallerRedirectDecision::RejectLimit => {
                attempt.error("installer redirect limit exceeded")
            }
        }
    });

    reqwest::Client::builder()
        .connect_timeout(INSTALLER_HTTP_CONNECT_TIMEOUT)
        .timeout(INSTALLER_HTTP_REQUEST_TIMEOUT)
        .redirect(redirect_policy)
        .user_agent(concat!("talking-moose-ai/", env!("CARGO_PKG_VERSION")))
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_installer_http_policy_freezes_timeouts_and_redirect_limit() {
        assert_eq!(INSTALLER_HTTP_CONNECT_TIMEOUT, Duration::from_secs(15));
        assert_eq!(
            INSTALLER_HTTP_REQUEST_TIMEOUT,
            Duration::from_secs(10 * 60)
        );
        assert_eq!(INSTALLER_HTTP_MAX_REDIRECTS, 3);
        build_secure_installer_http_client().expect("shared installer client must build");
    }

    #[test]
    fn shared_installer_redirect_policy_rejects_insecure_targets_and_limit() {
        let https = reqwest::Url::parse("https://example.com/model").unwrap();
        let http = reqwest::Url::parse("http://example.com/model").unwrap();
        assert_eq!(
            redirect_decision(&https, 0),
            InstallerRedirectDecision::Follow
        );
        assert_eq!(
            redirect_decision(&http, 0),
            InstallerRedirectDecision::RejectInsecureScheme
        );
        assert_eq!(
            redirect_decision(&https, INSTALLER_HTTP_MAX_REDIRECTS),
            InstallerRedirectDecision::RejectLimit
        );
    }
}
