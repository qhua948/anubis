#![feature(assert_matches)]
slint::include_modules!();

use slint::Model;
use gilrs::{Button, Event, EventType, Gilrs};
use std::{sync::mpsc, thread};

mod controller;

fn controller_loop(tx: mpsc::Sender<Button>) {
    let mut gilrs = Gilrs::new().unwrap();
    for (_id, gamepad) in gilrs.gamepads() {
        println!("{} is {:?}", gamepad.name(), gamepad.power_info());
    }

    let mut active_gamepad = None;

    loop {
        // Examine new events
        while let Some(Event { id, event, time }) = gilrs.next_event() {
            println!("{:?} New event from {}: {:?}", time, id, event);
            active_gamepad = Some(id);
            match event {
                EventType::ButtonPressed(b, _) => {
                    tx.send(b).unwrap()
                } 
                _ => (),
            }
        }
    }
}

fn navigation_controller_thread(handle: slint::Weak<HomeWindow>, rx: mpsc::Receiver<Button>) {
    let mut controller = controller::create_home_window_controller().unwrap();
    // TODO: Refactor grid navigation for games.
    let sublayout = controller.get_sublayout_by_id("Home@Games").unwrap();
    {
        let binding = sublayout.upgrade().unwrap();
        let mut b = binding.lock().unwrap();
        b.insert_to_growable_grid("GAME@aaaa").unwrap();
        b.insert_to_growable_grid("GAME@bbbb").unwrap();
    }
    loop {
        match rx.recv() {
            Ok(b) => {
                match b {
                    Button::DPadUp => controller.navigate(
                        controller::NavigationDirective::Direction(controller::Direction::Up),
                    ),
                    Button::DPadDown => controller.navigate(
                        controller::NavigationDirective::Direction(controller::Direction::Down),
                    ),
                    Button::DPadLeft => controller.navigate(
                        controller::NavigationDirective::Direction(controller::Direction::Left),
                    ),
                    Button::DPadRight => controller.navigate(
                        controller::NavigationDirective::Direction(controller::Direction::Right),
                    ),
                    _ => Ok(controller::NavigationResult::NoNextItem),
                }
                .unwrap();
                match controller.get_current_focus_id() {
                    Some(ref f_id) => {
                        let f_id_clone = f_id.clone();
                        println!("fid {}", f_id);
                        handle
                            .upgrade_in_event_loop(move |e| {
                                e.global::<HomeWindowFocus>()
                                    .set_focused_id(f_id_clone.into());
                            })
                            .unwrap();
                    }
                    None => {},
                }
            }
            Err(_) => {} // TODO: Handle error.
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = HomeWindow::new()?;

    let mut game_tiles: Vec<GameData> = ui.global::<HomeWindowFocus>().get_games().iter().collect();

    game_tiles.push(GameData {
        title: "aaaa".into(),
        uuid: "aaaa".into(),
    });
    game_tiles.push(GameData {
        title: "bbbb".into(),
        uuid: "bbbb".into(),
    });

    let tiles_model = std::rc::Rc::new(slint::VecModel::from(game_tiles));
    ui.global::<HomeWindowFocus>().set_games(tiles_model.into());

    let (tx, rx) = mpsc::channel();

    let handle = ui.as_weak();
    thread::spawn(move || controller_loop(tx));
    thread::spawn(move || navigation_controller_thread(handle, rx));

    ui.run()
}
