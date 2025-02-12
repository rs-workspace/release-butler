use super::*;
use crate::{
    common::{File, ReferenceExt, UpdateFiles},
    config::PackageManager,
    webhook::{generate_gh_from_event, get_config},
};
use octocrab::{
    models::webhook_events::{
        payload::IssuesWebhookEventAction, WebhookEvent, WebhookEventPayload,
    },
    params::repos::Reference,
};
use std::path::PathBuf;
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
                let issues_handler = gh.issues(self.repository.0, self.repository.1);

                let Ok((package, version)) = parse_issue_title(&issues.issue.title) else {
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
                    if let Err(err) = issues_handler
                        .create_comment(
                            issues.issue.number,
                            config.issues_meta_data.unauthorized_author_comment,
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

                // check if package name is requried
                if package.is_empty() && config.packages.len() > 1 {
                    if let Err(err) = issues_handler
                        .create_comment(
                            issues.issue.number,
                            "The `release-butler.toml` contains information of multiple packages while no package \
                            name was specified in the issue title.\n\nPlease prefix the title with `<PACKAGE_NAME>@`."
                        )
                        .await {
                            error!(
                                "Failed to create comment on issue #{} in {}/{} regarding package name not specified. Error: {}", 
                                issues.issue.number,
                                self.repository.0,
                                self.repository.1, err
                            );

                            return Ok(HttpResponse::Ok().finish());
                        }
                }

                let package_information = if package.is_empty() {
                    config.packages.values().next()
                } else {
                    config.packages.get(package)
                };

                let Some(package_information) = package_information else {
                    if let Err(err) = issues_handler
                        .create_comment(
                            issues.issue.number,
                            format!(
                                "The package `{}` specified in the issue title was not found in the `release-butler.toml` \
                                configuration file.\n\nPlease check the package name and try again.",
                                if package.is_empty() { "default" } else { package }
                            ),
                        )
                        .await
                    {
                        error!(
                            "Failed to create comment on issue #{} in {}/{} regarding invalid package name. Error: {}",
                            issues.issue.number,
                            self.repository.0,
                            self.repository.1,
                            err
                        );
                    }

                    return Ok(HttpResponse::Ok().finish());
                };

                // Modify the files and create a commit
                let repos = gh.repos(self.repository.0, self.repository.1);
                match package_information.package_manager {
                    PackageManager::Cargo => {
                        let path = PathBuf::from(&package_information.path).join("Cargo.toml");
                        let Some(path_str) = path.to_str() else {
                            error!("Failed to convert path to absolute path of `Cargo.toml`");
                            return Ok(HttpResponse::Ok().finish());
                        };

                        let mut cargo_toml_content_items = match repos
                            .get_content()
                            .path(path_str)
                            .send()
                            .await
                        {
                            Ok(contents) => contents,
                            Err(err) => {
                                if let octocrab::Error::GitHub { source, .. } = err {
                                    if source.status_code.as_u16() == 404 {
                                        error!(
                                            "`{}` doesn't exists in {}/{}",
                                            path_str, self.repository.0, self.repository.1
                                        );
                                        if let Err(err) = issues_handler
                                            .create_comment(
                                                issues.issue.number,
                                                format!(
                                                    "Failed to find file with path `{}`. Please make sure the file `Cargo.toml` exists.\n\n\
                                                    If you believe this is a mistake please open a issue at [release-butler](https://github.com/rs-workspace/release-butler)",
                                                    path_str
                                                ),
                                            )
                                            .await
                                        {
                                            error!(
                                                "Failed to create a comment in issue #{} in {}/{} regarding non-existing `Cargo.toml`. Error: {}",
                                                issues.issue.number,
                                                self.repository.0,
                                                self.repository.1,
                                                err
                                            );
                                        };
                                    }
                                };

                                return Ok(HttpResponse::Ok().finish());
                            }
                        };

                        let mut updated_files = Vec::new();

                        let cargo_toml_files = cargo_toml_content_items.take_items();
                        for file_ in cargo_toml_files {
                            if PathBuf::from(&file_.path) != path {
                                continue;
                            }

                            let Some(cargo_toml_content) = file_.decoded_content() else {
                                error!("Failed to decode of `Cargo.toml`");
                                return Ok(HttpResponse::Ok().finish());
                            };
                            let Ok(mut doc) = cargo_toml_content.parse::<toml_edit::DocumentMut>()
                            else {
                                error!("Failed to parse `Cargo.toml`");
                                return Ok(HttpResponse::Ok().finish());
                            };
                            // TODO: Make it also work with workspace versions
                            doc["package"]["version"] = toml_edit::value(version.to_string());

                            updated_files.push(File {
                                name: file_.name,
                                new_content: doc.to_string(),
                            });

                            break;
                        }

                        let changelog_path_str = if version.pre.is_empty() {
                            &package_information.changelog_file
                        } else {
                            &package_information.pre_release_changelog_file
                        };

                        if !changelog_path_str.is_empty() {
                            let changelog_content_items =
                                match repos.get_content().path(changelog_path_str).send().await {
                                    Ok(mut contents) => contents.take_items(),
                                    Err(err) => {
                                        if let octocrab::Error::GitHub { source, .. } = err {
                                            if source.status_code.as_u16() == 404 {
                                                Vec::new()
                                            } else {
                                                return Ok(HttpResponse::Ok().finish());
                                            }
                                        } else {
                                            return Ok(HttpResponse::Ok().finish());
                                        }
                                    }
                                };
                            let changelog_file_path = PathBuf::from(changelog_path_str);

                            // Create the file if doesn't exists
                            if changelog_content_items.is_empty() {
                                let new_content = format!(
                                    "# Changelog\n\n## [{}] - {}\n{}",
                                    version,
                                    issues.issue.updated_at.format("%Y-%m-%d"),
                                    issues.issue.body.as_ref().unwrap_or(&String::new())
                                );

                                updated_files.push(File {
                                    name: changelog_path_str.to_owned(),
                                    new_content,
                                });
                            }

                            for content in changelog_content_items {
                                if PathBuf::from(&content.path) != changelog_file_path {
                                    continue;
                                }

                                let Some(changelog_content) = content.decoded_content() else {
                                    error!("Failed to decode changelog content");
                                    continue;
                                };

                                let changelog_lines: Vec<&str> =
                                    changelog_content.lines().collect();
                                let mut new_content = String::new();
                                let mut added_version = false;

                                // Find the first "## [" line to insert the new version before it
                                for line in changelog_lines {
                                    if !added_version && line.starts_with("## [") {
                                        // Add new version section
                                        new_content.push_str(&format!(
                                            "## [{}] - {}\n",
                                            version,
                                            issues.issue.updated_at.format("%Y-%m-%d")
                                        ));
                                        new_content.push_str(
                                            issues.issue.body.as_ref().unwrap_or(&String::new()),
                                        );
                                        new_content.push_str("\n\n");
                                        added_version = true;
                                    }
                                    new_content.push_str(line);
                                    new_content.push('\n');
                                }

                                // If no version headers found, append to the end
                                if !added_version {
                                    new_content.push_str(&format!(
                                        "## [{}] - {}\n",
                                        version,
                                        issues.issue.updated_at.format("%Y-%m-%d")
                                    ));
                                    new_content.push_str(
                                        issues.issue.body.as_ref().unwrap_or(&String::new()),
                                    );
                                    new_content.push('\n');
                                }

                                updated_files.push(File {
                                    name: content.name,
                                    new_content,
                                });
                                break;
                            }
                        }

                        // Push changes to branch
                        if !updated_files.is_empty() {
                            // Get the latest commit in default branch
                            let Ok(commits) = repos
                                .list_commits()
                                .branch(&config.default_branch)
                                .send()
                                .await
                            else {
                                error!("Failed to get commit history on default branch");
                                return Ok(HttpResponse::Ok().finish());
                            };
                            let latest_commit_sha = &commits.items[0].sha;

                            let branch = Reference::Branch(format!(
                                "release-butler/{}@{}",
                                package, version
                            ));

                            let updated_files = UpdateFiles::new(
                                &gh,
                                updated_files,
                                &branch,
                                format!("chore: RELEASE {}", version),
                            );

                            updated_files
                                .execute(self.repository.0, self.repository.1, latest_commit_sha)
                                .await;

                            // Check if PR is already opened
                            let pulls = gh.pulls(self.repository.0, self.repository.1);
                            let is_pull_already_there = match pulls
                                .list()
                                .base(&config.default_branch)
                                .head(branch.branch_name())
                                .state(octocrab::params::State::Open)
                                .send()
                                .await
                            {
                                Ok(mut res) => {
                                    let items = res.take_items();
                                    !items.is_empty()
                                }
                                Err(_) => true,
                            };

                            if !is_pull_already_there {
                                if let Err(err) = pulls
                                    .create(
                                        format!("RELEASE {}@v{}", package, version),
                                        branch.branch_name(),
                                        &config.default_branch,
                                    )
                                    .maintainer_can_modify(true)
                                    .body(format!("Fixes #{}\n\nThis is an automatically generated PR by [release-butler](https://github.com/rs-workspace/release-butler)", issues.issue.number))
                                    .send()
                                    .await
                                {
                                    error!("Failed to create a pull request. Error: {}", err);
                                }
                            }
                        }

                        // TODO: Open a PR, if not opened already
                    } // TODO: More Package Managers
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
