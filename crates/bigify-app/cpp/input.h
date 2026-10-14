// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once

#include <cstdint>

// Deliver a navigation action to the focused window as a key event, so that
// controllers, remotes and keyboards share exactly one focus/navigation path
// in QML. `action` is a bigify::input::Action value (see src/input.rs).
// Must be called on the GUI thread.
void bigifyPostAction(std::int32_t action, bool pressed, bool autorepeat);
