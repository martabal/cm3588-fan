use std::{thread, time::Duration};

use log::debug;

use cm3588_fan::cheker::Checker;

fn main() {
    let mut checker = Checker::new();

    loop {
        checker.adjust_speed();
        debug!("Sleeping for {} seconds", checker.config.sleep_time);

        thread::sleep(Duration::from_secs(checker.config.sleep_time));
    }
}
