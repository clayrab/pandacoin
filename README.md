
# Mothballed 

THIS IS A VERY EARLY VERSION OF LITHEUMCORE AND HAS BEEN DEPRECATED IN FAVOR OF A NEW REPOSITORY IN EARLY 2022

# Welcome

Welcome to PandaCoin.

PandaCoin is a generation 3 blockchain designed using Economic First Principles which creates a chain that is both highly scalable and fully decentralized.

More details will be shared here as the community is built and the codebase begins to approach test-net.

If you're intrested in contributing at this stage, you'll need to reach out to me directly.

clayton rabenda at gmail dot com.

# Contributing

- PRs should be made against the `main` branch
- PRs must pass [rust fmt (via Github Actions)](README.md#github-actions) check and should have full test coverage

## Tools

- [Convo](https://convco.github.io/check/)  
  To check your commits according to the format it can be [installed locally](https://convco.github.io/#installation)

### Commit format

We use [Conventional Commits](https://www.conventionalcommits.org) for commit messages.

In short a commit message should follow this format:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

For example:

```
feat(keypair): add format check
```

- For the full specification please have a look at https://www.conventionalcommits.org
- Reasons to use this format can be found in this post: https://marcodenisi.dev/en/blog/why-you-should-use-conventional-commits

#### Commit `type`

Most used types:

- `fix`
- `feat`

Further recommended types:

- `build`
- `chore`
- `ci`
- `docs`
- `style`
- `refactor`
- `perf`
- `test`

#### Issue numbers in a commit

If the commit relates to an issue the issue number can be added to the commit-`descrition` or -`body`, i.e.:

```
feat(keypair): add format check #123
```

### Deps

- (If on OSX: `xcode-select --install`)
- [Install Rust](https://www.rust-lang.org/tools/install)

```
rustup component add rustfmt
```

### Run the node

```
RUST_LOG=debug cargo run
```

Possible log levels are Error, Warn, Info, Debug, Trace.

#### Help/Options

WORK IN PROGRESS

```
cargo run -- --help
```

Will list possible options, i.e.

```bash
OPTIONS:
    -k, --key-path <key-path>    Path to key-file [default: ./keyFile]
```

### Tests

```
cargo test
```

### Code formatting

```
cargo fmt
```

Format code according to the [Rust style Guide](https://github.com/rust-dev-tools/fmt-rfcs/blob/master/guide/guide.md).


### Github Actions

GH Actions are located here: [.github/workflows](.github/workflows)

- cargo docs  
  Is creating and deploying the docs to GH pages

- [rustfmt](https://github.com/rust-lang/rustfmt#checking-style-on-a-ci-server) (**required**)  
  Is checking if the code is formatted according to rust style guidelines

- cargo build & test  
  Tries to build the code and run all tests

- [Convco](https://convco.github.io/check/) commit format check (**required**)  
  Check all commits or range for errors against [the convention](CONTRIBUTING.md#commit-format)

### VSCode

Extensions:

- https://github.com/rust-lang/vscode-rust

## Create release

```
cargo build --release
```
