use slint::Image;


/// Metadata for games.
/// The SoT can be from sources like igdb.com

/// Image source, can be either a path on the fs, or a based64 encoded image.
enum ImageSource {

    FilePath(String),
    Base64(String),
}

struct GameMetadata {
    /// Title of the game.
    title: String,
    /// Description of the game.
    desc: Option<String>,
    /// Genres of the game, can be multiple.
    /// All lower case formatted.
    genres: Vec<String>,
    /// Release date.
    /// TZ unaware really.
    relase_date: Option<chrono::DateTime<chrono::Utc>>,
    /// Devs, publishers.
    developers: Vec<String>,
    publishers: Vec<String>,
    /// The actually platform
    platform: Option<String>,
    /// Links if any.
    links: Vec<String>,
    /// User defined tags.
    tags: Vec<String>,
    /// Cover art to display.
    cover_art: Option<ImageSource>,
    /// Bg art to display.
    bg_art: Option<ImageSource>,
    /// Playtime.
    playtime: Option<chrono::Duration>,
    /// Fav.
    favorate: bool,
    /// UUID. Required for all games, given by the application.
    uuid: Option<String>,
    /// Install source.
    install_source: Option<String>,
    /// Launch options.
    launch_options: Vec<String>,
}