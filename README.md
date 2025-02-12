# Release Butler

Release Butler is a GitHub App that automates the process of creating pull requests for version bumps and changelogs based on issues with a specific label. When the pull request is merged, it can also create a tag and GitHub release.

<iframe width="560" height="315" src="https://www.youtube.com/embed/gJtMNcaxnDw?si=ctR0EkgrCpogKGmq" title="YouTube video player" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" referrerpolicy="strict-origin-when-cross-origin" allowfullscreen></iframe>

## Features

- Listens to issues created with a `release-butler` label.
- Creates a pull request with:
    - Semver version bump (version specified in the issue title).
    - Changelog (issue body).
- Optionally creates a tag and GitHub release when the pull request is merged.

## Usage

1. Install the Release Butler GitHub App on your repository.
2. Create `.github/release-butler.toml`
3. Create an issue with the label `release-butler`.
4. In the issue title, specify the new version (e.g., `v1.2.3`).
5. In the issue body, provide the changelog details.
6. Release Butler will automatically create a pull request with the version bump and changelog.
7. Merge the pull request to apply the changes.
8. Optionally, a tag and GitHub release will be created upon merging the pull request.

## Configuration

Refer to [`repository.template.toml`](./repository.template.toml) for a sample configuration file with information
regarding every field.

## Example

1. Create an issue with the title `v1.2.3` and the label `release-butler`.
2. Add the following to the issue body:

     ```
     ### Added
     - Added new feature X
     - Fixed bug Y
     ```

3. Release Butler will create a pull request with the version bump to `v1.2.3` and the provided changelog.
4. Merge the pull request to complete the release process.

## Languages Supported

Currently, only rust is supported and we are planning to add support for numerous other languages/package manager 
support. If you are interested, please communicate with the maintainers (via issues) before contributing.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.

## Contact

For questions or support, please open an issue on the [GitHub repository](https://github.com/rs-workspace/release-butler).
