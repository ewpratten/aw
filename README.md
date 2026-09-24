# Agentic Workspace

This repository contains `aw`, the tool that powers 90% of my agentic work.

I designed `aw` to help me orchestrate large numbers of "what if" scenarios across projects without interrupting my main work.

## Key features

- Tight integration into [Jujutsu VCS](https://jj-vcs.dev/) (my VCS of choice)
- Streamligned command-line argument parsing to quickly open projects from various dirs across my system
- Some Tmux trickery to make sessions easy to transfer to-and-from my phone while on the go

## Usage

`aw` can be configured with a TOML file. An example would be:

```toml
# ~/Library/Application Support/aw/config.toml

# List directories that contain your local JJ repos here
project_dirs = ["~/projects", "~/src"]

# If you have any repos that live outside of your usual work area, add them directly here
extra_repos = ["/external/repo"]
```

Then, if you wanted to start a new session called `my-idea` in `~/projects/my-repo`, you'd run:

```sh
aw my-repo my-idea
```

Maybe you organize things by "org" if you work with something like Bitbucket:

```sh
aw TEAM/our-repo fix-that-bug
```

Periodically, make sure to clean up any stale sessions with:

```sh
aw gc
```

## Contributions

This repository is a public push-mirror of a segment of my personal projects monorepo. As such, pull requests are not accepted here.

If you would like to contribute a patch, please email me at the address [on my website](https://ewpratten.com).

