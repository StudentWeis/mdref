**_mdref_**

[![crates.io](https://img.shields.io/crates/v/mdref.svg)](https://crates.io/crates/mdref)

Markdown Reference finding and migration tool, written in Rust.

# Basic Usage

## Install & Update

Install prebuilt binaries via shell script：

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/studentweis/mdref/releases/download/0.1.1/mdref-installer.sh | sh
```

Install prebuilt binaries via powershell script

```sh
powershell -ExecutionPolicy Bypass -c "irm https://github.com/studentweis/mdref/releases/download/0.1.1/mdref-installer.ps1 | iex"
```

Update mdref:

```sh
mdref-update
```

## Basic command

- find：🔥
- mv：🚧

# Acknowledge

- clap
- walkdir
- regex
- cargo-dist

# Todo

[ ] Fix the case of link path with space.
