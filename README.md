# mudup

`mudup` is the installer and version manager for MuduDB binaries.

## Install

Install `mudup` from its release artifact, then run:

```bash
curl --proto '=https' --tlsv1.2 -fsSL "https://github.com/scuptio/mudup/releases/download/latest/mudup-init.sh" | sh
mudup --help
```

Use another repository:

```bash
curl --proto '=https' --tlsv1.2 -fsSL "https://github.com/scuptio/mudup/releases/download/latest/mudup-init.sh" | sh -s -- <owner>/<repo>
```

Download link templates:

```text
https://github.com/<owner>/<repo>/releases/download/latest/mudup-init.sh
https://github.com/<owner>/<repo>/releases/download/latest/mudup-x86_64-unknown-linux-gnu.tar.gz
```

`mudup-init.sh` downloads the latest `mudup` binary, verifies SHA256, installs it to `${HOME}/.local/bin`, and updates
PATH for both current shell and `${HOME}/.bashrc`.

## Usage

Install and activate the latest MuduDB:

```bash
mudup install
```

Install a specific version:

```bash
mudup install v20260514.1144
```

Update to the latest release:

```bash
mudup update
```

Update `mudup` itself:

```bash
mudup self update
```

List installed versions:

```bash
mudup list
```

Uninstall one version:

```bash
mudup uninstall <version>
```

## Configuration and Paths

- `MUDUP_HOME` can override the default install root.
- Default install root: `${HOME}/.mududb`
- Tool proxies: `${HOME}/.mududb/bin`
- Server config file: `${HOME}/.mududb/mududb_cfg.toml`

After install, if `${HOME}/.mududb/bin` is not in `PATH`, `mudup` prints copy-paste commands for current shell and
persistent bash setup.
