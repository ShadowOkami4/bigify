use cxx_qt_build::{CxxQtBuilder, QmlFile, QmlModule};

const QML_FILES: &[&str] = &[
    "qml/Main.qml",
    "qml/components/Artwork.qml",
    "qml/components/Chip.qml",
    "qml/components/Equalizer.qml",
    "qml/components/FocusRing.qml",
    "qml/components/Icon.qml",
    "qml/components/IconButton.qml",
    "qml/components/LoadingIndicator.qml",
    "qml/components/MediaCard.qml",
    "qml/components/MiniPlayer.qml",
    "qml/components/NavRail.qml",
    "qml/components/OnScreenKeyboard.qml",
    "qml/components/PillButton.qml",
    "qml/components/PlayPauseButton.qml",
    "qml/components/PositionClock.qml",
    "qml/components/SettingRow.qml",
    "qml/components/Shelf.qml",
    "qml/components/StatePanel.qml",
    "qml/components/Toast.qml",
    "qml/components/TrackRow.qml",
    "qml/components/WavyProgress.qml",
    "qml/screens/ArtistScreen.qml",
    "qml/screens/CollectionScreen.qml",
    "qml/screens/HomeScreen.qml",
    "qml/screens/LibraryScreen.qml",
    "qml/screens/NowPlayingScreen.qml",
    "qml/screens/QueuePanel.qml",
    "qml/screens/SearchScreen.qml",
    "qml/screens/SettingsScreen.qml",
    "qml/screens/SignInScreen.qml",
];

const QML_SINGLETONS: &[&str] = &["qml/theme/Theme.qml", "qml/PreviewData.qml"];

fn main() {
    let module = QmlModule::new("Bigify")
        .qml_files(QML_FILES.iter().copied())
        .qml_files(
            QML_SINGLETONS
                .iter()
                .map(|path| QmlFile::from(*path).singleton(true)),
        );

    CxxQtBuilder::new_qml_module(module)
        .qt_module("Network")
        .qt_module("Quick")
        .files(["src/bridge.rs"])
        .cpp_file("cpp/input.cpp")
        .build();

    println!("cargo:rerun-if-changed=cpp");
    println!("cargo:rerun-if-changed=qml");
}
