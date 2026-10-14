pragma Singleton
import QtQuick
import Bigify

// Which items are saved in the user's library (followed, for artists).
// Only confirmed answers from Spotify are recorded; a toggle shows as
// pending until Spotify confirms it. Unknown items hide their save button
// rather than guessing.
Item {
    id: root
    visible: false

    property var states: ({})
    property var pending: ({})
    // Bumped on every change; bindings read it to re-evaluate.
    property int revision: 0
    signal confirmed(string uri, bool saved, string kind)

    function kindOf(uri) { return (uri || "").split(":")[1] || "" }
    function isKnown(uri) { revision; return uri !== "" && uri in states }
    function isSaved(uri) { revision; return states[uri] === true }
    function isPending(uri) { revision; return pending[uri] === true }

    function check(uris) {
        const wanted = uris.filter(u => u && !(u in states))
        if (wanted.length === 0) return
        if (Backend.designPreview) {
            for (const u of wanted) states[u] = false
            revision++
            return
        }
        Backend.checkSaved(JSON.stringify(wanted))
    }

    function toggle(uri) {
        if (!uri || pending[uri]) return
        const target = !isSaved(uri)
        if (Backend.designPreview) {
            states[uri] = target
            revision++
            confirmed(uri, target, kindOf(uri))
            return
        }
        pending[uri] = true
        revision++
        Backend.setSaved(uri, target)
    }

    Connections {
        target: Backend
        function onSavedChanged(uri, saved) {
            const wasPending = root.pending[uri] === true
            root.states[uri] = saved
            delete root.pending[uri]
            root.revision++
            if (wasPending) root.confirmed(uri, saved, root.kindOf(uri))
        }
        function onSaveFailed(uri, message) {
            delete root.pending[uri]
            root.revision++
        }
        // A new account must not inherit the previous account's answers.
        function onUserNameChanged() {
            root.states = {}
            root.pending = {}
            root.revision++
        }
    }
}
