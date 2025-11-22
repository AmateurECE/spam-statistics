# Build Workflow

My build and test workflow involves three terminals:

1. One with a bitbake devshell, created with `kas shell -c "bitbake
   spam-statistics && bitbake -c devshell spam-statistics"`. The first build is
   important, because it creates the `do_compile` script necessary to build
   within the devshell. Typically, I also need to manually edit the
   `run.do_compile` script to remove the `--frozen` flag passed to cargo.
2. One with the sources open. Just `cd
   build/tmp/work/<platform>/spam-statistics/<version>/sources/`
3. One with an SSH shell open to the platform.

Typically, I will also use the Neovim integrated shell to copy the build
artifact over to the target.
