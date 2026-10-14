// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once

#include <cstdint>

#include <QtCore/QObject>

// Deliver a navigation action to the focused window as a key event, so that
// controllers, remotes and keyboards share exactly one focus/navigation path
// in QML. `action` is a bigify::input::Action value (see src/input.rs).
// Must be called on the GUI thread.
void bigifyPostAction(std::int32_t action, bool pressed, bool autorepeat);

// Watch all input reaching the application and keep `target`'s `inputMode`
// property at "gamepad" (events from bigifyPostAction) or "keyboard"
// (physical keys, mouse clicks/movement, wheel, touch). QML uses it to show
// matching button hints. Must be called on the GUI thread.
void bigifyInstallInputModeFilter(QObject &target);
