# Agent Instructions

## Releases

When the user asks to create and push a new release or release tag, create release notes before tagging or pushing.

Release notes live in `docs/official/releases/` and use one file per Git tag:

```text
docs/official/releases/vX.Y.Z.md
```

The file name must match the Git tag exactly. The GitHub release workflow extracts the release body from the matching docs file, so the content shown in the docs and in GitHub Releases stays in parity.

Include a direct link to the matching GitHub release URL:

```text
https://github.com/BrunoV21/FerrisGrid-CLI/releases/tag/vX.Y.Z
```

Use this structure:

````md
---
title: FerrisGrid vX.Y.Z
description: Release notes for FerrisGrid vX.Y.Z.
---

# FerrisGrid vX.Y.Z

<!-- release-notes:start -->

[GitHub release](https://github.com/BrunoV21/FerrisGrid-CLI/releases/tag/vX.Y.Z)

### Highlights

- Added ...
- Fixed ...

### Install

```sh
cargo install ferrisgrid-cli --version X.Y.Z
```

<!-- release-notes:end -->
````

Only content between `<!-- release-notes:start -->` and `<!-- release-notes:end -->` is published as the GitHub release body.

Do not manually edit `docs/official/releases/index.md` for each release. It dynamically reads every `v*.md` file in `docs/official/releases/` and renders a scrollable release page.

After drafting the release notes and before tagging or pushing, check the TypeScript wrapper in `../FerrisGrid-CLI-ts` and update it as needed so it covers the same specification and behavior as the Rust package release.
