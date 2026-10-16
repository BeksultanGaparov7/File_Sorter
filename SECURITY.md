# Security Policy

## Supported versions

Only the latest release gets security fixes.

## Reporting a vulnerability

Please **do not open a public issue** for security problems.

Report them privately through GitHub instead:
**[Report a vulnerability](https://github.com/BeksultanGaparov7/File_Sorter/security/advisories/new)**

Please include what you found, how to reproduce it, and what impact you think it has.
You should get a reply within a week. Once a fix is released, the report is published
with credit to you, unless you'd rather stay anonymous.

## Scope

file-sorter runs as your user and moves or trashes files in your Downloads folder. Things
worth reporting include:

- a crafted file name or download that makes it move, overwrite or delete files outside the configured folders
- following symlinks out of the watched directory
- command injection through file names (via `xdg-open` / `xdg-terminal-exec` / `mv`)
