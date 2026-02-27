# Release Checklist

* Ensure local `master` is up to date with respect to `origin/master`.
* Run `cargo update` and review dependency updates. Commit updated
  `Cargo.lock`.
* Run `cargo outdated` and review semver incompatible updates. Unless there is
  a strong motivation otherwise, review and update every dependency. Also
  run `--aggressive`, but don't update to crates that are still in beta.
* Update date in `crates/core/flags/doc/template.rg.1`.
* Update the CHANGELOG as appropriate.
* Review changes for every crate in `crates` since the last ripgrep release.
  If the set of changes is non-empty, issue a new release for that crate. Check
  crates in the following order. After updating a crate, ensure minimal
  versions are updated as appropriate in dependents. If an update is required,
  run `cargo-up --no-push crates/{CRATE}/Cargo.toml`.
    * crates/globset
    * crates/ignore
    * crates/cli
    * crates/matcher
    * crates/regex
    * crates/pcre2
    * crates/searcher
    * crates/printer
    * crates/grep (bump minimal versions as necessary)
    * crates/core (do **not** bump version, but update dependencies as needed)
* Edit the `Cargo.toml` to set the new ripgrep version. Run
  `cargo update -p ripgrep` so that the `Cargo.lock` is updated. Commit the
  changes and create a new signed tag. Alternatively, use
  `cargo-up --no-push --no-release Cargo.toml {VERSION}` to automate this.
* Run `cargo package` and ensure it succeeds.
* Push changes to GitHub, NOT including the tag. (But do not publish a new
  version of ripgrep to crates.io yet.)
* Once CI for `master` finishes successfully, push the version tag. (Trying to
  do this in one step seems to result in GitHub Actions not seeing the tag
  push and thus not running the release workflow.)
* Wait for CI to finish creating the release. If the release build fails, then
  delete the tag from GitHub, make fixes, re-tag, delete the release and push.
* Copy the relevant section of the CHANGELOG to the tagged release notes.
  Include this blurb describing what ripgrep is:
  > In case you haven't heard of it before, ripgrep is a line-oriented search
  > tool that recursively searches the current directory for a regex pattern.
  > By default, ripgrep will respect gitignore rules and automatically skip
  > hidden files/directories and binary files.
* Run `cargo publish`.
* Run `ci/sha256-releases {VERSION} >> pkg/brew/ripgrep-bin.rb`. Then edit
  `pkg/brew/ripgrep-bin.rb` to update the version number and sha256 hashes.
  Remove extraneous stuff added by `ci/sha256-releases`. Commit changes.
* Add TBD section to the top of the CHANGELOG:
  ```
  TBD
  ===
  Unreleased changes. Release notes have not yet been written.
  ```

Note that [`cargo-up` can be found in BurntSushi's dotfiles][dotfiles].

[dotfiles]: https://github.com/BurntSushi/dotfiles/blob/master/bin/cargo-up
