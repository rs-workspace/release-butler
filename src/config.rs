use std::{collections::BTreeMap, path::PathBuf};

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

pub static CONFIG_VERSION: u8 = 1;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    /// Default base branch for PR(s)
    pub default_branch: String,
    pub issues_meta_data: IssueMetaData,
    pub pr_meta_data: PRMetaData,
    pub packages: BTreeMap<String, Package>,
    /// Configuration Version
    pub version: u8,
}

#[derive(Serialize, Debug, Clone)]
pub struct IssueMetaData {
    /// Authors that are allowed to open the issue for release
    pub allowed_authors: Vec<String>,
    /// Comment which will be commented by the app if unauthorized user has used the label
    /// and will remove the label. (Automatically generated, if not provided)
    pub unauthorized_author_comment: String,
    /// The label on the issue that will be considered. (Default: `release-butler`)
    pub label: String,
}

impl<'de> Deserialize<'de> for IssueMetaData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            AllowedAuthors,
            UnauthorizedAuthorComment,
            Label,
        }

        struct IssueMetaDataVisitor;

        impl<'de> Visitor<'de> for IssueMetaDataVisitor {
            type Value = IssueMetaData;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct IssueMetaData")
            }

            fn visit_map<V>(self, mut map: V) -> Result<IssueMetaData, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut allowed_authors = None;
                let mut unauthorized_author_comment = None;
                let mut label = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::AllowedAuthors => {
                            if allowed_authors.is_some() {
                                return Err(de::Error::duplicate_field("allowed_authors"));
                            }
                            allowed_authors = Some(map.next_value()?);
                        }
                        Field::UnauthorizedAuthorComment => {
                            if unauthorized_author_comment.is_some() {
                                return Err(de::Error::duplicate_field(
                                    "unauthorized_author_comment",
                                ));
                            }
                            unauthorized_author_comment = Some(map.next_value()?);
                        }
                        Field::Label => {
                            if label.is_some() {
                                return Err(de::Error::duplicate_field("label"));
                            }
                            label = Some(map.next_value()?);
                        }
                    }
                }

                let allowed_authors =
                    allowed_authors.ok_or_else(|| de::Error::missing_field("allowed_authors"))?;
                let label = label.ok_or_else(|| de::Error::missing_field("label"))?;
                let unauthorized_author_comment =
                    unauthorized_author_comment.unwrap_or_else(|| {
                        format!(
                            "Hi, there you can't use the label `{}`, only some designated people are \
                            allowed to use this label. I will be removing this label soon.
                            \n\n
                            Refer to `release-butler.toml` for more information",
                            label
                        )
                    });

                Ok(IssueMetaData {
                    allowed_authors,
                    unauthorized_author_comment,
                    label,
                })
            }
        }

        const FIELDS: &[&str] = &["allowed_authors", "unauthorized_author_comment", "label"];
        deserializer.deserialize_struct("IssueMetaData", FIELDS, IssueMetaDataVisitor)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PRMetaData {
    /// When this label is on the PR, the release-butler will hold the PR as it and will
    /// not push any further changes until this label is removed
    #[serde(default = "defaults::on_hold_label")]
    pub on_hold_label: String,
    /// The Prefix that will be used on head branch. (Default: `release-butler`)
    #[serde(default = "defaults::branch_prefix")]
    pub branch_prefix: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Package {
    /// The path to root of the package. This path must be relative to repository root.
    ///
    /// (Default: `.`)
    #[serde(default = "defaults::path")]
    pub path: PathBuf,
    /// The path to Changelog file for this package. This path must be relative to repository root.
    #[serde(default = "defaults::empty_path")]
    /// (Default: ``, i.e. if empty string is provided then no Changelog file will be appended with changes)
    pub changelog_file: PathBuf,
    /// The path to changelog file for this package that is designated for pre-release versions. This path must
    /// be relative to repository root.
    ///
    /// (Default: ``, i.e. if empty string is provided then no Changelog file will be appended with changes)
    #[serde(default = "defaults::empty_path")]
    pub pre_release_changelog_file: PathBuf,
    /// The package manager used by this package.
    pub package_manager: PackageManager,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum PackageManager {
    Cargo,
    // TODO: Support for more package manager
}

mod defaults {
    use std::path::PathBuf;

    pub fn branch_prefix() -> String {
        String::from("release-butler/")
    }

    pub fn on_hold_label() -> String {
        String::from("release-butler-hold")
    }

    pub fn path() -> PathBuf {
        PathBuf::from(".")
    }

    pub fn empty_path() -> PathBuf {
        PathBuf::new()
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
