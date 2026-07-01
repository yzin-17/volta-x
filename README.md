<p align="center">
  <a href="https://github.com/yzin-17/volta-x">
    <img alt="Volta" src="./volta.png?raw=true" width="360">
  </a>
</p>

<p align="center">
  Volta-X: The Hassle-Free JavaScript Tool Manager
</p>

<p align="center">
  <a href="https://github.com/yzin-17/volta-x/actions/workflows/release.yml">
    <img alt="Release Build Status" src="https://github.com/yzin-17/volta-x/actions/workflows/release.yml/badge.svg" />
  </a>
  <a href="https://github.com/yzin-17/volta-x/actions/workflows/test.yml">
    <img alt="Test Status" src="https://github.com/yzin-17/volta-x/actions/workflows/test.yml/badge.svg" />
  </a>
</p>

---

> [!IMPORTANT]
> **Volta-X is a maintained fork of Volta.** It keeps Volta's fast toolchain workflow while adding explicit defaults, directory-level tool versions, `.nvmrc` / `.node-version` switching, and fork-owned release artifacts.

---


**Fast:** Install and run any JS tool quickly and seamlessly! Volta is built in Rust and ships as a snappy static binary.

**Reliable:** Ensure everyone in your project has the same tools—without interfering with their workflow.

**Universal:** No matter the package manager, Node runtime, or OS, Volta-X keeps the same fast workflow with explicit control over defaults.

## Features

- Speed 🚀
- Seamless, per-project version switching
- Cross-platform support, including Windows and all Unix shells
- Support for multiple package managers
- Stable tool installation—no reinstalling on every Node upgrade!
- Extensibility hooks for site-specific customization

## Installing Volta

Install Volta-X from this fork's GitHub Releases:

```sh
curl https://raw.githubusercontent.com/yzin-17/volta-x/main/dev/unix/boot-install.sh | bash
```

The installer downloads release artifacts from `https://github.com/yzin-17/volta-x/releases`. Set `VOLTA_REPO=owner/repo` to use release artifacts from another fork.

## Using Volta

Use `volta install` to fetch tool versions. The first installed version of a tool becomes the default; later installs keep the existing default instead of silently replacing it.

```sh
volta install node@18
volta install node@20
```

Use `volta default` when you want to change the global default to an already-installed version. It does not download missing tools, so install the version first if it is not local yet.

```sh
volta default node@20
volta default npm@bundled
volta default yarn@1.22.22
```

Use `volta use` to set local tool versions for the current directory and its children without editing project files.

```sh
volta use node@18 pnpm@7.7.1
volta use list
volta unuse node
volta unuse --dir ../project-a node
volta unuse --all
```

Version selection is resolved in this order: `volta use`, `volta pin`, `.nvmrc`, `.node-version`, then the global default. `.nvmrc` and `.node-version` affect Node only; package managers can still inherit from lower-priority sources.

Use `volta uninstall <tool>@<version>` to remove a specific installed Node, npm, pnpm, or Yarn version when it is not referenced by defaults, the current project, or a `volta use` directory mapping.

## Contributing to Volta-X

Contributions are always welcome, no matter how large or small. Before contributing, please read the [code of conduct](CODE_OF_CONDUCT.md).

## Who is using Volta?

<table>
  <tbody>
    <tr>
      <td align="center">
        <a href="https://github.com/microsoft/TypeScript" target="_blank">
          <img src="https://raw.githubusercontent.com/microsoft/TypeScript-Website/v2/packages/typescriptlang-org/static/branding/ts-logo-512.svg" alt="TypeScript" width="100" height="100">
        </a>
      </td>
      <td align="center">
        <a href="https://github.com/getsentry/sentry-javascript" target="_blank">
          <img src="https://avatars.githubusercontent.com/u/1396951?s=100" alt="Sentry" width="100" height="100">
        </a>
      </td>
    </tr>
  </tbody>
</table>

See [here](https://sourcegraph.com/search?q=context:global+%22volta%22+file:package.json&patternType=literal) for more Volta users.
