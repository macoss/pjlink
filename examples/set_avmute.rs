// Copyright 2018 Rick Russell
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

extern crate pjlink;

use pjlink::{AvMute, PjlinkDevice};
use std::env;

static USAGE: &'static str = "[host][video mute (true, false)][audio mute (true, false)][password]";

fn main() {
    let my_name = env::args().nth(0).unwrap();

    let host = match env::args().nth(1) {
        Some(hst) => hst,
        None => {
            panic!("Usage: {} {}", my_name, USAGE);
        }
    };

    let password = match env::args().nth(4) {
        Some(pwd) => pwd,
        None => String::from(""),
    };

    let device: PjlinkDevice = if password != "" {
        PjlinkDevice::new_with_password(&host, &password).unwrap()
    } else {
        PjlinkDevice::new(&host).unwrap()
    };

    let mutes = AvMute {
        video: match env::args().nth(2) {
            Some(arg) => arg.to_lowercase() == "true",
            None => panic!("Usage: {} {}", my_name, USAGE),
        },
        audio: match env::args().nth(3) {
            Some(arg) => arg.to_lowercase() == "true",
            None => panic!("Usage: {} {}", my_name, USAGE),
        },
    };

    match device.set_avmute(mutes) {
        Ok(mutes) => println!(
            "{} Video Mute: {} Audio Mute: {}",
            host, mutes.video, mutes.audio
        ),
        Err(err) => println!("An error occurred: {}", err),
    }
}
