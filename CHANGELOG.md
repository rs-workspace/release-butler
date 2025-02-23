# Changelog

## [0.1.2] - 2025-02-23
### Added

- Support for `cargo_workspace` [#9](https://github.com/rs-workspace/release-butler/pull/9)

## [0.1.1] - 2025-02-12
### Fixes
- incorrect pre-release condition for creating release

## [0.1.0] - 2025-02-12
### Added
- Initial release of Release Butler.
- Listens to issues created with a `release-butler` label.
- Automatically creates pull requests for version bumps and changelogs.
- Optionally creates tags and GitHub releases upon merging pull requests.
