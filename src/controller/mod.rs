use self::grid::Rect;
use anyhow::Result;

mod grid;

pub use self::grid::{Direction, NavigationController, NavigationDirective, NavigationResult};

// ╔═════════╦════════════════╦═════════╦══════════╦══╦══╦══╦══╦══╦══╗
// ║ Games   ║ RecentlyPlayed ║         ║ Settings ║  ║  ║  ║  ║  ║  ║
// ╠═════════╬════════════════╬═════════╬══════════╬══╬══╬══╬══╬══╬══╣
// ║ S_Games ║ S_Games        ║ S_Games ║ S_Games  ║  ║  ║  ║  ║  ║  ║
// ╠═════════╬════════════════╬═════════╬══════════╬══╬══╬══╬══╬══╬══╣
// ║ S_Games ║ S_Games        ║ S_Games ║ S_Games  ║  ║  ║  ║  ║  ║  ║
// ╠═════════╬════════════════╬═════════╬══════════╬══╬══╬══╬══╬══╬══╣
// ║ S_Games ║ S_Games        ║ S_Games ║ S_Games  ║  ║  ║  ║  ║  ║  ║
// ╠═════════╬════════════════╬═════════╬══════════╬══╬══╬══╬══╬══╬══╣
// ║ S_Games ║ S_Games        ║ S_Games ║ S_Games  ║  ║  ║  ║  ║  ║  ║
// ╠═════════╬════════════════╬═════════╬══════════╬══╬══╬══╬══╬══╬══╣
// ║ S_Games ║ S_Games        ║ S_Games ║ S_Games  ║  ║  ║  ║  ║  ║  ║
// ╠═════════╬════════════════╬═════════╬══════════╬══╬══╬══╬══╬══╬══╣
// ║         ║                ║         ║          ║  ║  ║  ║  ║  ║  ║
// ╠═════════╬════════════════╬═════════╬══════════╬══╬══╬══╬══╬══╬══╣
// ║         ║                ║         ║          ║  ║  ║  ║  ║  ║  ║
// ╠═════════╬════════════════╬═════════╬══════════╬══╬══╬══╬══╬══╬══╣
// ║         ║                ║         ║          ║  ║  ║  ║  ║  ║  ║
// ╠═════════╬════════════════╬═════════╬══════════╬══╬══╬══╬══╬══╬══╣
// ║         ║                ║         ║          ║  ║  ║  ║  ║  ║  ║
// ╚═════════╩════════════════╩═════════╩══════════╩══╩══╩══╩══╩══╩══╝

pub fn create_home_window_controller() -> Result<NavigationController> {
    let mut builder = grid::LayoutGridBuilder::new(4, 6, "Home".to_owned());
    builder
        .add_element(Rect::new(0, 0, 0, 0)?, "BTN@GAMES".to_owned())?
        .add_element(Rect::new(1, 1, 0, 0)?, "BTN@RECENTLY_PLAYED".to_owned())?
        .add_element(Rect::new(3, 3, 0, 0)?, "BTN@SETTINGS".to_owned())?;
    let sub = builder.with_sublayout(Rect::new(0, 3, 1, 5)?, "Home@Games".to_owned(), 7, 10);
    sub.set_growable(1, 1, grid::GrowDirection::GrowX)?;
    let controller = grid::NavigationController::new(builder.build()?);
    controller
}
