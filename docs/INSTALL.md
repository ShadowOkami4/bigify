# Installing Bigify

Bigify ships as a Flatpak bundle on the
[Releases page](https://github.com/ShadowOkami4/bigify/releases). It needs a
Spotify **Premium** account and an x86_64 Linux system with Flatpak.

## Steam Deck / SteamOS

### 1. Install (Desktop Mode)

1. Hold the **power button** → **Switch to Desktop**.
2. Open the [latest release](https://github.com/ShadowOkami4/bigify/releases/latest)
   in a browser and download `bigify-<version>-x86_64.flatpak`.
3. Open the **Dolphin** file manager, go to **Downloads**, right-click empty
   space → **Open Terminal Here**, and run:

   ```sh
   flatpak install --user ./bigify-*-x86_64.flatpak
   ```

   Confirm with `y`. On the first install this also downloads the KDE
   runtime (about 1 GB, once).

   *Alternative:* double-clicking the file opens it in **Discover**, which
   installs it the same way.

4. Test it: Application Launcher → **Multimedia → Bigify**.

### 2. Add to Steam (for Gaming Mode)

1. Open **Steam** in Desktop Mode.
2. **Games → Add a Non-Steam Game to My Library…**
3. Tick **Bigify** and click **Add Selected Programs**.
4. Return to Gaming Mode (desktop shortcut **Return to Gaming Mode**).

Bigify is now in your Library under **Non-Steam**.

### 3. Controller layout

In Gaming Mode, select Bigify → ⚙ → **Controller Settings** and choose the
**Gamepad** template (Steam may pick *Keyboard (WASD) and Mouse* for
non-Steam games). Bigify reads the controller directly:

| Button | Action |
|---|---|
| D-pad / left stick | Move |
| A | Select |
| B | Back |
| X | Play / pause |
| Y | Add focused track to queue |
| L1 / R1 | Previous / next track |
| L2 / R2 | Volume |
| ☰ (Menu) | Now Playing |
| ⧉ (View) | Up next |

If buttons trigger twice, turn off *Settings → Controls → Read controllers
directly* in Bigify. Full details: [CONTROLS.md](CONTROLS.md).

### 4. Sign in

On first start, Bigify shows the sign-in screen:

- **With your phone (recommended):** phone and Deck on the same Wi-Fi →
  open Spotify → start any song → tap the devices icon → choose **Bigify**.
- **With a browser:** in Desktop Mode choose *Sign in with a browser on this
  device* and log in. (Gaming Mode has no browser, so use your phone there.)

Bigify remembers the sign-in. *Settings → Sign out* removes it.

### 5. Library access (Home, Search, Library) — one-time setup

Playback works right after signing in. To see **Home, Search and your
Library**, Bigify needs a Spotify app registered by you: Spotify blocks
third-party apps from using its own built-in access ("Spotify is busy").
It's free and takes about three minutes. Do it in **Desktop Mode**:

1. Start Bigify, go to **Settings → Library access → Set up library
   access**. Your browser opens a Bigify setup page.
2. Follow the steps on that page:
   - Open the [Spotify Developer Dashboard](https://developer.spotify.com/dashboard)
     and log in with your Spotify account.
   - **Create app** — name `Bigify`, any description.
   - **Redirect URI:** `http://127.0.0.1:8899/callback` → **Add**.
   - Tick **Web API**, accept the terms, **Save**.
   - Copy the app's **Client ID** and paste it into the setup page →
     **Continue to Spotify** → **Agree**.
3. The page says *Done* and Bigify shows *Library access connected*. Home,
   Search and Library now load — also in Gaming Mode.

Tip: Steam + X opens the on-screen keyboard in Desktop Mode.

Bigify stores only a revocable access token
(`~/.var/app/io.github.ShadowOkami4.bigify/.local/state/bigify/webapi-token.json`,
readable only by you). Remove it with *Settings → Remove library access*, or
revoke it in your Spotify account under *Manage apps*.

### Update

Download the new release and run the same command; Flatpak replaces the
installed version and keeps your settings and sign-in:

```sh
flatpak install --user --reinstall ./bigify-*-x86_64.flatpak
```

### Uninstall

```sh
flatpak uninstall --user io.github.ShadowOkami4.bigify
# also delete settings, sign-in and cache:
rm -rf ~/.var/app/io.github.ShadowOkami4.bigify
```

Remove the Steam shortcut via right-click → Manage → Remove non-Steam game.

## Other Linux distributions

Any distribution with Flatpak works (Fedora, Ubuntu, Debian, Arch, openSUSE,
Bazzite, ChimeraOS, …). If Flathub isn't set up yet:

```sh
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user ./bigify-*-x86_64.flatpak
flatpak run io.github.ShadowOkami4.bigify
```

## Troubleshooting

| Problem | Fix |
|---|---|
| Home/Search/Library: "Spotify is limiting Bigify's built-in library access" | Do the one-time [library access setup](#5-library-access-home-search-library--one-time-setup). |
| Setup page says the Client ID is invalid | Copy the **Client ID** (32 characters), not the Client secret. |
| Spotify shows "INVALID_CLIENT: Invalid redirect URI" | The app's Redirect URI must be exactly `http://127.0.0.1:8899/callback`. |
| "Bigify" doesn't appear in the phone's device list | Same Wi-Fi network? Some routers block device discovery (mDNS/"client isolation"); use browser sign-in instead. |
| No sound | Bigify uses the system's default output. Check the volume/output in SteamOS quick settings. |
| Controller does nothing | Use the **Gamepad** controller template; make sure *Read controllers directly* is on. |
| Something else | Attach `~/.var/app/io.github.ShadowOkami4.bigify/.local/state/bigify/bigify.log` to a [bug report](https://github.com/ShadowOkami4/bigify/issues). It contains no passwords or tokens. |

Building from source instead: [DEVELOPMENT.md](DEVELOPMENT.md).
