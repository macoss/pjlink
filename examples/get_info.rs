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

use pjlink::{InputType, PjlinkDevice, PowerStatus};
use std::env;

fn main() {
    let host = match env::args().nth(1) {
        Some(hst) => hst,
        None => {
            let my_name = env::args().nth(0).unwrap();
            panic!("Usage: {} [host][password]", my_name)
        }
    };

    let password = match env::args().nth(2) {
        Some(pwd) => pwd,
        None => String::from(""),
    };

    let device: PjlinkDevice = if password != "" {
        PjlinkDevice::new_with_password(&host, &password).unwrap()
    } else {
        PjlinkDevice::new(&host).unwrap()
    };

    match device.get_device_name() {
        Ok(response) => println!("{} Device Name: {}", host, response),
        Err(err) => println!("{} Device: error occurred: {}", host, err),
    }

    match device.get_manufacturer() {
        Ok(response) => println!("{} Manufacturer: {}", host, response),
        Err(err) => println!("{} Manugacturer: error occurred: {}", host, err),
    }

    match device.get_product_name() {
        Ok(response) => println!("{} Product: {}", host, response),
        Err(err) => println!("{} Product: error occurred: {}", host, err),
    }

    match device.get_info() {
        Ok(response) => println!("{} Infomation: {}", host, response),
        Err(err) => println!("{} Info: error occurred: {}", host, err),
    }

    match device.get_class() {
        Ok(response) => println!("{} Class: {}", host, response),
        Err(err) => println!("{} Class: error occurred: {}", host, err),
    }

    match device.get_power_status() {
        Ok(response) => match response {
            PowerStatus::Off => println!("{} Power: off", host),
            PowerStatus::On => println!("{} Power: on", host),
            PowerStatus::Cooling => println!("{} Power: cooling", host),
            PowerStatus::Warmup => println!("{} Power: warming up", host),
        },
        Err(err) => println!("{} Power: error occurred: {}", host, err),
    }

    match device.get_input() {
        Ok(input) => match input {
            InputType::RGB(input_number) => println!("{} Input: RGB {}", host, input_number),
            InputType::Video(input_number) => println!("{} Input: Video {}", host, input_number),
            InputType::Digital(input_number) => {
                println!("{} Input: Digital {}", host, input_number)
            }
            InputType::Storage(input_number) => {
                println!("{} Input: Storage {}", host, input_number)
            }
            InputType::Network(input_number) => {
                println!("{} Input: Network {}", host, input_number)
            }
        },
        Err(err) => println!("{} Input: error occurred: {}", host, err),
    }

    match device.get_avmute() {
        Ok(response) => println!(
            "{} Video Mute: {} Audio Mute: {}",
            host, response.video, response.audio
        ),
        Err(err) => println!("{} AvMute: error occurred: {}", host, err),
    }
}
