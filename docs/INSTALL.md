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
| "Bigify" doesn't appear in the phone's device list | Same Wi-Fi network? Some routers block device discovery (mDNS/"client isolation"); use browser sign-in instead. |
| No sound | Bigify uses the system's default output. Check the volume/output in SteamOS quick settings. |
| Controller does nothing | Use the **Gamepad** controller template; make sure *Read controllers directly* is on. |
| Something else | Attach `~/.var/app/io.github.ShadowOkami4.bigify/.local/state/bigify/bigify.log` to a [bug report](https://github.com/ShadowOkami4/bigify/issues). It contains no passwords or tokens. |

Building from source instead: [DEVELOPMENT.md](DEVELOPMENT.md).
