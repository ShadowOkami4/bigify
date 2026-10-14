**Bigify** is a 10-foot, controller-first Spotify Connect player for Linux TVs, Steam Machines and Steam Deck.

> Preview release. Requires Spotify Premium. Bigify is an independent open-source project, not affiliated with Spotify.

### Install on Steam Deck / SteamOS

1. Switch to **Desktop Mode** and download `bigify-…-x86_64.flatpak` below.
2. Open **Konsole** in your Downloads folder and run:
   ```sh
   flatpak install --user ./bigify-*-x86_64.flatpak
   ```
3. In Steam (Desktop Mode): **Add a Game → Add a Non-Steam Game → Bigify**, then return to Gaming Mode.

Full guide, including other distributions and updating: [docs/INSTALL.md](https://github.com/ShadowOkami4/bigify/blob/main/docs/INSTALL.md)

### Highlights

- Sign in from the Spotify app on your phone (no typing on the TV), or with a browser
- Home, Search, Library, artist, album and playlist pages; save and follow
- Expressive Now Playing with colors from the album art; Up next queue
- Full controller support; media keys and desktop widgets via MPRIS

See [LIMITATIONS.md](https://github.com/ShadowOkami4/bigify/blob/main/docs/LIMITATIONS.md) for what Spotify's interfaces do and don't allow.

Verify the download: `sha256sum -c bigify-*-x86_64.flatpak.sha256`
