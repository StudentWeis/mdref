**_mdref_**

[![crates.io](https://img.shields.io/crates/v/mdref.svg)](https://crates.io/crates/mdref)

A fast, Rust-based tool for discovering and migrating Markdown references — it processed 155 directories and 1,561 files in just 1 ms.

> [!CAUTION]
> This project is still in early development, and some features may not be fully functional. Please use it with caution and report any issues you encounter.

# Basic Usage

## Install & Update

Install prebuilt binaries via shell script：

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/studentweis/mdref/releases/download/0.3.5/mdref-installer.sh | sh
```

Install prebuilt binaries via powershell script

```sh
powershell -ExecutionPolicy Bypass -c "irm https://github.com/studentweis/mdref/releases/download/0.3.5/mdref-installer.ps1 | iex"
```

Update mdref:

```sh
mdref-update
```

## Basic command

- find 🔍：Find all markdown references in the specified directory.
- mv 🔥：Move file and update markdown references.
- rename 🔄：Rename file and update markdown references.

```sh
$ mdref find ./examples/main.md

References to ./examples/main.md:
./examples/inner/sub/other.md:3:1 - ../../main.md
./examples/other.md:7:1 - main.md
./examples/inner/other.md:3:1 - ../main.md
./examples/main.md:7:1 - main.md
./examples/inner/sub/main.md:3:1 - ../../main.md
./examples/inner/main.md:3:1 - ../main.md
Links in ./examples/main.md:
./examples/main.md:3:2 - main.jpg
./examples/main.md:5:2 - main.jpg
./examples/main.md:7:1 - main.md
./examples/main.md:7:25 - inner/main.md
./examples/main.md:7:55 - inner/sub/main.md
./examples/main.md:9:1 - other.md
./examples/main.md:9:27 - inner/other.md
./examples/main.md:9:59 - inner/sub/other.md
```

# Todo

- [ ] Fix the case of link path with space.
- [ ] Preview mode of mv command.
- [ ] More tests.
- [ ] More documentations.
- [ ] Cargo-dist oranda homepage.
- [ ] VSCode extension.

# Acknowledge

- clap
- walkdir
- pathdiff
- regex
- rayon
- cargo-dist

# Contributing

If you want to submit code to this repository, please first refer to [CONTRIBUTING](./CONTRIBUTING).

Thanks for your help!
