// SPDX-License-Identifier: GPL-3.0-or-later
#include "bigify/cpp/input.h"

#include <QGuiApplication>
#include <QKeyEvent>
#include <QMouseEvent>
#include <QPointer>
#include <QWindow>

namespace {

// Marks key events synthesised from controller input, so the input-mode
// filter can tell them apart from a physical keyboard.
constexpr quint32 kGamepadScanCode = 0xB16F;

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

class InputModeFilter : public QObject
{
public:
    explicit InputModeFilter(QObject *target)
        : QObject(target)
        , m_target(target)
    {
    }

protected:
    bool eventFilter(QObject *watched, QEvent *event) override
    {
        // Only look at events delivered to windows; Qt forwards the same
        // event to items afterwards, so this sees each input once.
        if (!watched->isWindowType())
            return false;
        switch (event->type()) {
        case QEvent::KeyPress:
        case QEvent::MouseButtonPress:
        case QEvent::Wheel:
        case QEvent::TouchBegin:
            bump();
            break;
        default:
            break;
        }
        switch (event->type()) {
        case QEvent::KeyPress: {
            const auto *key = static_cast<QKeyEvent *>(event);
            setMode(key->nativeScanCode() == kGamepadScanCode ? "gamepad" : "keyboard");
            break;
        }
        case QEvent::MouseButtonPress:
        case QEvent::Wheel:
        case QEvent::TouchBegin:
            setMode("keyboard");
            break;
        case QEvent::MouseMove: {
            // Ignore tiny or synthetic moves (e.g. content scrolling under a
            // resting cursor); require a deliberate movement.
            const auto *mouse = static_cast<QMouseEvent *>(event);
            const QPointF pos = mouse->globalPosition();
            if (m_hasLastPos && (pos - m_lastPos).manhattanLength() > 12.0) {
                setMode("keyboard");
                bump();
            }
            m_lastPos = pos;
            m_hasLastPos = true;
            break;
        }
        default:
            break;
        }
        return false;
    }

private:
    // Any deliberate input: lets QML track inactivity (immersive Now Playing).
    void bump()
    {
        if (m_target)
            m_target->setProperty("inputSerial", ++m_serial);
    }

    void setMode(const char *mode)
    {
        if (m_mode == mode || !m_target)
            return;
        m_mode = mode;
        m_target->setProperty("inputMode", QString::fromLatin1(mode));
    }

    QPointer<QObject> m_target;
    const char *m_mode = nullptr;
    int m_serial = 0;
    QPointF m_lastPos;
    bool m_hasLastPos = false;
};

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
                      kGamepadScanCode, 0, 0, QString(), autorepeat));
}

void bigifyInstallInputModeFilter(QObject &target)
{
    if (auto *app = QGuiApplication::instance())
        app->installEventFilter(new InputModeFilter(&target));
}
