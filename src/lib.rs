// Copyright 2018 Rick Russell
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::io::prelude::*;
use std::io::{Error, ErrorKind};
use std::net::TcpStream;

extern crate md5;

const AUTH: char = '1';
const NOAUTH: char = '0';
static PORT: &'static str = "4352";

// Return the correct error message based on the PJ Link specification
fn pjlink_error(error_msg: &str) -> Error {
    match &error_msg[0..4] {
        "ERR1" => Error::new(ErrorKind::InvalidData, "Undefined command".to_string()),
        "ERR2" => Error::new(ErrorKind::InvalidData, "Invalid parameter".to_string()),
        "ERR3" => Error::new(
            ErrorKind::InvalidData,
            "Unavailable at this time".to_string(),
        ),
        "ERR4" => Error::new(
            ErrorKind::InvalidData,
            "Projector/Display Failure".to_string(),
        ),
        "ERRA" => Error::new(
            ErrorKind::PermissionDenied,
            "Authorization Error".to_string(),
        ),
        _ => Error::new(
            ErrorKind::InvalidData,
            format!("Error reported from the projector {}", error_msg),
        ),
    }
}

// Parse the response from the device
fn parse_response(response: &str) -> Result<PjlinkResponse, Error> {
    let mut equals_sign: usize = 0;
    let len = response.len();
    //lets find the equals sign
    for (i, c) in response.chars().enumerate() {
        if c == '=' || c == ' ' {
            equals_sign = i;
            break;
        }
    }

    let command = if &response[0..1] != "%" {
        CommandType::PJLINK
    } else {
        match &response[2..equals_sign] {
            "POWR" => CommandType::Power,
            "INPT" => CommandType::Input,
            "AVMT" => CommandType::AvMute,
            "ERST" => CommandType::ErrorStatus,
            "LAMP" => CommandType::Lamp,
            "INST" => CommandType::InputList,
            "NAME" => CommandType::Name,
            "INF1" => CommandType::Manufacturer,
            "INF2" => CommandType::ProductName,
            "INFO" => CommandType::Information,
            "CLSS" => CommandType::Class,
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Invalid command type returned.",
                ));
            }
        }
    };

    let value = &response[equals_sign + 1..len];

    // Did we get and error report and if so lets return it so the functions don't have check for errors.
    if value.len() == 4 && &value[0..3] == "ERR" {
        return Err(pjlink_error(value));
    }

    Ok(PjlinkResponse {
        action: command,
        value: value.to_string(),
    })
}

// This is the list of standard command/response types from the PJLink spec.
// At this point I would think that this would only be used internally.
enum CommandType {
    PJLINK,
    Power,
    Input,
    AvMute,
    ErrorStatus,
    Lamp,
    InputList,
    Name,
    Manufacturer,
    ProductName,
    Information,
    Class,
}

/// Power status is based off of the PJLink specification and is used to be returned
pub enum PowerStatus {
    Off,
    On,
    Cooling,
    Warmup,
}

pub enum InputType {
    RGB(u8),
    Video(u8),
    Digital(u8),
    Storage(u8),
    Network(u8),
}

pub enum ErrorType {
    NoError,
    Warning,
    Error,
}

pub struct AvMute {
    pub audio: bool,
    pub video: bool,
}

pub struct Lamp {
    pub hours: u16,
    pub on: bool,
}

pub struct ErrorStatus {
    pub fan_error: ErrorType,
    pub lamp_error: ErrorType,
    pub temperature_error: ErrorType,
    pub cover_open_error: ErrorType,
    pub filter_error: ErrorType,
    pub other_error: ErrorType,
}

struct PjlinkResponse {
    action: CommandType,
    value: String,
}

pub struct PjlinkDevice {
    pub host: String,
    password: String,
    //managed: bool, // Currently not implemented but will add managed monitoring support with call backs with the status changes
    //monitored: bool, // Currenly not implemented by will allow you to monitor a device with out mainting authority over it.
}

impl PjlinkDevice {
    /// Constructs a new PjlinkDevice.
    pub fn new(host: &str) -> Result<PjlinkDevice, Error> {
        let pwd = String::from("");
        PjlinkDevice::new_with_password(host, &pwd)
    }

    /// Contructs a new PjlinkDevice that has a password
    pub fn new_with_password(host: &str, password: &str) -> Result<PjlinkDevice, Error> {
        Ok(PjlinkDevice {
            host: host.to_string(),
            password: String::from(password),
            //managed: false, // Hard coded for now until it is implemented
            //monitored: false, // Hard coded for now until it is implemented
        })
    }

    /// Send a command and a Result with the raw string or an error
    pub fn send_command(&self, command: &str) -> Result<String, Error> {
        let host_port = [&self.host, ":", PORT].concat();
        let mut client_buffer = [0u8; 256];
        let mut stream = try!(TcpStream::connect(host_port));

        let _ = stream.read(&mut client_buffer); //Did we get the hello string?

        let cmd: String = match client_buffer[7] as char {
            // Does the connection require auth or not
            AUTH => {
                // Connection requires auth
                let rnd_num = String::from_utf8_lossy(&client_buffer[9..17]).to_string();
                if &self.password != "" {
                    // We got a password
                    let pwd_str = format!("{}{}", rnd_num, &self.password);
                    let digest = md5::compute(pwd_str);
                    format!("{:x}%1{}\r", digest, command)
                } else {
                    // No password was supplied so we are going to raise an error.
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "This device requires a password and one was not supplied.",
                    ));
                }
            }
            NOAUTH => {
                // Connection requires no auth
                format!("%1{}\r", command)
            }

            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Invalid response or is not a PJLink device",
                ));
            }
        };

        let result = stream.write(cmd.as_bytes());
        match result {
            Ok(_) => (),
            Err(e) => return Err(e),
        };
        let result = stream.read(&mut client_buffer);
        let len = match result {
            Ok(len) => len,
            Err(e) => return Err(e),
        };

        let response = String::from_utf8_lossy(&client_buffer[0..len - 1]).to_string();
        Ok(response)
    }

    // a wrapper around send_command that will parse the response
    fn send(&self, cmd: &str) -> Result<PjlinkResponse, Error> {
        match self.send_command(cmd) {
            Ok(send_result) => match parse_response(&send_result) {
                Ok(parse_result) => Ok(parse_result),
                Err(e) => Err(e),
            },
            Err(e) => Err(e),
        }
    }

    /// Check the power status of the device and returns an enum
    pub fn get_power_status(&self) -> Result<PowerStatus, Error> {
        match self.send("POWR ?") {
            Ok(result) => {
                match result.action {
                    CommandType::Power => {
                        match &result.value[0..1] {
                            "0" => Ok(PowerStatus::Off),
                            "1" => Ok(PowerStatus::On),
                            "2" => Ok(PowerStatus::Cooling),
                            "3" => Ok(PowerStatus::Warmup),
                            _ => Err(Error::new(
                                ErrorKind::InvalidInput,
                                format!("Invalid Response: {}", result.value),
                            )), // Invalid Response
                        }
                    }
                    _ => Err(Error::new(
                        ErrorKind::InvalidInput,
                        format!("Got a response we didn't expect: {}", result.value),
                    )),
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Turn on the device and will return a Result enum with
    /// Ok being a [pjlink::PowerStatus](enum.PowerStatus.html) or Err being a std::io::Error
    ///
    pub fn power_on(&self) -> Result<PowerStatus, Error> {
        match self.send("POWR 1") {
            Ok(result) => {
                match result.action {
                    CommandType::Power => {
                        match &result.value[0..2] {
                            "OK" => match self.get_power_status() {
                                Ok(status) => Ok(status),
                                Err(e) => Err(e),
                            },
                            _ => Err(Error::new(
                                ErrorKind::InvalidInput,
                                format!("Invalid Response: {}", result.value),
                            )), // Invalid Response
                        }
                    }
                    _ => Err(Error::new(
                        ErrorKind::InvalidInput,
                        format!("Got a response we didn't expect: {}", result.value),
                    )),
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Turn off the device and will return a Result enum with
    /// Ok being a [pjlink::PowerStatus](enum.PowerStatus.html) or Err being a std::io::Error
    ///
    pub fn power_off(&self) -> Result<PowerStatus, Error> {
        match self.send("POWR 0") {
            Ok(result) => {
                match result.action {
                    CommandType::Power => {
                        match &result.value[0..2] {
                            "OK" => match self.get_power_status() {
                                Ok(status) => Ok(status),
                                Err(e) => Err(e),
                            },
                            _ => Err(Error::new(
                                ErrorKind::InvalidInput,
                                format!("Invalid Response: {}", result.value),
                            )), // Invalid Response
                        }
                    }
                    _ => Err(Error::new(
                        ErrorKind::InvalidInput,
                        format!("Got a response we didn't expect: {}", result.value),
                    )),
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Get the information (INFO ?) from theand returns a
    /// string with the information or a std::io::Error
    ///
    pub fn get_info(&self) -> Result<String, Error> {
        match self.send("INFO ?") {
            Ok(result) => match result.action {
                CommandType::Information => Ok(result.value),
                _ => Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("Invalid Response:: {}", result.value),
                )),
            },
            Err(e) => Err(e),
        }
    }

    /// Get the manufacturer (INF1 ?) from the deviceand returns a
    /// string with the information or a std::io::Error
    ///
    pub fn get_manufacturer(&self) -> Result<String, Error> {
        match self.send("INF1 ?") {
            Ok(result) => match result.action {
                CommandType::Manufacturer => Ok(result.value),
                _ => Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("Invalid Response:: {}", result.value),
                )),
            },
            Err(e) => Err(e),
        }
    }

    /// Get the product name (INF2 ?) from the deviceand returns a
    /// string with the information or a std::io::Error
    ///
    pub fn get_product_name(&self) -> Result<String, Error> {
        match self.send("INF2 ?") {
            Ok(result) => match result.action {
                CommandType::ProductName => Ok(result.value),
                _ => Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("Invalid Response:: {}", result.value),
                )),
            },
            Err(e) => Err(e),
        }
    }
    /// Get the product class (CLSS ?) from the deviceand returns a
    /// string with the information or a std::io::Error
    ///
    pub fn get_class(&self) -> Result<String, Error> {
        match self.send("CLSS ?") {
            Ok(result) => match result.action {
                CommandType::Class => Ok(result.value),
                _ => Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("Invalid Response:: {}", result.value),
                )),
            },
            Err(e) => Err(e),
        }
    }

    /// Get the device name (NAME ?) from the device and returns a
    /// string with the information or a std::io::Error
    ///
    pub fn get_device_name(&self) -> Result<String, Error> {
        match self.send("NAME ?") {
            Ok(result) => match result.action {
                CommandType::Name => Ok(result.value),
                _ => Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("Invalid Response:: {}", result.value),
                )),
            },
            Err(e) => Err(e),
        }
    }

    /// Get the current input (INPT ?) from the device
    /// Returns a Result enum with an Ok type of [pjlink::InputType](enum.InputType.html) example would be:
    /// ```
    /// pjlink::InputType::RGB(input_num) //with input_num being the number of the input with a type of u8
    ///
    /// ```
    ///
    pub fn get_input(&self) -> Result<InputType, Error> {
        match self.send("INPT ?") {
            Ok(result) => {
                let input = result.value.parse::<u8>().unwrap();
                match input {
                    11...19 => Ok(InputType::RGB(input - 10)),
                    21...29 => Ok(InputType::Video(input - 20)),
                    31...39 => Ok(InputType::Digital(input - 30)),
                    41...49 => Ok(InputType::Storage(input - 40)),
                    51...59 => Ok(InputType::Network(input - 50)),
                    _ => Err(Error::new(
                        ErrorKind::InvalidInput,
                        format!("Invalid input:: {}", input),
                    )),
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Change the current input (INPT 31 for) on the device
    /// Returns a result enum with Ok type of [pjlink::InputType](enum.InputType.html) with a value associated
    ///  of the input number or an std::io::Error
    ///
    /// ```
    /// let result = pjlink::PjlinkDevice::set_input(&self, input: InputType).?
    /// match device.get_input() {
    ///    Ok(input) => {
    ///        match input {
    ///            InputType::RGB(input_number) => println!("Input: RGB {}", input_number),
    ///            InputType::Video(input_number) => println!("Input: Video {}", input_number),
    ///            InputType::Digital(input_number) => println!("Input: Digital {}", input_number),
    ///            InputType::Storage(input_number) => println!("Input: Storage {}", input_number),
    ///            InputType::Network(input_number) => println!("Input: Network {}", input_number),
    ///        }
    ///    },
    ///    Err(err) => println!("An error occurred: {}", err),
    /// }
    /// ```
    ///
    pub fn set_input(&self, input: InputType) -> Result<InputType, Error> {
        let input_number: u8 = match input {
            InputType::RGB(i_num) => i_num + 10,
            InputType::Video(i_num) => i_num + 20,
            InputType::Digital(i_num) => i_num + 30,
            InputType::Storage(i_num) => i_num + 40,
            InputType::Network(i_num) => i_num + 50,
        };

        let command = format!("INPT {}", input_number);
        match self.send(&command) {
            Ok(result) => {
                match result.action {
                    CommandType::Input => {
                        match &result.value[0..2] {
                            "OK" => match self.get_input() {
                                Ok(status) => Ok(status),
                                Err(e) => Err(e),
                            },
                            _ => Err(Error::new(
                                ErrorKind::InvalidInput,
                                format!("Invalid Response: {}", result.value),
                            )), // Invalid Response
                        }
                    }
                    _ => Err(Error::new(
                        ErrorKind::InvalidInput,
                        format!("Got a response we didn't expect: {}", result.value),
                    )),
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Get the current Av Mute (AVMT ?) from the device
    /// Returns a Result enum with an Ok type of [pjlink::AvMute](struct.AvMute.html) example would be:
    /// ```
    /// pjlink::AvMute::Audio or Video //with Audio and Video being a bool with the status.
    ///
    /// ```
    ///
    pub fn get_avmute(&self) -> Result<AvMute, Error> {
        match self.send("AVMT ?") {
            Ok(result) => {
                let status = result.value.parse::<u8>().unwrap();
                match status {
                    11 => Ok(AvMute {
                        audio: false,
                        video: true,
                    }),
                    21 => Ok(AvMute {
                        audio: true,
                        video: false,
                    }),
                    31 => Ok(AvMute {
                        audio: true,
                        video: true,
                    }),
                    30 => Ok(AvMute {
                        audio: false,
                        video: false,
                    }),
                    _ => Err(Error::new(
                        ErrorKind::InvalidInput,
                        format!("Invalid result:: {}", status),
                    )),
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Set the AV Mute (AVMT 30) on the current device
    /// Returns a Result enum with an Ok type of [pjlink::AvMute](struct.AvMute.html) example would be:
    /// ```
    /// let mutes = AvMute {
    ///     video: true,
    ///     audio: true,
    /// }
    ///
    /// match device.set_avmute(mutes) {
    ///     Ok(mutes) => println!(
    ///         "{} Video Mute: {} Audio Mute: {}",
    ///         host, mutes.video, mutes.audio
    ///     ),
    ///     Err(err) => println!("An error occurred: {}", err),
    /// }
    ///
    /// ```
    ///
    pub fn set_avmute(&self, mute_status: AvMute) -> Result<AvMute, Error> {
        let mutes: u8 = match mute_status {
            AvMute {
                video: true,
                audio: false,
            } => 11,
            AvMute {
                video: false,
                audio: true,
            } => 21,
            AvMute {
                video: true,
                audio: true,
            } => 31,
            _ => 30,
        };

        let command = format!("AVMT {}", mutes);
        match self.send(&command) {
            Ok(result) => {
                match result.action {
                    CommandType::AvMute => {
                        match &result.value[0..2] {
                            "OK" => match self.get_avmute() {
                                Ok(status) => Ok(status),
                                Err(e) => Err(e),
                            },
                            _ => Err(Error::new(
                                ErrorKind::InvalidInput,
                                format!("Invalid Response: {}", result.value),
                            )), // Invalid Response
                        }
                    }
                    _ => Err(Error::new(
                        ErrorKind::InvalidInput,
                        format!("Got a response we didn't expect: {}", result.value),
                    )),
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Get the current lamp status (LAMP ?) from the device
    /// Returns a Result enum with an Ok vector of [pjlink::Lamp](struct.Lamp.html) example would be:
    /// ```
    /// pjlink::Lamp::hours and on  //with hours being the total hours on that lamp
    /// and "on" being a bool with the status of the lamp.
    ///
    /// ```
    ///
    pub fn get_lamp(&self) -> Result<Vec<Lamp>, Error> {
        match self.send("LAMP ?") {
            Ok(result) => {
                let mut status = result.value.split_whitespace();
                let mut lamps = Vec::new();
                while let Some(l) = status.next() {
                    let hours = l.parse::<u16>().unwrap();

                    let on = match status.next() {
                        Some(x) => x == "1",
                        None => false,
                    };
                    lamps.push(Lamp {
                        hours: hours,
                        on: on,
                    });
                }
                Ok(lamps)
            }
            Err(e) => Err(e),
        }
    }

    /// Get the current error status of the device (ERST ?)
    /// Returns a Result enum with an Ok being a [pjlink::ErrorStatus](struct.Lamp.html) example would be:
    /// ```
    /// match device.get_error_status() {
    ///    Ok(error_status) => {
    ///        match error_status.fan_error {
    ///            ErrorType::Warning => println!("{} Error Status: Fan Warning", host),
    ///            ErrorType::Error => println!("{} Error Status: Fan Error", host),
    ///            _ => (),
    ///        }
    ///        match error_status.lamp_error {
    ///            ErrorType::Warning => println!("{} Error Status: Lamp Warning", host),
    ///            ErrorType::Error => println!("{} Error Status: Lamp Error", host),
    ///            _ => (),
    ///        }
    ///        match error_status.temperature_error {
    ///            ErrorType::Warning => println!("{} Error Status: Temperature Warning", host),
    ///            ErrorType::Error => println!("{} Error Status: Temperature Error", host),
    ///            _ => (),
    ///        }
    ///        match error_status.cover_open_error {
    ///            ErrorType::Warning => println!("{} Error Status: Cover Open Warning", host),
    ///            ErrorType::Error => println!("{} Error Status: Cover Open Error", host),
    ///            _ => (),
    ///        }
    ///        match error_status.filter_error {
    ///            ErrorType::Warning => println!("{} Error Status: Filter Warning", host),
    ///            ErrorType::Error => println!("{} Error Status: Filter Error", host),
    ///            _ => (),
    ///        }
    ///        match error_status.other_error {
    ///            ErrorType::Warning => println!("{} Error Status: Other Warning", host),
    ///            ErrorType::Error => println!("{} Error Status: Other Error", host),
    ///            _ => (),
    ///        }
    ///    }
    ///    Err(err) => println!("{} Error Status: error occurred: {}", host, err),
    /// }
    ///
    /// ```
    ///
    pub fn get_error_status(&self) -> Result<ErrorStatus, Error> {
        match self.send("ERST ?") {
            Ok(result) => {
                let mut status = result.value.chars();

                Ok(ErrorStatus {
                    fan_error: match status.next() {
                        Some(e) => match e {
                            '0' => ErrorType::NoError,
                            '1' => ErrorType::Warning,
                            '2' => ErrorType::Error,
                            _ => ErrorType::NoError,
                        },
                        None => ErrorType::NoError,
                    },
                    lamp_error: match status.next() {
                        Some(e) => match e {
                            '0' => ErrorType::NoError,
                            '1' => ErrorType::Warning,
                            '2' => ErrorType::Error,
                            _ => ErrorType::NoError,
                        },
                        None => ErrorType::NoError,
                    },
                    temperature_error: match status.next() {
                        Some(e) => match e {
                            '0' => ErrorType::NoError,
                            '1' => ErrorType::Warning,
                            '2' => ErrorType::Error,
                            _ => ErrorType::NoError,
                        },
                        None => ErrorType::NoError,
                    },
                    cover_open_error: match status.next() {
                        Some(e) => match e {
                            '0' => ErrorType::NoError,
                            '1' => ErrorType::Warning,
                            '2' => ErrorType::Error,
                            _ => ErrorType::NoError,
                        },
                        None => ErrorType::NoError,
                    },
                    filter_error: match status.next() {
                        Some(e) => match e {
                            '0' => ErrorType::NoError,
                            '1' => ErrorType::Warning,
                            '2' => ErrorType::Error,
                            _ => ErrorType::NoError,
                        },
                        None => ErrorType::NoError,
                    },
                    other_error: match status.next() {
                        Some(e) => match e {
                            '0' => ErrorType::NoError,
                            '1' => ErrorType::Warning,
                            '2' => ErrorType::Error,
                            _ => ErrorType::NoError,
                        },
                        None => ErrorType::NoError,
                    },
                })
            }
            Err(e) => Err(e),
        }
    }
}
