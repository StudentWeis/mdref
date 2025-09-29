**_mdref_**

[![crates.io](https://img.shields.io/crates/v/mdref.svg)](https://crates.io/crates/mdref)

Markdown Reference finding and migration tool which processed 155 directories with 1561 files in just 1ms, written in Rust

> [!CAUTION]
> This project is still in early development, and some features may not be fully functional. Please use it with caution and report any issues you encounter.

# Basic Usage

## Install & Update

Install prebuilt binaries via shell script：

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/studentweis/mdref/releases/download/0.3.2/mdref-installer.sh | sh
```

Install prebuilt binaries via powershell script

```sh
powershell -ExecutionPolicy Bypass -c "irm https://github.com/studentweis/mdref/releases/download/0.3.2/mdref-installer.ps1 | iex"
```

Update mdref:

```sh
mdref-update
```

## Basic command

- find 🔍：Find all markdown references in the specified directory.
- mv 🔥：Move file and update markdown references.
- rename 🔄：Rename file and update markdown references.

# Todo

- [ ] Fix the case of link path with space.
- [ ] Preview mode of mv command.
- [ ] More tests.
- [ ] More documentations.
- [ ] Error handling.
- [ ] Cargo-dist oranda homepage.

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
