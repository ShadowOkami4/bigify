// SPDX-License-Identifier: GPL-3.0-or-later
#include "bigify/cpp/input.h"

#include <QGuiApplication>
#include <QKeyEvent>
#include <QWindow>

namespace {

// Keep in sync with `Action` in src/input.rs and docs/CONTROLS.md.
int keyForAction(std::int32_t action)
{
    switch (action) {
    case 0: return Qt::Key_Up;
    case 1: return Qt::Key_Down;
    case 2: return Qt::Key_Left;
    case 3: return Qt::Key_Right;
    case 4: return Qt::Key_Return;            // A / Cross: activate
    case 5: return Qt::Key_Escape;            // B / Circle: back
    case 6: return Qt::Key_MediaTogglePlayPause; // X / Square
    case 7: return Qt::Key_Menu;              // Y / Triangle: more actions
    case 8: return Qt::Key_MediaPrevious;     // LB
    case 9: return Qt::Key_MediaNext;         // RB
    case 10: return Qt::Key_VolumeDown;       // LT
    case 11: return Qt::Key_VolumeUp;         // RT
    case 12: return Qt::Key_F2;               // Start: Now Playing
    case 13: return Qt::Key_F3;               // Select/View: Queue
    default: return 0;
    }
}

} // namespace

void bigifyPostAction(std::int32_t action, bool pressed, bool autorepeat)
{
    const int key = keyForAction(action);
    if (key == 0)
        return;
    QWindow *window = QGuiApplication::focusWindow();
    if (!window) {
        // Gamescope (Steam gaming mode) may not report a focus window.
        const auto windows = QGuiApplication::topLevelWindows();
        for (QWindow *w : windows) {
            if (w->isVisible()) {
                window = w;
                break;
            }
        }
    }
    if (!window)
        return;
    QGuiApplication::postEvent(
        window,
        new QKeyEvent(pressed ? QEvent::KeyPress : QEvent::KeyRelease, key, Qt::NoModifier,
                      QString(), autorepeat));
}
