use super::*;

pub enum Msg {
    Playing(Option<u64>),
    Storage(Result<Box<crate::storage::Report>, String>),
    Releases(Result<Vec<Release>, String>),
    Progress(String, f32),
    Installed(String, Result<(), String>),
    /// A version's files are gone (or could not be deleted).
    Removed(String, Result<(), String>),
    /// `prune_old_versions` ended; these versions were deleted.
    Pruned(Vec<String>),
    /// What each installed version takes on disk (`disk_use`).
    DiskUse(Vec<(String, u64)>),
    Line(Job, String),
    Exit(Job, Result<(), String>),
    /// `reuse_prepared_everywhere` prepared these versions.
    Reused(Vec<(String, String)>),
    /// The game's assets were checked and the game can start on the main thread.
    SessionDone,
    JarConsent {
        jars: Vec<horizon_platform::mod_package::manifest::JarRef>,
        reply: std::sync::mpsc::Sender<bool>,
    },
    JarClient {
        run: Option<String>,
        prepared: Box<Result<Command, String>>,
    },
    /// A modpack step's line (`modpacks`).
    PackLine(String),
    /// A modpack step ended: the pack as it now is.
    PackDone(Result<Box<crate::modpacks::Imported>, String>),
    /// The modpacks other launchers keep on this computer (`modpacks::found_on_this_computer`).
    FoundPacks(Vec<crate::other_launchers::Found>),
    /// Where the native-mods build is (`native_mods::Step::label`), and how far along.
    NativeProgress(String, crate::native_mods::Progress),
    NativeRegistryConsent(
        crate::native_mods::RegistryInventory,
        std::sync::mpsc::Sender<bool>,
    ),
    /// The native-mods build ended: the version built, or why not.
    Native(Result<String, crate::native_mods::Failed>),
    /// An imported game build is installed (`build_import::import`): its version, or why not.
    Imported(Result<String, String>),
    /// A Java instance's game is ready to start (`java_game`, wrapped): the instance's id and
    /// game folder, and the command or why not.
    /// A Java instance's game ready to start, and its variables for the post-exit command.
    JavaGame(
        String,
        PathBuf,
        Result<(Command, crate::launch::Vars), String>,
    ),
}
