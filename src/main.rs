slint::include_modules!();

use gilrs::{Button, Event, EventType, Gilrs};
use std::thread;


fn controller_loop(handle: slint::Weak<HomeWindow>) {
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
                EventType::ButtonPressed(Button::DPadDown, _) => handle.upgrade_in_event_loop(move|e| {
                    e.global::<HomeWindowFocus>().set_focused_id("SETTINGS".into());
                }).unwrap(),
                _ => (),
            }
        }
    }
}


fn main() -> Result<(), slint::PlatformError> {

    let ui = HomeWindow::new()?;

    let handle = ui.as_weak();
    thread::spawn(move||controller_loop(handle));

    ui.run()
}
