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

use std::io::prelude::*;
use std::net::TcpStream;
use std::io::{Error,ErrorKind};

extern crate md5;

const AUTH: char = '1';
const NOAUTH: char = '0';
static PORT: &'static str = "4352";

// Return the correct error message based on the PJ Link specification
fn pjlink_error(error_msg: &str) -> Error {
    match &error_msg[7..11] {
        "ERR1" => Error::new(ErrorKind::InvalidData, format!("Undefined command")),
        "ERR2" => Error::new(ErrorKind::InvalidData, format!("Invalid parameter")),
        "ERR3" => Error::new(ErrorKind::InvalidData, format!("Unavaiable at this time")),
        "ERR4" => Error::new(ErrorKind::InvalidData, format!("Projector/Display Failure")),
        "ERRA" => Error::new(ErrorKind::PermissionDenied, format!("Authorization Error")),
        _ => Error::new(ErrorKind::InvalidData, format!("Error reported from the projector {}", error_msg)),
    }
}

/// Power status definitions
pub enum PowerStatus {
    Off,
    On,
    Cooling,
    Warmup,
}

pub struct PjlinkDevice {
    
    host: String,
    password: String,
    managed: bool, // Currently not implemented but will add monitoring support with call backs with the status changes
}

impl PjlinkDevice {
    /// Constructs a new PjlinkDevice.
    pub fn new(host: &str) -> Result<PjlinkDevice,Box<Error>> {
        let pwd = String::from("");
        PjlinkDevice::new_with_password(host, &pwd)
    }

    /// Contructs a new PjlinkDevice that has a password
    pub fn new_with_password(host: &str, password: &str) -> Result<PjlinkDevice,Box<Error>> {
        Ok(PjlinkDevice {
            host: host.to_string(),
            password: String::from(password),
            managed: false, // Hard coded to start until implemented.
        })
    }

    /// Send a command and a Result with the raw string or an error
    pub fn send_command(&self, command: &str) -> Result<String, Error> {
        let host_port = [&self.host, ":", PORT].concat();
        let mut client_buffer = [0u8; 256];
        let mut stream = try!(TcpStream::connect(host_port));

        let _ = stream.read(&mut client_buffer); //Did we get the hello string?

        let cmd: String = match client_buffer[7] as char { // Does the connection require auth or not
            AUTH => { // Connection requires auth
                let rnd_num = String::from_utf8_lossy(&client_buffer[9..17]).to_string();
                if &self.password != "" { // We got a password
                    let pwd_str = format!("{}{}", rnd_num, &self.password);
                    let digest = md5::compute(pwd_str);
                    format!("{:x}%1{}\r", digest, command)
                } else { // No password was supplied so we are going to raise an error.
                    return Err(Error::new(ErrorKind::InvalidInput, "This device requires a password and one was not supplied."))
                }
            },
            NOAUTH => { // Connection requires no auth
                format!("%1{}\r", command)
            },

            _ => return Err(Error::new(ErrorKind::InvalidInput, "Invalid response or is not a PJLink device"))
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

        let response = String::from_utf8_lossy(&client_buffer[0..len]).to_string();   
        Ok(response)
    }

    /// Check the power status of the device and returns an enum 
    pub fn get_power_status(&self) -> Result<PowerStatus, Error> {
        match self.send_command("POWR ?") {
            Ok(result) => {
                if &result[2..6] == "POWR" {
                    match &result[7..8] {
                        "0" => Ok(PowerStatus::Off),
                        "1" => Ok(PowerStatus::On),
                        "2" => Ok(PowerStatus::Cooling),
                        "3" => Ok(PowerStatus::Warmup),
                        "E" => Err(pjlink_error(&result)),
                        _ => Err(Error::new(ErrorKind::InvalidInput, format!("Invalid Response: {}", result))), // Invalid Response
                    }
                } else {
                    Err(Error::new(ErrorKind::InvalidInput, format!("Got a response we didn't expect: {}", result)))
                }
            },
            Err(e) => Err(e),
        }
    }

    /// Turn on the device
    pub fn power_on(&self) -> Result<PowerStatus, Error> {
        match self.send_command("POWR 1") {
            Ok(result) => {
                if &result[2..6] == "POWR" {
                    match &result[7..9] {
                        "OK" => {
                            match self.get_power_status() {
                                Ok(status) => Ok(status),
                                Err(e) => Err(e),
                            }
                        },
                        "ER" => Err(pjlink_error(&result)),
                        _ => Err(Error::new(ErrorKind::InvalidInput, format!("Invalid Response: {}", result))), // Invalid Response
                    }
                } else {
                    Err(Error::new(ErrorKind::InvalidInput, format!("Got a response we didn't expect: {}", result)))
                }
            },
            Err(e) => Err(e),            
        }
    }

    /// Turn off the device
    pub fn power_off(&self) -> Result<PowerStatus, Error> {
        match self.send_command("POWR 0") {
            Ok(result) => {
                if &result[2..6] == "POWR" {
                    match &result[7..9] {
                        "OK" => {
                            match self.get_power_status() {
                                Ok(status) => Ok(status),
                                Err(e) => Err(e),
                            }
                        },
                        "ER" => Err(pjlink_error(&result)),
                        _ => Err(Error::new(ErrorKind::InvalidInput, format!("Invalid Response: {}", result))), // Invalid Response
                    }
                } else {
                    Err(Error::new(ErrorKind::InvalidInput, format!("Got a response we didn't expect: {}", result)))
                }
            },
            Err(e) => Err(e),            
        }
    }
}
