<div align="center" >
  <img width="182" src="resources/icons/hicolor/scalable/apps/dev.lunarsrl.NovaMusic.svg">
  <h1>Nova Music</h1>
  <p>A music player written with the libcosmic toolkit</p>
</div>

<div align="center">
    <img width="600" src="https://lunarsrl.dev/static/images/NovaMusic/NowPlaying.png">
</div>

## Contents:

- [Installation](#installation)
- [Updates](#updates)
- [Todo](#todo)
- [Translators](#translators)
- [Screenshots](#screenshots)

## Installation

> [!NOTE]
> This project is still a work in progress

#### Flathub:
<a href='https://flathub.org/apps/dev.lunarsrl.NovaMusic'>
  <img width='240' alt='Get it on Flathub' src='https://flathub.org/api/badge?locale=en'/>
</a>

#### Arch Linux:

Thank you to [@rwinkhart](https://github.com/rwinkhart) for publishing the AUR Package!  
AUR: https://aur.archlinux.org/packages/nova-music-git

#### Compile it yourself for any platform:

A [justfile](./justfile) is included by default for the [casey/just][just] command runner.

- `just` builds the application with the default `just build-release` recipe
- `just run` builds and runs the application
- `just install` installs the project into the system
- `just vendor` creates a vendored tarball
- `just build-vendored` compiles with vendored dependencies from that tarball
- `just check` runs clippy on the project to check for linter warnings
- `just check-json` can be used by IDEs that support LSP

## Updates

- Added artists view
- UI changes accross most pages
- Released on flatpak

## Todo:

#### Important:
- Exploring CPAL instead of Rodio
- Gapless Playback
- MPRIS support
- Crossfade support

#### Other:
- Shuffle options
- Optional network features (ex: Musicbrainz integration for automatic metadata)
- Genres

#### Completed:
- ~~Artist View~~
- ~~Basic Keybinds~~
- ~~[ ! ] Replace current playlist implementation with m3u~~
- ~~Playlist updates: Allow M3U import from inside app, allow custom cover art, delete playlists from inside app~~
- ~~Localization update~~
- ~~Flatpak release~~

## Translators

[Fluent][fluent] is used for localization of the software. Fluent's translation files are found in
the [i18n directory](./i18n). New translations may copy the [English (en) localization](./i18n/en) of the project,
rename `en` to the desired [ISO 639-1 language code][iso-codes], and then translations can be provided for
each [message identifier][fluent-guide]. If no translation is necessary, the message may be omitted.

## Screenshots

<img src="https://lunarsrl.dev/static/images/NovaMusic/NowPlaying.png">
<img src="https://lunarsrl.dev/static/images/NovaMusic/Album.png">
<img src="https://lunarsrl.dev/static/images/NovaMusic/Tracks.png">


[fluent]: https://projectfluent.org/

[fluent-guide]: https://projectfluent.org/fluent/guide/hello.html

[iso-codes]: https://en.wikipedia.org/wiki/List_of_ISO_639-1_codes

[just]: https://github.com/casey/just

[rustup]: https://rustup.rs/

[rust-analyzer]: https://rust-analyzer.github.io/

[mold]: https://github.com/rui314/mold

[sccache]: https://github.com/mozilla/sccache
