use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub static CONFIG_VERSION: u8 = 1;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    /// Default base branch for PR(s)
    pub default_branch: String,
    pub issues_meta_data: IssueMetaData,
    pub packages: BTreeMap<String, Package>,
    /// Configuration Version
    pub version: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IssueMetaData {
    /// Authors that are allowed to open the issue for release
    pub allowed_authors: Vec<String>,
    /// Comment which will be commented by the app if unauthorized user has used the label
    /// and will remove the label. (Automatically generated, if not provided)
    #[serde(default = "defaults::unauthorized_author_comment")]
    pub unauthorized_author_comment: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Package {
    /// The path to root of the package. This path must be relative to repository root.
    ///
    /// (Default: ``)
    #[serde(default = "defaults::path")]
    pub path: String,
    /// The path to Changelog file for this package. This path must be relative to repository root.
    #[serde(default = "defaults::path")]
    /// (Default: ``, i.e. if empty string is provided then no Changelog file will be appended with changes)
    pub changelog_file: String,
    /// The path to changelog file for this package that is designated for pre-release versions. This path must
    /// be relative to repository root.
    ///
    /// (Default: ``, i.e. if empty string is provided then no Changelog file will be appended with changes)
    #[serde(default = "defaults::path")]
    pub pre_release_changelog_file: String,
    /// Creates the GitHub Release with the changelog
    ///
    /// (Default: `false`)
    #[serde(default = "defaults::false_")]
    pub create_gh_release: bool,
    /// The package manager used by this package.
    pub package_manager: PackageManager,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum PackageManager {
    Cargo,
    CargoWorkspace, // TODO: Support for more package manager
}

mod defaults {
    pub fn unauthorized_author_comment() -> String {
        format!(
            "Hi, there you can't use the label `{}`, only some designated people are \
            allowed to use this label. I will be removing this label for now. \
            \n\nRefer to `.github/release-butler.toml` for more information",
            crate::RELEASE_ISSUE_LABEL
        )
    }

    pub fn false_() -> bool {
        false
    }

    pub fn path() -> String {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_serialization() {
        let configuration = include_str!("../repository.template.toml");

        let config = toml::from_str::<Config>(configuration).unwrap();

        assert_eq!(config.version, CONFIG_VERSION)
    }
}
