# snipped

`<these shields will be active later>`

[![MIT License](https://img.shields.io/badge/license-MIT-blue)](LICENSE-MIT)
[![crates.io](https://img.shields.io/crates/v/snipped)](https://crates.io/crates/snipped)
[![download](https://img.shields.io/crates/d/snipped)](https://crates.io/crates/snipped)
[![docs.rs](https://docs.rs/snipped/badge.svg)](https://docs.rs/snipped)

![Wayland](https://img.shields.io/badge/scan_on_wayland-slurp/grim-000000.svg?style=flat)
![X11](https://img.shields.io/badge/scan_on_x11-slop/imagemagick-000000.svg?style=flat)
![Windows 10+](https://img.shields.io/badge/scan_on_windows-10+-000000.svg?style=flat)

_What if I wanted to use the clipboard anyway?_

This application allows you to paste text into windows that do not support using clipboard (e.g.
some non-graphical VMs that do not support shared clipboard), as well as copy some text from them
via OCR.

Also check out [some existing snippets by the author](https://github.com/kirillsemyonkin/snipped-snippets) (and help make them better).

## Installation

There are some dependencies needed to install for this application:

`<currently only windows support for "scan" cmd>`

For scan subcommand (just have following commands in your `PATH`):

- For all: [`tesseract-ocr`](https://tesseract-ocr.github.io/tessdoc/Installation.html)
  (`<might be replaced by a custom/built-in monospace font OCR later>`)
- For Wayland: [`slurp`](https://wayland.emersion.fr/slurp/), [`grim`](https://wayland.emersion.fr/grim/)
- For X11: [`slop`](https://github.com/naelstrof/slop), [`imagemagick`](https://imagemagick.org/)
- For Windows 10+: `explorer ms-screenclip:` (no extra install needed, but make sure it works)

Install `snipped` via the Cargo system from this repository
([Install Rust](https://www.rust-lang.org/tools/install) first):

```sh
cargo install --git https://github.com/kirillsemyonkin/snipped.git
```

Quick Linux setup:

```sh
# apt-based: apt update
#   wayland: apt install slurp grim tesseract-ocr
#       x11: apt install slop imagemagick tesseract-ocr
#      rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# pacman-based: pacman -Sy
#      wayland: pacman -S rustup slurp grim tesseract
#          x11: pacman -S rustup slop imagemagick tesseract

rustup install stable
cargo install snipped
```

## Usage

If you do not specify anything, you will be asked what you want to do in a wizard-like manner. This
prevents you from typing some extra options, but helps you learn how to use the application without
reading the documentation below.

### Pasting text

```text
snipped paste (p)
    [--no-processing/-n]
    [-o=<output-target>/--output=<output-target>]
    <input-target>
    [<arg-key-1> <arg-value-1> ...]

-n, --no-processing
    do not process `$@...$` and others, leaving the target snippet as-is

-o=<output-target>, --output=<output-target>
    set the target to paste the snippet into
    when unset, the program emulates pressing alt+tab and strokes to type the resulting snippet
    -
        print the resulting interpolated snippet
    !
        write the results to clipboard
    <file-path>
        write the resulting interpolated snippet to a file

<input-target>
    the target snippet to paste
    when using remote targets, you will be warned and asked to type randomly generated keywords
    -
        read from stdin
    !
        read from current clipboard
    gist:<gist-id>, gist:<user>/<file-name>
        download a github gist (pre-downloading it as a local file instead is recommended)
    github:<user>/<repo>/<filepath>, github:<user>/<repo>/<filepath>#<branch>
        download a github file (using local files via `git clone` instead is recommended)
    https://<url>, http://<url>
        download via url (pre-downloading it as a local file instead is recommended)
    <file-path>
        read from a local file

<arg-key-1> <arg-value-1> ...
    arguments to interpolate into the snippet
```

Pasting works by typing out every key using the [enigo](https://crates.io/crates/enigo) library. You
input a target snippet into the program - a regular text file with some special additions:

- Arguments will be interpolated into the snippet file, allowing you to put things like IP addresses
  for actual machines into it. An argument begins with the `$@` combination and ends with the `$`
  character. The arguments with same name will be replaced with the same value.

  If you do not specify a value for an argument, the program will ask you to fill it in. An empty
  value will be replaced by a default value, which you can specify after a `::` in the argument
  content:

  ```text
  echo $@Message::Hello world!$
  ```

  The example above will ask the user the `Message` argument, defaulting to `Hello world!`:

  ```text
  $ snippet paste -o=- - 
  echo $@Message::Hello world!${ENTER}{CTRL+D}
  Message (Default: `Hello world!`): {ENTER}
  echo Hello world!
  ```

- Argument lists (Arglists `$[`) are the same as previous, except the line containing them will be
  repeated for every argument value that user inputs, until user presses the enter key. For usage as
  a command parameter, this involves repeating the key-value pair multiple times.

- The `$!` combination switches from the text mode to the manual mode: you will have to tell the
  program to press all the necessary keys instead. The keys will be pressed until the end (`$`), and
  then released in reverse order. If the key is already pressed, it will be toggled to release
  state and forgotten (so it will not try to release or press a key that is set to released).

  Here is an example of a snippet doing `alt+tab`, and while holding `alt`, doing another `tab`:

  ```text
  $!Alt+Tab+Tab+Tab$
  ```

  The `Alt` key will be pressed, `Tab` will be pressed, released, pressed again, and then the `$`
  ends with releasing all currently held keys (first `Tab`, then `Alt`).

- The `$'` combination will introduce delay in milliseconds, e.g. `$'1000$` is 1 second.

- Processed comments are written with `##` at the beginning of the line. With a single `#` they are
  printed into the output as normal text. The arguments written in the comment will still be asked,
  so you can use comments to order the arguments and to give each snippet a source-code explanation.

### Copying text

```text
snipped scan (s)
    [-i=<input-target>/--input=<input-target>]
    [-y, --overwrite]
    <output-target>

-i=<input-target>, --input=<input-target>
    set the target image to scan from
    when unset, the program will use applications to take a clip region for a screenshot
        on windows, this uses the `explorer ms-screenclip:` command
        on wayland, this uses the `slurp` to select a region and `grim` to screenshot that region
        on x11, this uses the `slop` to select a region and imagemagick's `import` to screenshot
    -
        read image file from stdin
    !
        read bitmap from current clipboard
    <file-path>
        read from a local file

-y, --overwrite
    overwrite the output file (if it is a file) if it already exists

<output-target>
    set the target to put the resulting snippet into
    -
        print the resulting snippet instead
    !
        put the resulting snippet into the clipboard
    <file-path>
        write the resulting snippet to a file
```

The command will execute your system's command to take a screenshot of an area of your screen, which
then will be processed using OCR to get some text from it. This lets you get text from program
output from the VMs that do not support sharing clipboard with host.

There is no further processing on the text (like interpolation in previous mode, or autocorrection,
so you will have to fix any OCR problems yourself), so there is not much else to document here.
