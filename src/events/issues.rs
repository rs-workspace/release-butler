use super::*;
use crate::webhook::{generate_gh_from_event, get_config};
use octocrab::models::webhook_events::{
    payload::IssuesWebhookEventAction, WebhookEvent, WebhookEventPayload,
};
use tracing::error;

pub struct IssuesHandler<'a> {
    payload: &'a WebhookEvent,
    state: &'a State,
    repository: (&'a str, &'a str),
}

#[async_trait::async_trait]
impl<'a> Handler<'a> for IssuesHandler<'a> {
    fn new(repository: (&'a str, &'a str), payload: &'a WebhookEvent, state: &'a State) -> Self {
        Self {
            payload,
            state,
            repository,
        }
    }

    async fn execute(&self) -> Result<HttpResponse, WebhookError> {
        let WebhookEventPayload::Issues(issues) = &self.payload.specific else {
            error!("Got an event, with inner payload not an issue. Ignoring the event...");
            return Err(WebhookError::MalformatedBody {
                msg: String::from("Inner Payload must be of issues event"),
            });
        };

        match issues.action {
            IssuesWebhookEventAction::Labeled | IssuesWebhookEventAction::Edited => {
                // Continue if issue contains `release-butler` label
                if !issues
                    .issue
                    .labels
                    .iter()
                    .any(|label| label.name == crate::RELEASE_ISSUE_LABEL)
                {
                    return Ok(HttpResponse::Ok().finish());
                }

                let gh = generate_gh_from_event(self.payload, &self.state.gh)?;

                let Ok((package, version)) = parse_issue_title(&issues.issue.title) else {
                    let issues_handler = gh.issues(self.repository.0, self.repository.1);
                    if let Err(err) = issues_handler
                        .create_comment(
                            issues.issue.number,
                            format!("\
                                The title `{}` doesn't follow the required format. The title must follow one of the \
                                following patterns:\n\
                                - `v1.2.3`\n\
                                - `1.2.3`\n\
                                - `package@v1.2.3`\n\
                                - `package@1.2.3`\n\n\
                                Prerelease and build metadata are supported: (e.g., `v1.2.3-beta.1+1234`)\n\n\
                                **The `release-butler` label is reserved for automated release management.**\n\n\
                                <details>\n\
                                <summary>Action taken</summary>\n\
                                The `release-butler` label has been removed. You can add it back once the title format is correct.\n\
                                </details>",
                                &issues.issue.title
                            ),
                        )
                        .await
                    {
                        error!(
                            "Failed to create comment on issue with wrong title format in issue #{} in {}/{}. Error: {}",
                            issues.issue.number,
                            self.repository.0,
                            self.repository.1,
                            err
                        );
                    };
                    if let Err(err) = issues_handler
                        .remove_label(issues.issue.number, crate::RELEASE_ISSUE_LABEL)
                        .await
                    {
                        error!(
                            "Failed to remove the label {} from issue #{} in {}/{}. Error: {}",
                            crate::RELEASE_ISSUE_LABEL,
                            issues.issue.number,
                            self.repository.0,
                            self.repository.1,
                            err
                        );
                    };

                    return Ok(HttpResponse::Ok().finish());
                };

                let Some(config) =
                    get_config(self.repository.0, self.repository.1, self.state, &gh).await
                else {
                    return Ok(HttpResponse::InternalServerError().finish());
                };

                // Check if issue is created by a valid user
                if !config
                    .issues_meta_data
                    .allowed_authors
                    .iter()
                    .any(|author| author.to_lowercase() == issues.issue.user.login.to_lowercase())
                {
                    let issues_handler = gh.issues(self.repository.0, self.repository.1);
                    if let Err(err) = issues_handler
                        .create_comment(
                            issues.issue.number,
                            "I see you are using `release-butler` label. This label is reserved for automated release \
                            system and can only be used by certain authorized people.\n\nI will be removing the label \
                            `release-butler` in the favor this comment."
                        )
                        .await
                    {
                        error!(
                            "Failed to create comment on issue #{} in {}/{} regarding unauthorized issue author. Error: {}", 
                            issues.issue.number,
                            self.repository.0,
                            self.repository.1, err
                        );
                    }

                    if let Err(err) = issues_handler
                        .remove_label(issues.issue.number, crate::RELEASE_ISSUE_LABEL)
                        .await
                    {
                        error!(
                            "Failed to remove the label `{}` on issue #{} in {}/{}. Error: {}",
                            crate::RELEASE_ISSUE_LABEL,
                            issues.issue.number,
                            self.repository.0,
                            self.repository.1,
                            err
                        );
                    }

                    return Ok(HttpResponse::Ok().finish());
                }
            }
            _ => {
                return Err(WebhookError::UnsupportedEvent);
            }
        }

        Ok(HttpResponse::Ok().finish())
    }
}

pub fn parse_issue_title(title: &str) -> Result<(&str, semver::Version), semver::Error> {
    let (package, ver_str) = match title.split_once('@') {
        Some((pkg, ver)) => (pkg, ver),
        None => ("", title),
    };

    let ver_str = ver_str.strip_prefix('v').unwrap_or(ver_str);

    let version = semver::Version::parse(ver_str)?;
    Ok((package, version))
}

#[cfg(test)]
mod tests {
    use semver::{BuildMetadata, Prerelease};

    use super::*;

    #[test]
    fn test_valid_semver() {
        let v = "v0.1.2";
        let v_actual = parse_issue_title(v).unwrap();
        let v_expected = semver::Version::new(0, 1, 2);
        assert_eq!(v_actual.0, "");
        assert_eq!(v_actual.1, v_expected);

        let v = "0.1.2";
        let v_actual = parse_issue_title(v).unwrap();
        let v_expected = semver::Version::new(0, 1, 2);
        assert_eq!(v_actual.0, "");
        assert_eq!(v_actual.1, v_expected);

        let v = "package@0.1.2";
        let v_actual = parse_issue_title(v).unwrap();
        let v_expected = semver::Version::new(0, 1, 2);
        assert_eq!(v_actual.0, "package");
        assert_eq!(v_actual.1, v_expected);

        let v = "package@v0.1.2";
        let v_actual = parse_issue_title(v).unwrap();
        let v_expected = semver::Version::new(0, 1, 2);
        assert_eq!(v_actual.0, "package");
        assert_eq!(v_actual.1, v_expected);

        let v = "v0.1.2-alpha.1";
        let v_actual = parse_issue_title(v).unwrap();
        let v_expected = semver::Version {
            major: 0,
            minor: 1,
            patch: 2,
            pre: Prerelease::new("alpha.1").unwrap(),
            build: BuildMetadata::EMPTY,
        };
        assert_eq!(v_actual.0, "");
        assert_eq!(v_actual.1, v_expected);

        let v = "0.1.2-alpha.1";
        let v_actual = parse_issue_title(v).unwrap();
        let v_expected = semver::Version {
            major: 0,
            minor: 1,
            patch: 2,
            pre: Prerelease::new("alpha.1").unwrap(),
            build: BuildMetadata::EMPTY,
        };
        assert_eq!(v_actual.0, "");
        assert_eq!(v_actual.1, v_expected);

        let v = "package-12@0.1.2-alpha.1";
        let v_actual = parse_issue_title(v).unwrap();
        let v_expected = semver::Version {
            major: 0,
            minor: 1,
            patch: 2,
            pre: Prerelease::new("alpha.1").unwrap(),
            build: BuildMetadata::EMPTY,
        };
        assert_eq!(v_actual.0, "package-12");
        assert_eq!(v_actual.1, v_expected);

        let v = "package-12@v0.1.2-alpha.1";
        let v_actual = parse_issue_title(v).unwrap();
        let v_expected = semver::Version {
            major: 0,
            minor: 1,
            patch: 2,
            pre: Prerelease::new("alpha.1").unwrap(),
            build: BuildMetadata::EMPTY,
        };
        assert_eq!(v_actual.0, "package-12");
        assert_eq!(v_actual.1, v_expected);

        let v = "v0.1.2-alpha.1+0.0.0";
        let v_actual = parse_issue_title(v).unwrap();
        let v_expected = semver::Version {
            major: 0,
            minor: 1,
            patch: 2,
            pre: Prerelease::new("alpha.1").unwrap(),
            build: BuildMetadata::new("0.0.0").unwrap(),
        };
        assert_eq!(v_actual.0, "");
        assert_eq!(v_actual.1, v_expected);

        let v = "0.1.2-alpha.1+0.0.0";
        let v_actual = parse_issue_title(v).unwrap();
        let v_expected = semver::Version {
            major: 0,
            minor: 1,
            patch: 2,
            pre: Prerelease::new("alpha.1").unwrap(),
            build: BuildMetadata::new("0.0.0").unwrap(),
        };
        assert_eq!(v_actual.0, "");
        assert_eq!(v_actual.1, v_expected);

        let v = "package-ff@0.1.2-alpha.1+0.0.0";
        let v_actual = parse_issue_title(v).unwrap();
        let v_expected = semver::Version {
            major: 0,
            minor: 1,
            patch: 2,
            pre: Prerelease::new("alpha.1").unwrap(),
            build: BuildMetadata::new("0.0.0").unwrap(),
        };
        assert_eq!(v_actual.0, "package-ff");
        assert_eq!(v_actual.1, v_expected);

        let v = "package-ff@v0.1.2-alpha.1+0.0.0";
        let v_actual = parse_issue_title(v).unwrap();
        let v_expected = semver::Version {
            major: 0,
            minor: 1,
            patch: 2,
            pre: Prerelease::new("alpha.1").unwrap(),
            build: BuildMetadata::new("0.0.0").unwrap(),
        };
        assert_eq!(v_actual.0, "package-ff");
        assert_eq!(v_actual.1, v_expected);
    }
}
